//! River centrelines at square scale (goal 46).
//!
//! A saved channel edge `A → B` crosses the boundary between the two cells
//! at a point fixed from the pair's coordinates and the seed alone, so both
//! blocks compute it without seeing each other. Inside a cell the channel
//! runs from each entry crossing to the cell's node (the valley floor near
//! its centre) and on to the exit as a
//! cubic Hermite curve whose tangent at every crossing is the direction
//! between the two cell centres: the curve is C1-continuous across block
//! edges. A gentle bow with zero slope at both ends adds the meander. The
//! water surface follows the smooth terrain under the centreline.

use crate::context::{Ctx, N, SQUARE_M};
use crate::hash::{hash2, signed, unit};
use crate::source::{CellKey, Edge};

/// Narrowest drawn half-width in squares: a brook still fills one square.
pub const MIN_HALF_WIDTH: f64 = 0.72;
/// Sample spacing along a centreline, in squares.
const STEP: f64 = 0.4;

/// A sampled centreline piece inside one cell.
#[derive(Debug, Clone, Default)]
pub struct Piece {
    /// Points in global square units.
    pub pts: Vec<(f64, f64)>,
    /// Half-width at each point, squares.
    pub half: Vec<f64>,
    /// Water surface at each point, metres.
    pub surface: Vec<f64>,
    /// Full channel depth at the thalweg at each point, metres.
    pub depth: Vec<f64>,
    /// Saved discharge of the channel edge the piece draws, milli-m³/s
    /// (relief tiles colour rivers by it, logic/17 §rivers).
    pub discharge_milli: u64,
}

fn pair_key(a: CellKey, b: CellKey) -> (CellKey, CellKey) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Where the channel between neighbouring cells `a` and `b` crosses their
/// shared boundary, in global square units. Symmetric in `a` and `b`.
#[must_use]
pub fn crossing(seed: u64, a: CellKey, b: CellKey) -> (f64, f64) {
    let (lo, hi) = pair_key(a, b);
    let (dx, dy) = (hi.x - lo.x, hi.y - lo.y);
    let n = N as f64;
    let t = 16.0
        + 32.0
            * unit(hash2(
                seed ^ 0x51_7E,
                0xC4,
                lo.x * 3 + hi.x,
                lo.y * 3 + hi.y,
            ));
    match (dx.abs(), dy.abs()) {
        (1, 0) => ((lo.x.max(hi.x) as f64) * n, lo.y as f64 * n + t),
        (0, 1) => (lo.x as f64 * n + t, (lo.y.max(hi.y) as f64) * n),
        (1, 1) => ((lo.x.max(hi.x) as f64) * n, (lo.y.max(hi.y) as f64) * n),
        _ => (
            (centre(lo).0 + centre(hi).0) * 0.5,
            (centre(lo).1 + centre(hi).1) * 0.5,
        ),
    }
}

/// The centre of a cell in global square units.
#[must_use]
pub fn centre(c: CellKey) -> (f64, f64) {
    (c.x as f64 * N as f64 + 32.0, c.y as f64 * N as f64 + 32.0)
}

/// The node a cell's channels pass through: the lowest point of the smooth
/// terrain on a small lattice around the cell centre, lightly biased toward
/// the centre and tie-broken by the seed. It depends only on the cell, so
/// every block that draws the channel finds the same node.
#[must_use]
pub fn node(ctx: &Ctx, c: CellKey) -> (f64, f64) {
    let (x, y) = centre(c);
    let jitter = |tag: u64| 1.5 * signed(hash2(ctx.seed, tag, c.x, c.y));
    let (jx, jy) = (jitter(0x40DE), jitter(0x40DF));
    let mut best = (f64::INFINITY, (x + jx, y + jy));
    for j in -3..=3 {
        for i in -3..=3 {
            let (dx, dy) = (f64::from(i) * 3.5 + jx, f64::from(j) * 3.5 + jy);
            let (px, py) = (x + dx, y + dy);
            let score = ctx.base_height(px, py) + 0.004 * (dx * dx + dy * dy);
            if score < best.0 {
                best = (score, (px, py));
            }
        }
    }
    best.1
}

fn norm(v: (f64, f64)) -> (f64, f64) {
    let l = (v.0 * v.0 + v.1 * v.1).sqrt();
    if l < 1e-9 {
        (1.0, 0.0)
    } else {
        (v.0 / l, v.1 / l)
    }
}

fn sub(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 - b.0, a.1 - b.1)
}

fn half_width(dm: u32) -> f64 {
    (f64::from(dm) / 10.0 / 2.0 / SQUARE_M).max(MIN_HALF_WIDTH)
}

/// Thalweg depth in metres for a channel of `width_dm`.
fn channel_depth(dm: u32) -> f64 {
    (0.3 + 0.12 * f64::from(dm) / 10.0).min(5.0)
}

/// Water level of a cell's node: lake or sea level for standing water,
/// otherwise the cell height.
fn level(ctx: &Ctx, c: CellKey) -> f64 {
    let cell = ctx.cell_at(c);
    match cell.terrain {
        arda::TerrainKind::Sea => 0.0,
        arda::TerrainKind::Lake => {
            ctx.lake_at(c)
                .map_or(f64::from(cell.height.raw()), |l| f64::from(l.surface_mm))
                / 1000.0
        }
        arda::TerrainKind::Land => f64::from(cell.height.raw()) / 1000.0,
    }
}

/// The edges entering and leaving `c`, main (largest discharge) first.
fn ends(ctx: &Ctx, c: CellKey) -> (Vec<Edge>, Vec<Edge>) {
    let key = |e: &Edge| (std::cmp::Reverse(e.discharge_milli), e.from, e.to);
    let mut inc: Vec<Edge> = ctx.edges.iter().filter(|e| e.to == c).copied().collect();
    let mut out: Vec<Edge> = ctx.edges.iter().filter(|e| e.from == c).copied().collect();
    inc.sort_by_key(key);
    out.sort_by_key(key);
    (inc, out)
}

struct End {
    at: (f64, f64),
    tangent: (f64, f64),
    half: f64,
    surface: f64,
    depth: f64,
}

fn hermite(seed: u64, tag: (i64, i64), a: &End, b: &End, pieces: &mut Vec<Piece>) {
    let chord = sub(b.at, a.at);
    let len = (chord.0 * chord.0 + chord.1 * chord.1).sqrt();
    if len < 1e-6 {
        return;
    }
    let perp = (-chord.1 / len, chord.0 / len);
    let calm = 1.0 - crate::noise::smoothstep(4.0, 16.0, a.half.max(b.half));
    let bow = signed(hash2(seed, 0xB0, tag.0, tag.1)) * (0.18 * len).min(6.0) * calm;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let steps = ((len * 1.3 / STEP).ceil() as usize).max(2);
    let mut p = Piece::default();
    let end_surface = b.surface.min(a.surface);
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let (t2, t3) = (t * t, t * t * t);
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;
        let s = 16.0 * t2 * (1.0 - t) * (1.0 - t) * bow;
        let x = h00 * a.at.0 + h10 * len * a.tangent.0 + h01 * b.at.0 + h11 * len * b.tangent.0;
        let y = h00 * a.at.1 + h10 * len * a.tangent.1 + h01 * b.at.1 + h11 * len * b.tangent.1;
        p.pts.push((x + perp.0 * s, y + perp.1 * s));
        p.half.push(a.half + (b.half - a.half) * t);
        p.surface.push(a.surface + (end_surface - a.surface) * t);
        p.depth.push(a.depth + (b.depth - a.depth) * t);
    }
    pieces.push(p);
}

/// Signed distance from the square centre `q` (global square units) to the
/// banks of segment `i - 1 → i` of `p`, and the fraction along it: the one
/// formula the blocks rasterise water with and [`crate::water`] repeats.
#[must_use]
pub fn bank_distance(p: &Piece, i: usize, q: (f64, f64)) -> (f64, f64) {
    let (a, b) = (p.pts[i - 1], p.pts[i]);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = (dx * dx + dy * dy).max(1e-12);
    let t = (((q.0 - a.0) * dx + (q.1 - a.1) * dy) / l2).clamp(0.0, 1.0);
    let (px, py) = (a.0 + dx * t, a.1 + dy * t);
    let dist = ((q.0 - px) * (q.0 - px) + (q.1 - py) * (q.1 - py)).sqrt();
    let half = p.half[i - 1] + (p.half[i] - p.half[i - 1]) * t;
    (dist - half, t)
}

/// Depth of the water surface below the smooth terrain under the
/// centreline, metres.
const INCISION: f64 = 0.35;

/// Sets each point's water surface from the smooth terrain beneath it, so
/// the channel lies in the terrain rather than perched on it. The surface
/// is a function of the point alone, so pieces meeting at a crossing agree.
fn settle(ctx: &Ctx, p: &mut Piece) {
    for (s, &(x, y)) in p.surface.iter_mut().zip(&p.pts) {
        *s = ctx.base_height(x, y) - INCISION;
    }
}

/// Every centreline piece of the cells within the block's edge radius.
#[must_use]
pub fn pieces(ctx: &Ctx) -> Vec<Piece> {
    let mut cells: Vec<CellKey> = ctx.edges.iter().flat_map(|e| [e.from, e.to]).collect();
    cells.sort();
    cells.dedup();
    let mut out = Vec::new();
    for c in cells {
        if (c.x - ctx.cell.x).abs() > crate::context::EDGE_R
            || (c.y - ctx.cell.y).abs() > crate::context::EDGE_R
        {
            continue;
        }
        cell_pieces_into(ctx, c, &mut out);
    }
    for p in &mut out {
        settle(ctx, p);
    }
    out
}

/// The centreline pieces of cell `c` alone, water surfaces settled. They
/// are the same pieces every block within [`crate::context::EDGE_R`] of
/// `c` draws, so any `ctx` gathered for such a block gives the same answer.
#[must_use]
pub fn cell_pieces(ctx: &Ctx, c: CellKey) -> Vec<Piece> {
    let mut out = Vec::new();
    cell_pieces_into(ctx, c, &mut out);
    for p in &mut out {
        settle(ctx, p);
    }
    out
}

fn cell_pieces_into(ctx: &Ctx, c: CellKey, out: &mut Vec<Piece>) {
    let seed = ctx.seed;
    let (inc, outs) = ends(ctx, c);
    let n = node(ctx, c);
    let first_in = inc.first().map(|e| crossing(seed, e.from, e.to));
    let first_out = outs.first().map(|e| crossing(seed, e.from, e.to));
    let tangent = match (first_in, first_out) {
        (Some(i), Some(o)) => norm(sub(o, i)),
        (None, Some(o)) => norm(sub(o, n)),
        (Some(i), None) => norm(sub(n, i)),
        (None, None) => return,
    };
    let here = level(ctx, c);
    let dir = |e: &Edge| norm(sub(centre(e.to), centre(e.from)));
    let mid = |e: &Edge| {
        let s = (level(ctx, e.from) + level(ctx, e.to)) * 0.5;
        s.min(level(ctx, e.from))
    };
    for e in &inc {
        let a = End {
            at: crossing(seed, e.from, e.to),
            tangent: dir(e),
            half: half_width((e.from_width_dm + e.to_width_dm) / 2),
            surface: mid(e),
            depth: channel_depth((e.from_width_dm + e.to_width_dm) / 2),
        };
        // With no outlet each branch keeps its own heading into the node,
        // so two branches ending together never form a cusp.
        let into = if outs.is_empty() {
            norm(sub(n, crossing(seed, e.from, e.to)))
        } else {
            tangent
        };
        let b = End {
            at: n,
            tangent: into,
            half: half_width(e.to_width_dm),
            surface: here,
            depth: channel_depth(e.to_width_dm),
        };
        let before = out.len();
        hermite(
            seed,
            (c.x * 8 + e.from.x - c.x, c.y * 8 + e.from.y - c.y),
            &a,
            &b,
            out,
        );
        tag_discharge(out, before, e.discharge_milli);
    }
    for e in &outs {
        let from = if inc.is_empty() {
            norm(sub(crossing(seed, e.from, e.to), n))
        } else {
            tangent
        };
        let a = End {
            at: n,
            tangent: from,
            half: half_width(e.from_width_dm),
            surface: here,
            depth: channel_depth(e.from_width_dm),
        };
        let b = End {
            at: crossing(seed, e.from, e.to),
            tangent: dir(e),
            half: half_width((e.from_width_dm + e.to_width_dm) / 2),
            surface: mid(e),
            depth: channel_depth((e.from_width_dm + e.to_width_dm) / 2),
        };
        let tag = (c.x * 8 + 4 + e.to.x - c.x, c.y * 8 + 4 + e.to.y - c.y);
        let before = out.len();
        hermite(seed, tag, &a, &b, out);
        tag_discharge(out, before, e.discharge_milli);
    }
}

fn tag_discharge(out: &mut [Piece], from: usize, discharge_milli: u64) {
    for p in out.iter_mut().skip(from) {
        p.discharge_milli = discharge_milli;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossings_are_symmetric_and_on_the_shared_edge() {
        let a = CellKey::new(10, 20);
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1)] {
            let b = a.offset(dx, dy);
            let p = crossing(42, a, b);
            assert_eq!(p, crossing(42, b, a));
            if dx == 1 && dy == 0 {
                assert_eq!(p.0, 11.0 * 64.0);
                assert!(p.1 >= 20.0 * 64.0 + 16.0 && p.1 <= 20.0 * 64.0 + 48.0);
            }
        }
    }
}
