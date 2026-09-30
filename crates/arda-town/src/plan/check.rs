//! Plan invariants, exposed for tests and the CLI: no overlaps, every door
//! reachable from a street, a closed wall with working gates, frontage
//! statistics and the building mix actually built.

use super::grid::{Kind, PlanGrid};
use super::types::{Building, BuildingId, TownPlan};
use crate::function::BuildingFunction;
use std::collections::{BTreeMap, VecDeque};

/// Pairs of buildings whose footprints share a square.
#[must_use]
pub fn overlaps(plan: &TownPlan) -> Vec<(BuildingId, BuildingId)> {
    let b = &plan.buildings;
    let mut out = Vec::new();
    for i in 0..b.len() {
        for j in i + 1..b.len() {
            if b[i].rect.overlaps(&b[j].rect) {
                out.push((b[i].id, b[j].id));
            }
        }
    }
    out
}

fn reachable(g: &PlanGrid, b: &Building) -> bool {
    let mut starts: Vec<(i64, i64)> = b.doors.iter().map(|d| d.outside()).collect();
    if starts.is_empty() {
        let r = b.rect;
        for x in r.x0..r.x1 {
            starts.push((x, r.y0 - 1));
            starts.push((x, r.y1));
        }
        for y in r.y0..r.y1 {
            starts.push((r.x0 - 1, y));
            starts.push((r.x1, y));
        }
    }
    let own_plot = b.plot.map_or(0, |p| p.0 + 1);
    let mut seen = std::collections::BTreeSet::new();
    let mut q: VecDeque<(i64, i64)> = starts.into_iter().collect();
    let mut steps = 0;
    while let Some((x, y)) = q.pop_front() {
        steps += 1;
        if steps > 40_000 || !seen.insert((x, y)) {
            continue;
        }
        let Some(k) = g.gidx(x, y) else { continue };
        let kind = g.kind[k];
        if kind.public() {
            return true;
        }
        let passable = match kind {
            Kind::Open | Kind::Croft | Kind::Bailey => g.building[k] == 0,
            Kind::Front | Kind::Yard | Kind::Garden | Kind::Churchyard => g.plot[k] == own_plot,
            _ => false,
        };
        if passable {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                q.push_back((x + dx, y + dy));
            }
        }
    }
    false
}

/// Buildings whose doors do not lead to a street, square or gate without
/// crossing other plots, walls, water or buildings.
#[must_use]
pub fn unreachable(plan: &TownPlan) -> Vec<BuildingId> {
    plan.buildings
        .iter()
        .filter(|b| !reachable(&plan.grid, b))
        .map(|b| b.id)
        .collect()
}

/// Flood fill from the market; `gates_open` lets it pass gates.
/// Returns whether the fill escaped to the grid border.
#[must_use]
pub fn escapes(plan: &TownPlan, gates_open: bool) -> bool {
    let g = &plan.grid;
    let (si, sj) = g.cell_of(plan.focal.market);
    let mut seen = vec![false; g.kind.len()];
    let mut q = VecDeque::new();
    q.push_back((si, sj));
    while let Some((i, j)) = q.pop_front() {
        let Some(k) = g.idx(i, j) else { continue };
        if seen[k] {
            continue;
        }
        seen[k] = true;
        let blocked = match g.kind[k] {
            Kind::Wall => true,
            Kind::Gate | Kind::WaterGate => !gates_open,
            _ => false,
        };
        if blocked {
            continue;
        }
        if i == 0 || j == 0 || i == g.w - 1 || j == g.h - 1 {
            return true;
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            q.push_back((i + dx, j + dy));
        }
    }
    false
}

/// Frontage widths in metres, sorted.
#[must_use]
pub fn frontages(plan: &TownPlan) -> Vec<f64> {
    let mut v: Vec<f64> = plan.plots.iter().map(|p| p.frontage_m).collect();
    v.sort_by(f64::total_cmp);
    v
}

/// Non-ancillary buildings per function.
#[must_use]
pub fn built_mix(plan: &TownPlan) -> BTreeMap<BuildingFunction, u32> {
    let mut m = BTreeMap::new();
    for b in plan.buildings.iter().filter(|b| !b.ancillary) {
        *m.entry(b.function).or_default() += 1;
    }
    m
}

/// A one-line summary for logs.
#[must_use]
pub fn summary(plan: &TownPlan) -> String {
    let f = frontages(plan);
    let med = f.get(f.len() / 2).copied().unwrap_or(0.0);
    format!(
        "{}: {} streets, {} plots (median frontage {:.1} m), {} buildings, {} gates, {} overlaps, {} unreachable",
        plan.name,
        plan.streets.len(),
        plan.plots.len(),
        med,
        plan.buildings.len(),
        plan.wall.as_ref().map_or(0, |w| w.gates.len()),
        overlaps(plan).len(),
        unreachable(plan).len()
    )
}

/// Where a plan disagrees with the rivers, given the river water square
/// test `water(gx, gy)` (global squares): building footprints on the
/// water (docks and jetties excepted: they are decks over it), street
/// squares over the water that are not bridge deck, and deck rows that
/// stop short of a bank (or of a wall standing in the river). Empty when the plan keeps to its rivers.
#[must_use]
pub fn water(plan: &TownPlan, water: &dyn Fn(i64, i64) -> bool) -> Vec<String> {
    let g = &plan.grid;
    let mut out = Vec::new();
    for b in plan
        .buildings
        .iter()
        .filter(|b| b.function != BuildingFunction::Dock)
    {
        let r = b.rect;
        let wet = (r.y0..r.y1).any(|y| (r.x0..r.x1).any(|x| water(x, y)));
        if wet {
            out.push(format!(
                "building {} ({}) on water",
                b.id.0,
                b.function.key()
            ));
        }
    }
    for s in &plan.streets {
        let bad = super::bridge::samples(s)
            .into_iter()
            .map(|p| g.cell_of(p))
            .map(|(i, j)| (g.gx0 + i, g.gy0 + j))
            .filter(|&(x, y)| water(x, y))
            .find(|&(x, y)| {
                g.gidx(x, y)
                    .is_some_and(|k| !matches!(g.kind[k], Kind::Bridge | Kind::WaterGate))
            });
        if let Some((x, y)) = bad {
            out.push(format!("street {} over water at {x},{y}", s.id.0));
        }
    }
    for (k, &kind) in g.kind.iter().enumerate() {
        let (i, j) = g.ij(k);
        let (x, y) = (g.gx0 + i, g.gy0 + j);
        if kind.public() && kind != Kind::Bridge && water(x, y) {
            out.push(format!("{kind:?} square over water at {x},{y}"));
        }
    }
    for b in &plan.bridges {
        for &(across, from, to) in &b.rows {
            let sq = |along: i64| {
                if b.along_x {
                    (along, across)
                } else {
                    (across, along)
                }
            };
            let wet_grid = |along: i64| {
                let (x, y) = sq(along);
                g.gidx(x, y).is_some_and(|k| g.kind[k] == Kind::Water)
            };
            // A deck ends on land, or against a wall standing in the river.
            let ends_dry = [from - 1, to + 1].iter().all(|&e| {
                let (x, y) = sq(e);
                let walled = g.gidx(x, y).is_some_and(|k| {
                    matches!(g.kind[k], Kind::Wall | Kind::WaterGate | Kind::Gate)
                });
                !wet_grid(e) && (walled || !water(x, y))
            });
            if !ends_dry || (from..=to).any(wet_grid) {
                out.push(format!(
                    "bridge of street {} row {across} {from}..={to} short of a bank",
                    b.street.0
                ));
            }
        }
    }
    out
}
