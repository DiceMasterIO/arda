//! The optional world grade (goal 49: the tactical view is "visually
//! continuous with the world map when zooming").
//!
//! The ground and water layers are pulled toward the world map's own colour
//! at their location before any sprite is drawn, so props, walls and roofs
//! keep their art. The world colour comes from a world-anchored lattice
//! (see [`WorldTint`]) interpolated at each pixel's world position, so two
//! renders that share a pixel tint it identically and seams stay exact.
//!
//! Detail survives because the tint is a ratio: each channel is scaled by
//! `1 − s + s · world / mean`, where `mean` is the regional mean of the
//! ground textures (7 × 7 squares) or the water texture, interpolated
//! between square centres. Grain, stones, tufts, paths, yards and the bank
//! line keep their contrast; only the palette moves. Laid surfaces
//! (cobbles, flagstones, planks) take a quarter of the strength and worked
//! ground (farmland) half, so streets, floors and fields stay what they are.
// Pixel and square indices are bounded by the canvas size.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use super::sample::TextureSet;
use super::{WATER_DEEP, WATER_SHALLOW};
use crate::error::TacticalError;
use crate::layout::TacticalLayout;
use crate::raster::Rgba;
use rayon::prelude::*;

/// Q12 one.
const ONE: i64 = 4_096;
/// Default pull toward the world colour, Q12 (0.7).
pub const DEFAULT_STRENGTH_Q12: i64 = 2_867;
/// Share of the strength laid surfaces take, Q12 (0.25).
const LAID_Q12: i64 = 1_024;
/// Share of the strength worked ground takes, Q12 (0.5): the world map
/// draws no fields, so ploughland keeps half its own colour.
const WORKED_Q12: i64 = 2_048;

/// World-map colours on a lattice of world squares: point `(i, j)` sits at
/// the square-grid corner `origin + (i, j) · step`. The origin must be a
/// multiple of the step so every render samples the same points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldTint {
    /// World square of point `(0, 0)`.
    pub origin: [i64; 2],
    /// Squares between points.
    pub step: u32,
    /// Points across and down.
    pub size: (usize, usize),
    /// World ground colour per point, row-major.
    pub ground: Vec<[u8; 3]>,
    /// World water colour per point, row-major.
    pub water: Vec<[u8; 3]>,
    /// Pull toward the world colour, Q12.
    pub strength_q12: i64,
}

impl WorldTint {
    /// Checks the lattice against the canvas it must cover.
    fn check(&self, layout: &TacticalLayout) -> Result<[i64; 2], TacticalError> {
        let bad = |m: &str| TacticalError::Options(format!("world tint: {m}"));
        let step = i64::from(self.step);
        let n = self.size.0 * self.size.1;
        if step == 0 || self.size.0 < 2 || self.size.1 < 2 {
            return Err(bad("the lattice needs a step and two points a side"));
        }
        if self.ground.len() != n || self.water.len() != n {
            return Err(bad("colour count does not match the lattice"));
        }
        if self.origin.iter().any(|o| o.rem_euclid(step) != 0) {
            return Err(bad("the origin is not on the world lattice"));
        }
        let origin = layout
            .origin
            .ok_or_else(|| bad("the layout has no world origin"))?;
        let last = |o: i64, k: usize| o + step * (i64::try_from(k).unwrap_or(0) - 1);
        let covers = |a: usize, extent: u32| {
            self.origin[a] <= origin[a]
                && last(self.origin[a], [self.size.0, self.size.1][a])
                    >= origin[a] + i64::from(extent)
        };
        if !covers(0, layout.width) || !covers(1, layout.height) {
            return Err(bad("the lattice does not cover the layout"));
        }
        Ok(origin)
    }

    /// Bilinear ground and water colours at a world position in squares
    /// (Q16).
    fn at(&self, x_q16: i64, y_q16: i64) -> ([i64; 3], [i64; 3]) {
        let span = i64::from(self.step) << 16;
        let fx = x_q16 - (self.origin[0] << 16);
        let fy = y_q16 - (self.origin[1] << 16);
        let (i, j) = (fx.div_euclid(span), fy.div_euclid(span));
        let (tx, ty) = (
            fx.rem_euclid(span) * ONE / span,
            fy.rem_euclid(span) * ONE / span,
        );
        let (w, h) = (self.size.0 as i64, self.size.1 as i64);
        let idx = |a: i64, b: i64| (b.clamp(0, h - 1) * w + a.clamp(0, w - 1)) as usize;
        let lerp = |v: &[[u8; 3]]| -> [i64; 3] {
            let p = |a, b| v[idx(a, b)].map(i64::from);
            let (a, b, c, d) = (p(i, j), p(i + 1, j), p(i, j + 1), p(i + 1, j + 1));
            std::array::from_fn(|ch| {
                let top = a[ch] * (ONE - tx) + b[ch] * tx;
                let bottom = c[ch] * (ONE - tx) + d[ch] * tx;
                (top * (ONE - ty) + bottom * ty) / (ONE * ONE)
            })
        };
        (lerp(&self.ground), lerp(&self.water))
    }
}

/// The share of the strength a ground key takes, Q12: laid surfaces
/// (streets, floors) a quarter, worked ground half, the rest all of it.
fn share(key: &str) -> i64 {
    match key {
        "cobbles" | "flagstone" | "stone_floor" | "planks" | "rug" => LAID_Q12,
        "farmland" | "packed_earth" => WORKED_Q12,
        _ => ONE,
    }
}

/// Squares on each side of the region whose mean ground colour the tint
/// moves: materials that change within about 10 m (paths, yards, scree
/// patches) keep their contrast; only the regional palette shifts. Within
/// the render apron (6 squares) with the interpolation's one square.
const REGION_RADIUS: i64 = 3;

/// Mean of `v` over the clamped `(2r + 1)²` box around each square.
fn box_mean(v: &[[i64; 3]], (w, h): (i64, i64), r: i64) -> Vec<[i64; 3]> {
    let at = |x: i64, y: i64| v[(y * w + x) as usize];
    let mut out = Vec::with_capacity(v.len());
    for y in 0..h {
        for x in 0..w {
            let (mut sum, mut n) = ([0_i64; 3], 0_i64);
            for yy in (y - r).max(0)..=(y + r).min(h - 1) {
                for xx in (x - r).max(0)..=(x + r).min(w - 1) {
                    let c = at(xx, yy);
                    for ch in 0..3 {
                        sum[ch] += c[ch];
                    }
                    n += 1;
                }
            }
            out.push(sum.map(|s| (s / n.max(1)).max(1)));
        }
    }
    out
}

/// Per-square regional ground means, water texture means and strength, Q12.
struct Squares {
    width: i64,
    height: i64,
    ground: Vec<[i64; 3]>,
    water: Vec<[i64; 3]>,
    strength: Vec<i64>,
}

fn squares(layout: &TacticalLayout, textures: &TextureSet, strength: i64) -> Squares {
    let mean = |key: &str| {
        textures
            .get(key)
            .map(|t| t.mean.map(|v| (v.round() as i64).clamp(1, 255)))
    };
    let shallow = mean(WATER_SHALLOW).unwrap_or([60, 110, 120]);
    let deep = mean(WATER_DEEP).unwrap_or(shallow);
    let mut out = Squares {
        width: i64::from(layout.width),
        height: i64::from(layout.height),
        ground: Vec::new(),
        water: Vec::with_capacity(layout.squares.len()),
        strength: Vec::with_capacity(layout.squares.len()),
    };
    let own: Vec<[i64; 3]> = layout
        .squares
        .iter()
        .map(|s| mean(&s.ground).unwrap_or([128, 128, 128]))
        .collect();
    out.ground = box_mean(&own, (out.width, out.height), REGION_RADIUS);
    for s in &layout.squares {
        out.water
            .push(if s.water_depth_ft >= 5 { deep } else { shallow });
        out.strength.push(strength * share(&s.ground) / ONE);
    }
    out
}

impl Squares {
    /// Values interpolated between square centres at local `(u, v)` in
    /// squares, Q16.
    fn at(&self, u: i64, v: i64) -> ([i64; 3], [i64; 3], i64) {
        let half = 1 << 15;
        let (fu, fv) = (u - half, v - half);
        let (i, j) = (fu >> 16, fv >> 16);
        let (tx, ty) = (((fu & 0xFFFF) * ONE) >> 16, ((fv & 0xFFFF) * ONE) >> 16);
        let idx = |a: i64, b: i64| {
            (b.clamp(0, self.height - 1) * self.width + a.clamp(0, self.width - 1)) as usize
        };
        let corners = [idx(i, j), idx(i + 1, j), idx(i, j + 1), idx(i + 1, j + 1)];
        let wts = [
            (ONE - tx) * (ONE - ty),
            tx * (ONE - ty),
            (ONE - tx) * ty,
            tx * ty,
        ];
        let mut g = [0_i64; 3];
        let mut w = [0_i64; 3];
        let mut s = 0_i64;
        for (k, wt) in corners.iter().zip(wts) {
            for ch in 0..3 {
                g[ch] += self.ground[*k][ch] * wt;
                w[ch] += self.water[*k][ch] * wt;
            }
            s += self.strength[*k] * wt;
        }
        let q = ONE * ONE;
        (g.map(|v| (v / q).max(1)), w.map(|v| (v / q).max(1)), s / q)
    }
}

/// Scales `p` toward `world` by `s`, relative to the local mean `mean`.
fn pull(p: i64, world: i64, mean: i64, s: i64) -> i64 {
    // Ratio clamped to 0.4..2.5 so a mismatched mean cannot blow out.
    let ratio = (world * ONE / mean).clamp(1_638, 10_240);
    p * (ONE - s + s * ratio / ONE) / ONE
}

/// Tints the ground and water already painted on `canvas` (pixels
/// `[x0, y0, x1, y1)` only) toward the world colours of `tint`; `water` is
/// the water layer's coverage, 0–255 per pixel.
///
/// # Errors
/// A lattice that is malformed, off the world grid or too small.
pub fn apply(
    canvas: &mut Rgba,
    layout: &TacticalLayout,
    (textures, water): (&TextureSet, &[u8]),
    ppsq: u32,
    tint: &WorldTint,
    [x0, y0, x1, y1]: [u32; 4],
) -> Result<(), TacticalError> {
    let origin = tint.check(layout)?;
    let sq = squares(layout, textures, tint.strength_q12.clamp(0, ONE));
    let w = canvas.width as usize;
    let p = i64::from(ppsq);
    canvas
        .data
        .par_chunks_mut(w * 4)
        .enumerate()
        .for_each(|(y, row)| {
            let y = y as u32;
            if y < y0 || y >= y1 {
                return;
            }
            // Pixel centre in local squares (Q16) and in world squares.
            let v = (2 * i64::from(y) + 1) * 32_768 / p;
            let wy = (2 * (origin[1] * p + i64::from(y)) + 1) * 32_768 / p;
            for x in x0..x1.min(canvas.width) {
                let i = x as usize;
                let u = (2 * i64::from(x) + 1) * 32_768 / p;
                let wx = (2 * (origin[0] * p + i64::from(x)) + 1) * 32_768 / p;
                let (world_ground, world_water) = tint.at(wx, wy);
                let (mean_ground, mean_water, s) = sq.at(u, v);
                let m = i64::from(water.get(y as usize * w + i).copied().unwrap_or(0));
                let Some(px) = row.get_mut(i * 4..i * 4 + 3) else {
                    continue;
                };
                for ch in 0..3 {
                    let c = i64::from(px[ch]);
                    let g = pull(c, world_ground[ch], mean_ground[ch], s);
                    let wv = pull(c, world_water[ch], mean_water[ch], tint.strength_q12);
                    let out = (g * (255 - m) + wv * m) / 255;
                    px[ch] = u8::try_from(out.clamp(0, 255)).unwrap_or(255);
                }
            }
        });
    Ok(())
}

#[cfg(test)]
mod tests;
