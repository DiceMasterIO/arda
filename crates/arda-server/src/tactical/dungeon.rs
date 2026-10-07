//! `GET /v1/tactical/dungeon`, `/dungeon.png` and `/dungeon/scene`: dungeon
//! and cave battle maps from `arda-dungeon`.
//!
//! The level is chosen by `?seed=N`, or by `?gx=&gy=` (a world cell), whose
//! seed is `arda_dungeon::site_seed(world seed, gx, gy, kind)`, so a world's
//! site always opens onto the same level. `kind` is `dungeon` (default) or
//! `cave`; `w` and `h` are the size in squares (default 48 × 36, each
//! 16–160). Images and scenes are drawn with the served library and world
//! seed, like every other tactical image.

use super::routes::{image, log, TACTICAL_CACHE_CONTROL};
use super::world_routes::cell_of;
use super::{Anchor, DEFAULT_PPSQ, PPSQ_OPTIONS};
use crate::error::{ServerError, ServerResult};
use crate::routes::{blocking, params, parse, Params};
use crate::AppState;
use arda_dungeon::{generate, site_seed, Dungeon, Kind};
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::Json;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

type Shared = State<Arc<AppState>>;

/// Default size in squares.
pub const DEFAULT_SIZE: (u32, u32) = (48, 36);

/// The generation parameters a query names.
fn level(state: &AppState, q: &BTreeMap<String, String>) -> ServerResult<arda_dungeon::Params> {
    let kind = match q.get("kind") {
        None => Kind::Dungeon,
        Some(k) => Kind::parse(k).map_err(|e| ServerError::BadRequest(e.to_string()))?,
    };
    let side =
        |name: &str, default: u32| q.get(name).map_or(Ok(default), |t| parse::<u32>(name, t));
    let (width, height) = (side("w", DEFAULT_SIZE.0)?, side("h", DEFAULT_SIZE.1)?);
    let seed = match (q.get("seed"), q.get("gx"), q.get("gy")) {
        (Some(s), None, None) => parse::<u64>("seed", s)?,
        (None, Some(gx), Some(gy)) => {
            let (gx, gy) = cell_of(state, gx, gy)?;
            let salt = match kind {
                Kind::Dungeon => 0,
                Kind::Cave => 1,
            };
            site_seed(
                state.tactical.world_seed(),
                i64::from(gx),
                i64::from(gy),
                salt,
            )
        }
        _ => {
            return Err(ServerError::BadRequest(
                "give either seed, or gx and gy (a world cell)".into(),
            ))
        }
    };
    let p = arda_dungeon::Params {
        seed,
        kind,
        width,
        height,
    };
    p.check()
        .map_err(|e| ServerError::BadRequest(e.to_string()))?;
    Ok(p)
}

fn build(state: &AppState, q: &BTreeMap<String, String>) -> ServerResult<Dungeon> {
    let p = level(state, q)?;
    generate(&p).map_err(|e| ServerError::Internal(e.to_string()))
}

/// `GET /dungeon`: the level, its rules sidecar and its description.
pub(super) async fn dungeon(State(state): Shared, query: Params) -> ServerResult<Response> {
    let start = Instant::now();
    let q = params(query)?;
    let d = blocking(state, move |s| build(s, &q)).await?;
    log("dungeon", &d.layout.name, super::Hit::Miss, 0, start);
    Ok((
        [(axum::http::header::CACHE_CONTROL, TACTICAL_CACHE_CONTROL)],
        Json(d),
    )
        .into_response())
}

/// `GET /dungeon/scene`: the `arda-scene` scene of the level.
pub(super) async fn dungeon_scene(State(state): Shared, query: Params) -> ServerResult<Response> {
    let start = Instant::now();
    let q = params(query)?;
    let scene = blocking(state, move |s| {
        let d = build(s, &q)?;
        arda_scene::build_scene(
            &d.layout,
            s.tactical.library(),
            s.tactical.world_seed(),
            Some(&d.rules),
        )
        .map_err(|e| ServerError::Internal(format!("scene: {e}")))
    })
    .await?;
    log("dungeon_scene", &scene.name, super::Hit::Miss, 0, start);
    Ok(Json(scene).into_response())
}

/// `GET /dungeon.png?ppsq=&grid=`: the painted level.
pub(super) async fn dungeon_png(
    State(state): Shared,
    headers: HeaderMap,
    query: Params,
) -> ServerResult<Response> {
    let start = Instant::now();
    let q = params(query)?;
    let ppsq = match q.get("ppsq") {
        None => DEFAULT_PPSQ,
        Some(t) => {
            let v = parse::<u32>("ppsq", t)?;
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
    let (name, (encoded, hit)) = blocking(state, move |s| {
        let d = build(s, &q)?;
        let t = &s.tactical;
        t.admit(&d.layout, ppsq)?;
        let key = t.key(&d.layout, Anchor::Name(d.layout.name.clone()), ppsq, grid)?;
        Ok((d.layout.name.clone(), t.png(&d.layout, &key)?))
    })
    .await?;
    let ms = log(
        "dungeon_png",
        &format!("{name} ppsq={ppsq} grid={}", u8::from(grid)),
        hit,
        encoded.bytes.len(),
        start,
    );
    Ok(image(
        &headers,
        &encoded,
        "image/png",
        TACTICAL_CACHE_CONTROL,
        hit,
        ms,
    ))
}
