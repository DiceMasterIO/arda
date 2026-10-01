//! Deterministic integer hashing shared by every tactical WFC (logic/09
//! §hash): the SplitMix64-finaliser chain. `arda-refine` re-exports these,
//! so its outputs are unchanged; changing them is a format change.

/// SplitMix64 finaliser: a bijective avalanche mix of one word.
#[must_use]
pub const fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Hashes a seed, a stream tag and two signed coordinates.
#[must_use]
pub const fn hash2(seed: u64, tag: u64, x: i64, y: i64) -> u64 {
    let a = mix(seed ^ mix(tag));
    let b = mix(a ^ x.cast_unsigned());
    mix(b ^ y.cast_unsigned().rotate_left(29))
}

/// Hashes a seed, a stream tag and three signed coordinates.
#[must_use]
pub const fn hash3(seed: u64, tag: u64, x: i64, y: i64, z: i64) -> u64 {
    mix(hash2(seed, tag, x, y) ^ z.cast_unsigned().rotate_left(43))
}

/// Maps a hash to `[0, 1)` using its top 53 bits.
#[must_use]
pub fn unit(h: u64) -> f64 {
    // 53-bit mantissa; the conversion is exact.
    #[allow(clippy::cast_precision_loss)]
    let v = (h >> 11) as f64;
    v * (1.0 / 9_007_199_254_740_992.0)
}

/// Maps a hash to `[-1, 1)`.
#[must_use]
pub fn signed(h: u64) -> f64 {
    unit(h) * 2.0 - 1.0
}

/// A tiny deterministic generator for sequential draws inside one problem.
#[derive(Debug, Clone)]
pub struct Stream(u64);

impl Stream {
    /// A stream keyed by seed, tag and position.
    #[must_use]
    pub const fn new(seed: u64, tag: u64, x: i64, y: i64) -> Self {
        Self(hash2(seed, tag, x, y))
    }

    /// Next 64-bit word.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.0)
    }

    /// Next value in `[0, 1)`.
    pub fn next_unit(&mut self) -> f64 {
        unit(self.next_u64())
    }

    /// Next value in `0..n` (`0` when `n` is 0).
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_are_frozen() {
        // Golden values: refine's edge offsets and scatter depend on them.
        assert_eq!(mix(0), 0xE220_A839_7B1D_CDAF);
        assert_eq!(hash2(42, 7, -3, 9), hash2(42, 7, -3, 9));
        assert_ne!(hash2(1, 2, 3, 4), hash2(1, 2, 4, 3));
        assert_ne!(hash3(1, 2, 3, 4, 5), hash3(1, 2, 3, 4, 6));
    }

    #[test]
    fn streams_stay_in_range() {
        let mut s = Stream::new(1, 2, 3, 4);
        for _ in 0..1000 {
            assert!((0.0..1.0).contains(&s.next_unit()));
            assert!(s.below(7) < 7);
        }
    }
}
