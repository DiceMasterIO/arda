//! `GET /v1/tactical/cell/{gx}/{gy}/scene[?time=day|night]` (logic/16
//! §api-tactical, logic/12): the `arda-scene` scene of the same composed
//! block the images are drawn from (goal 48), with NPC tokens (A13).

use super::scene_dto::SceneDto;
use super::tokens::{self, Token};
use super::world_routes::{cell_of, opts, request, WINDOW_PPSQ};
use super::DEFAULT_PPSQ;
use crate::error::{ServerError, ServerResult};
use crate::routes::{blocking, params, segments, Params, Segments};
use crate::AppState;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use schemars::JsonSchema;
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use ts_rs::TS;

/// Envelope version of the scene body: the `tactical_format` family, shared
/// with `TacticalBlockDto` (logic/16 §api-versioning), so it can never
/// drift from [`super::dto::TACTICAL_FORMAT`].
pub const SCENE_FORMAT: u32 = super::dto::TACTICAL_FORMAT;

/// Time of day a scene's tokens are placed for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SceneTime {
    /// Notables stand in their workplace when it lies in the block.
    Day,
    /// Everyone is at home.
    Night,
}

/// `GET /v1/tactical/cell/{gx}/{gy}/scene`: the scene of one block and the
/// NPCs on it.
#[derive(Debug, Clone, Serialize, TS, JsonSchema)]
pub struct TacticalScene {
    /// Envelope version ([`SCENE_FORMAT`]).
    pub tactical_format: u32,
    /// World square of the block's top-left square.
    #[ts(type = "[number, number]")]
    pub origin_gs: [i64; 2],
    /// `day` or `night`.
    pub time: SceneTime,
    /// The `arda-scene` scene (format 1), described by [`SceneDto`].
    #[ts(as = "SceneDto")]
    #[schemars(with = "SceneDto")]
    pub scene: arda_scene::Scene,
    /// NPC tokens.
    pub tokens: Vec<Token>,
}

/// Builds the scene of `req` with tokens, from the block the cell body
/// and image come from (review 3.5-8, 3.6-4).
///
/// # Errors
/// Block, scene or people failures.
pub fn build(
    state: &AppState,
    req: &super::block::BlockRequest,
    night: bool,
) -> ServerResult<TacticalScene> {
    let tactical = &state.tactical;
    let library = tactical.library();
    let (_, world) = super::world_routes::extent(state);
    let block = tactical.world_block_cut(req, world)?;
    let seed = tactical.world_render_seed();
    let scene = arda_scene::build_scene(&block.layout, library, seed, block.rules.as_ref())
        .map_err(|e| ServerError::Internal(format!("scene: {e}")))?;
    let tokens = match (&block.rules, state.people.as_deref()) {
        (Some(rules), Some(people)) => place_tokens(state, req, world, rules, people, night)?,
        _ => Vec::new(),
    };
    Ok(TacticalScene {
        tactical_format: SCENE_FORMAT,
        origin_gs: block.origin,
        time: if night {
            SceneTime::Night
        } else {
            SceneTime::Day
        },
        scene,
        tokens,
    })
}

/// The tokens standing in `req` (logic/12 §scene-tokens): every notable
/// of each settlement the block shows is placed globally, in the whole
/// building as composed, and kept when its square lies in the block.
fn place_tokens(
    state: &AppState,
    req: &super::block::BlockRequest,
    world: (i64, i64),
    rules: &arda_scene::RulesSidecar,
    people: &arda_people::World,
    night: bool,
) -> ServerResult<Vec<Token>> {
    let tactical = &state.tactical;
    let (library, seed) = (tactical.library(), tactical.world_render_seed());
    // The apron block holds every building that reaches a few squares
    // past the block; a larger one is read from its own window.
    let apron = req.with_apron(super::cells::APRON, world);
    let main = tokens::View::new(tactical.world_block(&apron)?, library, seed);
    let inside = |(gx, gy): (i64, i64)| {
        let (x, y) = (gx - req.gsx0, gy - req.gsy0);
        let x = u32::try_from(x).ok().filter(|&x| x < req.w)?;
        let y = u32::try_from(y).ok().filter(|&y| y < req.h)?;
        Some((x, y))
    };
    let meets = |r: &arda_town::plan::grid::SquareRect| {
        r.x0 < req.gsx0 + i64::from(req.w)
            && r.y0 < req.gsy0 + i64::from(req.h)
            && r.x1 > req.gsx0
            && r.y1 > req.gsy0
    };
    let mut out = Vec::new();
    for sid in tokens::settlements(rules) {
        let plan = people
            .plan(sid)
            .map_err(|e| ServerError::Internal(format!("plan {sid}: {e}")))?;
        let Some(plan) = plan.as_ref() else { continue };
        for (b, npcs) in tokens::targets(people.notables_of(sid), plan, night) {
            let Some(building) = plan.buildings.iter().find(|x| x.id.0 == b) else {
                continue;
            };
            let rect = building.rect;
            if !meets(&rect) {
                continue;
            }
            let own;
            let view = if main.holds(&rect) {
                &main
            } else {
                let r = super::block::BlockRequest {
                    gsx0: rect.x0.max(0),
                    gsy0: rect.y0.max(0),
                    w: u32::try_from(rect.x1.min(world.0) - rect.x0.max(0)).unwrap_or(1),
                    h: u32::try_from(rect.y1.min(world.1) - rect.y0.max(0)).unwrap_or(1),
                    demo_at: None,
                };
                own = tokens::View::new(tactical.world_block_cut(&r, world)?, library, seed);
                &own
            };
            for (n, kind, at) in tokens::spots(view, (sid, b), &rect, &npcs) {
                let Some((x, y)) = inside(at) else { continue };
                out.push(Token {
                    npc_id: n.id.0.to_string(),
                    name: n.name.full(),
                    x,
                    y,
                    building_id: b.to_string(),
                    settlement_id: sid.to_string(),
                    kind,
                });
            }
        }
    }
    Ok(out)
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
