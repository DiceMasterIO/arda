//! Keyed, order-independent randomness (goal-prompt §8: hand-rolled PRNG).
//!
//! Every decision is keyed by `(seed, domain, a, b)` through blake3, so a
//! value never depends on how many other values were drawn before it.

/// A 64-bit keyed hash of `(seed, domain, a, b)`.
#[must_use]
pub fn hash(seed: u64, domain: &str, a: u64, b: u64) -> u64 {
    let mut h = blake3::Hasher::new();
    h.update(&seed.to_le_bytes());
    h.update(domain.as_bytes());
    h.update(&[0xA5]);
    h.update(&a.to_le_bytes());
    h.update(&b.to_le_bytes());
    let out = h.finalize();
    let mut bytes = [0_u8; 8];
    for (dst, src) in bytes.iter_mut().zip(out.as_bytes().iter()) {
        *dst = *src;
    }
    u64::from_le_bytes(bytes)
}

/// A splitmix64 stream seeded from a keyed hash, for short local sequences.
#[derive(Debug, Clone)]
pub struct Stream(u64);

impl Stream {
    /// A stream for one entity and purpose.
    #[must_use]
    pub fn new(seed: u64, domain: &str, a: u64, b: u64) -> Self {
        Self(hash(seed, domain, a, b))
    }

    /// Next raw value.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n` (0 when `n` is 0).
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }

    /// Uniform in `lo..=hi` (returns `lo` when the range is empty).
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        let span = hi.abs_diff(lo).saturating_add(1);
        lo.saturating_add(crate::num::i64_of(self.below(span)))
    }

    /// True with probability `per_mille / 1000`.
    pub fn chance(&mut self, per_mille: u64) -> bool {
        self.below(1000) < per_mille
    }

    /// A uniformly chosen element.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        let n = crate::num::u64_of_usize(items.len());
        items.get(crate::num::usize_of(self.below(n)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_keyed_and_stable() {
        assert_eq!(hash(1, "x", 2, 3), hash(1, "x", 2, 3));
        assert_ne!(hash(1, "x", 2, 3), hash(1, "y", 2, 3));
        assert_ne!(hash(1, "x", 2, 3), hash(2, "x", 2, 3));
        let mut s = Stream::new(7, "t", 0, 0);
        for _ in 0..200 {
            let v = s.range(-3, 5);
            assert!((-3..=5).contains(&v));
        }
    }
}
