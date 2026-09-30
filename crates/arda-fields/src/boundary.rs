//! Field boundaries: hedgerows, drystone walls and hurdle fences on every
//! edge of an enclosed field, with gates facing the lanes and the farms.

use crate::fields::{FieldKind, SQUARE_M2};
use crate::geom::{Edge, SQUARE_M};
use crate::input::{FieldInputs, Region};
use crate::plan::{Cover, Plan};
use arda_tactical::catalog::WallRole;
use std::collections::{BTreeMap, BTreeSet};

/// Slope (rise over run) above which mixed country builds walls.
const STONY_SLOPE: f64 = 0.12;
/// Paddocks smaller than this (m²) are hurdle-fenced on modest farms.
const PADDOCK_M2: f64 = 3_500.0;
/// How far a gate looks for a farm or village to face, in squares (800 m).
const TARGET_REACH: f64 = 512.0;

/// Cultures that build in stone wherever they farm.
#[must_use]
pub fn stone_culture(culture: &str) -> bool {
    matches!(
        culture.to_ascii_lowercase().as_str(),
        "dwarf" | "dwarven" | "highland"
    )
}

/// Cultures that keep hedgerows even in the hills.
#[must_use]
pub fn hedge_culture(culture: &str) -> bool {
    matches!(
        culture.to_ascii_lowercase().as_str(),
        "elf" | "elven" | "halfling"
    )
}

/// The boundary kit of an enclosed field, by culture, region, slope and size.
#[must_use]
pub fn kit_for(
    inputs: &FieldInputs<'_>,
    kind: FieldKind,
    slope: f64,
    squares: u32,
) -> &'static str {
    if !kind.enclosed() {
        return "";
    }
    let small = f64::from(squares) * SQUARE_M2 < PADDOCK_M2;
    if kind == FieldKind::Pasture && small && inputs.wealth < 120 {
        return "wattle";
    }
    if stone_culture(inputs.culture) {
        return "drystone";
    }
    if hedge_culture(inputs.culture) {
        return "hedge";
    }
    match inputs.region {
        Region::Lowland => "hedge",
        Region::Upland => "drystone",
        Region::Mixed if slope > STONY_SLOPE => "drystone",
        Region::Mixed => "hedge",
    }
}

/// One wall edge in the plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WallInfo {
    /// Run, door, window or gate.
    pub role: WallRole,
    /// Kit.
    pub kit: &'static str,
    /// Id of the field whose kit it uses, or `u64::MAX` for compounds.
    pub owner: u64,
}

/// Boundaries of every enclosed field plus compound walls.
#[derive(Debug, Clone, Default)]
pub struct Boundaries {
    /// Every wall edge.
    pub walls: BTreeMap<Edge, WallInfo>,
    /// Gate edges per field index (empty for unenclosed fields).
    pub gates: Vec<Vec<Edge>>,
    /// Boundary edges per field index with the cover beyond them.
    pub edges: Vec<Vec<(Edge, Cover)>>,
}

fn passable(c: Cover) -> Option<u8> {
    match c {
        Cover::Lane | Cover::Road(_) => Some(0),
        Cover::Field(_) | Cover::Wild | Cover::Rough | Cover::Built | Cover::Apron => Some(1),
        Cover::Water | Cover::Compound(_) => None,
    }
}

/// Where a field's gate should face: the nearest farm gate or settlement.
fn target(targets: &[[f64; 2]], c: [f64; 2]) -> [f64; 2] {
    let d2 = |t: &[f64; 2]| (t[0] - c[0]).powi(2) + (t[1] - c[1]).powi(2);
    targets
        .iter()
        .filter(|t| d2(t) <= TARGET_REACH * TARGET_REACH)
        .min_by(|a, b| d2(a).total_cmp(&d2(b)))
        .copied()
        .unwrap_or(c)
}

/// Chooses a field's gate: the passable edge facing a lane or road if any,
/// nearest the target; widened to two edges where the boundary runs straight.
fn gate(edges: &[(Edge, Cover)], t: [f64; 2]) -> Vec<Edge> {
    let cand: BTreeMap<Edge, u8> = edges
        .iter()
        .filter_map(|(e, c)| passable(*c).map(|s| (*e, s)))
        .collect();
    let d2 = |e: &Edge| {
        let m = e.midpoint();
        (m[0] - t[0]).powi(2) + (m[1] - t[1]).powi(2)
    };
    let best = cand
        .iter()
        .min_by(|(ea, sa), (eb, sb)| sa.cmp(sb).then(d2(ea).total_cmp(&d2(eb))).then(ea.cmp(eb)))
        .map(|(e, s)| (*e, *s));
    let Some((e, s)) = best else {
        // Nothing passable: a gate anyway, so the field is never sealed.
        return edges.iter().map(|(e, _)| *e).min().into_iter().collect();
    };
    let mut out = vec![e];
    if cand.get(&e.next()) == Some(&s) {
        out.push(e.next());
    } else if cand.get(&e.prev()) == Some(&s) {
        out.push(e.prev());
    }
    out
}

/// Builds every boundary, gate and compound wall of a plan.
#[must_use]
pub fn build(plan: &Plan) -> Boundaries {
    let n = plan.fields.len();
    let mut edges: Vec<Vec<(Edge, Cover)>> = vec![Vec::new(); n];
    let g = &plan.cover;
    for s in g.squares() {
        let a = plan.at(s);
        for (dx, dy) in [(1, 0), (0, 1)] {
            let t = s.offset(dx, dy);
            if !g.contains(t) {
                continue;
            }
            let b = plan.at(t);
            if a == b {
                continue;
            }
            let e = Edge::between(s, t);
            for (me, other) in [(a, b), (b, a)] {
                if let Cover::Field(i) = me {
                    let i = usize::try_from(i).unwrap_or(usize::MAX);
                    if plan.fields.get(i).is_some_and(|f| f.kind.enclosed()) {
                        edges[i].push((e, other));
                    }
                }
            }
        }
    }
    let mut targets: Vec<[f64; 2]> = plan
        .compounds
        .iter()
        .filter_map(|c| c.lane_start.map(|s| s.centre()))
        .collect();
    // Settlements are passed in metres; the plan works in squares.
    targets.extend(plan_settlements(plan));
    let mut gates = vec![Vec::new(); n];
    let mut gate_set = BTreeSet::new();
    for (i, f) in plan.fields.iter().enumerate() {
        if f.kind.enclosed() && !edges[i].is_empty() {
            gates[i] = gate(&edges[i], target(&targets, f.centroid()));
            gate_set.extend(gates[i].iter().copied());
        }
    }
    let mut walls: BTreeMap<Edge, WallInfo> = BTreeMap::new();
    for (i, f) in plan.fields.iter().enumerate() {
        for (e, other) in &edges[i] {
            let skip = match other {
                Cover::Water => true,
                Cover::Compound(c) => plan
                    .compounds
                    .get(usize::try_from(*c).unwrap_or(usize::MAX))
                    .is_none_or(|c| c.walled),
                _ => false,
            };
            if skip {
                continue;
            }
            let role = if gate_set.contains(e) {
                WallRole::Gate
            } else {
                WallRole::Run
            };
            let info = WallInfo {
                role,
                kit: f.kit,
                owner: f.id,
            };
            walls
                .entry(*e)
                .and_modify(|w| {
                    if f.id < w.owner {
                        w.kit = f.kit;
                        w.owner = f.id;
                    }
                })
                .or_insert(info);
        }
    }
    for c in &plan.compounds {
        for w in &c.walls {
            walls.insert(
                w.edge,
                WallInfo {
                    role: w.role,
                    kit: w.kit,
                    owner: u64::MAX,
                },
            );
        }
    }
    Boundaries {
        walls,
        gates,
        edges,
    }
}

fn plan_settlements(plan: &Plan) -> Vec<[f64; 2]> {
    plan.settlements
        .iter()
        .map(|s| [s[0] / SQUARE_M, s[1] / SQUARE_M])
        .collect()
}
