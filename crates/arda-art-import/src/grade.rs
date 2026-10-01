//! An optional global colour grade toward a reference image or palette.
//!
//! One affine transform is applied to the whole library, so assets keep
//! their colours relative to each other. It matches the library's mean and
//! spread to the target's in an opponent space (`Y = (r+g+b)/3`,
//! `A = r − g`, `B = (r+g)/2 − b`), scaled by `strength`. The spread ratio
//! is clamped to 0.6–1.6 so a small palette cannot crush contrast.

use crate::ops::to_u8;
use arda_tactical::Rgba;

/// Mean and standard deviation per opponent channel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stats {
    /// Means of Y, A, B.
    pub mean: [f32; 3],
    /// Standard deviations of Y, A, B.
    pub std: [f32; 3],
}

fn opponent(p: [u8; 4]) -> [f32; 3] {
    let (r, g, b) = (f32::from(p[0]), f32::from(p[1]), f32::from(p[2]));
    [(r + g + b) / 3.0, r - g, (r + g) / 2.0 - b]
}

fn rgb(o: [f32; 3]) -> [f32; 3] {
    let [y, a, b] = o;
    [
        y + a / 2.0 + b / 3.0,
        y - a / 2.0 + b / 3.0,
        y - 2.0 * b / 3.0,
    ]
}

/// Accumulates alpha-weighted opponent statistics.
#[derive(Debug, Clone, Default)]
pub struct Accumulator {
    sum: [f64; 3],
    sq: [f64; 3],
    weight: f64,
}

impl Accumulator {
    /// Adds every pixel of an image, weighted by alpha.
    pub fn add_image(&mut self, img: &Rgba) {
        for p in img.data.as_chunks::<4>().0 {
            self.add(*p, f64::from(p[3]) / 255.0);
        }
    }

    /// Adds one colour with a weight.
    pub fn add(&mut self, p: [u8; 4], weight: f64) {
        if weight <= 0.0 {
            return;
        }
        let o = opponent(p);
        for (c, &v) in o.iter().enumerate() {
            let v = f64::from(v);
            self.sum[c] += v * weight;
            self.sq[c] += v * v * weight;
        }
        self.weight += weight;
    }

    /// The statistics, or `None` with no pixels.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn stats(&self) -> Option<Stats> {
        if self.weight <= 0.0 {
            return None;
        }
        let mean: [f64; 3] = std::array::from_fn(|c| self.sum[c] / self.weight);
        let std: [f64; 3] = std::array::from_fn(|c| {
            (self.sq[c] / self.weight - mean[c] * mean[c])
                .max(0.0)
                .sqrt()
        });
        Some(Stats {
            mean: mean.map(|v| v as f32),
            std: std.map(|v| v as f32),
        })
    }
}

/// The library-wide grade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grade {
    scale: [f32; 3],
    offset: [f32; 3],
}

impl Grade {
    /// The grade taking `source` statistics toward `target` by `strength`.
    #[must_use]
    pub fn new(source: Stats, target: Stats, strength: f32) -> Self {
        let s = strength.clamp(0.0, 1.0);
        let mut scale = [1.0; 3];
        let mut offset = [0.0; 3];
        for c in 0..3 {
            let k = if source.std[c] > 1.0 {
                (target.std[c] / source.std[c]).clamp(0.6, 1.6)
            } else {
                1.0
            };
            // v' = mt + (v − ms)·k, blended: v + s(v' − v).
            scale[c] = 1.0 - s + s * k;
            offset[c] = s * (target.mean[c] - source.mean[c] * k);
        }
        Self { scale, offset }
    }

    /// Applies the grade to every pixel (alpha untouched).
    pub fn apply(&self, img: &mut Rgba) {
        for p in img.data.as_chunks_mut::<4>().0.iter_mut() {
            if p[3] == 0 {
                continue;
            }
            let o = opponent(*p);
            let g: [f32; 3] = std::array::from_fn(|c| o[c] * self.scale[c] + self.offset[c]);
            let c = rgb(g);
            *p = [to_u8(c[0]), to_u8(c[1]), to_u8(c[2]), p[3]];
        }
    }

    /// The graded colour of one pixel, for the report.
    #[must_use]
    pub fn map(&self, p: [u8; 4]) -> [u8; 4] {
        let mut img = Rgba::filled(1, 1, p);
        self.apply(&mut img);
        img.get(0, 0)
    }
}

/// Statistics of a palette (each colour weighted equally).
#[must_use]
pub fn palette_stats(colours: &[[u8; 3]]) -> Option<Stats> {
    let mut acc = Accumulator::default();
    for c in colours {
        acc.add([c[0], c[1], c[2], 255], 1.0);
    }
    acc.stats()
}

/// The mean RGB a [`Stats`] describes, for the report.
#[must_use]
pub fn mean_rgb(s: &Stats) -> [u8; 3] {
    rgb(s.mean).map(to_u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opponent_round_trips() {
        let p = [200, 120, 40, 255];
        let back = rgb(opponent(p));
        for c in 0..3 {
            assert!((back[c] - f32::from(p[c])).abs() < 1e-3);
        }
    }

    #[test]
    fn full_strength_moves_the_mean_to_the_target() {
        let mut acc = Accumulator::default();
        let mut img = Rgba::filled(4, 4, [200, 200, 200, 255]);
        img.set(0, 0, [180, 180, 180, 255]);
        acc.add_image(&img);
        let src = acc.stats().unwrap();
        let tgt = palette_stats(&[[90, 110, 60], [110, 130, 70]]).unwrap();
        let g = Grade::new(src, tgt, 1.0);
        g.apply(&mut img);
        let mut after = Accumulator::default();
        after.add_image(&img);
        let m = after.stats().unwrap().mean;
        assert!((m[0] - tgt.mean[0]).abs() < 1.5, "{m:?} vs {:?}", tgt.mean);
    }
}
