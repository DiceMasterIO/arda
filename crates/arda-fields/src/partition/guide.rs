//! Guides: the roads that blocks are split along, and the rivers whose
//! floodplains are kept as meadow.
//!
//! A road's centreline is the uniform quadratic B-spline of its polyline
//! (straight to the first and last edge midpoints, a quadratic piece about
//! every inner vertex), as `arda-ways` draws it, sampled into short chords
//! and carried into the partition's warped frame point by point, so a cut
//! along it runs under the road the tactical and relief layers draw.

use super::lattice::warp;
use super::poly::{bbox, closest_on_segment, dot, sub, unit, P};
use crate::geom::SQUARE_M;
use crate::input::{RiverLine, Road, RoadClass};

/// Samples per quadratic piece.
const PIECE_SAMPLES: usize = 4;
/// Longest chord of a guide, squares (straight runs are subdivided so a
/// region always sees the road's vertices).
const CHORD: f64 = 12.0;

/// A road centreline in warped square units.
#[derive(Debug, Clone, PartialEq)]
pub struct Guide {
    /// Dense vertices.
    pub pts: Vec<P>,
    /// Bounding box `[x0, y0, x1, y1]`.
    pub bbox: [f64; 4],
}

/// The B-spline of a polyline, sampled.
fn spline(pts: &[P]) -> Vec<P> {
    let n = pts.len();
    if n < 3 {
        return pts.to_vec();
    }
    let mid = |a: P, b: P| [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
    let mut out = vec![pts[0]];
    for i in 1..n - 1 {
        let (a, c, b) = (mid(pts[i - 1], pts[i]), pts[i], mid(pts[i], pts[i + 1]));
        for k in 0..PIECE_SAMPLES {
            #[allow(clippy::cast_precision_loss)] // a handful of samples
            let t = k as f64 / PIECE_SAMPLES as f64;
            let u = 1.0 - t;
            out.push([
                u * u * a[0] + 2.0 * u * t * c[0] + t * t * b[0],
                u * u * a[1] + 2.0 * u * t * c[1] + t * t * b[1],
            ]);
        }
    }
    out.push(pts[n - 1]);
    out
}

/// A polyline with no chord longer than [`CHORD`].
fn densify(pts: &[P]) -> Vec<P> {
    let mut out = Vec::with_capacity(pts.len());
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = (b[0] - a[0]).hypot(b[1] - a[1]);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // short runs
        let n = (len / CHORD).ceil().max(1.0) as usize;
        for k in 0..n {
            #[allow(clippy::cast_precision_loss)] // small counts
            let t = k as f64 / n as f64;
            out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
        }
    }
    out.extend(pts.last().copied());
    out
}

/// Road guides near the warped rectangle `r` (fields follow carriage
/// roads, tracks and highways, not footpaths).
#[must_use]
pub fn roads(roads: &[Road], seed: u64, bias: f64, r: [f64; 4]) -> Vec<Guide> {
    let mut out = Vec::new();
    for road in roads {
        if !matches!(
            road.class,
            RoadClass::Track | RoadClass::Road | RoadClass::Highway
        ) || road.points.len() < 2
        {
            continue;
        }
        let sq: Vec<P> = road
            .points
            .iter()
            .map(|m| [m[0] / SQUARE_M, m[1] / SQUARE_M])
            .collect();
        let pts: Vec<P> = densify(&spline(&sq))
            .into_iter()
            .map(|p| warp(seed, p, bias))
            .collect();
        let b = bbox(std::slice::from_ref(&pts));
        if b[0] <= r[2] && b[2] >= r[0] && b[1] <= r[3] && b[3] >= r[1] {
            out.push(Guide { pts, bbox: b });
        }
    }
    out
}

/// The stretch of a guide within `pad` squares of a box, with one vertex
/// either side; `None` when it does not come that close.
#[must_use]
pub fn stretch(g: &Guide, b: [f64; 4], pad: f64) -> Option<Vec<P>> {
    let near = |p: &P| {
        p[0] >= b[0] - pad && p[0] <= b[2] + pad && p[1] >= b[1] - pad && p[1] <= b[3] + pad
    };
    let first = g.pts.iter().position(near)?;
    let last = g.pts.iter().rposition(near)?;
    let lo = first.saturating_sub(1);
    let hi = (last + 1).min(g.pts.len() - 1);
    (hi > lo).then(|| g.pts[lo..=hi].to_vec())
}

/// A straight line fitted to points: centre, unit direction and the
/// largest distance of any point from it.
#[must_use]
pub fn fit(pts: &[P]) -> (P, P, f64) {
    #[allow(clippy::cast_precision_loss)] // short lists
    let n = pts.len().max(1) as f64;
    let c = [
        pts.iter().map(|p| p[0]).sum::<f64>() / n,
        pts.iter().map(|p| p[1]).sum::<f64>() / n,
    ];
    let (mut xx, mut xy, mut yy) = (0.0, 0.0, 0.0);
    for p in pts {
        let d = sub(*p, c);
        xx += d[0] * d[0];
        xy += d[0] * d[1];
        yy += d[1] * d[1];
    }
    let d = super::poly::major_axis([xx, xy, yy]);
    let nrm = [-d[1], d[0]];
    let dev = pts
        .iter()
        .map(|p| dot(sub(*p, c), nrm).abs())
        .fold(0.0, f64::max);
    (c, unit(d), dev)
}

/// Distance in metres from a real point (square units) to the nearest
/// river bank, and that river's width; `None` without rivers nearby.
#[must_use]
pub fn river_distance(rivers: &[RiverLine], p: P) -> Option<(f64, f64)> {
    let m = [p[0] * SQUARE_M, p[1] * SQUARE_M];
    rivers
        .iter()
        .map(|r| {
            let (q, _) = closest_on_segment(m, r.a, r.b);
            let d = (q[0] - m[0]).hypot(q[1] - m[1]);
            ((d - r.width_m / 2.0).max(0.0), r.width_m)
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spline_runs_through_edge_midpoints() {
        let s = spline(&[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]]);
        assert_eq!(s[0], [0.0, 0.0]);
        assert_eq!(s[1], [5.0, 0.0]);
        assert_eq!(*s.last().unwrap(), [10.0, 10.0]);
        let (_, d, dev) = fit(&[[0.0, 0.0], [5.0, 0.1], [10.0, 0.0]]);
        assert!(d[0].abs() > 0.99 && dev < 0.1);
    }
}
