//! Handlers for world-derived tactical maps (logic/16 §api-tactical):
//!
//! - `GET /v1/tactical/cell/{gx}/{gy}[?ppsq&demo_overlays&demo_at]`: JSON;
//! - `GET /v1/tactical/cell/{gx}/{gy}.png[?ppsq&grid&demo_overlays&demo_at]`;
//! - `GET /v1/tactical/cell/{gx}/{gy}/tiles/{z}/{x}/{y}.webp[?…]`;
//! - `GET /v1/tactical/window?gx0&gy0&w&h[&…]` (JSON) and
//!   `GET /v1/tactical/window.png?…`, up to 3 × 3 cells;
//! - `GET /v1/tactical/library`: the catalogue summary.
//!
//! `demo_overlays=1` composes the synthetic ways, fields and town samples
//! anchored at `demo_at=gx,gy` (default: the requested cell, or the
//! window's first cell), until settlement data is integrated.

use super::block::BlockRequest;
use super::cells::MAX_WINDOW_CELLS;
use super::routes::{image, log, TACTICAL_CACHE_CONTROL};
use super::{Encoded, Hit, DEFAULT_PPSQ, PPSQ_OPTIONS};
use crate::error::{ServerError, ServerResult};
use crate::routes::{blocking, params, parse, segments, Params, Segments};
use crate::AppState;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::Json;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

type Shared = State<Arc<AppState>>;

/// Extra ppsq values world images may use: overview renders of cells and
/// windows (a 3 × 3 window at 128 would exceed the render limit).
pub(super) const WINDOW_PPSQ: [u32; 2] = [16, 32];
/// Default ppsq of `/window.png`.
const WINDOW_DEFAULT_PPSQ: u32 = 32;

/// Parsed world-map options.
pub(super) struct Opts {
    ppsq: u32,
    grid: bool,
    demo: bool,
    demo_at: Option<[i64; 2]>,
}

fn flag(q: &BTreeMap<String, String>, name: &str) -> ServerResult<bool> {
    match q.get(name).map(String::as_str) {
        None | Some("0") => Ok(false),
        Some("1") => Ok(true),
        Some(other) => Err(ServerError::BadRequest(format!(
            "{name} must be 0 or 1, not {other:?}"
        ))),
    }
}

pub(super) fn opts(
    q: &BTreeMap<String, String>,
    extra: &[u32],
    default: u32,
) -> ServerResult<Opts> {
    let ppsq = match q.get("ppsq") {
        None => default,
        Some(t) => {
            let v = parse::<u32>("ppsq", t)?;
            if !PPSQ_OPTIONS.contains(&v) && !extra.contains(&v) {
                return Err(ServerError::BadRequest(format!(
                    "ppsq must be one of {PPSQ_OPTIONS:?} {extra:?}, not {v}"
                )));
            }
            v
        }
    };
    let demo_at = q
        .get("demo_at")
        .map(|t| {
            let bad = || ServerError::BadRequest(format!("demo_at must be GX,GY, not {t:?}"));
            let (a, b) = t.split_once(',').ok_or_else(bad)?;
            let n = |v: &str| v.trim().parse::<i64>().map_err(|_| bad());
            Ok::<_, ServerError>([n(a)?, n(b)?])
        })
        .transpose()?;
    Ok(Opts {
        ppsq,
        grid: flag(q, "grid")?,
        demo: flag(q, "demo_overlays")?,
        demo_at,
    })
}

/// World extent in cells and in squares.
fn extent(state: &AppState) -> ((u32, u32), (i64, i64)) {
    let (w, h) = state.query.cells();
    ((w, h), (i64::from(w) * 64, i64::from(h) * 64))
}

pub(super) fn cell_of(state: &AppState, gx: &str, gy: &str) -> ServerResult<(u32, u32)> {
    let (gx, gy) = (parse::<u32>("gx", gx)?, parse::<u32>("gy", gy)?);
    let ((w, h), _) = extent(state);
    if gx >= w || gy >= h {
        return Err(ServerError::OutOfRange(format!(
            "cell {gx},{gy} is outside the world ({},{} is the last)",
            w - 1,
            h - 1
        )));
    }
    Ok((gx, gy))
}

pub(super) fn request(gx: u32, gy: u32, o: &Opts) -> BlockRequest {
    let mut r = BlockRequest::cell(gx, gy);
    if o.demo {
        r.demo_at = Some(o.demo_at.unwrap_or([i64::from(gx), i64::from(gy)]));
    }
    r
}

fn send(
    headers: &HeaderMap,
    route: &str,
    detail: &str,
    (encoded, hit): (Arc<Encoded>, Hit),
    content_type: &'static str,
    start: Instant,
) -> Response {
    let ms = log(route, detail, hit, encoded.bytes.len(), start);
    image(
        headers,
        &encoded,
        content_type,
        TACTICAL_CACHE_CONTROL,
        hit,
        ms,
    )
}

/// `GET /cell/{gx}/{gy}` (JSON) and `GET /cell/{gx}/{gy}.png`.
pub(super) async fn cell(
    State(state): Shared,
    headers: HeaderMap,
    path: Segments,
    query: Params,
) -> ServerResult<Response> {
    let start = Instant::now();
    let p = segments(path, 2)?;
    let (gy_text, png) = match p[1].strip_suffix(".png") {
        Some(t) => (t.to_owned(), true),
        None => (p[1].clone(), false),
    };
    let (gx, gy) = cell_of(&state, &p[0], &gy_text)?;
    let o = opts(&params(query)?, &WINDOW_PPSQ, DEFAULT_PPSQ)?;
    let req = request(gx, gy, &o);
    let (_, world) = extent(&state);
    let detail = format!(
        "cell {gx},{gy} ppsq={} demo={}",
        o.ppsq,
        u8::from(req.demo_at.is_some())
    );
    let (ppsq, grid) = (o.ppsq, o.grid);
    if png {
        let out = blocking(state, move |s| {
            s.tactical.world_png(&req, world, ppsq, grid)
        })
        .await?;
        return Ok(send(&headers, "cell.png", &detail, out, "image/png", start));
    }
    let out = blocking(state, move |s| s.tactical.world_body(&req, ppsq)).await?;
    Ok(send(
        &headers,
        "cell",
        &detail,
        out,
        "application/json",
        start,
    ))
}

/// `GET /cell/{gx}/{gy}/tiles/{z}/{x}/{y}.webp`.
pub(super) async fn cell_tile(
    State(state): Shared,
    headers: HeaderMap,
    path: Segments,
    query: Params,
) -> ServerResult<Response> {
    let start = Instant::now();
    let p = segments(path, 5)?;
    let (gx, gy) = cell_of(&state, &p[0], &p[1])?;
    let y_text = p[4]
        .strip_suffix(".webp")
        .ok_or_else(|| ServerError::NotFound("tactical tiles are served as {y}.webp".into()))?;
    let zxy = (
        parse::<u32>("z", &p[2])?,
        parse::<u32>("x", &p[3])?,
        parse::<u32>("y", y_text)?,
    );
    let o = opts(&params(query)?, &WINDOW_PPSQ, DEFAULT_PPSQ)?;
    let req = request(gx, gy, &o);
    let (_, world) = extent(&state);
    let detail = format!(
        "cell {gx},{gy} tile {}/{}/{} ppsq={}",
        zxy.0, zxy.1, zxy.2, o.ppsq
    );
    let pg = (o.ppsq, o.grid);
    let out = blocking(state, move |s| s.tactical.world_tile(&req, world, pg, zxy)).await?;
    Ok(send(
        &headers,
        "cell.tile",
        &detail,
        out,
        "image/webp",
        start,
    ))
}

/// The window of `?gx0&gy0&w&h` (cells; `w`, `h` in `1..=3`).
fn window_request(
    state: &AppState,
    q: &BTreeMap<String, String>,
    o: &Opts,
) -> ServerResult<BlockRequest> {
    let get = |k: &str| {
        q.get(k)
            .ok_or_else(|| ServerError::BadRequest(format!("missing ?{k}=")))
            .and_then(|t| parse::<u32>(k, t))
    };
    let (gx0, gy0, w, h) = (get("gx0")?, get("gy0")?, get("w")?, get("h")?);
    if !(1..=MAX_WINDOW_CELLS).contains(&w) || !(1..=MAX_WINDOW_CELLS).contains(&h) {
        return Err(ServerError::PayloadTooLarge(format!(
            "a window is 1..={MAX_WINDOW_CELLS} cells a side, not {w}x{h}"
        )));
    }
    let ((cw, ch), _) = extent(state);
    if gx0.saturating_add(w) > cw || gy0.saturating_add(h) > ch {
        return Err(ServerError::OutOfRange(format!(
            "window {gx0},{gy0} {w}x{h} leaves the {cw}x{ch}-cell world"
        )));
    }
    let mut r = BlockRequest::cell(gx0, gy0);
    r.w = w * 64;
    r.h = h * 64;
    if o.demo {
        r.demo_at = Some(o.demo_at.unwrap_or([i64::from(gx0), i64::from(gy0)]));
    }
    Ok(r)
}

/// `GET /window` (JSON).
pub(super) async fn window(
    State(state): Shared,
    headers: HeaderMap,
    query: Params,
) -> ServerResult<Response> {
    let start = Instant::now();
    let q = params(query)?;
    let o = opts(&q, &WINDOW_PPSQ, WINDOW_DEFAULT_PPSQ)?;
    let req = window_request(&state, &q, &o)?;
    let detail = format!("window {},{} {}x{}", req.gsx0, req.gsy0, req.w, req.h);
    let ppsq = o.ppsq;
    let out = blocking(state, move |s| s.tactical.world_body(&req, ppsq)).await?;
    Ok(send(
        &headers,
        "window",
        &detail,
        out,
        "application/json",
        start,
    ))
}

/// `GET /window.png`.
pub(super) async fn window_png(
    State(state): Shared,
    headers: HeaderMap,
    query: Params,
) -> ServerResult<Response> {
    let start = Instant::now();
    let q = params(query)?;
    let o = opts(&q, &WINDOW_PPSQ, WINDOW_DEFAULT_PPSQ)?;
    let req = window_request(&state, &q, &o)?;
    let (_, world) = extent(&state);
    let detail = format!(
        "window {},{} {}x{} ppsq={}",
        req.gsx0, req.gsy0, req.w, req.h, o.ppsq
    );
    let (ppsq, grid) = (o.ppsq, o.grid);
    let out = blocking(state, move |s| {
        s.tactical.world_png(&req, world, ppsq, grid)
    })
    .await?;
    Ok(send(
        &headers,
        "window.png",
        &detail,
        out,
        "image/png",
        start,
    ))
}

/// `GET /library`.
pub(super) async fn library(State(state): Shared) -> Response {
    let start = Instant::now();
    let body = super::library_dto::summarise(state.tactical.library());
    log("library", "-", Hit::Hit, 0, start);
    Json(body).into_response()
}
