//! Stratified ground drawn along the contours (review gallery 2026-10-15,
//! images 12 and 13).
//!
//! A cliff's art is a rock face in elevation: horizontal strata bands. The
//! stochastic tiling of [`super::sample`] gives every 1.75-square cell its
//! own texel offset, which staggers the bands from cell to cell into a
//! brick or plank pattern, and the bands ran east–west whichever way the
//! face looked, across an east-facing escarpment as well as along a
//! south-facing one. Here a banded key is sampled in a frame turned so its
//! bands follow the local contour (the steepest-descent direction read
//! from the canvas heights), and every cell keeps the band phase, shifting
//! its texels only along the bands. The turn is quantised to
//! [`DIRS`] (15° apart; bands look the same turned half round), and a
//! pixel between two directions blends both, so each direction is one
//! rotation of the whole world and windows agree with cells.
// Pixel and world coordinates are bounded by the canvas and world sizes.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]

use super::field::Frame;
use super::sample::{axis_weights, fetch, CellTable, KeyTextures, CELL};

/// Band directions `(cos, sin)` from 0° to 180° in 15° steps (exact
/// constants, so no platform trigonometry enters the image).
pub const DIRS: [(f64, f64); 13] = [
    (1.0, 0.0),
    (0.965_925_826_289_068_3, 0.258_819_045_102_520_74),
    (0.866_025_403_784_438_6, 0.5),
    (
        std::f64::consts::FRAC_1_SQRT_2,
        std::f64::consts::FRAC_1_SQRT_2,
    ),
    (0.5, 0.866_025_403_784_438_6),
    (0.258_819_045_102_520_74, 0.965_925_826_289_068_3),
    (0.0, 1.0),
    (-0.258_819_045_102_520_74, 0.965_925_826_289_068_3),
    (-0.5, 0.866_025_403_784_438_6),
    (
        -std::f64::consts::FRAC_1_SQRT_2,
        std::f64::consts::FRAC_1_SQRT_2,
    ),
    (-0.866_025_403_784_438_6, 0.5),
    (-0.965_925_826_289_068_3, 0.258_819_045_102_520_74),
    (-1.0, 0.0),
];

/// Ground keys whose art is a stratified face drawn in elevation.
#[must_use]
pub fn banded(key: &str) -> bool {
    key == "cliff"
}

/// The band direction at a pixel as a pair of neighbouring [`DIRS`]
/// entries `(a, a + 1)` and the share of the second, from the per-pixel
/// `heights` of a canvas `width` pixels wide. Flat ground keeps the
/// art's own east–west bands.
#[must_use]
pub fn contour(heights: &[i32], width: u32, px: u32, py: u32) -> (usize, f32) {
    let w = width as usize;
    let h = heights.len() / w.max(1);
    if w == 0 || h == 0 {
        return (0, 0.0);
    }
    let at = |x: i64, y: i64| {
        let xx = usize::try_from(x.clamp(0, w as i64 - 1)).unwrap_or(0);
        let yy = usize::try_from(y.clamp(0, h as i64 - 1)).unwrap_or(0);
        f64::from(heights[yy * w + xx])
    };
    // The same three-pixel reach as the hillshade (`terrain::shade`).
    let (x, y, b) = (i64::from(px), i64::from(py), 3);
    let gx = at(x + b, y) - at(x - b, y);
    let gy = at(x, y + b) - at(x, y - b);
    let len = (gx * gx + gy * gy).sqrt();
    if len < 1.0 {
        return (0, 0.0);
    }
    // Along the contour, turned into the upper half plane.
    let (mut dx, mut dy) = (-gy / len, gx / len);
    if dy < 0.0 || (dy == 0.0 && dx < 0.0) {
        (dx, dy) = (-dx, -dy);
    }
    let cross = |(c, s): (f64, f64)| c * dy - s * dx;
    let a = (0..DIRS.len() - 1)
        .rev()
        .find(|&i| cross(DIRS[i]) >= 0.0)
        .unwrap_or(0);
    let (pa, pb) = (cross(DIRS[a]), -cross(DIRS[a + 1]));
    let t = if pa + pb > 0.0 { pa / (pa + pb) } else { 0.0 };
    (a, t.clamp(0.0, 1.0) as f32)
}

/// Samples banded key `tex` at a pixel with its bands along direction
/// `(a, t)` from [`contour`]; otherwise as [`super::sample::sample_finish`].
#[must_use]
pub fn sample_banded(
    tex: &KeyTextures,
    cells: &CellTable,
    frame: &Frame,
    (px, py): (u32, u32),
    warp: (f32, f32),
    finish: Option<usize>,
    (a, t): (usize, f32),
) -> [f32; 3] {
    let one = |dir: (f64, f64)| turned(tex, cells, frame, (px, py), warp, finish, dir);
    let c0 = one(DIRS[a.min(DIRS.len() - 1)]);
    if t <= 0.0 || a + 1 >= DIRS.len() {
        return c0;
    }
    let c1 = one(DIRS[a + 1]);
    std::array::from_fn(|k| c0[k] + (c1[k] - c0[k]) * t)
}

/// Samples a key at a pixel: along the contours read from `along` (the
/// canvas heights and width) for a banded key, else as
/// [`super::sample::sample_finish`].
#[must_use]
pub fn sample(
    tex: &KeyTextures,
    cells: &CellTable,
    frame: &Frame,
    (px, py): (u32, u32),
    warp: (f32, f32),
    finish: Option<usize>,
    along: Option<(&[i32], u32)>,
) -> [f32; 3] {
    match along {
        Some((heights, width)) => {
            let dir = contour(heights, width, px, py);
            sample_banded(tex, cells, frame, (px, py), warp, finish, dir)
        }
        None => super::sample::sample_finish(tex, cells, frame, (px, py), warp, finish),
    }
}

/// One sample in the frame turned to `(cos, sin)`: cells blend linearly
/// (their bands are in phase), each shifting its texels along the bands.
fn turned(
    tex: &KeyTextures,
    cells: &CellTable,
    frame: &Frame,
    (px, py): (u32, u32),
    warp: (f32, f32),
    finish: Option<usize>,
    (c, s): (f64, f64),
) -> [f32; 3] {
    let (wx, wy) = frame.world_px(px, py);
    let (fx, fy) = (wx as f64 + 0.5, wy as f64 + 0.5);
    let tx = (c * fx + s * fy).floor() as i64;
    let ty = (c * fy - s * fx).floor() as i64;
    let (u, v) = frame.world_sq(px as f32 + 0.5, py as f32 + 0.5);
    let cx = (u + warp.0) / CELL;
    let cy = (v + warp.1) / CELL;
    let (ci, cj) = (cx.floor() as i64, cy.floor() as i64);
    let (ax, ay) = (axis_weights(cx - cx.floor()), axis_weights(cy - cy.floor()));
    let mut acc = [0.0f32; 3];
    let mut wsum = 0.0f32;
    for (dx, wxw) in ax {
        for (dy, wyw) in ay {
            let w = wxw * wyw;
            if w <= 0.0 {
                continue;
            }
            let (var, ox, _) = cells.get(tex, ci + dx, cj + dy);
            let var = finish.filter(|&f| f < tex.variants.len()).unwrap_or(var);
            let texel = fetch(tex, var, tx + ox, ty);
            for k in 0..3 {
                acc[k] += w * texel[k];
            }
            wsum += w;
        }
    }
    let k = 1.0 / wsum.max(1e-6);
    acc.map(|v| (v * k).clamp(0.0, 255.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Heights rising `(sx, sy)` units per pixel over a 64 × 64 canvas.
    fn plane(sx: i32, sy: i32) -> Vec<i32> {
        (0..64 * 64)
            .map(|i| sx * (i % 64) + sy * (i / 64))
            .collect()
    }

    /// The band angle in degrees from a [`contour`] answer.
    fn angle((a, t): (usize, f32)) -> f32 {
        (a as f32 + t) * 15.0
    }

    #[test]
    fn bands_follow_the_contours() {
        // Rising southward: contours run east–west.
        assert!(angle(contour(&plane(0, 50), 64, 32, 32)).abs() < 1e-3);
        // Rising eastward: contours run north–south.
        assert!((angle(contour(&plane(50, 0), 64, 32, 32)) - 90.0).abs() < 1e-3);
        // Rising to the south-east: contours run north-east to south-west.
        assert!((angle(contour(&plane(50, 50), 64, 32, 32)) - 135.0).abs() < 1e-3);
        // Between two table directions the share runs between them.
        let (a, t) = contour(&plane(20, 50), 64, 32, 32);
        assert!(t > 0.0 && t < 1.0, "{a} {t}");
        assert_eq!(a, 10);
        // Flat ground keeps the art's bands.
        assert_eq!(contour(&plane(0, 0), 64, 32, 32), (0, 0.0));
    }

    #[test]
    fn the_table_is_unit_and_fifteen_degrees_apart() {
        for w in DIRS.windows(2) {
            let ((c0, s0), (c1, s1)) = (w[0], w[1]);
            assert!((c0 * c0 + s0 * s0 - 1.0).abs() < 1e-12);
            let dot = c0 * c1 + s0 * s1;
            assert!((dot - 0.965_925_826_289_068_3).abs() < 1e-12);
        }
    }
}
