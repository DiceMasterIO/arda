//! Hand-rolled deterministic PRNG (SplitMix64) and stable hashing.
//!
//! Rule: every draw is a pure function of the seed and the salts that name
//! the decision, so reordering unrelated calls never changes a name.

/// SplitMix64 finaliser: a strong 64-bit mix.
#[must_use]
pub const fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Combines a seed with one salt.
#[must_use]
pub const fn combine(seed: u64, salt: u64) -> u64 {
    mix(seed ^ mix(salt.wrapping_add(0x9E37_79B9_7F4A_7C15)))
}

/// Stable FNV-1a hash of a string (platform independent).
#[must_use]
pub fn hash_str(s: &str) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01B3);
    }
    h
}

/// A small deterministic generator.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// A generator for `seed` narrowed by `salts`.
    #[must_use]
    pub fn new(seed: u64, salts: &[u64]) -> Self {
        let mut state = mix(seed);
        for &s in salts {
            state = combine(state, s);
        }
        Self { state }
    }

    /// Next raw 64-bit value.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.state)
    }

    /// Uniform integer in `0..n` (0 when `n` is 0).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        let n64 = u64::try_from(n).unwrap_or(u64::MAX);
        // Multiply-shift keeps the bias below 2^-32 for our small n.
        let r = (u128::from(self.next_u64()) * u128::from(n64)) >> 64;
        usize::try_from(r).unwrap_or(0)
    }

    /// True with probability `per_mille / 1000`.
    pub fn chance(&mut self, per_mille: u32) -> bool {
        self.below(1000) < per_mille as usize
    }

    /// Index drawn in proportion to `weights` (0 if all are zero).
    pub fn weighted(&mut self, weights: &[u32]) -> usize {
        let total: u64 = weights.iter().map(|&w| u64::from(w)).sum();
        if total == 0 {
            return 0;
        }
        let mut r = self.next_u64() % total;
        for (i, &w) in weights.iter().enumerate() {
            let w = u64::from(w);
            if r < w {
                return i;
            }
            r -= w;
        }
        weights.len().saturating_sub(1)
    }

    /// A uniformly picked element.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            items.get(self.below(items.len()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_stream() {
        let mut a = Rng::new(7, &[1, 2]);
        let mut b = Rng::new(7, &[1, 2]);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        assert_ne!(Rng::new(7, &[1]).next_u64(), Rng::new(7, &[2]).next_u64());
    }

    #[test]
    fn weighted_respects_zero_weights() {
        let mut r = Rng::new(1, &[]);
        for _ in 0..1000 {
            assert_eq!(r.weighted(&[0, 5, 0]), 1);
        }
        assert!(r.below(3) < 3);
    }
}
