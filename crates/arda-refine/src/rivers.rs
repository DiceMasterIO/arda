//! River centrelines at square scale (goal 46; logic/09 §linear-features).
//!
//! A saved channel edge `A → B` between orthogonal neighbours crosses their
//! shared boundary once. On recipe-9 worlds every river cell stores its
//! formed thalweg (`water.bin` layout 3), and the crossing is where the line
//! between the two thalwegs meets the boundary, so the drawn channel runs
//! through the channel the fine terrain formed; elsewhere it falls back to a
//! point hashed from the pair. A diagonal D8 step `A → D` is routed through
//! the via cell, the one of the two orthogonal cells beside both that lies
//! lower (ties to the lower `(gy, gx)`), making two orthogonal crossings and
//! a short piece across the via cell's corner instead of squeezing through
//! the four-cell corner point (review 3.4-7). Everything is a function of
//! the cells, edges and thalwegs involved, so every block that draws a
//! channel finds the same crossings.
//!
//! Inside a cell the channel runs from each entry crossing to the cell's
//! node (its thalweg, or the valley floor near its centre on older worlds)
//! and on to the exit as a cubic Hermite curve whose tangent at a crossing
//! is the direction between the anchors either side of it: the curve is
//! C1-continuous across block edges. A gentle bow with zero slope at both
//! ends adds the meander. The water surface follows the smooth terrain
//! under the centreline, never above the ground beside it.

use crate::context::{Ctx, N, SQUARE_M};
use crate::hash::{hash2, signed, unit};
use crate::source::{CellKey, Edge};

/// Narrowest drawn half-width in squares: a brook still fills one square.
pub const MIN_HALF_WIDTH: f64 = 0.72;
/// Sample spacing along a centreline, in squares.
const STEP: f64 = 0.4;
/// Closest a crossing comes to a cell corner, squares.
const CORNER_CLEAR: f64 = 4.0;

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
/// shared boundary when no thalwegs place it: a point hashed from the pair,
/// 16 to 48 squares along the edge, in global square units. Symmetric in
/// `a` and `b`.
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

/// The node a cell's channels pass through: its formed thalweg on worlds
/// that store one, otherwise the lowest point of the smooth terrain on a
/// small lattice around the cell centre, lightly biased toward the centre
/// and tie-broken by the seed. It depends only on the cell, so every block
/// that draws the channel finds the same node.
#[must_use]
pub fn node(ctx: &Ctx, c: CellKey) -> (f64, f64) {
    if let Some(t) = ctx.thalweg_at(c) {
        return t;
    }
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

/// The via cell of a diagonal step `a → d`: of the two orthogonal cells
/// beside both, the lower (cell height), ties to the lower `(gy, gx)`
/// (logic/09 §linear-features). Every block that sees `a` or `d` computes it.
#[must_use]
pub fn via(ctx: &Ctx, a: CellKey, d: CellKey) -> CellKey {
    let (b, c) = (CellKey::new(d.x, a.y), CellKey::new(a.x, d.y));
    let key = |k: CellKey| (ctx.cell_at(k).height.raw(), k.y, k.x);
    if key(b) <= key(c) {
        b
    } else {
        c
    }
}

/// The point a diagonal step's piece across its via cell `v` bends at:
/// halfway between the corner the step would have squeezed through and the
/// via cell's centre.
fn via_point(a: CellKey, d: CellKey, v: CellKey) -> (f64, f64) {
    let n = N as f64;
    let corner = (a.x.max(d.x) as f64 * n, a.y.max(d.y) as f64 * n);
    let (cx, cy) = centre(v);
    ((corner.0 + cx) * 0.5, (corner.1 + cy) * 0.5)
}

/// A cell and the point its channel is anchored at, if known.
type Anchor = (CellKey, Option<(f64, f64)>);

/// Where the channel between orthogonal neighbours `a` and `b` crosses
/// their boundary: on the line between their anchors when both are known
/// (kept clear of the corners), else the hashed point. Symmetric.
fn edge_crossing(seed: u64, (a, pa): Anchor, (b, pb): Anchor) -> (f64, f64) {
    let (Some(pa), Some(pb)) = (pa, pb) else {
        return crossing(seed, a, b);
    };
    // Order by cell so both sides compute with the same operands.
    let ((lo, plo), (hi, phi)) = if a <= b {
        ((a, pa), (b, pb))
    } else {
        ((b, pb), (a, pa))
    };
    let n = N as f64;
    if hi.x != lo.x {
        let x = lo.x.max(hi.x) as f64 * n;
        let y0 = lo.y as f64 * n;
        let t = ((x - plo.0) / (phi.0 - plo.0)).clamp(0.0, 1.0);
        let y = plo.1 + (phi.1 - plo.1) * t;
        (x, y.clamp(y0 + CORNER_CLEAR, y0 + n - CORNER_CLEAR))
    } else {
        let y = lo.y.max(hi.y) as f64 * n;
        let x0 = lo.x as f64 * n;
        let t = ((y - plo.1) / (phi.1 - plo.1)).clamp(0.0, 1.0);
        let x = plo.0 + (phi.0 - plo.0) * t;
        (x.clamp(x0 + CORNER_CLEAR, x0 + n - CORNER_CLEAR), y)
    }
}

/// One orthogonal crossing of an edge's course: the cells either side, the
/// point and the course's direction there.
#[derive(Debug, Clone, Copy)]
struct Leg {
    from: CellKey,
    to: CellKey,
    at: (f64, f64),
    tangent: (f64, f64),
}

fn leg(seed: u64, (a, pa): Anchor, (b, pb): Anchor) -> Leg {
    let at = edge_crossing(seed, (a, pa), (b, pb));
    let tangent = match (pa, pb) {
        (Some(pa), Some(pb)) => norm(sub(pb, pa)),
        _ => norm(sub(centre(b), centre(a))),
    };
    Leg {
        from: a,
        to: b,
        at,
        tangent,
    }
}

/// A diagonal step's two crossings and the bend point across its via cell.
type Through = (Leg, Leg, (f64, f64));

/// The crossings of edge `e`'s course: one for an orthogonal step, two
/// through the via cell for a diagonal one, with the bend point there.
fn course(ctx: &Ctx, e: &Edge) -> (Leg, Leg, Option<(f64, f64)>) {
    let anchor = |c: CellKey| ctx.thalweg_at(c);
    if e.to.x != e.from.x && e.to.y != e.from.y {
        let v = via(ctx, e.from, e.to);
        let pv = via_point(e.from, e.to, v);
        let first = leg(ctx.seed, (e.from, anchor(e.from)), (v, Some(pv)));
        let second = leg(ctx.seed, (v, Some(pv)), (e.to, anchor(e.to)));
        (first, second, Some(pv))
    } else {
        let only = leg(ctx.seed, (e.from, anchor(e.from)), (e.to, anchor(e.to)));
        (only, only, None)
    }
}

fn through(ctx: &Ctx, e: &Edge) -> Option<Through> {
    let (first, second, bend) = course(ctx, e);
    bend.map(|b| (first, second, b))
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

fn diagonal(e: &Edge) -> bool {
    e.to.x != e.from.x && e.to.y != e.from.y
}

/// The cells whose pieces draw the edges of `ctx`: every edge's two cells
/// and the via cell of every diagonal step, sorted and unique.
#[must_use]
pub fn river_cells(ctx: &Ctx) -> Vec<CellKey> {
    let mut cells: Vec<CellKey> = Vec::new();
    for e in &ctx.edges {
        cells.push(e.from);
        cells.push(e.to);
        if diagonal(e) {
            cells.push(via(ctx, e.from, e.to));
        }
    }
    cells.sort();
    cells.dedup();
    cells
}

/// The edges entering, leaving and passing through `c` (as a via cell),
/// main (largest discharge) first.
fn ends(ctx: &Ctx, c: CellKey) -> (Vec<Edge>, Vec<Edge>, Vec<Edge>) {
    let key = |e: &Edge| (std::cmp::Reverse(e.discharge_milli), e.from, e.to);
    let pick = |f: &dyn Fn(&Edge) -> bool| {
        let mut v: Vec<Edge> = ctx.edges.iter().filter(|e| f(e)).copied().collect();
        v.sort_by_key(key);
        v
    };
    (
        pick(&|e| e.to == c),
        pick(&|e| e.from == c),
        pick(&|e| diagonal(e) && via(ctx, e.from, e.to) == c),
    )
}

struct End {
    at: (f64, f64),
    tangent: (f64, f64),
    half: f64,
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
        p.surface.push(0.0);
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

/// Depth of the water surface below the lowest smooth terrain at and
/// beside the centreline, metres.
const INCISION: f64 = 0.35;
/// How far beyond the bank the surface looks for lower ground, squares.
const SURFACE_RING: f64 = 1.5;
/// Unit directions of the surface ring.
const RING: [(f64, f64); 8] = [
    (1.0, 0.0),
    (
        std::f64::consts::FRAC_1_SQRT_2,
        std::f64::consts::FRAC_1_SQRT_2,
    ),
    (0.0, 1.0),
    (
        -std::f64::consts::FRAC_1_SQRT_2,
        std::f64::consts::FRAC_1_SQRT_2,
    ),
    (-1.0, 0.0),
    (
        -std::f64::consts::FRAC_1_SQRT_2,
        -std::f64::consts::FRAC_1_SQRT_2,
    ),
    (0.0, -1.0),
    (
        std::f64::consts::FRAC_1_SQRT_2,
        -std::f64::consts::FRAC_1_SQRT_2,
    ),
];

/// Sets each point's water surface from the smooth terrain at the point and
/// on a ring just beyond its banks: the lowest of them, less the incision,
/// so water never stands above the ground beside it (review 3.4-8). The
/// surface is a function of the point and its half-width alone, so pieces
/// meeting at a crossing agree.
fn settle(ctx: &Ctx, p: &mut Piece) {
    for ((s, &(x, y)), &half) in p.surface.iter_mut().zip(&p.pts).zip(&p.half) {
        let r = half + SURFACE_RING;
        let low = RING
            .iter()
            .map(|&(dx, dy)| ctx.base_height(x + r * dx, y + r * dy))
            .fold(ctx.base_height(x, y), f64::min);
        *s = low - INCISION;
    }
}

/// Every centreline piece of the cells within the block's edge radius.
#[must_use]
pub fn pieces(ctx: &Ctx) -> Vec<Piece> {
    let mut out = Vec::new();
    for c in river_cells(ctx) {
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
    let (inc, outs, across) = ends(ctx, c);
    // The crossing where each entering edge arrives and each leaving edge
    // departs.
    let arrive = |e: &Edge| course(ctx, e).1;
    let depart = |e: &Edge| course(ctx, e).0;
    let n = node(ctx, c);
    let first_in = inc.first().map(|e| arrive(e).at);
    let first_out = outs.first().map(|e| depart(e).at);
    let tangent = match (first_in, first_out) {
        (Some(i), Some(o)) => Some(norm(sub(o, i))),
        (None, Some(o)) => Some(norm(sub(o, n))),
        (Some(i), None) => Some(norm(sub(n, i))),
        (None, None) => None,
    };
    if let Some(tangent) = tangent {
        for e in &inc {
            let leg = arrive(e);
            let width = (e.from_width_dm + e.to_width_dm) / 2;
            let a = End {
                at: leg.at,
                tangent: leg.tangent,
                half: half_width(width),
                depth: channel_depth(width),
            };
            // With no outlet each branch keeps its own heading into the
            // node, so two branches ending together never form a cusp.
            let into = if outs.is_empty() {
                norm(sub(n, leg.at))
            } else {
                tangent
            };
            let b = End {
                at: n,
                tangent: into,
                half: half_width(e.to_width_dm),
                depth: channel_depth(e.to_width_dm),
            };
            let before = out.len();
            hermite(
                seed,
                (c.x * 8 + leg.from.x - c.x, c.y * 8 + leg.from.y - c.y),
                &a,
                &b,
                out,
            );
            tag_discharge(out, before, e.discharge_milli);
        }
        for e in &outs {
            let leg = depart(e);
            let width = (e.from_width_dm + e.to_width_dm) / 2;
            let from = if inc.is_empty() {
                norm(sub(leg.at, n))
            } else {
                tangent
            };
            let a = End {
                at: n,
                tangent: from,
                half: half_width(e.from_width_dm),
                depth: channel_depth(e.from_width_dm),
            };
            let b = End {
                at: leg.at,
                tangent: leg.tangent,
                half: half_width(width),
                depth: channel_depth(width),
            };
            let tag = (c.x * 8 + 4 + leg.to.x - c.x, c.y * 8 + 4 + leg.to.y - c.y);
            let before = out.len();
            hermite(seed, tag, &a, &b, out);
            tag_discharge(out, before, e.discharge_milli);
        }
    }
    // Diagonal steps across this via cell's corner: in at one crossing,
    // through the bend point, out at the other.
    for e in &across {
        let Some((first, second, bend)) = through(ctx, e) else {
            continue;
        };
        let width = (e.from_width_dm + e.to_width_dm) / 2;
        let (half, depth) = (half_width(width), channel_depth(width));
        let mid = norm(sub(second.at, first.at));
        let spans = [
            (first.at, first.tangent, bend, mid),
            (bend, mid, second.at, second.tangent),
        ];
        for (k, &(p0, t0, p1, t1)) in spans.iter().enumerate() {
            let before = out.len();
            hermite(
                seed,
                (c.x * 8 + 2 + i64::try_from(k).unwrap_or(0), c.y * 8 + 2),
                &End {
                    at: p0,
                    tangent: t0,
                    half,
                    depth,
                },
                &End {
                    at: p1,
                    tangent: t1,
                    half,
                    depth,
                },
                out,
            );
            tag_discharge(out, before, e.discharge_milli);
        }
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

    #[test]
    fn thalweg_crossings_lie_on_the_line_between_the_thalwegs() {
        let (a, b) = (CellKey::new(3, 4), CellKey::new(4, 4));
        let pa = (3.0 * 64.0 + 40.0, 4.0 * 64.0 + 10.0);
        let pb = (4.0 * 64.0 + 24.0, 4.0 * 64.0 + 50.0);
        let x = edge_crossing(42, (a, Some(pa)), (b, Some(pb)));
        assert_eq!(x, edge_crossing(42, (b, Some(pb)), (a, Some(pa))));
        assert_eq!(x.0, 4.0 * 64.0);
        // Halfway from a's thalweg (24 of 48 squares east).
        assert!(
            (x.1 - (4.0 * 64.0 + 10.0 + 40.0 * 24.0 / 48.0)).abs() < 1e-9,
            "{x:?}"
        );
        // Without both thalwegs the hashed point stands.
        assert_eq!(
            edge_crossing(42, (a, None), (b, Some(pb))),
            crossing(42, a, b)
        );
    }
}
