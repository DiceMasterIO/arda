//! Hand-rolled deterministic hashing, a small PRNG and 1-D value noise.
//!
//! Every plot, building and prop draws from a stream seeded by the world
//! seed, the settlement id and its own key (goal-prompt §6 principles), so
//! any piece can be regenerated alone.

use crate::num::{bits, floor_i, index_of};

/// SplitMix64 finaliser.
#[must_use]
pub const fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Hash of a seed and two integers.
#[must_use]
pub const fn hash(seed: u64, a: u64, b: u64) -> u64 {
    mix(mix(seed ^ mix(a)) ^ b.wrapping_mul(0xD6E8_FEB8_6659_FD93))
}

/// Hash of a seed and two signed integers (grid coordinates).
#[must_use]
pub const fn hash_i(seed: u64, x: i64, y: i64) -> u64 {
    hash(seed, bits(x), bits(y))
}

/// Hash of a seed and a string key.
#[must_use]
pub fn hash_str(seed: u64, key: &str) -> u64 {
    key.bytes()
        .fold(mix(seed ^ 0x51ED_270B), |h, b| mix(h ^ u64::from(b)))
}

/// A unit float in `[0, 1)` from a hash.
#[must_use]
pub fn unit(h: u64) -> f64 {
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// A small deterministic PRNG (SplitMix64 stream).
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// A stream for `key` under `seed`.
    #[must_use]
    pub fn new(seed: u64, key: &str) -> Self {
        Self(hash_str(seed, key))
    }

    /// A stream for an integer key under `seed`.
    #[must_use]
    pub const fn keyed(seed: u64, a: u64, b: u64) -> Self {
        Self(hash(seed, a, b))
    }

    /// Next raw value.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.0)
    }

    /// Uniform in `[0, 1)`.
    pub fn f64(&mut self) -> f64 {
        unit(self.next_u64())
    }

    /// Uniform in `[lo, hi)`.
    pub fn range_f(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.f64()
    }

    /// Uniform integer in `[lo, hi]` (inclusive).
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let span = i64::from(hi) - i64::from(lo) + 1;
        let off = index_of(self.next_u64(), usize::try_from(span).unwrap_or(1));
        lo.saturating_add(i32::try_from(off).unwrap_or(0))
    }

    /// True with probability `p`.
    pub fn chance(&mut self, p: f64) -> bool {
        self.f64() < p
    }

    /// Index in `0..n` (0 when `n` is 0).
    pub fn index(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            index_of(self.next_u64(), n)
        }
    }
}

/// Smooth 1-D value noise in `[-1, 1]` with unit wavelength.
#[must_use]
pub fn noise1(seed: u64, t: f64) -> f64 {
    let i = t.floor();
    let f = t - i;
    let k = floor_i(t);
    let a = unit(hash_i(seed, k, 0)) * 2.0 - 1.0;
    let b = unit(hash_i(seed, k + 1, 0)) * 2.0 - 1.0;
    let s = f * f * (3.0 - 2.0 * f);
    a + (b - a) * s
}

/// Two octaves of [`noise1`].
#[must_use]
pub fn fbm1(seed: u64, t: f64) -> f64 {
    (noise1(seed, t) * 2.0 + noise1(seed ^ 0xABCD, t * 2.3)) / 3.0
}

/// Smooth 2-D value noise in `[-1, 1]` with unit wavelength.
#[must_use]
pub fn noise2(seed: u64, x: f64, y: f64) -> f64 {
    let (ix, iy) = (x.floor(), y.floor());
    let (fx, fy) = (x - ix, y - iy);
    let (kx, ky) = (floor_i(x), floor_i(y));
    let g = |dx: i64, dy: i64| unit(hash_i(seed, kx + dx, ky + dy)) * 2.0 - 1.0;
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let top = g(0, 0) + (g(1, 0) - g(0, 0)) * sx;
    let bot = g(0, 1) + (g(1, 1) - g(0, 1)) * sx;
    top + (bot - top) * sy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_are_stable_and_bounded() {
        let mut a = Rng::new(7, "plots");
        let mut b = Rng::new(7, "plots");
        for _ in 0..100 {
            let x = a.range(3, 9);
            assert_eq!(x, b.range(3, 9));
            assert!((3..=9).contains(&x));
        }
        for i in 0..100 {
            let n = noise1(3, f64::from(i) * 0.13);
            assert!((-1.0..=1.0).contains(&n));
        }
    }
}
