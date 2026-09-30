//! Hand-rolled deterministic hashing and per-entity streams (goal-prompt §8).

/// SplitMix64 finaliser.
#[must_use]
pub const fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Hash of a seed, a stage tag and two entity keys.
#[must_use]
pub fn hash(seed: u64, tag: &str, a: u64, b: u64) -> u64 {
    let mut h = mix(seed ^ 0xA5A5_5A5A_C3C3_3C3C);
    for byte in tag.bytes() {
        h = mix(h ^ u64::from(byte));
    }
    mix(mix(h ^ a) ^ b.rotate_left(17))
}

/// A per-entity random stream, subseeded from `(seed, tag, key)`.
#[derive(Debug, Clone)]
pub struct Stream(u64);

impl Stream {
    /// Opens the stream for one entity.
    #[must_use]
    pub fn new(seed: u64, tag: &str, key: u64) -> Self {
        Self(hash(seed, tag, key, 0))
    }

    /// Next raw value.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.0)
    }

    /// Uniform in `0..n` (0 when `n` is 0).
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            // Multiply-shift keeps the bias below 2^-64 per draw.
            let wide = u128::from(self.next_u64()) * u128::from(n);
            u64::try_from(wide >> 64).unwrap_or(0)
        }
    }

    /// Uniform index into a slice of length `n`.
    pub fn pick(&mut self, n: usize) -> usize {
        usize::try_from(self.below(u64::try_from(n).unwrap_or(u64::MAX))).unwrap_or(0)
    }

    /// Signed jitter in `-span..=span`.
    pub fn jitter(&mut self, span: i64) -> i64 {
        if span <= 0 {
            return 0;
        }
        let n = u64::try_from(2 * span + 1).unwrap_or(1);
        i64::try_from(self.below(n)).unwrap_or(0) - span
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_are_stable_and_distinct() {
        let a: Vec<u64> = (0..4)
            .scan(Stream::new(42, "t", 1), |s, _| Some(s.next_u64()))
            .collect();
        let b: Vec<u64> = (0..4)
            .scan(Stream::new(42, "t", 1), |s, _| Some(s.next_u64()))
            .collect();
        let c = Stream::new(42, "t", 2).next_u64();
        assert_eq!(a, b);
        assert_ne!(a[0], c);
        let mut s = Stream::new(1, "x", 0);
        assert!((0..1000).all(|_| s.below(7) < 7));
        assert!((0..1000).all(|_| s.jitter(3).abs() <= 3));
    }
}
