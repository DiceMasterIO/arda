//! Windowed A* least-cost routing over the cost surface.

use crate::cost::{self, CostSurface, MIN_PCT};
use crate::grid::{Grid, OFFSETS8};
use crate::num::{iu, ui};
use crate::roads::RoadClass;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Margin around the endpoints' bounding box that a search may use, cells.
const MIN_MARGIN: i64 = 80;

/// A found route.
#[derive(Debug, Clone)]
pub struct Path {
    /// Grid cells from source to target.
    pub cells: Vec<usize>,
    /// Total cost, metre-equivalents.
    pub cost: u64,
}

/// Octile length of a cell path in metres.
#[must_use]
pub fn length_m(g: &Grid, cells: &[usize]) -> u64 {
    cells
        .windows(2)
        .map(|w| {
            let (ax, ay) = g.xy(w[0]);
            let (bx, by) = g.xy(w[1]);
            if ax != bx && ay != by {
                141
            } else {
                100
            }
        })
        .sum()
}

/// A route this many times its straight line is a runaway detour.
pub const DETOUR_CAP_PCT: u64 = 200;
/// Routes shorter than this in a straight line are never re-routed, metres.
const DETOUR_MIN_M: u64 = 1500;

/// Replaces a runaway detour: when a route over the existing network runs
/// more than [`DETOUR_CAP_PCT`] per cent of the straight line (typically
/// along a trunk that winds round a hill), a direct route that ignores the
/// network is tried and kept when it is shorter. The direct route still
/// reuses any road cells it crosses.
#[must_use]
pub fn straighten(g: &Grid, cs: &CostSurface, p: Path, class: RoadClass) -> Path {
    let (Some(&from), Some(&to)) = (p.cells.first(), p.cells.last()) else {
        return p;
    };
    let (ax, ay) = g.xy(from);
    let (bx, by) = g.xy(to);
    let straight = u64::from(crate::num::dist_m(bx - ax, by - ay));
    let len = length_m(g, &p.cells);
    if straight < DETOUR_MIN_M || len * 100 <= straight * DETOUR_CAP_PCT {
        return p;
    }
    let none = |_: usize| false;
    match find(g, cs, &none, from, to, class) {
        Some(q) if length_m(g, &q.cells) < len => q,
        _ => p,
    }
}

/// Least-cost path from `from` to `to`, searching a window around both.
/// `on_road` marks cells an existing road already occupies.
#[must_use]
pub fn find(
    g: &Grid,
    cs: &CostSurface,
    on_road: &impl Fn(usize) -> bool,
    from: usize,
    to: usize,
    class: RoadClass,
) -> Option<Path> {
    nearest(g, cs, on_road, from, &[to], class).map(|(_, p)| p)
}

/// Least-cost path from `from` to whichever of `targets` is cheapest to
/// reach, with its index in `targets`; one A* whose heuristic is the
/// lower bound to the nearest target, over a window around all of them.
#[must_use]
pub fn nearest(
    g: &Grid,
    cs: &CostSurface,
    on_road: &impl Fn(usize) -> bool,
    from: usize,
    targets: &[usize],
    class: RoadClass,
) -> Option<(usize, Path)> {
    let (fx, fy) = g.xy(from);
    let goals: Vec<(i64, i64)> = targets.iter().map(|&t| g.xy(t)).collect();
    let (mut x0, mut y0, mut x1, mut y1) = (fx, fy, fx, fy);
    for &(tx, ty) in &goals {
        (x0, y0, x1, y1) = (x0.min(tx), y0.min(ty), x1.max(tx), y1.max(ty));
    }
    let margin = MIN_MARGIN.max((x1 - x0).max(y1 - y0) / 2);
    let x0 = (x0 - margin).max(0);
    let y0 = (y0 - margin).max(0);
    let x1 = (x1 + margin).min(ui(g.width) - 1);
    let y1 = (y1 + margin).min(ui(g.height) - 1);
    let ww = iu(x1 - x0 + 1);
    let wh = iu(y1 - y0 + 1);
    let local = |i: usize| {
        let (x, y) = g.xy(i);
        iu(y - y0) * ww + iu(x - x0)
    };
    let global = |l: usize| g.at(x0 + ui(l % ww), y0 + ui(l / ww));
    let heuristic = |i: usize| {
        let (x, y) = g.xy(i);
        goals
            .iter()
            .map(|&(tx, ty)| {
                let (dx, dy) = ((x - tx).unsigned_abs(), (y - ty).unsigned_abs());
                let (lo, hi) = (dx.min(dy), dx.max(dy));
                (lo * 141 + (hi - lo) * 100) * MIN_PCT / 100
            })
            .min()
            .unwrap_or(0)
    };
    let mut dist = vec![u64::MAX; ww * wh];
    let mut prev = vec![u32::MAX; ww * wh];
    let mut heap = BinaryHeap::new();
    let start = local(from);
    dist[start] = 0;
    heap.push(Reverse((heuristic(from), 0_u64, start)));
    let goal_ix: Vec<usize> = targets.iter().map(|&t| local(t)).collect();
    let mut reached = None;
    while let Some(Reverse((_, d, l))) = heap.pop() {
        if d > dist[l] {
            continue;
        }
        if let Some(k) = goal_ix.iter().position(|&gl| gl == l) {
            reached = Some(k);
            break;
        }
        let Some(a) = global(l) else { continue };
        let (ax, ay) = g.xy(a);
        for &(dx, dy) in &OFFSETS8 {
            let (nx, ny) = (ax + dx, ay + dy);
            if nx < x0 || ny < y0 || nx > x1 || ny > y1 {
                continue;
            }
            let Some(b) = g.at(nx, ny) else { continue };
            let Some(c) = cost::step(g, cs, on_road, a, b, (dx, dy), class) else {
                continue;
            };
            let nd = d + c;
            let lb = local(b);
            if nd < dist[lb] {
                dist[lb] = nd;
                prev[lb] = u32::try_from(l).unwrap_or(u32::MAX);
                heap.push(Reverse((nd + heuristic(b), nd, lb)));
            }
        }
    }
    let k = reached?;
    let goal = goal_ix[k];
    let mut cells = vec![targets[k]];
    let mut l = goal;
    while l != start {
        let p = prev[l];
        if p == u32::MAX {
            return None;
        }
        l = usize::try_from(p).unwrap_or(0);
        cells.push(global(l)?);
    }
    cells.reverse();
    Some((
        k,
        Path {
            cells,
            cost: dist[goal],
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda::{Cover, TerrainKind};

    /// Open grass rising at a 25 % grade to the south.
    fn slope() -> (Grid, CostSurface) {
        let (w, h) = (80, 80);
        let mut g = Grid::sea(w, h, crate::grid::MEMORY_BUDGET).unwrap();
        for i in 0..g.len() {
            let (_, y) = g.xy(i);
            g.terrain[i] = TerrainKind::Land;
            g.cover[i] = Cover::Grass;
            g.height_mm[i] = i32::try_from(y * 25_000).unwrap();
        }
        let cs = CostSurface {
            base: vec![100; w * h],
        };
        (g, cs)
    }

    #[test]
    fn highways_switch_back_up_a_slope_that_footpaths_climb_straight() {
        let (g, cs) = slope();
        let none = |_: usize| false;
        let (a, b) = (g.at(40, 10).unwrap(), g.at(40, 50).unwrap());
        let s = |c: RoadClass| {
            let p = find(&g, &cs, &none, a, b, c).unwrap();
            length_m(&g, &p.cells) as f64 / 4000.0
        };
        let (hw, fp) = (s(RoadClass::Highway), s(RoadClass::Footpath));
        assert!(fp < 1.1, "footpath {fp}");
        assert!((1.3..=2.0).contains(&hw), "highway {hw}");
    }
}
