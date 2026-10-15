//! NPC tokens of a composed block (adapter A13; logic/12 §scene-tokens).
//!
//! Placement is global (review 3.6-3): every stored notable of a
//! settlement goes to one building, its workplace by day (wherever that
//! lies) and its home otherwise, and one square of it, chosen from the
//! whole building as composed, whatever block asks. A block shows the
//! tokens whose square lies in it, so a notable stands in exactly one
//! block and two blocks never disagree.
//!
//! A building's notables stand in id order, each on the free square
//! nearest the building's centre (ties broken by a hash of the notable and
//! the world square): a floor square of the building with no water, no
//! blocking and no prop (furniture, a crate, a stall) on it.

use super::block::Block;
use arda_npc::Npc;
use arda_scene::RulesSidecar;
use arda_tactical::catalog::Layer;
use arda_tactical::Library;
use arda_town::plan::grid::SquareRect;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use ts_rs::TS;

/// Why a token stands where it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TokenKind {
    /// At its workplace (by day).
    Worker,
    /// At home (by night, or with no workplace).
    Resident,
}

/// One token on the scene.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Token {
    /// The NPC, a decimal string (resolve it with `GET /v1/npc/{id}`).
    pub npc_id: String,
    /// Display name.
    pub name: String,
    /// Square column within the block.
    pub x: u32,
    /// Square row within the block.
    pub y: u32,
    /// The building it stands in, a decimal string.
    pub building_id: String,
    /// Its settlement, a decimal string.
    pub settlement_id: String,
    /// `worker` (at its workplace) or `resident` (at home).
    pub kind: TokenKind,
}

fn text(v: &serde_json::Value) -> Option<u64> {
    v.as_str().and_then(|t| t.parse().ok())
}

/// The settlements whose buildings the rules tag (the town layer's
/// `ext.building` and `ext.settlement`).
#[must_use]
pub fn settlements(rules: &RulesSidecar) -> BTreeSet<u64> {
    rules
        .squares
        .iter()
        .filter_map(|c| c.ext.as_ref())
        .filter(|e| e.contains_key("building"))
        .filter_map(|e| e.get("settlement").and_then(text))
        .collect()
}

/// A composed block as the placer reads it, in world squares.
pub struct View {
    block: Block,
    /// World squares a prop stands on.
    furniture: BTreeSet<(i64, i64)>,
}

impl View {
    /// Reads `block`, resolving its placements with the render `seed`.
    #[must_use]
    pub fn new(block: Block, library: &Library, seed: u64) -> Self {
        let l = &block.layout;
        let [ox, oy] = block.origin;
        let furniture = arda_scene::squares::resolve_known(l, library, seed)
            .iter()
            .filter(|p| p.asset.layer == Layer::Prop)
            .flat_map(|p| arda_scene::squares::covered(p.rect, l.width, l.height))
            .map(|(x, y)| (ox + i64::from(x), oy + i64::from(y)))
            .collect();
        Self { block, furniture }
    }

    /// Whether the view shows every square of `r`.
    #[must_use]
    pub fn holds(&self, r: &SquareRect) -> bool {
        let [ox, oy] = self.block.origin;
        let (w, h) = (
            i64::from(self.block.layout.width),
            i64::from(self.block.layout.height),
        );
        r.x0 >= ox && r.y0 >= oy && r.x1 <= ox + w && r.y1 <= oy + h
    }

    fn index(&self, gx: i64, gy: i64) -> Option<usize> {
        let [ox, oy] = self.block.origin;
        let (x, y) = (u32::try_from(gx - ox).ok()?, u32::try_from(gy - oy).ok()?);
        let l = &self.block.layout;
        (x < l.width && y < l.height).then(|| y as usize * l.width as usize + x as usize)
    }

    /// Whether world square `(gx, gy)` is a square of building `b` of
    /// settlement `sid` in the composition.
    fn of_building(&self, (gx, gy): (i64, i64), sid: u64, b: u64) -> bool {
        let Some(rules) = &self.block.rules else {
            return false;
        };
        self.index(gx, gy)
            .and_then(|i| rules.squares.get(i))
            .and_then(|c| c.ext.as_ref())
            .is_some_and(|e| {
                e.get("building").and_then(text) == Some(b)
                    && e.get("settlement").and_then(text) == Some(sid)
            })
    }

    /// Whether a token may stand on world square `(gx, gy)`.
    fn free(&self, (gx, gy): (i64, i64)) -> bool {
        let Some(i) = self.index(gx, gy) else {
            return false;
        };
        let dry = self
            .block
            .layout
            .squares
            .get(i)
            .is_some_and(|s| s.water_depth_ft == 0);
        let open = self
            .block
            .rules
            .as_ref()
            .and_then(|r| r.squares.get(i))
            .is_some_and(|c| c.blocks_movement != Some(true) && c.water_depth_ft.unwrap_or(0) == 0);
        dry && open && !self.furniture.contains(&(gx, gy))
    }
}

/// Where `npcs` (one building's, in id order) stand in building `b` of
/// settlement `sid`, footprint `rect`, as composed in `view`: world
/// squares, one per notable while free squares last.
#[must_use]
pub fn spots<'n, T: Copy>(
    view: &View,
    (sid, b): (u64, u64),
    rect: &SquareRect,
    npcs: &[(&'n Npc, T)],
) -> Vec<(&'n Npc, T, (i64, i64))> {
    let squares: Vec<(i64, i64)> = (rect.y0..rect.y1)
        .flat_map(|y| (rect.x0..rect.x1).map(move |x| (x, y)))
        .filter(|&p| view.of_building(p, sid, b))
        .collect();
    let count = i64::try_from(squares.len()).unwrap_or(1).max(1);
    let (sx, sy) = squares
        .iter()
        .fold((0_i64, 0_i64), |a, &(x, y)| (a.0 + x, a.1 + y));
    let (cx, cy) = (sx.div_euclid(count), sy.div_euclid(count));
    let mut taken = BTreeSet::new();
    let mut out = Vec::new();
    for &(n, tag) in npcs {
        let d = |&(x, y): &(i64, i64)| x.abs_diff(cx).pow(2) + y.abs_diff(cy).pow(2);
        // A hash input: only the bits count.
        #[allow(clippy::cast_sign_loss)]
        let h = |&(x, y): &(i64, i64)| arda_ids::hash::mix(n.id.0 ^ ((x as u64) << 24) ^ y as u64);
        let spot = squares
            .iter()
            .filter(|&&p| view.free(p) && !taken.contains(&p))
            .min_by_key(|p| (d(p), h(p)));
        if let Some(&p) = spot {
            taken.insert(p);
            out.push((n, tag, p));
        }
    }
    out
}

/// The building each notable of a settlement stands in: its workplace by
/// day when the plan has it, else its home. Keyed by building, notables
/// in id order.
#[must_use]
pub fn targets<'n>(
    notables: &'n [Npc],
    plan: &arda_town::TownPlan,
    night: bool,
) -> BTreeMap<u64, Vec<(&'n Npc, TokenKind)>> {
    let has = |b: u64| plan.buildings.iter().any(|x| x.id.0 == b);
    let mut sorted: Vec<&Npc> = notables.iter().collect();
    sorted.sort_by_key(|n| n.id.0);
    let mut out: BTreeMap<u64, Vec<(&Npc, TokenKind)>> = BTreeMap::new();
    for n in sorted {
        let work = n
            .workplace_building
            .map(|b| b.0)
            .filter(|&b| !night && has(b));
        let (b, kind) = match work {
            Some(b) => (b, TokenKind::Worker),
            None => (n.home_building.0, TokenKind::Resident),
        };
        out.entry(b).or_default().push((n, kind));
    }
    out
}
