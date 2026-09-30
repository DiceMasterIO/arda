//! Deterministic integer hashing: every random choice in the crate is a pure
//! function of the world seed and global coordinates (goal 42, §8
//! determinism), so block generation order can never change the output.

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

/// A tiny deterministic generator for sequential draws inside one block.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_stays_in_range_and_is_spread() {
        let mut lo = 0;
        for i in 0..10_000 {
            let u = unit(hash2(1, 2, i, -i));
            assert!((0.0..1.0).contains(&u));
            if u < 0.5 {
                lo += 1;
            }
        }
        assert!((4_700..5_300).contains(&lo), "{lo}");
    }

    #[test]
    fn hash_depends_on_every_input() {
        let base = hash3(1, 2, 3, 4, 5);
        assert_ne!(base, hash3(0, 2, 3, 4, 5));
        assert_ne!(base, hash3(1, 0, 3, 4, 5));
        assert_ne!(base, hash3(1, 2, 0, 4, 5));
        assert_ne!(base, hash3(1, 2, 3, 0, 5));
        assert_ne!(base, hash3(1, 2, 3, 4, 0));
        assert_ne!(hash2(1, 2, 3, 4), hash2(1, 2, 4, 3));
    }
}
