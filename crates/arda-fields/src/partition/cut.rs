//! Cut lines of the field partition: straight, kinked or following a road,
//! all as one polyline whose ends run on as rays.
//!
//! A point's side is the sign against the nearest part of the line (a
//! segment's normal, or at a vertex the sum of its two segments' normals),
//! so a cut divides the whole plane in two. Its distance to the line is
//! exact; since every part of a cut outside its own region lies outside
//! every leaf below it, the distance from a point to its leaf's edge is
//! the least distance to the cuts above it (and the block outline).

use super::poly::{dot, perp, sub, unit, P};

/// How far the end rays reach, squares (far past any block).
const RAY: f64 = 1.0e6;

/// A cut: a polyline with its end segments extended into rays.
#[derive(Debug, Clone, PartialEq)]
pub struct Cut {
    pts: Vec<P>,
}

/// The nearest point of a cut.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Near {
    /// Distance, squares.
    pub dist: f64,
    /// The nearest point on the cut.
    pub at: P,
    /// Side of the query point: `true` on the left-normal side.
    pub left: bool,
}

impl Cut {
    /// A straight cut through `c` along direction `d`.
    #[must_use]
    pub fn line(c: P, d: P) -> Self {
        let d = unit(d);
        Self::new(vec![c], d, d)
    }

    /// A cut through `c` arriving along `d1` and leaving along `d2`.
    #[must_use]
    pub fn kinked(c: P, d1: P, d2: P) -> Self {
        Self::new(vec![c], unit(d1), unit(d2))
    }

    /// A cut along the polyline `pts` (at least two points), its ends
    /// continued in the directions of the end segments.
    #[must_use]
    pub fn along(pts: &[P]) -> Self {
        let n = pts.len();
        if n < 2 {
            return Self::line(pts.first().copied().unwrap_or([0.0; 2]), [1.0, 0.0]);
        }
        let d1 = unit(sub(pts[1], pts[0]));
        let d2 = unit(sub(pts[n - 1], pts[n - 2]));
        Self::new(pts.to_vec(), d1, d2)
    }

    fn new(mut mid: Vec<P>, d1: P, d2: P) -> Self {
        let (a, b) = (mid[0], mid[mid.len() - 1]);
        let mut pts = Vec::with_capacity(mid.len() + 2);
        pts.push([a[0] - d1[0] * RAY, a[1] - d1[1] * RAY]);
        pts.append(&mut mid);
        pts.push([b[0] + d2[0] * RAY, b[1] + d2[1] * RAY]);
        Self { pts }
    }

    /// The nearest point of the cut to `p` and `p`'s side.
    #[must_use]
    pub fn near(&self, p: P) -> Near {
        let n = self.pts.len();
        // (distance², segment, parameter class: 0 start, 1 inside, 2 end)
        let mut best = (f64::INFINITY, 0_usize, 1_u8, p);
        for i in 0..n - 1 {
            let (a, b) = (self.pts[i], self.pts[i + 1]);
            let ab = sub(b, a);
            let l2 = dot(ab, ab);
            let t = if l2 > 0.0 {
                dot(sub(p, a), ab) / l2
            } else {
                0.0
            };
            let (q, class) = if t <= 0.0 {
                (a, 0)
            } else if t >= 1.0 {
                (b, 2)
            } else {
                ([a[0] + ab[0] * t, a[1] + ab[1] * t], 1)
            };
            let d2 = (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2);
            if d2 < best.0 {
                best = (d2, i, class, q);
            }
        }
        let (d2, i, class, at) = best;
        let normal = |k: usize| perp(unit(sub(self.pts[k + 1], self.pts[k])));
        let nrm = match class {
            0 if i > 0 => {
                let (u, v) = (normal(i - 1), normal(i));
                [u[0] + v[0], u[1] + v[1]]
            }
            2 if i + 2 < n => {
                let (u, v) = (normal(i), normal(i + 1));
                [u[0] + v[0], u[1] + v[1]]
            }
            _ => normal(i),
        };
        let s = dot(sub(p, at), nrm);
        // On the line itself, fall back to the segment's own normal so a
        // point is never on both sides.
        let left = if s == 0.0 {
            dot(sub(p, self.pts[i]), normal(i)) >= 0.0
        } else {
            s > 0.0
        };
        Near {
            dist: d2.sqrt(),
            at,
            left,
        }
    }

    /// Which side of the cut `p` lies on (`1` left-normal side, `0` right).
    #[must_use]
    pub fn side(&self, p: P) -> usize {
        usize::from(self.near(p).left)
    }

    /// The finite vertices (without the far ray ends).
    #[must_use]
    pub fn inner(&self) -> &[P] {
        &self.pts[1..self.pts.len() - 1]
    }

    /// Every vertex, the far ray ends included.
    #[must_use]
    pub fn points(&self) -> &[P] {
        &self.pts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_splits_the_plane() {
        let c = Cut::line([0.0, 0.0], [1.0, 0.0]);
        // Left normal of east is south (y down): (0, 1).
        assert_eq!(c.side([3.0, 2.0]), 1);
        assert_eq!(c.side([-50.0, -2.0]), 0);
        let n = c.near([7.0, -3.0]);
        assert!((n.dist - 3.0).abs() < 1e-9 && (n.at[0] - 7.0).abs() < 1e-9);
    }

    #[test]
    fn a_kinked_cut_sides_agree_near_the_kink() {
        let d2 = super::super::poly::rotate([1.0, 0.0], 0.2);
        let c = Cut::kinked([0.0, 0.0], [1.0, 0.0], d2);
        for (p, side) in [
            ([-10.0, 1.0], 1),
            ([-10.0, -1.0], 0),
            ([10.0, 3.0], 1),
            ([10.0, 1.0], 0),
            ([0.0, 0.5], 1),
            ([0.0, -0.5], 0),
        ] {
            assert_eq!(c.side(p), side, "{p:?}");
        }
    }

    #[test]
    fn a_polyline_cut_follows_its_vertices() {
        let c = Cut::along(&[[0.0, 0.0], [10.0, 0.0], [20.0, 5.0], [30.0, 5.0]]);
        assert_eq!(c.side([15.0, 4.0]), 1);
        assert_eq!(c.side([15.0, 1.0]), 0);
        assert_eq!(c.side([40.0, 6.0]), 1);
        assert!((c.near([25.0, 8.0]).dist - 3.0).abs() < 1e-9);
    }
}
