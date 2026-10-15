//! Request-keyed `ETag`s (logic/16 §api-cache; review 3.6-11).
//!
//! An `ETag` names what a response is made from, never its bytes: the
//! server version (every generator crate shares it), the served world's
//! identity (its seed, `world.json` and every file of `society/`), the
//! catalogue `library_version`, the route and the request's own
//! coordinates and options. So a client revalidating an image or body
//! gets `304 Not Modified` before anything is composed or rendered, cold
//! or warm, and two servers on the same world, library and release give
//! the same tags. Every image, tile and JSON body of the tactical,
//! overview and relief routes carries one with `Cache-Control: no-cache`.

use crate::error::{ServerError, ServerResult};
use axum::body::Body;
use axum::http::header::{CACHE_CONTROL, ETAG, IF_NONE_MATCH};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use std::path::Path;

/// Hex digits of an `ETag` (128 bits of BLAKE3).
const TAG_HEX: usize = 32;

/// The identity every tag of one server derives from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Etags {
    base: [u8; 32],
}

impl Default for Etags {
    /// The tags of a server with no world identity (tests).
    fn default() -> Self {
        Self::new(&[0; 32], 0, "")
    }
}

impl Etags {
    /// Tags for the world identified by `world` (see [`world_identity`])
    /// and seed `seed`, with catalogue `library_version`.
    #[must_use]
    pub fn new(world: &[u8; 32], seed: u64, library_version: &str) -> Self {
        let mut h = blake3::Hasher::new();
        h.update(b"arda-server/etag/v1\0");
        h.update(crate::versions::SERVER_VERSION.as_bytes());
        h.update(b"\0");
        h.update(world);
        h.update(&seed.to_le_bytes());
        h.update(library_version.as_bytes());
        Self {
            base: *h.finalize().as_bytes(),
        }
    }

    /// The quoted tag of `route` with request `parts`.
    #[must_use]
    pub fn of(&self, route: &str, parts: &[&dyn std::fmt::Display]) -> String {
        let mut h = blake3::Hasher::new_keyed(&self.base);
        h.update(route.as_bytes());
        for p in parts {
            h.update(b"\0");
            h.update(p.to_string().as_bytes());
        }
        let hex = h.finalize().to_hex();
        format!("\"{}\"", &hex.as_str()[..TAG_HEX])
    }
}

impl Etags {
    /// The tag of a request: its route, path segments and query (sorted
    /// by name, so their order in the URL does not matter).
    #[must_use]
    pub fn of_request(
        &self,
        route: &str,
        path: &[String],
        query: &std::collections::BTreeMap<String, String>,
    ) -> String {
        let path = path.join("/");
        let query: Vec<String> = query.iter().map(|(k, v)| format!("{k}={v}")).collect();
        self.of(route, &[&path, &query.join("&")])
    }
}

/// BLAKE3 over `world.json` and every regular file directly in
/// `society/`, by name: what the served bodies and images are made from
/// besides the request, the library and the release.
///
/// # Errors
/// [`ServerError::Internal`] when a file cannot be read.
pub fn world_identity(dir: &Path) -> ServerResult<[u8; 32]> {
    let read = |e: std::io::Error| ServerError::Internal(format!("world identity: {e}"));
    let mut h = blake3::Hasher::new();
    let mut add = |name: &str, path: &Path| -> ServerResult<()> {
        h.update(name.as_bytes());
        h.update(b"\0");
        let f = std::fs::File::open(path).map_err(read)?;
        h.update_reader(f).map_err(read)?;
        Ok(())
    };
    add("world.json", &dir.join("world.json"))?;
    let society = dir.join("society");
    if society.is_dir() {
        let mut files: Vec<_> = std::fs::read_dir(&society)
            .map_err(read)?
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        files.sort();
        for name in files {
            add(&format!("society/{name}"), &society.join(&name))?;
        }
    }
    Ok(*h.finalize().as_bytes())
}

/// Whether `If-None-Match` names `etag` (or is `*`).
#[must_use]
pub fn fresh(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get(IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.split(',').any(|t| t.trim() == etag || t.trim() == "*"))
}

/// `304 Not Modified` with the tag and `cache_control`.
#[must_use]
pub fn not_modified(etag: &str, cache_control: &'static str) -> Response {
    let mut r = (StatusCode::NOT_MODIFIED, Body::empty()).into_response();
    let h = r.headers_mut();
    h.insert(
        ETAG,
        HeaderValue::from_str(etag).unwrap_or(HeaderValue::from_static("\"\"")),
    );
    h.insert(CACHE_CONTROL, HeaderValue::from_static(cache_control));
    r
}

/// Adds the tag and `cache_control` to a response.
#[must_use]
pub fn tagged(mut r: Response, etag: &str, cache_control: &'static str) -> Response {
    let h = r.headers_mut();
    h.insert(
        ETAG,
        HeaderValue::from_str(etag).unwrap_or(HeaderValue::from_static("\"\"")),
    );
    h.insert(CACHE_CONTROL, HeaderValue::from_static(cache_control));
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_name_the_request_and_the_identity() {
        let a = Etags::new(&[1; 32], 42, "v1");
        let t = a.of("cell", &[&1, &2]);
        assert_eq!(t.len(), TAG_HEX + 2);
        assert_eq!(t, a.of("cell", &[&1, &2]));
        assert_ne!(t, a.of("cell", &[&12]));
        assert_ne!(t, a.of("cell.png", &[&1, &2]));
        assert_ne!(t, Etags::new(&[2; 32], 42, "v1").of("cell", &[&1, &2]));
        assert_ne!(t, Etags::new(&[1; 32], 43, "v1").of("cell", &[&1, &2]));
        assert_ne!(t, Etags::new(&[1; 32], 42, "v2").of("cell", &[&1, &2]));
    }

    #[test]
    fn if_none_match_lists_and_stars_match() {
        let mut h = HeaderMap::new();
        h.insert(IF_NONE_MATCH, HeaderValue::from_static("\"a\", \"b\""));
        assert!(fresh(&h, "\"b\""));
        assert!(!fresh(&h, "\"c\""));
        h.insert(IF_NONE_MATCH, HeaderValue::from_static("*"));
        assert!(fresh(&h, "\"c\""));
    }
}
