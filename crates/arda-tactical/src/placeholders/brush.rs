//! A wrapping paint surface for seamless ground textures.
//!
//! Every stroke, dab and pebble wraps around the tile's edges, so whatever is
//! painted stays seamless. Strokes are tapered capsules with a one-pixel
//! anti-aliased edge, which reads as hand-painted brushwork at map scale.
// Art generation casts bounded pixel coordinates.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use super::paint::{byte, Rgb};
use crate::noise::value;
use crate::raster::Rgba;

/// A square, toroidally wrapping RGB canvas.
pub struct Tile {
    /// Side in pixels.
    pub side: u32,
    px: Vec<[f32; 3]>,
}

impl Tile {
    /// A tile filled by `f(x, y)` at each pixel centre.
    pub fn new(side: u32, f: impl Fn(f32, f32) -> Rgb) -> Self {
        let mut px = Vec::with_capacity(side as usize * side as usize);
        for y in 0..side {
            for x in 0..side {
                let c = f(x as f32 + 0.5, y as f32 + 0.5);
                px.push([f32::from(c[0]), f32::from(c[1]), f32::from(c[2])]);
            }
        }
        Self { side, px }
    }

    fn idx(&self, x: i64, y: i64) -> usize {
        let s = i64::from(self.side);
        (y.rem_euclid(s) * s + x.rem_euclid(s)) as usize
    }

    /// Blends `c` into a wrapped pixel with coverage `a`.
    pub fn put(&mut self, x: i64, y: i64, c: [f32; 3], a: f32) {
        let i = self.idx(x, y);
        for (k, v) in c.iter().enumerate() {
            self.px[i][k] += (v - self.px[i][k]) * a;
        }
    }

    /// Applies `f(x, y, colour)` to every pixel.
    pub fn map(&mut self, f: impl Fn(f32, f32, [f32; 3]) -> [f32; 3]) {
        let s = self.side as usize;
        for (i, p) in self.px.iter_mut().enumerate() {
            *p = f((i % s) as f32 + 0.5, (i / s) as f32 + 0.5, *p);
        }
    }

    /// Paints `alpha(x, y, d)` coverage of a shape given by `sdf` over the
    /// box `x0..x1 × y0..y1` (unwrapped pixel coordinates).
    fn shape(
        &mut self,
        (x0, y0, x1, y1): (f32, f32, f32, f32),
        sdf: impl Fn(f32, f32) -> f32,
        colour: impl Fn(f32, f32, f32) -> [f32; 3],
        opacity: f32,
    ) {
        for y in (y0.floor() as i64)..=(y1.ceil() as i64) {
            for x in (x0.floor() as i64)..=(x1.ceil() as i64) {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let d = sdf(px, py);
                let cov = (0.5 - d).clamp(0.0, 1.0) * opacity;
                if cov > 0.0 {
                    self.put(x, y, colour(px, py, d), cov);
                }
            }
        }
    }

    /// A tapered stroke from `a` (radius `r0`) to `b` (radius `r1`).
    #[allow(clippy::too_many_arguments)]
    pub fn stroke(&mut self, a: (f32, f32), b: (f32, f32), r0: f32, r1: f32, c: Rgb, opacity: f32) {
        let m = r0.max(r1) + 1.0;
        let (vx, vy) = (b.0 - a.0, b.1 - a.1);
        let len2 = (vx * vx + vy * vy).max(1e-6);
        let cf = rgbf(c);
        self.shape(
            (
                a.0.min(b.0) - m,
                a.1.min(b.1) - m,
                a.0.max(b.0) + m,
                a.1.max(b.1) + m,
            ),
            |x, y| {
                let t = (((x - a.0) * vx + (y - a.1) * vy) / len2).clamp(0.0, 1.0);
                let (dx, dy) = (x - a.0 - vx * t, y - a.1 - vy * t);
                (dx * dx + dy * dy).sqrt() - (r0 + (r1 - r0) * t)
            },
            |_, _, _| cf,
            opacity,
        );
    }

    /// An irregular round dab of radius `r` with a noise-wobbled edge.
    pub fn dab(&mut self, (cx, cy): (f32, f32), r: f32, c: Rgb, opacity: f32, seed: u64) {
        let cf = rgbf(c);
        let k = 1.6 / r.max(0.5);
        self.shape(
            (cx - r * 1.3, cy - r * 1.3, cx + r * 1.3, cy + r * 1.3),
            |x, y| {
                let n = value(seed, x * k, y * k, None);
                ((x - cx) * (x - cx) + (y - cy) * (y - cy)).sqrt() - r * (0.75 + 0.5 * n)
            },
            |_, _, _| cf,
            opacity,
        );
    }

    /// A small stone seen from above: an ellipse along `dir` with form
    /// shading (lit top-left, dark bottom-right) and a dark rim.
    #[allow(clippy::too_many_arguments)]
    pub fn pebble(
        &mut self,
        (cx, cy): (f32, f32),
        rx: f32,
        ry: f32,
        dir: (f32, f32),
        c: Rgb,
        opacity: f32,
    ) {
        let cf = rgbf(c);
        let r = rx.max(ry);
        self.shape(
            (cx - r - 1.0, cy - r - 1.0, cx + r + 1.0, cy + r + 1.0),
            |x, y| {
                let (dx, dy) = (x - cx, y - cy);
                let (u, v) = (
                    (dx * dir.0 + dy * dir.1) / rx,
                    (-dx * dir.1 + dy * dir.0) / ry,
                );
                ((u * u + v * v).sqrt() - 1.0) * rx.min(ry)
            },
            |x, y, d| {
                let (dx, dy) = ((x - cx) / r, (y - cy) / r);
                // Light from the top-left: brighter where -x-y is large.
                let lit = (1.0 - 0.45 * (dx + dy)).clamp(0.55, 1.35);
                let rim = if d > -1.2 { 0.62 } else { 1.0 };
                [cf[0] * lit * rim, cf[1] * lit * rim, cf[2] * lit * rim]
            },
            opacity,
        );
    }

    /// An angular stone: a convex polygon of `sides` jittered vertices,
    /// lit from the top-left with a bright upper lip and a dark rim.
    #[allow(clippy::too_many_arguments)]
    pub fn stone(&mut self, (cx, cy): (f32, f32), r: f32, sides: u32, c: Rgb, seed: u64) {
        let mut rng = super::material::Rng::new(seed);
        let n = sides.clamp(3, 9) as usize;
        let spin = rng.f();
        let mut verts = [(0.0f32, 0.0f32); 9];
        for (i, v) in verts.iter_mut().enumerate().take(n) {
            let a = spin + (i as f32 + rng.range(-0.25, 0.25)) / n as f32;
            let (s, co) = crate::noise::sincos(a);
            let rr = r * rng.range(0.7, 1.0);
            *v = (cx + co * rr, cy + s * rr * rng.range(0.75, 1.0));
        }
        let cf = rgbf(c);
        let tilt = (rng.range(-0.3, 0.3), rng.range(-0.3, 0.3));
        self.shape(
            (cx - r - 1.0, cy - r - 1.0, cx + r + 1.0, cy + r + 1.0),
            |x, y| {
                let mut d = f32::MIN;
                for i in 0..n {
                    let (a, b) = (verts[i], verts[(i + 1) % n]);
                    let (ex, ey) = (b.0 - a.0, b.1 - a.1);
                    let len = (ex * ex + ey * ey).sqrt().max(1e-3);
                    // Outward normal of a counter-clockwise edge (y south).
                    let (nx, ny) = (ey / len, -ex / len);
                    d = d.max((x - a.0) * nx + (y - a.1) * ny);
                }
                d
            },
            |x, y, d| {
                let (dx, dy) = ((x - cx) / r, (y - cy) / r);
                let face = 1.0 - 0.32 * (dx + dy) - (tilt.0 * dx + tilt.1 * dy);
                let edge = if d > -1.3 {
                    0.55
                } else if d > -3.0 && dx + dy < 0.0 {
                    1.22
                } else {
                    1.0
                };
                let k = (face * edge).clamp(0.4, 1.4);
                [cf[0] * k, cf[1] * k, cf[2] * k]
            },
            1.0,
        );
    }

    /// The finished opaque texture.
    #[must_use]
    pub fn into_rgba(self) -> Rgba {
        let mut img = Rgba::new(self.side, self.side);
        for (i, p) in self.px.iter().enumerate() {
            let (x, y) = (i as u32 % self.side, i as u32 / self.side);
            img.set(x, y, [byte(p[0]), byte(p[1]), byte(p[2]), 255]);
        }
        img
    }
}

/// An RGB byte triple as floats.
#[must_use]
pub fn rgbf(c: Rgb) -> [f32; 3] {
    [f32::from(c[0]), f32::from(c[1]), f32::from(c[2])]
}
