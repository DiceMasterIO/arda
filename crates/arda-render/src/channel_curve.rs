//! Recipe-5 river centrelines as smooth curves (logic/04 §atlas-formed
//! rivers): each saved D8 edge becomes a B-spline arc over its snapped
//! nodes, drawn as a run of convex quads with shared normals, and a stream
//! tapers to a point at its source (goal 28).
//!
//! Tangents depend only on the saved channel network around a node, so
//! every render that sees the node (area, halo, overview) draws the same arc.
use crate::channel_geometry::{hull, Point, Polygon};
use crate::RenderError;
use arda_core::GlobalCell;
use std::collections::BTreeMap;

/// Width at a source node relative to the saved width, Q12.
const SOURCE_TAPER_Q12: i64 = 1_024;
/// Most sub-segments per edge.
const MAX_SEGMENTS: i64 = 8;

/// Main-stem neighbours of every channel node.
pub(crate) struct Network {
    /// Downstream node of the largest outflow from each node.
    next: BTreeMap<GlobalCell, (u64, GlobalCell)>,
    /// Upstream node of the largest inflow into each node.
    prev: BTreeMap<GlobalCell, (u64, GlobalCell)>,
}

impl Network {
    /// Indexes directed edges `(from, to, discharge)`; terminal points are ignored.
    pub(crate) fn new(edges: impl IntoIterator<Item = (GlobalCell, GlobalCell, u64)>) -> Self {
        let mut next: BTreeMap<GlobalCell, (u64, GlobalCell)> = BTreeMap::new();
        let mut prev: BTreeMap<GlobalCell, (u64, GlobalCell)> = BTreeMap::new();
        // Ties go to the larger node so the choice is order-independent.
        let better = |old: Option<&(u64, GlobalCell)>, q: u64, n: GlobalCell| {
            old.is_none_or(|&(oq, on)| (q, n) > (oq, on))
        };
        for (from, to, q) in edges {
            if from == to {
                continue;
            }
            if better(next.get(&from), q, to) {
                next.insert(from, (q, to));
            }
            if better(prev.get(&to), q, from) {
                prev.insert(to, (q, from));
            }
        }
        Self { next, prev }
    }

    /// Node positions relaxed once along main stems, `(prev + 2 p + next) / 4`,
    /// so the D8 staircase between confluences eases into natural runs
    /// before the spline. Sources, mouths and nodes without both main-stem
    /// neighbours keep their position.
    pub(crate) fn relax(&self, at: &BTreeMap<GlobalCell, Point>) -> BTreeMap<GlobalCell, Point> {
        at.iter()
            .map(|(&cell, &p)| (cell, self.relaxed(cell, p, |n| at.get(&n).copied())))
            .collect()
    }

    /// One node's relaxed position (see [`Self::relax`]) from its own
    /// position `p` and the positions `at` knows.
    pub(crate) fn relaxed(
        &self,
        cell: GlobalCell,
        p: Point,
        at: impl Fn(GlobalCell) -> Option<Point>,
    ) -> Point {
        let before = self.prev.get(&cell).and_then(|&(_, n)| at(n));
        let after = self.next.get(&cell).and_then(|&(_, n)| at(n));
        match (before, after) {
            (Some(a), Some(b)) => ((a.0 + 2 * p.0 + b.0) / 4, (a.1 + 2 * p.1 + b.1) / 4),
            _ => p,
        }
    }

    /// Main-stem upstream and downstream nodes of `node`.
    pub(crate) fn main_stem(&self, node: GlobalCell) -> (Option<GlobalCell>, Option<GlobalCell>) {
        (
            self.prev.get(&node).map(|&(_, n)| n),
            self.next.get(&node).map(|&(_, n)| n),
        )
    }

    /// Whether no channel flows into `node`, so a stream starts there.
    pub(crate) fn is_source(&self, node: GlobalCell) -> bool {
        !self.prev.contains_key(&node)
    }

    /// Upstream and downstream tangent neighbours of the edge `from -> to`:
    /// the main-stem inflow of `from`, and the outflow of `to` when this edge
    /// is that node's main stem (a tributary keeps its own direction).
    pub(crate) fn neighbours(
        &self,
        from: GlobalCell,
        to: GlobalCell,
    ) -> (Option<GlobalCell>, Option<GlobalCell>) {
        let before = self.prev.get(&from).map(|&(_, n)| n);
        let main = self.prev.get(&to).is_some_and(|&(_, n)| n == from);
        let after = if main {
            self.next.get(&to).map(|&(_, n)| n)
        } else {
            None
        };
        (before, after)
    }
}

/// Sub-segments for an edge drawn `pixels_per_cell` wide: about one per two
/// pixels, so small exports keep a single straight quad.
pub(crate) fn segments(pixels_per_cell: i64) -> i64 {
    (pixels_per_cell / 2).clamp(1, MAX_SEGMENTS)
}

/// Minimum on-screen river width by discharge (litres per second), in
/// 1/256 pixel: none below 0.7 m³/s, then growing with log2 discharge from
/// 0.5 px at 1 m³/s through 1.3 px near 5 m³/s and 2.5 px near 50 m³/s, so
/// rivers widen steadily downstream at every export size (goal 28).
pub(crate) fn min_width_px_q8(discharge: u64) -> i64 {
    if discharge < 700 {
        return 0;
    }
    // log2(q / 1000) in Q8: integer part from the bit length, fraction
    // linear in the mantissa.
    let bits = i64::from(63 - discharge.leading_zeros());
    let base = 1_u64 << bits;
    let frac = i64::try_from((discharge - base) * 256 / base).unwrap_or(0);
    let log2_q8 = bits * 256 + frac - 2_551; // log2(1000) = 9.966
    (128 + 90 * log2_q8 / 256).clamp(0, 1_024)
}

/// Tapered start width for a source node.
pub(crate) fn source_width(width: i64) -> i64 {
    (width * SOURCE_TAPER_Q12 / 4_096).max(1)
}

fn perp(d: (i128, i128), width: i64) -> Result<Point, RenderError> {
    let len = (d.0 * d.0 + d.1 * d.1).isqrt();
    if len == 0 {
        return Err(RenderError::ChannelGeometry {
            reason: "river curve has a zero tangent",
        });
    }
    let w = i128::from(width);
    let conv = |v: i128| {
        i64::try_from(v).map_err(|_| RenderError::ChannelGeometry {
            reason: "river curve normal exceeds i64",
        })
    };
    Ok((conv(-d.1 * w / (2 * len))?, conv(d.0 * w / (2 * len))?))
}

/// Point and (scaled) tangent of the uniform cubic B-spline over `p` at
/// `t = i / n`, the span between `p[1]` and `p[2]`.
pub(crate) fn bspline_at(p: [(i128, i128); 4], i: i128, n: i128) -> ((i128, i128), (i128, i128)) {
    // Basis at t = i / n, scaled by n^3 (value, sum 6 n^3) and n^2 (slope).
    let j = n - i;
    let b = [
        j * j * j,
        3 * i * i * i - 6 * i * i * n + 4 * n * n * n,
        -3 * i * i * i + 3 * i * i * n + 3 * i * n * n + n * n * n,
        i * i * i,
    ];
    let d = [
        -3 * j * j,
        9 * i * i - 12 * i * n,
        -9 * i * i + 6 * i * n + 3 * n * n,
        3 * i * i,
    ];
    let at = |c: fn((i128, i128)) -> i128| {
        let value: i128 = b.iter().zip(p).map(|(w, q)| w * c(q)).sum();
        let slope: i128 = d.iter().zip(p).map(|(w, q)| w * c(q)).sum();
        (value.div_euclid(6 * n * n * n), slope)
    };
    let (x, dx) = at(|q| q.0);
    let (y, dy) = at(|q| q.1);
    ((x, y), (dx, dy))
}

/// Convex pieces of one curved edge and its end caps at `a` and `b`.
pub(crate) type CurvedStrip = (Vec<Polygon>, [Point; 2], [Point; 2]);

/// Curved strip for one edge `a -> b`, with optional neighbours `before`
/// (upstream of `a`) and `after` (downstream of `b`). The centreline is a
/// uniform cubic B-spline over the chain, so D8 staircases smooth into
/// natural runs; a missing neighbour is mirrored, so a source or a
/// tributary mouth ends exactly on its node. Returns the convex pieces and
/// the end caps at `a` and `b` for node joins.
pub(crate) fn curved_strip(
    before: Option<Point>,
    a: Point,
    b: Point,
    after: Option<Point>,
    (width_a, width_b): (i64, i64),
    segments: i64,
) -> Result<CurvedStrip, RenderError> {
    let bad = |reason| RenderError::ChannelGeometry { reason };
    if a == b || width_a < 0 || width_b < 0 || segments < 1 {
        return Err(bad(
            "river curve has zero length, negative width or no segments",
        ));
    }
    let wide = |p: Point| (i128::from(p.0), i128::from(p.1));
    let (p1, p2) = (wide(a), wide(b));
    let p0 = before.map_or((2 * p1.0 - p2.0, 2 * p1.1 - p2.1), wide);
    let p3 = after.map_or((2 * p2.0 - p1.0, 2 * p2.1 - p1.1), wide);
    let n = i128::from(segments);
    let mut left = Vec::new();
    let mut right = Vec::new();
    for i in 0..=n {
        let ((x, y), (dx, dy)) = bspline_at([p0, p1, p2, p3], i, n);
        let p = (
            i64::try_from(x).map_err(|_| bad("river curve point exceeds i64"))?,
            i64::try_from(y).map_err(|_| bad("river curve point exceeds i64"))?,
        );
        let d = if (dx, dy) == (0, 0) {
            (p2.0 - p1.0, p2.1 - p1.1)
        } else {
            (dx, dy)
        };
        let w = i64::try_from((i128::from(width_a) * (n - i) + i128::from(width_b) * i) / n)
            .map_err(|_| bad("river curve width exceeds i64"))?;
        let o = perp(d, w)?;
        left.push((p.0 + o.0, p.1 + o.1));
        right.push((p.0 - o.0, p.1 - o.1));
    }
    let mut pieces = Vec::new();
    for i in 0..left.len() - 1 {
        let quad = hull(vec![left[i], left[i + 1], right[i + 1], right[i]])?;
        if quad.len() >= 3 {
            pieces.push(quad);
        }
    }
    let last = left.len() - 1;
    Ok((pieces, [left[0], right[0]], [left[last], right[last]]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel_geometry::{area2, Q};

    fn cell(x: u32, y: u32) -> GlobalCell {
        GlobalCell { x, y }
    }

    #[test]
    fn straight_chain_stays_straight_with_exact_width() {
        let (pieces, a, b) = curved_strip(
            Some((-Q, 0)),
            (0, 0),
            (Q, 0),
            Some((2 * Q, 0)),
            (Q / 4, Q / 4),
            4,
        )
        .unwrap();
        assert_eq!(pieces.len(), 4);
        let total: i128 = pieces.iter().map(|p| area2(p)).sum();
        assert_eq!(total, 2 * i128::from(Q) * i128::from(Q / 4));
        assert_eq!(a, [(0, Q / 8), (0, -Q / 8)]);
        assert_eq!(b, [(Q, Q / 8), (Q, -Q / 8)]);
    }

    #[test]
    fn a_bend_rounds_off_and_joins_with_shared_normals() {
        // An L-shaped D8 chain: (0,0) -> (1,0) -> (1,1).
        let p = [(0, 0), (Q, 0), (Q, Q)];
        let first = curved_strip(None, p[0], p[1], Some(p[2]), (Q / 8, Q / 8), 8).unwrap();
        let second = curved_strip(Some(p[0]), p[1], p[2], None, (Q / 8, Q / 8), 8).unwrap();
        // Shared node: identical caps, so the curve has no kink or gap.
        assert_eq!(first.2, second.1);
        // The corner is cut: the arc leaves the node grid toward the inside.
        let end = (
            (first.2[0].0 + first.2[1].0) / 2,
            (first.2[0].1 + first.2[1].1) / 2,
        );
        assert_eq!(end, (5 * Q / 6, Q / 6));
    }

    #[test]
    fn network_marks_sources_and_main_stem_neighbours() {
        let net = Network::new([
            (cell(0, 0), cell(1, 0), 100),
            (cell(1, 0), cell(2, 0), 500),
            (cell(1, 1), cell(2, 0), 90),
            (cell(2, 0), cell(3, 0), 700),
        ]);
        assert!(net.is_source(cell(0, 0)));
        assert!(!net.is_source(cell(2, 0)));
        assert_eq!(
            net.neighbours(cell(1, 0), cell(2, 0)),
            (Some(cell(0, 0)), Some(cell(3, 0)))
        );
        // A tributary keeps its own direction into the confluence.
        assert_eq!(net.neighbours(cell(1, 1), cell(2, 0)), (None, None));
    }

    #[test]
    fn minimum_width_grows_steadily_with_discharge() {
        assert_eq!(min_width_px_q8(500), 0);
        let w: Vec<i64> = [1_000, 5_000, 50_000, 500_000]
            .map(min_width_px_q8)
            .to_vec();
        assert!(w.windows(2).all(|p| p[0] < p[1]));
        assert!((120..136).contains(&w[0]));
        assert!((320..350).contains(&w[1]));
        assert!((610..660).contains(&w[2]));
    }

    #[test]
    fn relaxing_eases_a_staircase_but_keeps_ends() {
        let net = Network::new([
            (cell(0, 0), cell(1, 0), 100),
            (cell(1, 0), cell(2, 1), 100),
            (cell(2, 1), cell(3, 1), 100),
        ]);
        let at: BTreeMap<_, _> = [
            ((0, 0), (0, 0)),
            ((1, 0), (4, 0)),
            ((2, 1), (8, 4)),
            ((3, 1), (12, 4)),
        ]
        .into_iter()
        .map(|((x, y), p)| (cell(x, y), p))
        .collect();
        let r = net.relax(&at);
        assert_eq!(r[&cell(0, 0)], (0, 0));
        assert_eq!(r[&cell(3, 1)], (12, 4));
        assert_eq!(r[&cell(1, 0)], (4, 1));
        assert_eq!(r[&cell(2, 1)], (8, 3));
    }

    #[test]
    fn segment_count_follows_drawn_scale() {
        assert_eq!(segments(1), 1);
        assert_eq!(segments(8), 4);
        assert_eq!(segments(64), MAX_SEGMENTS);
    }
}
