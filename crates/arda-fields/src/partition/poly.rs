//! Plane geometry of the field partition: polygons clipped by half-planes,
//! their area moments and principal axes, and distances to segments.
//!
//! A region is a list of polygon pieces (a kinked cut leaves one side as
//! two pieces meeting along a line); its moments are the pieces' sums.
//! Moments are taken about a local origin, since world squares reach 10^5
//! and their cubes would lose precision.

/// A point in (warped) square units.
pub type P = [f64; 2];

/// `a − b`.
#[must_use]
pub fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}

/// `a · b`.
#[must_use]
pub fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

/// The left normal of a direction (y south: a quarter turn).
#[must_use]
pub fn perp(d: P) -> P {
    [-d[1], d[0]]
}

/// A unit vector, or east for a null one.
#[must_use]
pub fn unit(v: P) -> P {
    let l = v[0].hypot(v[1]);
    if l > 1e-12 {
        [v[0] / l, v[1] / l]
    } else {
        [1.0, 0.0]
    }
}

/// `d` turned by `a` radians.
#[must_use]
pub fn rotate(d: P, a: f64) -> P {
    let (s, c) = a.sin_cos();
    [d[0] * c - d[1] * s, d[0] * s + d[1] * c]
}

/// The part of `poly` where `dot(n, p − c) ≥ 0` (Sutherland–Hodgman).
#[must_use]
pub fn clip(poly: &[P], n: P, c: P) -> Vec<P> {
    let f = |p: P| dot(n, sub(p, c));
    let mut out = Vec::with_capacity(poly.len() + 2);
    for (i, &a) in poly.iter().enumerate() {
        let b = poly[(i + 1) % poly.len()];
        let (fa, fb) = (f(a), f(b));
        if fa >= 0.0 {
            out.push(a);
        }
        if (fa >= 0.0) != (fb >= 0.0) {
            let t = fa / (fa - fb);
            out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
        }
    }
    if out.len() < 3 {
        out.clear();
    }
    out
}

/// Clips every piece of a region, dropping the empty ones.
#[must_use]
pub fn clip_region(region: &[Vec<P>], n: P, c: P) -> Vec<Vec<P>> {
    region
        .iter()
        .map(|p| clip(p, n, c))
        .filter(|p| p.len() >= 3)
        .collect()
}

/// Area moments of a region.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Moments {
    /// Area, squares².
    pub area: f64,
    /// Centroid.
    pub centroid: P,
    /// Central second moments per unit area: `[xx, xy, yy]`.
    pub cov: [f64; 3],
}

/// The moments of a region about local origin `o`.
#[must_use]
pub fn moments(region: &[Vec<P>], o: P) -> Moments {
    let (mut a, mut sx, mut sy, mut ixx, mut ixy, mut iyy) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for piece in region {
        let (mut pa, mut px, mut py, mut pxx, mut pxy, mut pyy) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        for (i, &p) in piece.iter().enumerate() {
            let q = piece[(i + 1) % piece.len()];
            let (x0, y0, x1, y1) = (p[0] - o[0], p[1] - o[1], q[0] - o[0], q[1] - o[1]);
            let cr = x0 * y1 - x1 * y0;
            pa += cr;
            px += (x0 + x1) * cr;
            py += (y0 + y1) * cr;
            pxx += (x0 * x0 + x0 * x1 + x1 * x1) * cr;
            pyy += (y0 * y0 + y0 * y1 + y1 * y1) * cr;
            pxy += (x0 * y1 + 2.0 * x0 * y0 + 2.0 * x1 * y1 + x1 * y0) * cr;
        }
        // Pieces keep their parent's winding; normalise each anyway.
        let s = if pa < 0.0 { -1.0 } else { 1.0 };
        a += s * pa / 2.0;
        sx += s * px / 6.0;
        sy += s * py / 6.0;
        ixx += s * pxx / 12.0;
        iyy += s * pyy / 12.0;
        ixy += s * pxy / 24.0;
    }
    if a <= 1e-9 {
        return Moments {
            area: 0.0,
            centroid: o,
            cov: [0.0; 3],
        };
    }
    let (cx, cy) = (sx / a, sy / a);
    Moments {
        area: a,
        centroid: [cx + o[0], cy + o[1]],
        cov: [ixx / a - cx * cx, ixy / a - cx * cy, iyy / a - cy * cy],
    }
}

/// The long (major) axis of a covariance.
#[must_use]
pub fn major_axis(cov: [f64; 3]) -> P {
    let th = 0.5 * (2.0 * cov[1]).atan2(cov[0] - cov[2]);
    let (s, c) = th.sin_cos();
    [c, s]
}

/// The extent `[min, max]` of a region projected onto `d`.
#[must_use]
pub fn extent(region: &[Vec<P>], d: P) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for p in region.iter().flatten() {
        let v = dot(*p, d);
        lo = lo.min(v);
        hi = hi.max(v);
    }
    (lo, hi)
}

/// Whether a polygon holds a point (even–odd rule; points on an edge
/// shared by two polygons go to exactly one of them).
#[must_use]
pub fn contains(poly: &[P], p: P) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}

/// Whether any piece of a region holds a point.
#[must_use]
pub fn region_contains(region: &[Vec<P>], p: P) -> bool {
    region.iter().any(|piece| contains(piece, p))
}

/// The bounding box `[x0, y0, x1, y1]` of a region.
#[must_use]
pub fn bbox(region: &[Vec<P>]) -> [f64; 4] {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in region.iter().flatten() {
        b = [
            b[0].min(p[0]),
            b[1].min(p[1]),
            b[2].max(p[0]),
            b[3].max(p[1]),
        ];
    }
    b
}

/// The closest point to `p` on segment `a–b` and its parameter.
#[must_use]
pub fn closest_on_segment(p: P, a: P, b: P) -> (P, f64) {
    let ab = sub(b, a);
    let l2 = dot(ab, ab);
    let t = if l2 > 0.0 {
        (dot(sub(p, a), ab) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    ([a[0] + ab[0] * t, a[1] + ab[1] * t], t)
}

/// The closest point to `p` on a closed polygon's outline.
#[must_use]
pub fn closest_on_outline(poly: &[P], p: P) -> (f64, P) {
    let mut best = (f64::INFINITY, p);
    for i in 0..poly.len() {
        let (q, _) = closest_on_segment(p, poly[i], poly[(i + 1) % poly.len()]);
        let d = (q[0] - p[0]).hypot(q[1] - p[1]);
        if d < best.0 {
            best = (d, q);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<P> {
        vec![[0.0, 0.0], [10.0, 0.0], [10.0, 4.0], [0.0, 4.0]]
    }

    #[test]
    fn moments_of_a_rectangle() {
        let m = moments(&[square()], [3.0, 1.0]);
        assert!((m.area - 40.0).abs() < 1e-9);
        assert!((m.centroid[0] - 5.0).abs() < 1e-9 && (m.centroid[1] - 2.0).abs() < 1e-9);
        assert!((m.cov[0] - 100.0 / 12.0).abs() < 1e-9);
        assert!((m.cov[2] - 16.0 / 12.0).abs() < 1e-9);
        assert!(m.cov[1].abs() < 1e-9);
        let a = major_axis(m.cov);
        assert!((a[0].abs() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn clipping_halves_the_area() {
        let left = clip(&square(), [-1.0, 0.0], [5.0, 0.0]);
        let right = clip(&square(), [1.0, 0.0], [5.0, 0.0]);
        let (a, b) = (moments(&[left], [0.0; 2]), moments(&[right], [0.0; 2]));
        assert!((a.area - 20.0).abs() < 1e-9 && (b.area - 20.0).abs() < 1e-9);
        assert!((a.centroid[0] - 2.5).abs() < 1e-9);
    }

    #[test]
    fn containment_and_outline_distance() {
        let s = square();
        assert!(contains(&s, [1.0, 1.0]) && !contains(&s, [11.0, 1.0]));
        let (d, q) = closest_on_outline(&s, [5.0, 1.0]);
        assert!((d - 1.0).abs() < 1e-9 && q[1].abs() < 1e-9);
    }
}
