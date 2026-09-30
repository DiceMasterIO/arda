//! Signed-distance painting primitives for the placeholder art.
//!
//! Shapes are signed distance functions in pixels (negative inside). Coverage
//! is `clamp(0.5 - d, 0, 1)`, a one-pixel anti-aliased edge, which the
//! validator's fringe rule accepts. Nothing here draws cast shadows: the
//! lighting pass owns shadows (goal 63).
// Art generation casts bounded pixel coordinates and cell indices.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use crate::noise::{fbm, hash2, unit};
use crate::raster::Rgba;

/// An RGB colour.
pub type Rgb = [u8; 3];

/// Composites a shape over `img` with a per-pixel colour.
///
/// `shade(x, y, d)` receives the pixel centre and its signed distance.
pub fn fill(img: &mut Rgba, sdf: impl Fn(f32, f32) -> f32, shade: impl Fn(f32, f32, f32) -> Rgb) {
    fill_alpha(img, 1.0, sdf, shade);
}

/// Like [`fill`] with an overall opacity in `[0, 1]`.
pub fn fill_alpha(
    img: &mut Rgba,
    opacity: f32,
    sdf: impl Fn(f32, f32) -> f32,
    shade: impl Fn(f32, f32, f32) -> Rgb,
) {
    for y in 0..img.height {
        for x in 0..img.width {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let d = sdf(px, py);
            let cov = (0.5 - d).clamp(0.0, 1.0) * opacity;
            if cov <= 0.0 {
                continue;
            }
            let c = shade(px, py, d);
            img.blend(x, y, [c[0], c[1], c[2], byte(cov * 255.0)]);
        }
    }
}

/// Rounds and clamps to a byte.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped first
pub fn byte(v: f32) -> u8 {
    (v + 0.5).clamp(0.0, 255.0) as u8
}

/// Scales a colour by `k`.
#[must_use]
pub fn tone(c: Rgb, k: f32) -> Rgb {
    [
        byte(f32::from(c[0]) * k),
        byte(f32::from(c[1]) * k),
        byte(f32::from(c[2]) * k),
    ]
}

/// Linear mix from `a` (t = 0) to `b` (t = 1).
#[must_use]
pub fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let l = |i: usize| byte(f32::from(a[i]) + (f32::from(b[i]) - f32::from(a[i])) * t);
    [l(0), l(1), l(2)]
}

/// Painterly shading: base colour varied by rotated noise and a dark rim so
/// shapes read at a glance. `scale` is the noise feature size in pixels.
pub fn painterly(seed: u64, base: Rgb, scale: f32) -> impl Fn(f32, f32, f32) -> Rgb {
    move |x, y, d| {
        let n = fbm(seed, x / scale, y / scale, 3, None);
        let grain = unit(hash2(seed ^ 0x55, x as i64, y as i64));
        let k = 0.78 + 0.4 * n + 0.08 * grain;
        let rim = if d > -2.5 { 0.72 } else { 1.0 };
        tone(base, k * rim)
    }
}

/// Circle.
pub fn circle(cx: f32, cy: f32, r: f32) -> impl Fn(f32, f32) -> f32 {
    move |x, y| ((x - cx) * (x - cx) + (y - cy) * (y - cy)).sqrt() - r
}

/// Axis-aligned ellipse (approximate distance, exact sign).
pub fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> impl Fn(f32, f32) -> f32 {
    move |x, y| {
        let (u, v) = ((x - cx) / rx, (y - cy) / ry);
        ((u * u + v * v).sqrt() - 1.0) * rx.min(ry)
    }
}

/// Axis-aligned box with rounded corners; `hw, hh` are half sizes.
pub fn rbox(cx: f32, cy: f32, hw: f32, hh: f32, r: f32) -> impl Fn(f32, f32) -> f32 {
    move |x, y| {
        let qx = (x - cx).abs() - hw + r;
        let qy = (y - cy).abs() - hh + r;
        let outside = (qx.max(0.0) * qx.max(0.0) + qy.max(0.0) * qy.max(0.0)).sqrt();
        outside + qx.max(qy).min(0.0) - r
    }
}

/// A box rotated by the unit vector `(c, s)` about its centre.
pub fn obox(
    cx: f32,
    cy: f32,
    hw: f32,
    hh: f32,
    r: f32,
    (c, s): (f32, f32),
) -> impl Fn(f32, f32) -> f32 {
    let inner = rbox(0.0, 0.0, hw, hh, r);
    move |x, y| {
        let (dx, dy) = (x - cx, y - cy);
        inner(dx * c + dy * s, -dx * s + dy * c)
    }
}

/// A thick line segment with round caps.
pub fn capsule(ax: f32, ay: f32, bx: f32, by: f32, r: f32) -> impl Fn(f32, f32) -> f32 {
    move |x, y| {
        let (px, py, vx, vy) = (x - ax, y - ay, bx - ax, by - ay);
        let len2 = (vx * vx + vy * vy).max(1e-6);
        let t = ((px * vx + py * vy) / len2).clamp(0.0, 1.0);
        let (dx, dy) = (px - vx * t, py - vy * t);
        (dx * dx + dy * dy).sqrt() - r
    }
}

/// A lumpy blob: a circle whose radius wobbles with rotated noise.
pub fn blob(seed: u64, cx: f32, cy: f32, r: f32, wobble: f32) -> impl Fn(f32, f32) -> f32 {
    move |x, y| {
        let n = fbm(seed, x / (r * 0.45), y / (r * 0.45), 2, None);
        ((x - cx) * (x - cx) + (y - cy) * (y - cy)).sqrt() - r * (1.0 - wobble + 2.0 * wobble * n)
    }
}

/// Union of two shapes.
pub fn union(a: impl Fn(f32, f32) -> f32, b: impl Fn(f32, f32) -> f32) -> impl Fn(f32, f32) -> f32 {
    move |x, y| a(x, y).min(b(x, y))
}

/// Flat colour shading.
pub fn flat(c: Rgb) -> impl Fn(f32, f32, f32) -> Rgb {
    move |_, _, _| c
}

/// Flat colour with a dark rim.
pub fn rimmed(c: Rgb) -> impl Fn(f32, f32, f32) -> Rgb {
    move |_, _, d| if d > -2.0 { tone(c, 0.7) } else { c }
}

/// A tapered capsule from `a` (radius `r0`) to `b` (radius `r1`).
pub fn taper(a: (f32, f32), b: (f32, f32), r0: f32, r1: f32) -> impl Fn(f32, f32) -> f32 {
    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
    let len2 = (vx * vx + vy * vy).max(1e-6);
    move |x, y| {
        let t = (((x - a.0) * vx + (y - a.1) * vy) / len2).clamp(0.0, 1.0);
        let (dx, dy) = (x - a.0 - vx * t, y - a.1 - vy * t);
        (dx * dx + dy * dy).sqrt() - (r0 + (r1 - r0) * t)
    }
}

/// A convex polygon from vertices in increasing angle order (approximate
/// distance, exact sign).
pub fn polygon(points: &[(f32, f32)]) -> impl Fn(f32, f32) -> f32 {
    let n = points.len();
    let edges: Vec<(f32, f32, f32, f32)> = (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            let (ex, ey) = (b.0 - a.0, b.1 - a.1);
            let len = (ex * ex + ey * ey).sqrt().max(1e-3);
            (a.0, a.1, ey / len, -ex / len)
        })
        .collect();
    move |x, y| {
        let mut d = f32::MIN;
        for &(ax, ay, nx, ny) in &edges {
            d = d.max((x - ax) * nx + (y - ay) * ny);
        }
        d
    }
}
