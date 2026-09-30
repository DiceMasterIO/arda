//! Painted materials for cut-out art, a small deterministic RNG and colour
//! helpers. Every material is a closure `(x, y, d) -> Rgb` for
//! [`super::relief::Relief::part`]; the shading comes later from the height
//! field, so materials only carry colour, grain and wear.
// Art generation casts bounded pixel coordinates and cell indices.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use super::paint::{mix, tone, Rgb};
use crate::noise::{fbm, hash2, mix as mix64, sincos, unit, value};

/// A tiny deterministic generator (SplitMix64 steps).
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// A generator from a seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(mix64(seed ^ 0xA5A5_5A5A_1234_4321))
    }

    /// The next raw value.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix64(self.0)
    }

    /// Uniform in `[0, 1)`.
    pub fn f(&mut self) -> f32 {
        unit(self.next_u64())
    }

    /// Uniform in `[a, b)`.
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }

    /// A random unit vector `(x, y)`.
    pub fn dir(&mut self) -> (f32, f32) {
        let (s, c) = sincos(self.f());
        (c, s)
    }

    /// A random element of a non-empty slice (the first if empty-safe).
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> Option<T> {
        let n = xs.len() as u64;
        if n == 0 {
            return None;
        }
        xs.get((self.next_u64() % n) as usize).copied()
    }
}

/// A unit vector at `turns` (1.0 = full circle), `(x, y)` with y south.
#[must_use]
pub fn heading(turns: f32) -> (f32, f32) {
    let (s, c) = sincos(turns);
    (c, s)
}

/// Warm/cool hue drift: `t` in `[0, 1]`, 0.5 leaves the colour unchanged.
#[must_use]
pub fn drift(c: Rgb, t: f32) -> Rgb {
    let k = (t - 0.5) * 2.0;
    let f = |v: u8, g: f32| super::paint::byte(f32::from(v) * (1.0 + g * k));
    [f(c[0], 0.05), f(c[1], 0.012), f(c[2], -0.06)]
}

/// Painterly colour: broad and fine rotated noise, hue drift and grain.
pub fn painted(seed: u64, base: Rgb, scale: f32) -> impl Fn(f32, f32, f32) -> Rgb {
    move |x, y, _| {
        let broad = fbm(seed, x / (scale * 3.0), y / (scale * 3.0), 2, None);
        let fine = fbm(seed ^ 0x51, x / scale, y / scale, 3, None);
        let grain = unit(hash2(seed ^ 0x55, x as i64, y as i64));
        let c = drift(base, broad);
        tone(c, 0.8 + 0.34 * fine + 0.06 * grain)
    }
}

/// Wood with grain along `dir` (a unit vector) and boards `board` pixels
/// wide across it. `board = 0` gives one solid piece.
pub fn wood(seed: u64, base: Rgb, dir: (f32, f32), board: f32) -> impl Fn(f32, f32, f32) -> Rgb {
    move |x, y, _| {
        let u = x * dir.0 + y * dir.1;
        let v = -x * dir.1 + y * dir.0;
        let (idx, seam) = if board > 0.0 {
            let b = (v / board).floor();
            (b as i64, (v - b * board) < 1.1)
        } else {
            (0, false)
        };
        let sd = mix64(seed ^ idx.cast_unsigned());
        let tint = unit(sd);
        let grain =
            0.6 * value(sd, u / 26.0, v / 1.6, None) + 0.4 * value(sd ^ 3, u / 9.0, v / 0.9, None);
        let c = drift(tone(base, 0.84 + 0.26 * tint), unit(mix64(sd)));
        let c = tone(c, 0.78 + 0.36 * grain);
        if seam {
            tone(base, 0.42)
        } else {
            c
        }
    }
}

/// Mottled stone.
pub fn stone(seed: u64, base: Rgb) -> impl Fn(f32, f32, f32) -> Rgb {
    move |x, y, _| {
        let broad = fbm(seed, x / 22.0, y / 22.0, 2, None);
        let mid = fbm(seed ^ 1, x / 6.0, y / 6.0, 3, None);
        let grain = unit(hash2(seed ^ 2, x as i64, y as i64));
        let lichen = fbm(seed ^ 3, x / 9.0, y / 9.0, 2, None);
        let mut c = tone(drift(base, broad), 0.78 + 0.3 * mid + 0.1 * grain);
        if lichen > 0.66 {
            c = mix(c, [132, 140, 84], ((lichen - 0.66) * 4.0).min(0.45));
        }
        c
    }
}

/// Woven cloth with soft folds along `dir`.
pub fn cloth(seed: u64, base: Rgb, dir: (f32, f32)) -> impl Fn(f32, f32, f32) -> Rgb {
    move |x, y, _| {
        let u = x * dir.0 + y * dir.1;
        let v = -x * dir.1 + y * dir.0;
        let fold = value(seed, u / 30.0, v / 7.0, None);
        let weave = if ((x as i64 + y as i64) & 1) == 0 {
            1.0
        } else {
            0.95
        };
        let blot = fbm(seed ^ 9, x / 14.0, y / 14.0, 2, None);
        tone(drift(base, blot), (0.8 + 0.3 * fold) * weave)
    }
}

/// Dull metal with faint rust.
pub fn metal(seed: u64, base: Rgb) -> impl Fn(f32, f32, f32) -> Rgb {
    move |x, y, _| {
        let n = fbm(seed, x / 5.0, y / 5.0, 2, None);
        let rust = fbm(seed ^ 7, x / 11.0, y / 11.0, 2, None);
        let c = tone(base, 0.86 + 0.24 * n);
        if rust > 0.64 {
            mix(c, [120, 70, 40], ((rust - 0.64) * 3.0).min(0.5))
        } else {
            c
        }
    }
}

/// Straw or hay: streaks along `dir`.
pub fn straw(seed: u64, base: Rgb, dir: (f32, f32)) -> impl Fn(f32, f32, f32) -> Rgb {
    move |x, y, _| {
        let u = x * dir.0 + y * dir.1;
        let v = -x * dir.1 + y * dir.0;
        let s = value(seed, u / 12.0, v / 1.2, None);
        let t = value(seed ^ 5, u / 5.0, v / 0.8, None);
        tone(drift(base, s), 0.72 + 0.28 * s + 0.2 * t)
    }
}

/// Foliage: leaf-cluster noise between a dark and a light tone.
pub fn leafy(seed: u64, base: Rgb, scale: f32) -> impl Fn(f32, f32, f32) -> Rgb {
    move |x, y, _| {
        let clusters = fbm(seed, x / scale, y / scale, 3, None);
        let fine = unit(hash2(seed ^ 1, x as i64 / 2, y as i64 / 2));
        let c = mix(tone(base, 0.72), tone(base, 1.18), clusters);
        tone(drift(c, fine), 0.94 + 0.12 * fine)
    }
}

/// A flat colour.
pub fn solid(c: Rgb) -> impl Fn(f32, f32, f32) -> Rgb {
    move |_, _, _| c
}
