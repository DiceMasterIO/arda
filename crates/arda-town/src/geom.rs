//! Plane geometry in world metres (x east, y south, as in `arda-settle`).
//!
//! Only IEEE `+ − × ÷` and `sqrt` are used, plus a polynomial sine, so every
//! result is bit-identical across platforms (goal-prompt §8, determinism).

use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul, Neg, Sub};

/// A point or vector in world metres.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Vec2 {
    /// Metres east.
    pub x: f64,
    /// Metres south.
    pub y: f64,
}

/// Shorthand constructor.
#[must_use]
pub const fn v2(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        v2(self.x + o.x, self.y + o.y)
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        v2(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f64> for Vec2 {
    type Output = Self;
    fn mul(self, k: f64) -> Self {
        v2(self.x * k, self.y * k)
    }
}

impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self {
        v2(-self.x, -self.y)
    }
}

impl Vec2 {
    /// Dot product.
    #[must_use]
    pub fn dot(self, o: Self) -> f64 {
        self.x * o.x + self.y * o.y
    }

    /// Z of the cross product.
    #[must_use]
    pub fn cross(self, o: Self) -> f64 {
        self.x * o.y - self.y * o.x
    }

    /// Length.
    #[must_use]
    pub fn len(self) -> f64 {
        self.dot(self).sqrt()
    }

    /// Unit vector; the zero vector maps to east.
    #[must_use]
    pub fn norm(self) -> Self {
        let l = self.len();
        if l < 1e-12 {
            v2(1.0, 0.0)
        } else {
            self * (1.0 / l)
        }
    }

    /// Rotated a quarter turn (east becomes south).
    #[must_use]
    pub fn perp(self) -> Self {
        v2(-self.y, self.x)
    }

    /// Distance to another point.
    #[must_use]
    pub fn dist(self, o: Self) -> f64 {
        (self - o).len()
    }

    /// Linear interpolation.
    #[must_use]
    pub fn lerp(self, o: Self, t: f64) -> Self {
        self + (o - self) * t
    }

    /// Rotated by `theta` radians.
    #[must_use]
    pub fn rotated(self, theta: f64) -> Self {
        let (s, c) = sin_cos(theta);
        v2(self.x * c - self.y * s, self.x * s + self.y * c)
    }
}

/// π.
pub const PI: f64 = std::f64::consts::PI;

/// Deterministic `(sin θ, cos θ)` by range reduction and a Taylor series.
#[must_use]
pub fn sin_cos(theta: f64) -> (f64, f64) {
    let tau = 2.0 * PI;
    let t = theta - tau * (theta / tau).round();
    (sin_reduced(t), sin_reduced(t + PI / 2.0))
}

fn sin_reduced(mut t: f64) -> f64 {
    // Fold into [-π, π], then into [-π/2, π/2].
    if t > PI {
        t -= 2.0 * PI;
    }
    if t > PI / 2.0 {
        t = PI - t;
    } else if t < -PI / 2.0 {
        t = -PI - t;
    }
    let t2 = t * t;
    let mut term = t;
    let mut sum = t;
    for k in 1..10 {
        let kf = f64::from(k);
        term *= -t2 / ((2.0 * kf) * (2.0 * kf + 1.0));
        sum += term;
    }
    sum
}

/// Deterministic `atan2(y, x)` in radians, in `(-π, π]`.
#[must_use]
pub fn atan2(y: f64, x: f64) -> f64 {
    if x == 0.0 && y == 0.0 {
        return 0.0;
    }
    let a = crate::site::atan_deg(y.abs() / x.abs().max(1e-300)).to_radians();
    let a = if x.abs() < 1e-300 { PI / 2.0 } else { a };
    let a = if x < 0.0 { PI - a } else { a };
    if y < 0.0 {
        -a
    } else {
        a
    }
}

/// Distance from `p` to segment `ab`, and the parameter of the closest point.
#[must_use]
pub fn seg_dist(p: Vec2, a: Vec2, b: Vec2) -> (f64, f64) {
    let ab = b - a;
    let l2 = ab.dot(ab);
    let t = if l2 < 1e-12 {
        0.0
    } else {
        ((p - a).dot(ab) / l2).clamp(0.0, 1.0)
    };
    (p.dist(a + ab * t), t)
}

/// Total length of a polyline.
#[must_use]
pub fn length(poly: &[Vec2]) -> f64 {
    poly.windows(2).map(|w| w[0].dist(w[1])).sum()
}

/// Point and unit tangent at arc length `s` (clamped to the ends).
#[must_use]
pub fn at(poly: &[Vec2], s: f64) -> (Vec2, Vec2) {
    let mut acc = 0.0;
    for w in poly.windows(2) {
        let l = w[0].dist(w[1]);
        if acc + l >= s && l > 0.0 {
            let t = ((s - acc) / l).clamp(0.0, 1.0);
            return (w[0].lerp(w[1], t), (w[1] - w[0]).norm());
        }
        acc += l;
    }
    match poly {
        [.., a, b] => (*b, (*b - *a).norm()),
        [a] => (*a, v2(1.0, 0.0)),
        [] => (Vec2::default(), v2(1.0, 0.0)),
    }
}

/// Closest point on a polyline: `(distance, arc length)`.
#[must_use]
pub fn project(poly: &[Vec2], p: Vec2) -> (f64, f64) {
    let mut best = (f64::MAX, 0.0);
    let mut acc = 0.0;
    for w in poly.windows(2) {
        let l = w[0].dist(w[1]);
        let (d, t) = seg_dist(p, w[0], w[1]);
        if d < best.0 {
            best = (d, acc + t * l);
        }
        acc += l;
    }
    if poly.len() == 1 {
        best = (poly[0].dist(p), 0.0);
    }
    best
}

/// Distance from `p` to a polyline.
#[must_use]
pub fn dist_to(poly: &[Vec2], p: Vec2) -> f64 {
    project(poly, p).0
}

/// Resamples a polyline at roughly `step` metres, keeping both ends.
#[must_use]
pub fn resample(poly: &[Vec2], step: f64) -> Vec<Vec2> {
    let total = length(poly);
    if poly.len() < 2 || total < 1e-9 {
        return poly.to_vec();
    }
    let n = (total / step).ceil().max(1.0);
    let count = crate::num::clamp_u32(crate::num::round_i(n));
    (0..=count)
        .map(|i| at(poly, total * f64::from(i) / n).0)
        .collect()
}

/// Chaikin corner cutting, keeping the end points.
#[must_use]
pub fn chaikin(poly: &[Vec2], iterations: u32) -> Vec<Vec2> {
    let mut p = poly.to_vec();
    for _ in 0..iterations {
        if p.len() < 3 {
            return p;
        }
        let mut q = Vec::with_capacity(p.len() * 2);
        q.push(p[0]);
        for w in p.windows(2) {
            q.push(w[0].lerp(w[1], 0.25));
            q.push(w[0].lerp(w[1], 0.75));
        }
        if let Some(last) = p.last() {
            q.push(*last);
        }
        p = q;
    }
    p
}

/// Cuts the part of a polyline from arc length `s0` to `s1`.
#[must_use]
pub fn slice(poly: &[Vec2], s0: f64, s1: f64) -> Vec<Vec2> {
    let mut out = vec![at(poly, s0).0];
    let mut acc = 0.0;
    for w in poly.windows(2) {
        acc += w[0].dist(w[1]);
        if acc > s0 && acc < s1 {
            out.push(w[1]);
        }
    }
    out.push(at(poly, s1).0);
    out
}

/// Even–odd point-in-polygon test (the ring is implicitly closed).
#[must_use]
pub fn inside(poly: &[Vec2], p: Vec2) -> bool {
    let mut c = false;
    let n = poly.len();
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + n - 1) % n];
        if (a.y > p.y) != (b.y > p.y) {
            let x = (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x;
            if p.x < x {
                c = !c;
            }
        }
    }
    c
}

/// Distance from `p` to a closed ring.
#[must_use]
pub fn ring_dist(ring: &[Vec2], p: Vec2) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| seg_dist(p, ring[i], ring[(i + 1) % n]).0)
        .fold(f64::MAX, f64::min)
}

/// Signed area (positive when clockwise on screen, y south).
#[must_use]
pub fn area(poly: &[Vec2]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| poly[i].cross(poly[(i + 1) % n]))
        .sum::<f64>()
        * 0.5
}

/// Mean of the vertices.
#[must_use]
pub fn centroid(poly: &[Vec2]) -> Vec2 {
    if poly.is_empty() {
        return Vec2::default();
    }
    let s = poly.iter().fold(Vec2::default(), |a, &b| a + b);
    s * (1.0 / poly.len() as f64)
}

/// Intersection of segments `ab` and `cd`, if they cross.
#[must_use]
pub fn seg_intersect(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> Option<Vec2> {
    let r = b - a;
    let s = d - c;
    let den = r.cross(s);
    if den.abs() < 1e-12 {
        return None;
    }
    let t = (c - a).cross(s) / den;
    let u = (c - a).cross(r) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then(|| a + r * t)
}

/// Every crossing of two polylines.
#[must_use]
pub fn crossings(p: &[Vec2], q: &[Vec2]) -> Vec<Vec2> {
    let mut out = Vec::new();
    for a in p.windows(2) {
        for b in q.windows(2) {
            if let Some(x) = seg_intersect(a[0], a[1], b[0], b[1]) {
                out.push(x);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_matches_std_closely() {
        for i in -40..40 {
            let t = f64::from(i) * 0.37;
            let (s, c) = sin_cos(t);
            assert!((s - t.sin()).abs() < 1e-9, "sin {t}");
            assert!((c - t.cos()).abs() < 1e-9, "cos {t}");
        }
    }

    #[test]
    fn atan2_matches_std() {
        for i in -20..20 {
            for j in -20..20 {
                let (y, x) = (f64::from(i) * 0.7, f64::from(j) * 0.3);
                if x == 0.0 && y == 0.0 {
                    continue;
                }
                assert!((atan2(y, x) - y.atan2(x)).abs() < 1e-7, "{y} {x}");
            }
        }
    }

    #[test]
    fn projection_and_inside() {
        let line = [v2(0.0, 0.0), v2(10.0, 0.0), v2(10.0, 10.0)];
        let (d, s) = project(&line, v2(12.0, 5.0));
        assert!((d - 2.0).abs() < 1e-9 && (s - 15.0).abs() < 1e-9);
        let sq = [v2(0.0, 0.0), v2(4.0, 0.0), v2(4.0, 4.0), v2(0.0, 4.0)];
        assert!(inside(&sq, v2(1.0, 1.0)) && !inside(&sq, v2(5.0, 1.0)));
        assert!((area(&sq).abs() - 16.0).abs() < 1e-9);
    }
}
