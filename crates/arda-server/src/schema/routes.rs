//! `GET /v1/schema` (the index) and `GET /v1/schema/{Name}.json`: the
//! committed schema files, byte for byte.

use super::files;
use crate::error::{ServerError, ServerResult};
use crate::routes::{segments, Segments};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::HeaderValue;
use axum::response::{IntoResponse, Response};

fn body(name: &str) -> ServerResult<Response> {
    let text = files()?
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, t)| t.clone())
        .ok_or_else(|| ServerError::NotFound(format!("no schema {name:?}; see /v1/schema")))?;
    Ok((
        [
            (
                CONTENT_TYPE,
                HeaderValue::from_static("application/schema+json"),
            ),
            (
                CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=3600"),
            ),
        ],
        text,
    )
        .into_response())
}

/// `GET /v1/schema`.
pub(crate) async fn index() -> ServerResult<Response> {
    let mut r = body("index.json")?;
    r.headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    Ok(r)
}

/// `GET /v1/schema/{Name}.json`.
pub(crate) async fn one(path: Segments) -> ServerResult<Response> {
    let p = segments(path, 1)?;
    let name = &p[0];
    if !name.ends_with(".json") {
        return Err(ServerError::NotFound(format!(
            "no schema {name:?}; see /v1/schema"
        )));
    }
    body(name)
}
