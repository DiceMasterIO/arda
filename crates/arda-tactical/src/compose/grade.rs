//! Optional warm colour grading, matched by eye to the reference maps in
//! `assets/reference/tactical-target/`: a touch less saturation, a warm
//! cast (more red, less blue), a gentle contrast lift and warm shadows.
//! Integer maths on a per-pixel basis, so it stays byte-deterministic.
// Pixel, grid and cell indices are bounded by the canvas size (at most
// 1024 ppsq × a few hundred squares), so these conversions cannot lose
// meaningful range.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]

use crate::raster::Rgba;
use rayon::prelude::*;

/// Grades the canvas in place.
pub fn warm(canvas: &mut Rgba) {
    canvas.data.par_chunks_mut(4 * 1024).for_each(|chunk| {
        for p in chunk.as_chunks_mut::<4>().0.iter_mut() {
            let [r, g, b] = [i32::from(p[0]), i32::from(p[1]), i32::from(p[2])];
            // Luma in 1/1024.
            let l = (306 * r + 601 * g + 117 * b) / 1024;
            // Desaturate by 5 %.
            let d = |c: i32| l + (c - l) * 19 / 20;
            let (r, g, b) = (d(r), d(g), d(b));
            // Warm cast, fading out in the highlights so whites stay white.
            let t = (1024 - (l - 128).max(0) * 6).clamp(0, 1024);
            let (r, g, b) = (
                r + r * 52 * t / (1024 * 1024) + 2,
                g + g * 30 * t / (1024 * 1024),
                b - b * 80 * t / (1024 * 1024),
            );
            // Contrast about mid-grey, then a warm lift in the shadows.
            let c = |v: i32| 128 + (v - 128) * 1100 / 1024;
            let shadow = (255 - l).max(0);
            let lift = shadow * shadow / 255;
            let (r, g, b) = (c(r) + lift * 9 / 255, c(g) + lift * 5 / 255, c(b));
            for (o, v) in p.iter_mut().zip([r, g, b]) {
                *o = u8::try_from(v.clamp(0, 255)).unwrap_or(255);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grading_warms_neutral_grey_and_keeps_alpha() {
        let mut img = Rgba::filled(4, 4, [128, 128, 128, 255]);
        warm(&mut img);
        let p = img.get(1, 1);
        assert!(p[0] > p[1] && p[1] > p[2], "{p:?}");
        assert_eq!(p[3], 255);
    }
}
