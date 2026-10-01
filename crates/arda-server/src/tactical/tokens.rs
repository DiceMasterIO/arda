//! NPC tokens of a composed block (adapter A13; logic/12 §scene-tokens):
//! the stored notables of every settlement whose buildings lie in the
//! block, placed in their workplace by day (else their home) and in their
//! home by night, on a free floor square nearest the building's centre.

use arda_npc::Npc;
use arda_scene::RulesSidecar;
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
    /// At home (by night, or with no workplace in the block).
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

/// `(settlement, building)` → its squares in the block, row-major order.
type Footprints = BTreeMap<(u64, u64), Vec<(u32, u32)>>;

fn text(v: &serde_json::Value) -> Option<u64> {
    v.as_str().and_then(|t| t.parse().ok())
}

/// The plan buildings of the block, from the town layer's `ext.building`
/// and `ext.settlement` tags.
#[must_use]
pub fn footprints(rules: &RulesSidecar) -> Footprints {
    let mut out: Footprints = BTreeMap::new();
    for (i, cell) in rules.squares.iter().enumerate() {
        let Some(ext) = &cell.ext else { continue };
        let (Some(b), Some(s)) = (
            ext.get("building").and_then(text),
            ext.get("settlement").and_then(text),
        ) else {
            continue;
        };
        let w = rules.width.max(1) as usize;
        let (x, y) = (i % w, i / w);
        if let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) {
            out.entry((s, b)).or_default().push((x, y));
        }
    }
    out
}

fn free(rules: &RulesSidecar, (x, y): (u32, u32)) -> bool {
    rules
        .cell(x, y)
        .is_some_and(|c| c.blocks_movement != Some(true) && c.water_depth_ft.unwrap_or(0) == 0)
}

/// Places `notables` of settlement `sid`. `night` sends everyone home.
pub fn place(
    rules: &RulesSidecar,
    feet: &Footprints,
    sid: u64,
    notables: &[Npc],
    night: bool,
    taken: &mut BTreeSet<(u32, u32)>,
    out: &mut Vec<Token>,
) {
    for n in notables {
        let work = n.workplace_building.filter(|_| !night).map(|b| b.0);
        let (building, kind) = match work {
            Some(b) if feet.contains_key(&(sid, b)) => (b, TokenKind::Worker),
            _ => (n.home_building.0, TokenKind::Resident),
        };
        let Some(squares) = feet.get(&(sid, building)) else {
            continue;
        };
        let count = u64::try_from(squares.len()).unwrap_or(1).max(1);
        let (sx, sy) = squares.iter().fold((0_u64, 0_u64), |a, &(x, y)| {
            (a.0 + u64::from(x), a.1 + u64::from(y))
        });
        let (cx, cy) = (sx / count, sy / count);
        let d = |&(x, y): &(u32, u32)| {
            u64::from(x).abs_diff(cx).pow(2) + u64::from(y).abs_diff(cy).pow(2)
        };
        let spot = squares
            .iter()
            .filter(|&&p| free(rules, p) && !taken.contains(&p))
            .min_by_key(|p| {
                (
                    d(p),
                    arda_ids::hash::mix(n.id.0 ^ (u64::from(p.0) << 20) ^ u64::from(p.1)),
                )
            });
        if let Some(&p) = spot {
            taken.insert(p);
            out.push(Token {
                npc_id: n.id.0.to_string(),
                name: n.name.full(),
                x: p.0,
                y: p.1,
                building_id: building.to_string(),
                settlement_id: sid.to_string(),
                kind,
            });
        }
    }
}
