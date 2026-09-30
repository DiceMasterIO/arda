//! Seed keys and the hand-rolled deterministic PRNG.
//!
//! Every inhabitant draws from a stream keyed by
//! `blake3(world_seed, settlement_id, building_id, person_index, purpose)`
//! (`arda_ids::digest`),
//! so what one person gets never depends on generation order.

use crate::input::{BuildingId, SettlementId};

/// The inputs that identify one random stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeedKey {
    /// World seed.
    pub world_seed: u64,
    /// Settlement.
    pub settlement: SettlementId,
    /// Building (home building for people).
    pub building: BuildingId,
    /// Person index within the building, or a household/slot number.
    pub index: u32,
}

impl SeedKey {
    /// Key for a whole settlement (no building).
    #[must_use]
    pub fn settlement(world_seed: u64, settlement: SettlementId) -> Self {
        Self {
            world_seed,
            settlement,
            building: BuildingId(u64::MAX),
            index: u32::MAX,
        }
    }

    /// Key for a building.
    #[must_use]
    pub fn building(world_seed: u64, settlement: SettlementId, building: BuildingId) -> Self {
        Self {
            world_seed,
            settlement,
            building,
            index: u32::MAX,
        }
    }

    /// Key for one person or slot inside a building.
    #[must_use]
    pub fn with_index(self, index: u32) -> Self {
        Self { index, ..self }
    }

    /// A PRNG for one purpose of this key.
    #[must_use]
    pub fn rng(&self, purpose: &str) -> Rng {
        // `logic/13` §npc-seed through the shared `arda_ids::digest` scheme:
        // fixed-width parts, the variable-length purpose last.
        Rng::from_state(arda_ids::hash::digest_words(&[
            b"arda-npc/v1",
            &self.world_seed.to_le_bytes(),
            &self.settlement.0.to_le_bytes(),
            &self.building.0.to_le_bytes(),
            &self.index.to_le_bytes(),
            purpose.as_bytes(),
        ]))
    }

    /// A single 64-bit hash of this key and purpose, for ordering.
    #[must_use]
    pub fn hash(&self, purpose: &str) -> u64 {
        self.rng(purpose).next_u64()
    }
}

/// xoshiro256** seeded from blake3 output.
#[derive(Debug, Clone)]
pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    fn from_state(mut s: [u64; 4]) -> Self {
        if s == [0; 4] {
            s[0] = 0x9E37_79B9_7F4A_7C15;
        }
        Self { s }
    }

    /// Next raw 64-bit value.
    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// Uniform value in `0..n` (0 when `n == 0`), by multiply-shift.
    pub fn below(&mut self, n: u32) -> u32 {
        let wide = u128::from(self.next_u64() >> 32) * u128::from(n);
        u32::try_from(wide >> 32).unwrap_or(0)
    }

    /// Uniform value in `lo..=hi`.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        if hi <= lo {
            return lo;
        }
        lo + self.below(hi - lo + 1)
    }

    /// True with probability `per_mille / 1000`.
    pub fn chance(&mut self, per_mille: u32) -> bool {
        self.below(1000) < per_mille
    }

    /// Index into a slice of the given length.
    pub fn index(&mut self, len: usize) -> usize {
        let n = u32::try_from(len).unwrap_or(u32::MAX);
        usize::try_from(self.below(n)).unwrap_or(0)
    }

    /// One element of a non-empty slice.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            items.get(self.index(items.len()))
        }
    }

    /// Index chosen by integer weights; `None` when all weights are zero.
    pub fn weighted(&mut self, weights: &[u32]) -> Option<usize> {
        let total: u64 = weights.iter().map(|&w| u64::from(w)).sum();
        if total == 0 {
            return None;
        }
        let total32 = u32::try_from(total).unwrap_or(u32::MAX);
        let mut roll = u64::from(self.below(total32));
        for (i, &w) in weights.iter().enumerate() {
            let w = u64::from(w);
            if roll < w {
                return Some(i);
            }
            roll -= w;
        }
        None
    }

    /// Fisher–Yates shuffle.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.index(i + 1);
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_are_stable_and_distinct() {
        let key = SeedKey::building(7, SettlementId(1), BuildingId(2)).with_index(3);
        assert_eq!(key.rng("a").next_u64(), key.rng("a").next_u64());
        assert_ne!(key.rng("a").next_u64(), key.rng("b").next_u64());
        assert_ne!(
            key.rng("a").next_u64(),
            key.with_index(4).rng("a").next_u64()
        );
    }

    #[test]
    fn below_stays_in_range() {
        let mut rng = SeedKey::settlement(1, SettlementId(1)).rng("t");
        for n in [1u32, 2, 3, 10, 1000] {
            for _ in 0..200 {
                assert!(rng.below(n) < n);
            }
        }
        assert_eq!(rng.below(0), 0);
    }
}
