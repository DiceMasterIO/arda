//! `GET /v1/tactical/library/asset/{id}.png`: the image of any asset in the
//! loaded (stacked) library, so a consumer can draw effect and damage art
//! (1.0 task 05).

use super::routes::{image, log, TACTICAL_CACHE_CONTROL};
use super::{Encoded, Hit};
use crate::error::{ServerError, ServerResult};
use crate::routes::{segments, Segments};
use crate::AppState;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Response;
use std::sync::Arc;
use std::time::Instant;

/// Longest asset id accepted.
const MAX_ID_LEN: usize = 96;

/// Whether `id` looks like an asset id: lowercase letters, digits, `_` and
/// `.`, with no empty part (`prop.barrel`, `effect.frost.0`).
fn well_formed(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_LEN
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'.')
        && id.split('.').all(|part| !part.is_empty())
}

/// `GET /library/asset/{file}`: `file` is the asset id plus `.png`.
pub(super) async fn asset_png(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    path: Segments,
) -> ServerResult<Response> {
    let start = Instant::now();
    let p = segments(path, 1)?;
    let id = p[0]
        .strip_suffix(".png")
        .filter(|id| well_formed(id))
        .ok_or_else(|| {
            ServerError::BadRequest(format!(
                "{:?} is not an asset image name; expected `<asset id>.png`",
                p[0]
            ))
        })?;
    let lib = state.tactical.library();
    let img = lib
        .asset(id)
        .and_then(|a| lib.image(&a.id))
        .ok_or_else(|| ServerError::NotFound(format!("no asset {id:?} in the library")))?;
    let png = super::encode::encode_png(img.width, img.height, &img.data)?;
    let encoded = Encoded::new(png);
    let ms = log("library.asset", id, Hit::Miss, encoded.bytes.len(), start);
    Ok(image(
        &headers,
        &encoded,
        "image/png",
        TACTICAL_CACHE_CONTROL,
        Hit::Miss,
        ms,
    ))
}

#[cfg(test)]
mod tests {
    use super::well_formed;

    #[test]
    fn ids_are_checked_for_shape() {
        for ok in [
            "prop.barrel",
            "effect.frost.0",
            "ground.grass.1",
            "wall.stone.run",
        ] {
            assert!(well_formed(ok), "{ok}");
        }
        for bad in [
            "",
            ".",
            "a..b",
            ".a",
            "a.",
            "../x",
            "Prop.barrel",
            "a b",
            "a/b",
            "é",
        ] {
            assert!(!well_formed(bad), "{bad}");
        }
        assert!(!well_formed(&"a".repeat(97)));
    }
}
