//! Elevation shading from `Square.elevation_ft` (goal 62, vocabulary I20).
//!
//! Square elevations are interpolated into a smooth height field, sampled
//! through a gentle domain warp so contours meander instead of following
//! square edges. From it come:
//! - a slope hillshade lit from the top-left, the compositor's one sun;
//! - 5-ft contour steps drawn as ledges: a riser band on the uphill side of
//!   each contour, lit or shaded by the way it faces, with an inked lip and
//!   a soft contact shade at its foot. Where the slope is steep the ledges
//!   crowd together into a striated cliff face.
//!
//! Contours sit at 5k + 2.5 ft, halfway between the 5-ft steps squares use,
//! so flat terraces never lie on a contour.
// Pixel, grid and cell indices are bounded by the canvas size (at most
// 1024 ppsq × a few hundred squares), so these conversions cannot lose
// meaningful range.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]

use super::field::{centred, Field, Frame};
use crate::layout::TacticalLayout;

/// Contour interval in feet.
const STEP_FT: f32 = 5.0;
/// Elevation-warp amplitude in squares.
const WARP: f32 = 0.06;
/// Height-grid spacing in pixels.
const GRID: u32 = 4;
/// Slope (ft per ft) below which no ledges are drawn, and the ramp above
/// it over which they fade in. Smoothed 5-ft steps stay under about 1;
/// a 10-ft step across one square reaches 2 or more.
const LEDGE_SLOPE: (f32, f32) = (1.25, 0.75);

/// Square elevations relative to `base`, smoothed over a 5 × 5 window.
///
/// Elevations come rounded to 5-ft steps (vocabulary I20), so a steady
/// grade arrives as flat terraces joined by 5-ft risers. Averaging with
/// neighbours no more than 5 ft away recovers the underlying grade, while
/// neighbours across a rise of 10 ft or more are left out, so cliffs stay
/// sharp.
fn smoothed(layout: &TacticalLayout, base: i32) -> Vec<f32> {
    const K: [i32; 5] = [1, 4, 6, 4, 1];
    let (w, h) = (i64::from(layout.width), i64::from(layout.height));
    let mut out = Vec::with_capacity(layout.squares.len());
    for y in 0..h {
        for x in 0..w {
            let own = i32::from(layout.square(x, y).elevation_ft);
            let (mut sum, mut n) = (0i64, 0i64);
            for (j, ky) in K.iter().enumerate() {
                for (i, kx) in K.iter().enumerate() {
                    let (dx, dy) = (
                        i64::try_from(i).unwrap_or(2) - 2,
                        i64::try_from(j).unwrap_or(2) - 2,
                    );
                    let e = i32::from(layout.square(x + dx, y + dy).elevation_ft);
                    if (e - own).abs() <= 5 {
                        let k = i64::from(kx * ky);
                        sum += k * i64::from(e - base);
                        n += k;
                    }
                }
            }
            // Bounded elevation sums: exact enough in f32.
            #[allow(clippy::cast_precision_loss)]
            out.push(sum as f32 / n.max(1) as f32);
        }
    }
    out
}

/// The elevation field of a layout at one resolution.
pub struct Terrain<'a> {
    layout: &'a TacticalLayout,
    frame: Frame,
    ppsq: f32,
    /// Lowest elevation; heights are stored relative to it for precision.
    pub base: i32,
    elev: Vec<f32>,
    flat: Vec<bool>,
    grid: Option<(usize, usize, Vec<f32>)>,
}

impl<'a> Terrain<'a> {
    /// Builds the field. Flat maps skip every per-pixel cost.
    #[must_use]
    pub fn new(layout: &'a TacticalLayout, frame: &Frame, seed: u64) -> Self {
        let base = layout
            .squares
            .iter()
            .map(|s| i32::from(s.elevation_ft))
            .min()
            .unwrap_or(0);
        let (w, h) = (i64::from(layout.width), i64::from(layout.height));
        let elev = smoothed(layout, base);
        let at = |x: i64, y: i64| {
            elev[usize::try_from(y.clamp(0, h - 1) * w + x.clamp(0, w - 1)).unwrap_or(0)]
        };
        let mut flat = vec![true; elev.len()];
        for y in 0..h {
            for x in 0..w {
                let e = at(x, y);
                let same =
                    (-1..=1).all(|dy| (-1..=1).all(|dx| (at(x + dx, y + dy) - e).abs() < 1e-3));
                flat[usize::try_from(y * w + x).unwrap_or(0)] = same;
            }
        }
        #[allow(clippy::cast_precision_loss)]
        let ppsq = frame.ppsq as f32;
        let mut t = Self {
            layout,
            frame: *frame,
            ppsq,
            base,
            elev,
            flat,
            grid: None,
        };
        if !t.flat.iter().all(|f| *f) {
            let (cw, ch) = (layout.width * frame.ppsq, layout.height * frame.ppsq);
            let wx = Field::new(frame, cw, ch, |u, v| {
                centred(seed ^ 0xE1, u / 1.4, v / 1.4, 3)
            });
            let wy = Field::new(frame, cw, ch, |u, v| {
                centred(seed ^ 0xE2, u / 1.4, v / 1.4, 3)
            });
            let (gw, gh) = ((cw / GRID + 2) as usize, (ch / GRID + 2) as usize);
            let mut g = vec![0.0f32; gw * gh];
            {
                use rayon::prelude::*;
                let tr = &t;
                g.par_chunks_mut(gw).enumerate().for_each(|(j, row)| {
                    for (i, out) in row.iter_mut().enumerate() {
                        #[allow(clippy::cast_precision_loss)]
                        let (px, py) = (
                            (i as u32 * GRID) as f32 + 0.5,
                            (j as u32 * GRID) as f32 + 0.5,
                        );
                        *out = tr.exact(px, py, &wx, &wy);
                    }
                });
            }
            t.grid = Some((gw, gh, g));
        }
        t
    }

    /// The smoothed relative elevation of square `(x, y)` (clamped).
    #[must_use]
    pub fn square(&self, x: i64, y: i64) -> f32 {
        let (w, h) = (i64::from(self.layout.width), i64::from(self.layout.height));
        let i = (y.clamp(0, h - 1)) * w + x.clamp(0, w - 1);
        self.elev[usize::try_from(i).unwrap_or(0)]
    }

    fn own(&self, px: f32, py: f32) -> usize {
        let (x, y) = super::field::own_square(self.layout, px / self.ppsq, py / self.ppsq);
        usize::try_from(y * i64::from(self.layout.width) + x).unwrap_or(0)
    }

    /// Whether the pixel lies on flat ground (no shading, constant height).
    #[must_use]
    pub fn is_flat(&self, px: u32, py: u32) -> bool {
        #[allow(clippy::cast_precision_loss)]
        let i = self.own(px as f32 + 0.5, py as f32 + 0.5);
        self.flat[i]
    }

    /// The warped, interpolated height at a pixel position (no grid).
    ///
    /// Catmull-Rom bicubic interpolation of the smoothed square elevations:
    /// its slope is continuous, so a steady grade shades evenly instead of
    /// pulsing once per square as a smoothstep blend would.
    fn exact(&self, px: f32, py: f32, wx: &Field, wy: &Field) -> f32 {
        // World squares, so adjacent windows agree bit for bit.
        let (u, v) = self.frame.world_sq(px, py);
        let (u, v) = (u + WARP * wx.at(px, py), v + WARP * wy.at(px, py));
        let (gx, gy) = (u - 0.5, v - 0.5);
        let (fx, fy) = (gx.floor(), gy.floor());
        let (tx, ty) = (gx - fx, gy - fy);
        let (i, j) = (
            fx as i64 - self.frame.origin.0,
            fy as i64 - self.frame.origin.1,
        );
        let row = |y: i64| {
            catmull(
                self.square(i - 1, y),
                self.square(i, y),
                self.square(i + 1, y),
                self.square(i + 2, y),
                tx,
            )
        };
        catmull(row(j - 1), row(j), row(j + 1), row(j + 2), ty)
    }

    /// Height in feet above [`Terrain::base`] at a pixel position.
    #[must_use]
    // Grid indices of in-canvas positions.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn height(&self, px: f32, py: f32) -> f32 {
        let Some((gw, gh, g)) = &self.grid else {
            return self.elev[self.own(px, py)];
        };
        let (gx, gy) = ((px - 0.5) / GRID as f32, (py - 0.5) / GRID as f32);
        let (fx, fy) = (gx.max(0.0).floor(), gy.max(0.0).floor());
        let (i, j) = ((fx as usize).min(gw - 2), (fy as usize).min(gh - 2));
        let (tx, ty) = (
            (gx - i as f32).clamp(0.0, 1.0),
            (gy - j as f32).clamp(0.0, 1.0),
        );
        let a = g[j * gw + i];
        let b = g[j * gw + i + 1];
        let c = g[(j + 1) * gw + i];
        let d = g[(j + 1) * gw + i + 1];
        let top = a + (b - a) * tx;
        top + (c + (d - c) * tx - top) * ty
    }

    /// Heights of every pixel in 1/256 ft above [`Terrain::base`],
    /// row-major, computed in parallel.
    #[must_use]
    pub fn heights(&self, width: u32, height: u32) -> Vec<i32> {
        use rayon::prelude::*;
        let w = width as usize;
        let mut out = vec![0i32; w * height as usize];
        out.par_chunks_mut(w).enumerate().for_each(|(py, row)| {
            #[allow(clippy::cast_precision_loss)]
            let y = py as f32 + 0.5;
            for (px, v) in row.iter_mut().enumerate() {
                #[allow(clippy::cast_precision_loss)]
                let x = px as f32 + 0.5;
                *v = to_units(self.height(x, y));
            }
        });
        out
    }

    /// The shading multiplier at a pixel, hillshade times ledge shading,
    /// read from precomputed [`Terrain::heights`].
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn shade(&self, heights: &[i32], width: u32, px: u32, py: u32) -> f32 {
        if self.is_flat(px, py) {
            return 1.0;
        }
        let w = width as usize;
        let h = heights.len() / w.max(1);
        let at = |x: i64, y: i64| {
            let xx = usize::try_from(x.clamp(0, w as i64 - 1)).unwrap_or(0);
            let yy = usize::try_from(y.clamp(0, h as i64 - 1)).unwrap_or(0);
            heights[yy * w + xx] as f32 / 256.0
        };
        let (x, y) = (i64::from(px), i64::from(py));
        let e = at(x, y);
        // Central differences over most of a coarse-field cell smooth out
        // the kinks of the bilinearly interpolated warp.
        let b = 3;
        let grad = |x: i64, y: i64| {
            (
                (at(x + b, y) - at(x - b, y)) / (2.0 * b as f32),
                (at(x, y + b) - at(x, y - b)) / (2.0 * b as f32),
            )
        };
        let (gx, gy) = grad(x, y);
        // Slope in feet per foot: a pixel is 5/ppsq ft across.
        let k = self.ppsq / 5.0;
        let (sx, sy) = (gx * k, gy * k);
        let len = (sx * sx + sy * sy + 1.0).sqrt();
        let (lx, ly, lz) = (-0.5f32, -0.5f32, std::f32::consts::FRAC_1_SQRT_2);
        let dot = (-sx * lx - sy * ly + lz) / len;
        let mut hill = (1.0 + 0.55 * (dot - lz)).clamp(0.62, 1.25);
        let g = (gx * gx + gy * gy).sqrt();
        if g < 1e-4 {
            return hill;
        }
        let slope = g * k;
        // The rim of a steep face: where the slope steepens downhill the
        // ground breaks over a lip (lit); where it eases, a foot (shaded).
        if slope > LEDGE_SLOPE.0 * 0.4 {
            let reach = (self.ppsq * 0.08).max(3.0);
            let (dx, dy) = (-gx / g, -gy / g);
            #[allow(clippy::cast_possible_truncation)]
            let (ox, oy) = ((dx * reach).round() as i64, (dy * reach).round() as i64);
            let s_of = |(ax, ay): (f32, f32)| (ax * ax + ay * ay).sqrt() * k;
            let (down, upv) = (s_of(grad(x + ox, y + oy)), s_of(grad(x - ox, y - oy)));
            let rim = super::field::smooth(
                ((down.max(upv) - LEDGE_SLOPE.0) / LEDGE_SLOPE.1).clamp(0.0, 1.0),
            );
            if down > slope * 1.25 {
                hill *= 1.0 + 0.2 * rim * ((down / slope - 1.25) * 2.0).min(1.0);
            } else if upv > slope * 1.25 {
                hill *= 1.0 - 0.22 * rim * ((upv / slope - 1.25) * 2.0).min(1.0);
            }
        }
        let strength =
            super::field::smooth(((slope - LEDGE_SLOPE.0) / LEDGE_SLOPE.1).clamp(0.0, 1.0));
        if strength <= 0.0 {
            return hill;
        }
        let r = (e - STEP_FT / 2.0).rem_euclid(STEP_FT);
        let lip = r / g;
        let foot = (STEP_FT - r) / g;
        let width = (self.ppsq * 0.055).max(3.0);
        let mut ledge = 1.0;
        if lip < width {
            // The riser faces downhill; it is lit when that faces the sun.
            let facing = (gx + gy) / (g * std::f32::consts::SQRT_2);
            ledge *= 1.0 + 0.32 * facing * strength * (1.0 - lip / width);
            if lip < 1.4 {
                ledge *= 1.0 - 0.34 * strength;
            }
        } else if foot < width * 0.7 {
            ledge *= 1.0 - 0.1 * strength * (1.0 - foot / (width * 0.7));
        }
        hill * ledge
    }
}

/// Catmull-Rom spline through `p1` (t = 0) and `p2` (t = 1).
fn catmull(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let a = -0.5 * p0 + 1.5 * p1 - 1.5 * p2 + 0.5 * p3;
    let b = p0 - 2.5 * p1 + 2.0 * p2 - 0.5 * p3;
    let c = -0.5 * p0 + 0.5 * p2;
    ((a * t + b) * t + c) * t + p1
}

/// Feet to the height map's 1/256 ft.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // bounded elevations
pub fn to_units(ft: f32) -> i32 {
    (ft * 256.0).round() as i32
}
