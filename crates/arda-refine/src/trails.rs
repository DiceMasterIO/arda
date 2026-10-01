//! Game trails and footpaths across open country (goal 43): a sparse
//! network that links water, passes and settlements, follows low-cost
//! ground and runs on unbroken across block edges.
//!
//! The network is decided per cell edge, like a river crossing (logic/09
//! §linear-features): whether a trail crosses the edge between cells A and
//! B, and where, depends on A, B and the seed alone, so both blocks agree.
//! Inside a cell every crossing is joined to the cell's hub (its river
//! node, where the trail fords, or a hashed clearing) by the cheapest route
//! over the smooth terrain, which avoids steep ground and stream banks.
//! A route stays inside its cell, so a block needs the routes of its own
//! cell and its eight neighbours only, and every block computes them the
//! same way.

use crate::context::{Ctx, N, SQUARE_M};
use crate::grid::{span, Grid};
use crate::hash::{hash3, unit};
use crate::noise::fbm;
use crate::source::CellKey;
use arda::{RoadClass, TerrainKind};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Ground key of a trail or footpath square.
pub const TRAIL: &str = "trail";
/// Hash tag for trail edges.
const TAG: u64 = 0x7A11_0001;
/// Score an edge needs to carry a trail (tuned for roughly one trail cell
/// in four on open land).
const THRESHOLD: f64 = 0.8;
/// Steepest cell slope a trail crosses, degrees.
const MAX_SLOPE: f64 = 32.0;

/// Squares per cell side as a count.
const SIDE: usize = 64;

/// Row-major index of local square `(i, j)` of a cell (both in `0..N`).
fn idx(i: i64, j: i64) -> usize {
    usize::try_from(j * N + i).unwrap_or(0)
}

/// What a trail square is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Way {
    /// No trail.
    None,
    /// A faint game trail.
    Game,
    /// A footpath between settled places.
    Foot,
}

/// Trail squares over a rectangle of cells.
#[derive(Debug, Clone)]
pub struct Trails {
    /// Per global square.
    pub grid: Grid<Way>,
}

impl Trails {
    /// The way at a global square (none outside the computed cells).
    #[must_use]
    pub fn at(&self, x: i64, y: i64) -> Way {
        self.grid.get(x, y).copied().unwrap_or(Way::None)
    }

    /// Whether `(x, y)` or one of its four neighbours is a trail square.
    #[must_use]
    pub fn near(&self, x: i64, y: i64) -> bool {
        [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .any(|&(dx, dy)| self.at(x + dx, y + dy) != Way::None)
    }
}

fn settled(c: &arda::Cell) -> bool {
    c.built_by != 0 || c.road != RoadClass::None
}

fn slope(c: &arda::Cell) -> f64 {
    f64::from(c.slope_milli_deg) / 1000.0
}

/// A saddle: lower than both neighbours on one axis, higher on the other.
fn pass(ctx: &Ctx, k: CellKey) -> bool {
    let h = |dx: i64, dy: i64| ctx.cell_at(k.offset(dx, dy)).height.raw();
    let c = h(0, 0);
    let (n, s, e, w) = (h(0, -1), h(0, 1), h(1, 0), h(-1, 0));
    (n > c && s > c && e < c && w < c) || (e > c && w > c && n < c && s < c)
}

/// The way crossing the edge between neighbouring cells `a` and `b`, a
/// function of the two cells (and their four-neighbours) alone.
#[must_use]
pub fn edge_way(ctx: &Ctx, a: CellKey, b: CellKey) -> Way {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    let (ca, cb) = (ctx.cell_at(lo), ctx.cell_at(hi));
    if ca.terrain != TerrainKind::Land || cb.terrain != TerrainKind::Land {
        return Way::None;
    }
    let steep = slope(ca).max(slope(cb));
    if steep > MAX_SLOPE {
        return Way::None;
    }
    let water = ca.watercourse_order > 0 || cb.watercourse_order > 0;
    let big = ca.watercourse_width_dm.max(cb.watercourse_width_dm) > 80;
    let home = settled(ca) || settled(cb);
    let saddle = pass(ctx, lo) || pass(ctx, hi);
    let u = unit(hash3(
        ctx.seed,
        TAG,
        lo.x * 2 + hi.x - lo.x,
        lo.y * 2 + hi.y - lo.y,
        0,
    ));
    let s = u
        + 0.2 * f64::from(u8::from(water))
        + 0.3 * f64::from(u8::from(home))
        + 0.3 * f64::from(u8::from(saddle))
        - 0.006 * steep
        - 0.3 * f64::from(u8::from(big));
    if s <= THRESHOLD {
        Way::None
    } else if home {
        Way::Foot
    } else {
        Way::Game
    }
}

/// The square of cell `c` where the trail to neighbour `d` crosses their
/// shared edge (local coordinates).
fn crossing(seed: u64, c: CellKey, d: CellKey) -> (i64, i64) {
    let (lo, hi) = if c <= d { (c, d) } else { (d, c) };
    let along = 12
        + span(
            usize::try_from(
                hash3(seed, TAG, lo.x * 2 + hi.x - lo.x, lo.y * 2 + hi.y - lo.y, 1) % 40,
            )
            .unwrap_or(0),
        );
    match (d.x - c.x, d.y - c.y) {
        (1, _) => (N - 1, along),
        (-1, _) => (0, along),
        (_, 1) => (along, N - 1),
        _ => (along, 0),
    }
}

/// The hub of cell `c` in local squares: its river node, or a clearing.
fn hub(ctx: &Ctx, c: CellKey) -> (i64, i64) {
    if ctx.cell_at(c).watercourse_order > 0 {
        let (x, y) = crate::rivers::node(ctx, c);
        #[allow(clippy::cast_possible_truncation)]
        let local = ((x.floor() as i64) - c.x * N, (y.floor() as i64) - c.y * N);
        return (local.0.clamp(1, N - 2), local.1.clamp(1, N - 2));
    }
    let h = hash3(ctx.seed, TAG, c.x, c.y, 2);
    let pick = |v: u64| 16 + span(usize::try_from(v % 33).unwrap_or(0));
    (pick(h), pick(h >> 20))
}

/// Squares of cell `c` beside its own channels: costly to walk along.
fn banks(ctx: &Ctx, c: CellKey) -> Vec<bool> {
    let mut wet = vec![false; SIDE * SIDE];
    let (x0, y0) = (c.x * N, c.y * N);
    for p in crate::rivers::cell_pieces(ctx, c) {
        for i in 1..p.pts.len() {
            let (a, b) = (p.pts[i - 1], p.pts[i]);
            let r = p.half[i].max(p.half[i - 1]) + 1.5;
            #[allow(clippy::cast_possible_truncation)]
            let (ax, bx, ay, by) = (
                (a.0.min(b.0) - r).floor() as i64,
                (a.0.max(b.0) + r).ceil() as i64,
                (a.1.min(b.1) - r).floor() as i64,
                (a.1.max(b.1) + r).ceil() as i64,
            );
            for y in ay.max(y0)..by.min(y0 + N) {
                for x in ax.max(x0)..bx.min(x0 + N) {
                    let (d, _) =
                        crate::rivers::bank_distance(&p, i, (x as f64 + 0.5, y as f64 + 0.5));
                    if d < 1.0 {
                        wet[idx(x - x0, y - y0)] = true;
                    }
                }
            }
        }
    }
    wet
}

/// Cheapest routes from the hub of `c` to each of `ends`, written into
/// `out` with the given way.
fn route(ctx: &Ctx, c: CellKey, ends: &[(i64, i64)], way: &[Way], out: &mut Grid<Way>) {
    let n = SIDE;
    let (x0, y0) = (c.x * N, c.y * N);
    let mut height = vec![0.0; n * n];
    let mut rough = vec![0.0; n * n];
    for j in 0..N {
        for i in 0..N {
            let (u, v) = ((x0 + i) as f64 + 0.5, (y0 + j) as f64 + 0.5);
            let k = idx(i, j);
            height[k] = ctx.base_height(u, v);
            rough[k] = 0.5 + 0.5 * fbm(ctx.seed, TAG ^ 0x5, u, v, 9.0, 2, 0.5);
        }
    }
    let wet = banks(ctx, c);
    let step = |a: usize, b: usize, diag: bool| {
        let d = if diag { std::f64::consts::SQRT_2 } else { 1.0 };
        let grade = (height[b] - height[a]).abs() / (d * SQUARE_M);
        d * (1.0 + 40.0 * grade * grade + 1.2 * rough[b] + 12.0 * f64::from(u8::from(wet[b])))
    };
    let (hx, hy) = hub(ctx, c);
    let start = idx(hx, hy);
    let mut dist = vec![f64::INFINITY; n * n];
    let mut prev = vec![usize::MAX; n * n];
    let mut heap = BinaryHeap::new();
    dist[start] = 0.0;
    heap.push(Reverse((0_u64, start)));
    while let Some(Reverse((bits, a))) = heap.pop() {
        let da = f64::from_bits(bits);
        if da > dist[a] {
            continue;
        }
        let (ai, aj) = (span(a % n), span(a / n));
        for (dx, dy) in [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (-1, 1),
            (1, -1),
            (-1, -1),
        ] {
            let (bi, bj) = (ai + dx, aj + dy);
            if !(0..N).contains(&bi) || !(0..N).contains(&bj) {
                continue;
            }
            let b = idx(bi, bj);
            let nd = da + step(a, b, dx != 0 && dy != 0);
            if nd < dist[b] {
                dist[b] = nd;
                prev[b] = a;
                heap.push(Reverse((nd.to_bits(), b)));
            }
        }
    }
    for (&(ex, ey), &w) in ends.iter().zip(way) {
        let mut path = vec![(ex as f64 + 0.5, ey as f64 + 0.5)];
        let mut at = idx(ex, ey);
        while prev[at] != usize::MAX {
            at = prev[at];
            path.push((span(at % n) as f64 + 0.5, span(at / n) as f64 + 0.5));
        }
        let line = smooth(&path);
        let mut last: Option<(i64, i64)> = None;
        let mut mark = |i: i64, j: i64| {
            if let Some(s) = out.get_mut(x0 + i.clamp(0, N - 1), y0 + j.clamp(0, N - 1)) {
                *s = (*s).max(w);
            }
        };
        for k in 1..line.len() {
            let (a, b) = (line[k - 1], line[k]);
            let len = ((b.0 - a.0) * (b.0 - a.0) + (b.1 - a.1) * (b.1 - a.1)).sqrt();
            let steps = (len * 4.0).ceil().max(1.0);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            for t in 0..=(steps as u32) {
                let f = f64::from(t) / steps;
                #[allow(clippy::cast_possible_truncation)]
                let q = (
                    (a.0 + (b.0 - a.0) * f).floor() as i64,
                    (a.1 + (b.1 - a.1) * f).floor() as i64,
                );
                if let Some(l) = last {
                    if l.0 != q.0 && l.1 != q.1 {
                        // Keep the trail four-connected through a corner.
                        mark(q.0, l.1);
                    }
                }
                mark(q.0, q.1);
                last = Some(q);
            }
        }
    }
}

/// A Dijkstra path (square centres, crossing first) thinned to every
/// fourth point and rounded by three Chaikin passes, its ends fixed: a
/// trail that bends rather than stepping in 45° runs.
fn smooth(path: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut pts: Vec<(f64, f64)> = path.iter().step_by(4).copied().collect();
    if let (Some(&l), Some(&e)) = (path.last(), pts.last()) {
        if l != e {
            pts.push(l);
        }
    }
    for _ in 0..3 {
        if pts.len() < 3 {
            break;
        }
        let mut next = vec![pts[0]];
        for k in 1..pts.len() {
            let (a, b) = (pts[k - 1], pts[k]);
            if k > 1 {
                next.push((0.75 * a.0 + 0.25 * b.0, 0.75 * a.1 + 0.25 * b.1));
            }
            if k < pts.len() - 1 {
                next.push((0.25 * a.0 + 0.75 * b.0, 0.25 * a.1 + 0.75 * b.1));
            }
        }
        next.push(pts[pts.len() - 1]);
        pts = next;
    }
    pts
}

/// Trails of every cell within one cell of `ctx.cell`.
#[must_use]
pub fn trails(ctx: &Ctx) -> Trails {
    let c0 = ctx.cell.offset(-1, -1);
    let mut grid = Grid::new(c0.x * N, c0.y * N, 3 * SIDE, 3 * SIDE, Way::None);
    for dy in -1..=1 {
        for dx in -1..=1 {
            let c = ctx.cell.offset(dx, dy);
            let mut ends = Vec::new();
            let mut ways = Vec::new();
            for (ox, oy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let d = c.offset(ox, oy);
                let w = edge_way(ctx, c, d);
                if w != Way::None {
                    ends.push(crossing(ctx.seed, c, d));
                    ways.push(w);
                }
            }
            if !ends.is_empty() {
                route(ctx, c, &ends, &ways, &mut grid);
            }
        }
    }
    Trails { grid }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossings_meet_across_the_edge() {
        let (a, b) = (CellKey::new(5, 7), CellKey::new(6, 7));
        let (pa, pb) = (crossing(42, a, b), crossing(42, b, a));
        assert_eq!(pa.0, N - 1);
        assert_eq!(pb.0, 0);
        assert_eq!(pa.1, pb.1);
        let (c, d) = (CellKey::new(5, 7), CellKey::new(5, 8));
        let (pc, pd) = (crossing(42, c, d), crossing(42, d, c));
        assert_eq!((pc.0, pc.1, pd.1), (pd.0, N - 1, 0));
    }
}
