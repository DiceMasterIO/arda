//! A small RGB canvas that coverage masks are composited onto.
//!
//! Blending works in floating point and rounds back to bytes; the casts
//! are bounded by the clamps.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use crate::num::{iu, ui};
use crate::render::mask::Mask;

/// An RGB colour.
pub type Rgb = [u8; 3];

/// An 8-bit RGB raster.
#[derive(Debug, Clone)]
pub struct Canvas {
    /// Pixels across.
    pub width: usize,
    /// Pixels down.
    pub height: usize,
    /// Row-major RGB bytes.
    pub rgb: Vec<u8>,
}

impl Canvas {
    /// Blends `c` into a pixel with opacity `alpha` (0–1).
    pub fn blend(&mut self, x: i64, y: i64, c: Rgb, alpha: f32) {
        if x < 0 || y < 0 || x >= ui(self.width) || y >= ui(self.height) || alpha <= 0.0 {
            return;
        }
        let a = alpha.min(1.0);
        let k = (iu(y) * self.width + iu(x)) * 3;
        for (ch, &v) in c.iter().enumerate() {
            let old = f32::from(self.rgb[k + ch]);
            let new = old + (f32::from(v) - old) * a;
            self.rgb[k + ch] = new.round().clamp(0.0, 255.0) as u8;
        }
    }

    /// Multiplies a pixel by `c` (a tint that keeps the relief beneath)
    /// with opacity `alpha`.
    pub fn multiply(&mut self, x: i64, y: i64, c: Rgb, alpha: f32) {
        if x < 0 || y < 0 || x >= ui(self.width) || y >= ui(self.height) || alpha <= 0.0 {
            return;
        }
        let k = (iu(y) * self.width + iu(x)) * 3;
        for (ch, &v) in c.iter().enumerate() {
            let old = f32::from(self.rgb[k + ch]);
            let mul = old * f32::from(v) / 255.0;
            let new = old + (mul - old) * alpha.min(1.0);
            self.rgb[k + ch] = new.round().clamp(0.0, 255.0) as u8;
        }
    }

    /// Composites `c` through a coverage mask at `opacity`.
    pub fn paint(&mut self, m: &Mask, c: Rgb, opacity: f32) {
        for y in 0..m.h {
            for x in 0..m.w {
                let a = m.a[y * m.w + x];
                if a > 0.0 {
                    self.blend(m.x0 + ui(x), m.y0 + ui(y), c, a * opacity);
                }
            }
        }
    }

    /// A mask covering the whole canvas.
    #[must_use]
    pub fn mask(&self) -> Mask {
        Mask::new(0, 0, self.width, self.height)
    }
}
