//! Relief painting for cut-out art: props, walls, vegetation and rocks.
//!
//! A [`Relief`] holds colour, coverage, a height field and an ink amount per
//! pixel. Parts are painted back to front, each with a signed distance
//! function, a material (colour) and a height profile. [`Relief::finish`]
//! then shades the height field from the top-left (the compositor's sun
//! direction) and inks the outlines, which gives the painted, outlined look
//! of the reference maps. This is *form* shading on the object itself: cast
//! shadows stay the lighting pass's job (goal 63), so nothing here darkens
//! pixels outside a shape.
// Art generation casts bounded pixel coordinates.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use super::paint::{byte, Rgb};
use crate::raster::Rgba;

/// How a part's edge is inked.
#[derive(Debug, Clone, Copy)]
pub struct Ink {
    /// Ink band width in pixels inside the edge.
    pub width: f32,
    /// Darkening at the edge, 0–1.
    pub strength: f32,
}

/// The standard outline.
pub const INK: Ink = Ink {
    width: 1.8,
    strength: 0.78,
};
/// A lighter outline for inner details.
pub const SOFT_INK: Ink = Ink {
    width: 1.3,
    strength: 0.42,
};
/// No outline.
pub const NO_INK: Ink = Ink {
    width: 0.0,
    strength: 0.0,
};

/// Colour, coverage, height and ink buffers for one sprite.
pub struct Relief {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    col: Vec<[f32; 3]>,
    alpha: Vec<f32>,
    z: Vec<f32>,
    ink: Vec<f32>,
}

/// A pixel rectangle `[x0, y0, x1, y1)` to limit a part's work.
#[derive(Debug, Clone, Copy)]
pub struct Bounds(pub f32, pub f32, pub f32, pub f32);

impl Bounds {
    /// A square of half-size `r` around `(cx, cy)`, plus a margin.
    #[must_use]
    pub fn around(cx: f32, cy: f32, r: f32) -> Self {
        Self(cx - r - 3.0, cy - r - 3.0, cx + r + 3.0, cy + r + 3.0)
    }

    /// The box spanned by two corners, plus a margin of `m` pixels.
    #[must_use]
    pub fn span(ax: f32, ay: f32, bx: f32, by: f32, m: f32) -> Self {
        Self(
            ax.min(bx) - m - 3.0,
            ay.min(by) - m - 3.0,
            ax.max(bx) + m + 3.0,
            ay.max(by) + m + 3.0,
        )
    }
}

impl Relief {
    /// Empty, fully transparent buffers.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        let n = width as usize * height as usize;
        Self {
            width,
            height,
            col: vec![[0.0; 3]; n],
            alpha: vec![0.0; n],
            z: vec![0.0; n],
            ink: vec![0.0; n],
        }
    }

    /// The whole image as bounds.
    #[must_use]
    pub fn all(&self) -> Bounds {
        Bounds(0.0, 0.0, self.width as f32, self.height as f32)
    }

    /// Paints a part over what is already there.
    ///
    /// `sdf` is negative inside; `mat(x, y, d)` gives the colour and
    /// `z(x, y, d)` the height in pixels. The part's edge is inked by `ink`.
    pub fn part(
        &mut self,
        b: Bounds,
        sdf: impl Fn(f32, f32) -> f32,
        mat: impl Fn(f32, f32, f32) -> Rgb,
        z: impl Fn(f32, f32, f32) -> f32,
        ink: Ink,
    ) {
        let x0 = b.0.floor().max(0.0) as u32;
        let y0 = b.1.floor().max(0.0) as u32;
        let x1 = (b.2.ceil().max(0.0) as u32).min(self.width);
        let y1 = (b.3.ceil().max(0.0) as u32).min(self.height);
        for y in y0..y1 {
            for x in x0..x1 {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let d = sdf(px, py);
                let cov = (0.5 - d).clamp(0.0, 1.0);
                if cov <= 0.0 {
                    continue;
                }
                let i = y as usize * self.width as usize + x as usize;
                let c = mat(px, py, d);
                let zn = z(px, py, d);
                let cf = [f32::from(c[0]), f32::from(c[1]), f32::from(c[2])];
                let a0 = self.alpha[i];
                if a0 <= 0.0 {
                    self.col[i] = cf;
                    self.z[i] = zn;
                } else {
                    for (k, v) in cf.iter().enumerate() {
                        self.col[i][k] += (v - self.col[i][k]) * cov;
                    }
                    self.z[i] += (zn - self.z[i]) * cov;
                }
                self.alpha[i] = a0 + cov * (1.0 - a0);
                let inkv = if ink.width > 0.0 {
                    ink.strength * (1.0 - (-d).max(0.0) / ink.width).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                self.ink[i] += (inkv - self.ink[i]) * cov;
            }
        }
    }

    /// Shades the height field from the top-left, inks outlines and returns
    /// straight-alpha RGBA. `relief` scales the form shading (about 1).
    #[must_use]
    pub fn finish(&self, relief: f32) -> Rgba {
        let (w, h) = (self.width as usize, self.height as usize);
        let mut out = Rgba::new(self.width, self.height);
        // Unit vector towards the light: from the top-left, 50° up.
        let (lx, ly, lz) = (-0.45_f32, -0.45_f32, 0.771_f32);
        let zat = |x: usize, y: usize, own: f32| {
            let i = y * w + x;
            if self.alpha[i] > 0.5 {
                self.z[i]
            } else {
                own
            }
        };
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let a = self.alpha[i];
                let ab = byte(a * 255.0);
                if ab == 0 {
                    continue;
                }
                let own = self.z[i];
                let zx = (zat((x + 1).min(w - 1), y, own) - zat(x.saturating_sub(1), y, own)) / 2.0;
                let zy = (zat(x, (y + 1).min(h - 1), own) - zat(x, y.saturating_sub(1), own)) / 2.0;
                let len = (zx * zx + zy * zy + 1.0).sqrt();
                let dot = (-zx * lx - zy * ly + lz) / len;
                let shade = (1.0 + relief * (dot - lz) / lz).clamp(0.3, 1.5);
                let ink = self.ink[i].clamp(0.0, 1.0);
                let mut px = [0u8; 4];
                for (k, v) in px.iter_mut().take(3).enumerate() {
                    let lit = self.col[i][k] * shade;
                    let dark = self.col[i][k] * 0.2 + 8.0;
                    *v = byte(lit + (dark - lit) * ink);
                }
                px[3] = ab;
                out.set(x as u32, y as u32, px);
            }
        }
        out
    }
}

/// A flat height `z0`.
pub fn level(z0: f32) -> impl Fn(f32, f32, f32) -> f32 {
    move |_, _, _| z0
}

/// A rounded bevel: `z0` at the edge rising by `rise` over `width` pixels.
pub fn bevel(z0: f32, rise: f32, width: f32) -> impl Fn(f32, f32, f32) -> f32 {
    move |_, _, d| {
        let t = (-d / width).clamp(0.0, 1.0);
        z0 + rise * t * (2.0 - t)
    }
}

/// A dome: a quarter-circle profile of radius `r` rising `rise` above `z0`.
pub fn dome(z0: f32, rise: f32, r: f32) -> impl Fn(f32, f32, f32) -> f32 {
    move |_, _, d| {
        let t = (-d / r).clamp(0.0, 1.0);
        let u = 1.0 - t;
        z0 + rise * (1.0 - u * u).max(0.0).sqrt()
    }
}

/// Adds `bump(x, y)` scaled by `amp` to a profile.
pub fn bumpy(
    profile: impl Fn(f32, f32, f32) -> f32,
    amp: f32,
    bump: impl Fn(f32, f32) -> f32,
) -> impl Fn(f32, f32, f32) -> f32 {
    move |x, y, d| profile(x, y, d) + amp * bump(x, y)
}
