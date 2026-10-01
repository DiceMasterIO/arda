//! Structured texture variants (cobbles, slabs, boards…) must share one
//! layout (README): the compositor cross-fades them in place, so offset
//! layouts would ghost double grout lines. Each variant is compared with the
//! first by the correlation of their high-pass luminance; a variant whose
//! layout is merely shifted is rolled into register (it tiles, so a roll is
//! lossless), and one that still differs is flagged.

// Pixel indices are bounded by image dimensions (u32), so casts are exact.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use crate::ops::{box_blur, luma};
use arda_tactical::Rgba;

/// Correlation below which variants' layouts count as different.
pub const MIN_LAYOUT_NCC: f32 = 0.35;
/// Coarse search grid step, px.
const STEP: usize = 4;

/// High-pass luminance (luma minus a 4-px wrapping blur), zero mean.
fn high_pass(img: &Rgba) -> Vec<f32> {
    let (w, h) = (img.width as usize, img.height as usize);
    let lum: Vec<f32> = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| luma(*p))
        .collect();
    let mut low = lum.clone();
    box_blur(&mut low, w, h, 4, true);
    let hp: Vec<f32> = lum.iter().zip(&low).map(|(a, b)| a - b).collect();
    let m = hp.iter().sum::<f32>() / hp.len().max(1) as f32;
    hp.iter().map(|v| v - m).collect()
}

/// Box-downsamples a plane by `STEP`.
fn shrink(p: &[f32], w: usize, h: usize) -> (Vec<f32>, usize, usize) {
    let (sw, sh) = (w / STEP, h / STEP);
    let mut out = vec![0.0; sw * sh];
    for y in 0..sh * STEP {
        for x in 0..sw * STEP {
            out[(y / STEP) * sw + x / STEP] += p[y * w + x];
        }
    }
    (out, sw, sh)
}

/// Normalised cross-correlation of `a` with `b` rolled by `(dx, dy)`.
fn ncc(a: &[f32], b: &[f32], w: usize, h: usize, (dx, dy): (usize, usize)) -> f32 {
    let (mut ab, mut aa, mut bb) = (0.0f32, 0.0f32, 0.0f32);
    for y in 0..h {
        for x in 0..w {
            let va = a[y * w + x];
            let vb = b[((y + dy) % h) * w + (x + dx) % w];
            ab += va * vb;
            aa += va * va;
            bb += vb * vb;
        }
    }
    let den = (aa * bb).sqrt();
    if den <= f32::EPSILON {
        0.0
    } else {
        ab / den
    }
}

/// Rolls a tileable image so that pixel `(dx, dy)` lands on `(0, 0)`.
#[must_use]
pub fn roll(img: &Rgba, dx: u32, dy: u32) -> Rgba {
    let (w, h) = (img.width, img.height);
    let mut out = Rgba::new(w, h);
    for y in 0..h {
        for x in 0..w {
            out.set(x, y, img.get((x + dx) % w, (y + dy) % h));
        }
    }
    out
}

/// The result of registering one variant against the first.
#[derive(Debug, Clone, PartialEq)]
pub struct Registration {
    /// Correlation before any roll.
    pub before: f32,
    /// Correlation after the chosen roll.
    pub after: f32,
    /// The roll applied, px.
    pub roll: (u32, u32),
}

/// Registers `img` against `reference` (same size), rolling it when a
/// shift correlates clearly better than none.
#[must_use]
pub fn register(reference: &Rgba, img: &Rgba) -> (Rgba, Registration) {
    let (w, h) = (img.width as usize, img.height as usize);
    let (ha, hb) = (high_pass(reference), high_pass(img));
    let before = ncc(&ha, &hb, w, h, (0, 0));
    let none = Registration {
        before,
        after: before,
        roll: (0, 0),
    };
    if (reference.width, reference.height) != (img.width, img.height)
        || w < STEP * 4
        || h < STEP * 4
    {
        return (img.clone(), none);
    }
    let (sa, sw, sh) = shrink(&ha, w, h);
    let (sb, _, _) = shrink(&hb, w, h);
    let mut best = (f32::MIN, (0usize, 0usize));
    for dy in 0..sh {
        for dx in 0..sw {
            let v = ncc(&sa, &sb, sw, sh, (dx, dy));
            if v > best.0 {
                best = (v, (dx, dy));
            }
        }
    }
    // Refine around the coarse optimum at full resolution.
    let (cx, cy) = (best.1 .0 * STEP, best.1 .1 * STEP);
    let mut fine = (f32::MIN, (0usize, 0usize));
    for oy in -3i64..=3 {
        for ox in -3i64..=3 {
            let dx = (cx as i64 + ox).rem_euclid(w as i64) as usize;
            let dy = (cy as i64 + oy).rem_euclid(h as i64) as usize;
            let v = ncc(&ha, &hb, w, h, (dx, dy));
            if v > fine.0 {
                fine = (v, (dx, dy));
            }
        }
    }
    if fine.1 == (0, 0) || fine.0 < before + 0.1 {
        return (img.clone(), none);
    }
    let r = (fine.1 .0 as u32, fine.1 .1 as u32);
    (
        roll(img, r.0, r.1),
        Registration {
            before,
            after: fine.0,
            roll: r,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bricks(tint: u8) -> Rgba {
        let mut img = Rgba::new(64, 64);
        for y in 0..64u32 {
            for x in 0..64u32 {
                let row = y / 16;
                let xo = (x + row % 2 * 8) % 16;
                let grout = y % 16 < 2 || xo < 2;
                let v = if grout { 60 } else { 150 + tint };
                img.set(x, y, [v, v - 20, v - 40, 255]);
            }
        }
        img
    }

    #[test]
    fn a_shifted_layout_is_rolled_into_register() {
        let a = bricks(0);
        let shifted = roll(&bricks(20), 37, 21);
        let (fixed, reg) = register(&a, &shifted);
        assert!(reg.before < MIN_LAYOUT_NCC, "{reg:?}");
        assert!(reg.after > 0.9, "{reg:?}");
        assert!(ncc(&high_pass(&a), &high_pass(&fixed), 64, 64, (0, 0)) > 0.9);
    }
}
