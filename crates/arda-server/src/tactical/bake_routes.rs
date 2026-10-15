//! Handlers of the bake job API (1.0 task 04; see [`super::bake`]):
//!
//! - `POST /v1/tactical/bake` → `202 BakeAccepted` (`200` once finished);
//! - `GET /v1/tactical/bake/{id}` → `BakeStatus`;
//! - `GET /v1/tactical/bake/{id}/tiles/{z}/{x}/{y}.webp` → one tile
//!   (strong `ETag`, `Cache-Control: no-store`);
//! - `DELETE /v1/tactical/bake/{id}` → `204`, freeing the tiles.
//!
//! Each request logs one timing line.

use super::bake::{job_id, BakeRequest, BakeState};
use super::routes::log;
use super::Hit;
use crate::error::{ServerError, ServerResult};
use crate::routes::{blocking, parse, segments, Segments};
use crate::AppState;
use axum::body::{Body, Bytes};
use axum::extract::rejection::BytesRejection;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH, LOCATION};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use std::sync::Arc;
use std::time::Instant;

type Shared = State<Arc<AppState>>;

/// `Cache-Control` of baked tiles: the client stores them itself.
pub const BAKE_CACHE_CONTROL: &str = "no-store";

/// The bake routes; `max_body_bytes` bounds `POST /bake`.
pub fn routes(max_body_bytes: usize) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/bake",
            post(submit).layer(DefaultBodyLimit::max(max_body_bytes)),
        )
        .route("/bake/{id}", get(status).delete(delete))
        .route("/bake/{id}/tiles/{z}/{x}/{y}", get(tile))
}

/// Whether the request declares a JSON body (a CORS-preflight guard, as
/// on `POST /render`).
fn is_json(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"))
}

async fn submit(
    State(state): Shared,
    headers: HeaderMap,
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
    if !is_json(&headers) {
        return Err(ServerError::UnsupportedMediaType(
            "POST /bake takes a BakeRequest as application/json".into(),
        ));
    }
    let request: BakeRequest =
        serde_json::from_slice(&body).map_err(|e| ServerError::BadRequest(e.to_string()))?;
    let n = body.len();
    let (id, parsed) = blocking(Arc::clone(&state), move |s| {
        let t = &s.tactical;
        let parsed = t.check_bake(&request, s.query.cells())?;
        let release = crate::versions::SERVER_VERSION;
        let id = job_id((release, t.world_seed()), t.library_version(), &parsed)?;
        Ok((id, parsed))
    })
    .await?;
    let detail = format!(
        "bake {id} cell {},{} ppsq={}",
        parsed.gx, parsed.gy, parsed.ppsq
    );
    let accepted = state.bake.submit(&state, id, parsed)?;
    log("bake", &detail, Hit::Miss, n, start);
    let status = match accepted.state {
        BakeState::Done | BakeState::Failed => StatusCode::OK,
        BakeState::Queued | BakeState::Running => StatusCode::ACCEPTED,
    };
    let location = HeaderValue::from_str(&accepted.status_url)
        .map_err(|e| ServerError::Internal(format!("location: {e}")))?;
    Ok((status, [(LOCATION, location)], Json(accepted)).into_response())
}

async fn status(State(state): Shared, path: Segments) -> ServerResult<Response> {
    let start = Instant::now();
    let p = segments(path, 1)?;
    let body = state.bake.status(&p[0])?;
    log("bake.status", &p[0], Hit::Miss, 0, start);
    Ok((
        [(CACHE_CONTROL, HeaderValue::from_static(BAKE_CACHE_CONTROL))],
        Json(body),
    )
        .into_response())
}

async fn delete(State(state): Shared, path: Segments) -> ServerResult<Response> {
    let start = Instant::now();
    let p = segments(path, 1)?;
    state.bake.delete(&p[0])?;
    log("bake.delete", &p[0], Hit::Miss, 0, start);
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn tile(State(state): Shared, headers: HeaderMap, path: Segments) -> ServerResult<Response> {
    let start = Instant::now();
    let p = segments(path, 4)?;
    let y_text = p[3]
        .strip_suffix(".webp")
        .ok_or_else(|| ServerError::NotFound("baked tiles are served as {y}.webp".into()))?;
    let zxy = (
        parse::<u32>("z", &p[1])?,
        parse::<u32>("x", &p[2])?,
        parse::<u32>("y", y_text)?,
    );
    let encoded = state.bake.tile(&p[0], zxy)?;
    let detail = format!("{} tile {}/{}/{}", p[0], zxy.0, zxy.1, zxy.2);
    log("bake.tile", &detail, Hit::Miss, encoded.bytes.len(), start);
    let fresh = headers
        .get(IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(',')
                .any(|t| t.trim() == encoded.etag || t.trim() == "*")
        });
    let etag = HeaderValue::from_str(&encoded.etag)
        .map_err(|e| ServerError::Internal(format!("etag: {e}")))?;
    let mut response = if fresh {
        (StatusCode::NOT_MODIFIED, Body::empty()).into_response()
    } else {
        let mut r = (StatusCode::OK, Body::from(encoded.bytes.clone())).into_response();
        r.headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("image/webp"));
        r
    };
    let h = response.headers_mut();
    h.insert(ETAG, etag);
    h.insert(CACHE_CONTROL, HeaderValue::from_static(BAKE_CACHE_CONTROL));
    Ok(response)
}
