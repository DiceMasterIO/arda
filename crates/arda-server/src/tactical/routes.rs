//! `/v1/tactical` handlers. Every image response carries a strong `ETag`,
//! `Cache-Control`, `X-Arda-Cache: hit|miss` and `Server-Timing`, and each
//! request logs one timing line to stderr.

use super::{Anchor, Encoded, Hit, RenderKey, DEFAULT_PPSQ, PPSQ_OPTIONS};
use crate::error::{ServerError, ServerResult};
use crate::routes::{blocking, params, parse, segments, Params, Segments};
use crate::AppState;
use arda_tactical::TacticalLayout;
use axum::body::{Body, Bytes};
use axum::extract::rejection::BytesRejection;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use std::sync::Arc;
use std::time::Instant;

type Shared = State<Arc<AppState>>;

/// `X-Arda-Cache`: whether an image came from cache.
pub const X_ARDA_CACHE: HeaderName = HeaderName::from_static("x-arda-cache");
const SERVER_TIMING: HeaderName = HeaderName::from_static("server-timing");
/// `Cache-Control` of images of built-in layouts.
pub const NAMED_CACHE_CONTROL: &str = "public, max-age=3600";
/// `Cache-Control` of world-derived tactical bodies and images
/// (logic/16 §api-cache).
pub const TACTICAL_CACHE_CONTROL: &str = "public, max-age=3600";
/// `Cache-Control` of `POST /render` responses: revalidate by ETag.
pub const POSTED_CACHE_CONTROL: &str = "no-cache";

/// The `/v1/tactical` routes; `max_body_bytes` bounds `POST /render`.
pub fn router(max_body_bytes: usize) -> Router<Arc<AppState>> {
    Router::new()
        .route("/layouts", get(layouts))
        .route("/layout/{file}", get(layout))
        .route("/layout/{name}/tiles/{z}/{x}/{y}", get(tile))
        .route(
            "/render",
            post(render_posted).layer(DefaultBodyLimit::max(max_body_bytes)),
        )
        .route("/cell/{gx}/{gy}", get(super::world_routes::cell))
        .route("/cell/{gx}/{gy}/scene", get(super::scene::cell_scene))
        .route(
            "/cell/{gx}/{gy}/tiles/{z}/{x}/{y}",
            get(super::world_routes::cell_tile),
        )
        .route("/window", get(super::world_routes::window))
        .route("/window.png", get(super::world_routes::window_png))
        .route("/library", get(super::world_routes::library))
        .route("/prefetch", post(prefetch))
}

/// `POST /v1/tactical/prefetch`: queues the neighbours of a cell for the
/// background workers and answers `202` at once (goal 67).
async fn prefetch(
    State(state): Shared,
    body: Result<Json<super::prefetch::PrefetchRequest>, axum::extract::rejection::JsonRejection>,
) -> ServerResult<Response> {
    let start = Instant::now();
    let Json(request) = body.map_err(|e| {
        if e.status() == StatusCode::UNSUPPORTED_MEDIA_TYPE {
            ServerError::UnsupportedMediaType(e.body_text())
        } else {
            ServerError::BadRequest(e.body_text())
        }
    })?;
    let accepted = state.prefetch.submit(&state, &request)?;
    log(
        "prefetch",
        &format!(
            "cell {},{} radius={}",
            request.gx, request.gy, accepted.radius
        ),
        Hit::Miss,
        accepted.cells.len(),
        start,
    );
    Ok((StatusCode::ACCEPTED, Json(accepted)).into_response())
}

/// Render options parsed from `?ppsq=64|96|128&grid=0|1`; `origin` is
/// refused (it only applies to `POST /render`).
fn options(query: Params) -> ServerResult<(u32, bool)> {
    let (ppsq, grid, origin) = options_with_origin(query)?;
    if origin.is_some() {
        return Err(ServerError::BadRequest(
            "origin applies only to POST /v1/tactical/render".into(),
        ));
    }
    Ok((ppsq, grid))
}

/// Render options plus `?origin=X,Y` in world squares.
fn options_with_origin(query: Params) -> ServerResult<(u32, bool, Option<Anchor>)> {
    let q = params(query)?;
    let origin = q
        .get("origin")
        .map(|o| Anchor::parse_origin(o).map_err(ServerError::BadRequest))
        .transpose()?;
    let ppsq = match q.get("ppsq") {
        None => DEFAULT_PPSQ,
        Some(text) => {
            let v = parse::<u32>("ppsq", text)?;
            if !PPSQ_OPTIONS.contains(&v) {
                return Err(ServerError::BadRequest(format!(
                    "ppsq must be one of {PPSQ_OPTIONS:?}, not {v}"
                )));
            }
            v
        }
    };
    let grid = match q.get("grid").map(String::as_str) {
        None | Some("0") => false,
        Some("1") => true,
        Some(other) => {
            return Err(ServerError::BadRequest(format!(
                "grid must be 0 or 1, not {other:?}"
            )))
        }
    };
    Ok((ppsq, grid, origin))
}

pub(super) fn log(route: &str, detail: &str, hit: Hit, bytes: usize, start: Instant) -> f64 {
    let ms = start.elapsed().as_secs_f64() * 1e3;
    let hit = match hit {
        Hit::Hit => "hit",
        Hit::Miss => "miss",
    };
    eprintln!("arda-server tactical {route} {detail} cache={hit} bytes={bytes} {ms:.3} ms");
    ms
}

pub(super) fn image(
    headers: &HeaderMap,
    encoded: &Encoded,
    content_type: &'static str,
    cache_control: &'static str,
    hit: Hit,
    ms: f64,
) -> Response {
    let hit_text = match hit {
        Hit::Hit => "hit",
        Hit::Miss => "miss",
    };
    let etag = HeaderValue::from_str(&encoded.etag).unwrap_or(HeaderValue::from_static("\"\""));
    let timing = HeaderValue::from_str(&format!("tactical;dur={ms:.3}"))
        .unwrap_or(HeaderValue::from_static("tactical"));
    let fresh = headers
        .get(IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(',')
                .any(|t| t.trim() == encoded.etag || t.trim() == "*")
        });
    let (status, body) = if fresh {
        (StatusCode::NOT_MODIFIED, Body::empty())
    } else {
        (StatusCode::OK, Body::from(encoded.bytes.clone()))
    };
    let mut response = (status, body).into_response();
    let h = response.headers_mut();
    if !fresh {
        h.insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    }
    h.insert(ETAG, etag);
    h.insert(CACHE_CONTROL, HeaderValue::from_static(cache_control));
    h.insert(X_ARDA_CACHE, HeaderValue::from_static(hit_text));
    h.insert(SERVER_TIMING, timing);
    response
}

async fn layouts(State(state): Shared) -> Response {
    let start = Instant::now();
    let body = state.tactical.listing();
    log("layouts", "-", Hit::Hit, 0, start);
    Json(body).into_response()
}

async fn layout(
    State(state): Shared,
    headers: HeaderMap,
    path: Segments,
    query: Params,
) -> ServerResult<Response> {
    let start = Instant::now();
    let p = segments(path, 1)?;
    let Some(name) = p[0].strip_suffix(".png") else {
        let l = state.tactical.named(&p[0])?.clone();
        return Ok((
            [(CACHE_CONTROL, HeaderValue::from_static(NAMED_CACHE_CONTROL))],
            Json(l),
        )
            .into_response());
    };
    let (ppsq, grid) = options(query)?;
    let name = name.to_owned();
    let detail = format!("png {name} ppsq={ppsq} grid={}", u8::from(grid));
    let (encoded, hit) = named_work(state, name, ppsq, grid, |t, l, k| t.png(l, k)).await?;
    let ms = log("layout", &detail, hit, encoded.bytes.len(), start);
    Ok(image(
        &headers,
        &encoded,
        "image/png",
        NAMED_CACHE_CONTROL,
        hit,
        ms,
    ))
}

/// Runs `work` on a built-in layout on the blocking pool.
async fn named_work(
    state: Arc<AppState>,
    name: String,
    ppsq: u32,
    grid: bool,
    work: impl Fn(&super::Tactical, &TacticalLayout, &RenderKey) -> ServerResult<(Arc<Encoded>, Hit)>
        + Send
        + 'static,
) -> ServerResult<(Arc<Encoded>, Hit)> {
    blocking(state, move |s| {
        let t = &s.tactical;
        let l = t.named(&name)?;
        let key = t.key(l, Anchor::Name(name.clone()), ppsq, grid)?;
        work(t, l, &key)
    })
    .await
}

async fn tile(
    State(state): Shared,
    headers: HeaderMap,
    path: Segments,
    query: Params,
) -> ServerResult<Response> {
    let start = Instant::now();
    let p = segments(path, 4)?;
    let y_text = p[3]
        .strip_suffix(".webp")
        .ok_or_else(|| ServerError::NotFound("tactical tiles are served as {y}.webp".into()))?;
    let zxy = (
        parse::<u32>("z", &p[1])?,
        parse::<u32>("x", &p[2])?,
        parse::<u32>("y", y_text)?,
    );
    let (ppsq, grid) = options(query)?;
    let name = p[0].clone();
    let detail = format!(
        "tile {name} {}/{}/{} ppsq={ppsq} grid={}",
        zxy.0,
        zxy.1,
        zxy.2,
        u8::from(grid)
    );
    let (encoded, hit) =
        named_work(state, name, ppsq, grid, move |t, l, k| t.tile(l, k, zxy)).await?;
    let ms = log("tile", &detail, hit, encoded.bytes.len(), start);
    Ok(image(
        &headers,
        &encoded,
        "image/webp",
        NAMED_CACHE_CONTROL,
        hit,
        ms,
    ))
}

async fn render_posted(
    State(state): Shared,
    headers: HeaderMap,
    query: Params,
    body: Result<Bytes, BytesRejection>,
) -> ServerResult<Response> {
    let start = Instant::now();
    let limit = state.tactical.limits().max_body_bytes;
    let body = body.map_err(|e| {
        if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
            ServerError::PayloadTooLarge(format!("request body exceeds {limit} bytes"))
        } else {
            ServerError::BadRequest(e.body_text())
        }
    })?;
    // A JSON content type forces a CORS preflight, so pages on other
    // origins cannot fire renders with a "simple" text/plain POST.
    let json = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"));
    if !json {
        return Err(ServerError::UnsupportedMediaType(
            "POST /render takes a layout as application/json".into(),
        ));
    }
    let (ppsq, grid, origin) = options_with_origin(query)?;
    let text = std::str::from_utf8(&body)
        .map_err(|_| ServerError::BadRequest("layout body is not UTF-8".into()))?;
    let layout =
        TacticalLayout::from_json(text).map_err(|e| ServerError::BadRequest(e.to_string()))?;
    let anchor = origin.unwrap_or_else(|| Anchor::Name(layout.name.clone()));
    let detail = format!(
        "render {} {}x{} {anchor} ppsq={ppsq} grid={}",
        layout.name,
        layout.width,
        layout.height,
        u8::from(grid)
    );
    let (encoded, hit) = blocking(state, move |s| {
        let t = &s.tactical;
        t.admit(&layout, ppsq)?;
        let key = t.key(&layout, anchor, ppsq, grid)?;
        t.png(&layout, &key)
    })
    .await?;
    let ms = log("render", &detail, hit, encoded.bytes.len(), start);
    Ok(image(
        &headers,
        &encoded,
        "image/png",
        POSTED_CACHE_CONTROL,
        hit,
        ms,
    ))
}
