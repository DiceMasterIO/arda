//! The `/v1` axum router. Handlers parse, then run world work on the blocking pool.

use crate::columnar;
use crate::dto::{self, AreaLakes, AreaRivers, Health, Origin, TilePyramidDto, WorldInfo};
use crate::error::{ServerError, ServerResult};
use crate::overview::{Style, TileFormat};
use crate::AppState;
use axum::extract::rejection::JsonRejection;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use axum::http::StatusCode;
use axum::http::{HeaderValue, Method};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};

type Shared = State<Arc<AppState>>;
pub(crate) type Params =
    Result<Query<BTreeMap<String, String>>, axum::extract::rejection::QueryRejection>;
pub(crate) type Segments = Result<Path<Vec<String>>, axum::extract::rejection::PathRejection>;

/// Builds the router with CORS for localhost development origins.
pub fn router(state: Arc<AppState>) -> Router {
    let v1 = Router::new()
        .route("/health", get(health))
        .route("/world", get(world))
        .route("/cell/{gx}/{gy}", get(cell))
        .route("/area/{ax}/{ay}/cells", get(area_cells))
        .route("/area/{ax}/{ay}/rivers", get(area_rivers))
        .route("/area/{ax}/{ay}/lakes", get(area_lakes))
        .route("/point", get(point))
        .route("/overview.png", get(overview_png))
        .route("/tiles/overview/{z}/{x}/{y}", get(overview_tile))
        .route("/tiles/relief/{z}/{x}/{y}", get(relief_tile))
        .route(
            "/npc/population",
            post(npc_population).layer(DefaultBodyLimit::max(crate::npc::MAX_BODY_BYTES)),
        )
        .route("/npc/demo", get(npc_demo))
        .route("/npc/demo/{npc_id}", get(npc_demo_one))
        .route("/npc/{npc_id}", get(crate::npcs::one))
        .route("/npcs", get(crate::npcs::list))
        .route("/buildings/{id}/residents", get(crate::npcs::residents))
        .route("/buildings/{id}/workers", get(crate::npcs::workers))
        .route("/settlements", get(crate::people::list))
        .route("/settlements/{id}", get(crate::people::one))
        .route("/settlements/{id}/plan", get(crate::people::plan))
        .route("/settlements/{id}/npcs", get(crate::people::npcs))
        .nest(
            "/tactical",
            crate::tactical::routes::router(state.tactical.limits().max_body_bytes),
        );
    Router::new()
        .nest("/v1", v1)
        .fallback(not_found)
        .layer(cors())
        .with_state(state)
}

/// Whether `origin` is `http(s)://localhost`, `127.0.0.1` or `[::1]`, any port.
#[must_use]
pub fn is_local_origin(origin: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(origin) else {
        return false;
    };
    let Some(rest) = text
        .strip_prefix("http://")
        .or_else(|| text.strip_prefix("https://"))
    else {
        return false;
    };
    let host = if rest.starts_with('[') {
        rest.split_inclusive(']').next().unwrap_or("")
    } else {
        rest.split(':').next().unwrap_or("")
    };
    let port_ok = rest[host.len()..]
        .strip_prefix(':')
        .map_or(rest.len() == host.len(), |p| {
            !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())
        });
    matches!(host, "localhost" | "127.0.0.1" | "[::1]") && port_ok
}

fn cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin: &HeaderValue, _| {
            is_local_origin(origin.as_bytes())
        }))
        .allow_methods([Method::GET, Method::HEAD, Method::POST, Method::OPTIONS])
        .allow_headers([CONTENT_TYPE, IF_NONE_MATCH])
        .expose_headers([ETAG, crate::tactical::routes::X_ARDA_CACHE])
}

pub(crate) async fn blocking<T: Send + 'static>(
    state: Arc<AppState>,
    work: impl FnOnce(&AppState) -> ServerResult<T> + Send + 'static,
) -> ServerResult<T> {
    tokio::task::spawn_blocking(move || work(&state))
        .await
        .map_err(|e| ServerError::Internal(format!("worker: {e}")))?
}

pub(crate) fn segments(path: Segments, count: usize) -> ServerResult<Vec<String>> {
    let Path(parts) = path.map_err(|e| ServerError::BadRequest(e.body_text()))?;
    if parts.len() == count {
        Ok(parts)
    } else {
        Err(ServerError::BadRequest("unexpected path parameters".into()))
    }
}

pub(crate) fn parse<T: FromStr>(name: &str, text: &str) -> ServerResult<T> {
    text.parse().map_err(|_| {
        ServerError::BadRequest(format!(
            "{name} {text:?} is not a valid {}",
            std::any::type_name::<T>()
        ))
    })
}

pub(crate) fn params(query: Params) -> ServerResult<BTreeMap<String, String>> {
    query
        .map(|Query(q)| q)
        .map_err(|e| ServerError::BadRequest(e.body_text()))
}

fn png(bytes: &[u8]) -> Response {
    image("image/png", bytes)
}

fn image(content_type: &'static str, bytes: &[u8]) -> Response {
    (
        [
            (CONTENT_TYPE, HeaderValue::from_static(content_type)),
            (
                CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=3600"),
            ),
        ],
        bytes.to_vec(),
    )
        .into_response()
}

async fn not_found() -> ServerError {
    ServerError::NotFound("no such route; every endpoint lives under /v1".into())
}

async fn health(State(state): Shared) -> Json<Health> {
    Json(Health {
        status: "ok".into(),
        contract_version: crate::contract::CONTRACT_VERSION,
        seed: state.query.world().seed().to_string(),
    })
}

async fn world(State(state): Shared) -> Json<WorldInfo> {
    let tiles = TilePyramidDto {
        relief_max_zoom: state.relief.max_zoom(),
        ..state.overview.pyramid()
    };
    Json(WorldInfo::new(&state.query, tiles))
}

async fn cell(State(state): Shared, path: Segments) -> ServerResult<Response> {
    let p = segments(path, 2)?;
    let (gx, gy) = (parse::<u32>("gx", &p[0])?, parse::<u32>("gy", &p[1])?);
    let sample = blocking(state, move |s| s.query.cell(gx, gy)).await?;
    Ok(Json(sample).into_response())
}

fn area_coords(path: Segments) -> ServerResult<(i32, i32)> {
    let p = segments(path, 2)?;
    Ok((parse("ax", &p[0])?, parse("ay", &p[1])?))
}

async fn area_cells(State(state): Shared, path: Segments, query: Params) -> ServerResult<Response> {
    let (ax, ay) = area_coords(path)?;
    let binary = match params(query)?.get("format").map(String::as_str) {
        None | Some("json") => false,
        Some("bin") => true,
        Some(other) => {
            return Err(ServerError::BadRequest(format!(
                "format must be json or bin, not {other:?}"
            )))
        }
    };
    // The build is the server's largest transient allocation; admission
    // counts `area_builds` of them, so the permit covers building and encoding.
    let _slot = state
        .area_builds
        .acquire()
        .await
        .map_err(|_| ServerError::Internal("area build gate closed".into()))?;
    let bytes = blocking(Arc::clone(&state), move |s| {
        let body = columnar::build(&s.query.area_samples(ax, ay)?)?;
        if binary {
            columnar::encode(&body)
        } else {
            serde_json::to_vec(&body).map_err(|e| ServerError::Internal(format!("json: {e}")))
        }
    })
    .await?;
    let kind = if binary {
        "application/octet-stream"
    } else {
        "application/json"
    };
    Ok(([(CONTENT_TYPE, HeaderValue::from_static(kind))], bytes).into_response())
}

fn origin(ax: i32, ay: i32) -> ServerResult<Origin> {
    let o = |a: i32| {
        u32::try_from(a)
            .ok()
            .and_then(|a| a.checked_mul(u32::from(arda_core::AREA_CELLS)))
            .ok_or_else(|| ServerError::OutOfRange(format!("area {ax},{ay} is outside the world")))
    };
    Ok(Origin {
        gx: o(ax)?,
        gy: o(ay)?,
    })
}

async fn area_rivers(State(state): Shared, path: Segments) -> ServerResult<Json<AreaRivers>> {
    let (ax, ay) = area_coords(path)?;
    let body = blocking(state, move |s| {
        let area = s.query.area(ax, ay)?;
        let at = origin(ax, ay)?;
        Ok(AreaRivers {
            contract_version: crate::contract::CONTRACT_VERSION,
            ax,
            ay,
            rivers: area.rivers().iter().map(|r| dto::river(r, at)).collect(),
        })
    })
    .await?;
    Ok(Json(body))
}

async fn area_lakes(State(state): Shared, path: Segments) -> ServerResult<Json<AreaLakes>> {
    let (ax, ay) = area_coords(path)?;
    let body = blocking(state, move |s| {
        let area = s.query.area(ax, ay)?;
        let at = origin(ax, ay)?;
        Ok(AreaLakes {
            contract_version: crate::contract::CONTRACT_VERSION,
            ax,
            ay,
            lakes: area.lakes().iter().map(|l| dto::lake(l, at)).collect(),
        })
    })
    .await?;
    Ok(Json(body))
}

async fn point(State(state): Shared, query: Params) -> ServerResult<Response> {
    let q = params(query)?;
    let get = |name: &str| -> ServerResult<f64> {
        let text = q
            .get(name)
            .ok_or_else(|| ServerError::BadRequest(format!("missing query parameter {name}")))?;
        parse(name, text)
    };
    let (x_m, y_m) = (get("x_m")?, get("y_m")?);
    let sample = blocking(state, move |s| s.query.sample_point(x_m, y_m)).await?;
    Ok(Json(sample).into_response())
}

async fn overview_png(State(state): Shared, query: Params) -> ServerResult<Response> {
    let q = params(query)?;
    let quality = state
        .overview
        .quality(q.get("quality").map(String::as_str))?;
    let style = q
        .get("style")
        .map_or(Ok(Style::Atlas), |s| Style::parse(s))?;
    let bytes = blocking(state, move |s| s.overview.png(&s.query, quality, style)).await?;
    Ok(png(&bytes))
}

/// `/v1/tiles/overview/{z}/{x}/{y}.webp` (goal 68) and `.png`;
/// `?oblique=1` serves the opt-in oblique pyramid (goal 24).
async fn overview_tile(
    State(state): Shared,
    path: Segments,
    query: Params,
) -> ServerResult<Response> {
    let oblique = crate::tactical::world_routes::flag(&params(query)?, "oblique")?;
    let p = segments(path, 3)?;
    let (y_text, format) = TileFormat::split(&p[2]).ok_or_else(|| {
        ServerError::NotFound("overview tiles are served as {y}.webp or {y}.png".into())
    })?;
    let (z, x, y) = (
        parse::<u32>("z", &p[0])?,
        parse::<u32>("x", &p[1])?,
        parse::<u32>("y", y_text)?,
    );
    if z > 31 {
        return Err(ServerError::NotFound(format!(
            "zoom {z} is outside the pyramid"
        )));
    }
    let bytes = blocking(state, move |s| {
        s.overview.tile(&s.query, (z, x, y), (format, oblique))
    })
    .await?;
    Ok(image(format.content_type(), &bytes))
}

/// `/v1/tiles/relief/{z}/{x}/{y}.webp`: mid-zoom relief past native zoom.
async fn relief_tile(State(state): Shared, path: Segments) -> ServerResult<Response> {
    let p = segments(path, 3)?;
    let y_text = p[2]
        .strip_suffix(".webp")
        .ok_or_else(|| ServerError::NotFound("relief tiles are served as {y}.webp".into()))?;
    let (z, x, y) = (
        parse::<u32>("z", &p[0])?,
        parse::<u32>("x", &p[1])?,
        parse::<u32>("y", y_text)?,
    );
    let bytes = blocking(state, move |s| s.relief.tile(z, x, y)).await?;
    Ok((
        [
            (CONTENT_TYPE, HeaderValue::from_static("image/webp")),
            (
                CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=3600"),
            ),
        ],
        bytes.to_vec(),
    )
        .into_response())
}

async fn npc_population(
    State(state): Shared,
    body: Result<Json<crate::npc::PopulationRequest>, JsonRejection>,
) -> ServerResult<Response> {
    let Json(request) = body.map_err(|e| {
        if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
            ServerError::PayloadTooLarge(format!(
                "request body is above {} bytes",
                crate::npc::MAX_BODY_BYTES
            ))
        } else {
            ServerError::BadRequest(e.body_text())
        }
    })?;
    let population = blocking(state, move |_| crate::npc::populate(&request)).await?;
    Ok(Json(population).into_response())
}

async fn npc_demo(State(state): Shared) -> ServerResult<Response> {
    let demo = blocking(state, |s| s.npc.demo().cloned()).await?;
    Ok(Json(demo).into_response())
}

async fn npc_demo_one(State(state): Shared, path: Segments) -> ServerResult<Response> {
    let p = segments(path, 1)?;
    let id = crate::npc::parse_npc_id(&p[0])?;
    let npc = blocking(state, move |s| s.npc.demo_npc(id)).await?;
    Ok(Json(npc).into_response())
}

#[cfg(test)]
mod tests {
    use super::is_local_origin;

    #[test]
    fn only_loopback_dev_origins_are_allowed() {
        for ok in [
            "http://localhost:5173",
            "http://127.0.0.1:3000",
            "https://localhost",
            "http://[::1]:8080",
        ] {
            assert!(is_local_origin(ok.as_bytes()), "{ok}");
        }
        for bad in [
            "http://localhost.evil.com",
            "http://example.com",
            "http://127.0.0.1.nip.io:80",
            "file://localhost",
            "http://localhost:",
            "http://localhost:80x",
        ] {
            assert!(!is_local_origin(bad.as_bytes()), "{bad}");
        }
    }
}
