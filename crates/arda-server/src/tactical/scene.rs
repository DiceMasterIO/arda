//! `GET /v1/tactical/cell/{gx}/{gy}/scene[?time=day|night]` (logic/16
//! §api-tactical, logic/12): the `arda-scene` scene of the same composed
//! block the images are drawn from (goal 48), with NPC tokens (A13).

use super::tokens::{self, Token};
use super::world_routes::{cell_of, opts, request, WINDOW_PPSQ};
use super::DEFAULT_PPSQ;
use crate::error::{ServerError, ServerResult};
use crate::routes::{blocking, params, segments, Params, Segments};
use crate::AppState;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Envelope version of the scene body.
pub const SCENE_FORMAT: u32 = 1;

/// The scene of one block and the NPCs on it.
#[derive(Debug, Clone, Serialize)]
pub struct TacticalScene {
    /// Envelope version ([`SCENE_FORMAT`]).
    pub tactical_format: u32,
    /// World square of the block's top-left square.
    pub origin_gs: [i64; 2],
    /// `day` or `night`.
    pub time: &'static str,
    /// The `arda-scene` scene (format 1).
    pub scene: arda_scene::Scene,
    /// NPC tokens.
    pub tokens: Vec<Token>,
}

/// Builds the scene of `req` with tokens.
///
/// # Errors
/// Block, scene or people failures.
pub fn build(
    state: &AppState,
    req: &super::block::BlockRequest,
    night: bool,
) -> ServerResult<TacticalScene> {
    let library = state.tactical.library();
    let block = state.tactical.blocks().block(req, library)?;
    let rules = block.rules.as_ref();
    let seed = state.tactical.world_seed();
    let scene = arda_scene::build_scene(&block.layout, library, seed, rules)
        .map_err(|e| ServerError::Internal(format!("scene: {e}")))?;
    let mut out = Vec::new();
    if let (Some(rules), Some(people)) = (rules, state.people.as_deref()) {
        let feet = tokens::footprints(rules);
        let settlements: BTreeSet<u64> = feet.keys().map(|k| k.0).collect();
        let mut taken = BTreeSet::new();
        for sid in settlements {
            let notables = people.notables_of(sid);
            tokens::place(rules, &feet, sid, notables, night, &mut taken, &mut out);
        }
    }
    Ok(TacticalScene {
        tactical_format: SCENE_FORMAT,
        origin_gs: block.origin,
        time: if night { "night" } else { "day" },
        scene,
        tokens: out,
    })
}

/// `GET /cell/{gx}/{gy}/scene`.
pub(super) async fn cell_scene(
    State(state): State<Arc<AppState>>,
    path: Segments,
    query: Params,
) -> ServerResult<Response> {
    let p = segments(path, 2)?;
    let (gx, gy) = cell_of(&state, &p[0], &p[1])?;
    let q: BTreeMap<String, String> = params(query)?;
    let night = match q.get("time").map(String::as_str) {
        None | Some("day") => false,
        Some("night") => true,
        Some(t) => {
            return Err(ServerError::BadRequest(format!(
                "time must be day or night, not {t:?}"
            )))
        }
    };
    let o = opts(&q, &WINDOW_PPSQ, DEFAULT_PPSQ)?;
    let req = request(gx, gy, &o);
    let body = blocking(state, move |s| build(s, &req, night)).await?;
    Ok(Json(body).into_response())
}
