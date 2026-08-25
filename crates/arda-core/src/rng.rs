//! Counter-based subseed derivation (`architecture-interview.md` §Q4).
//!
//! Every random draw in arda comes from here, keyed by
//! `(seed, tier, stage, coords, attempt)` — never from iteration order, so
//! stage order and thread count cannot change a world.

use rand_chacha::ChaCha8Rng;
use rand_core::SeedableRng;

/// Domain separator. Bump only with a `format_version` major — changing it
/// changes every world ever generated.
const DOMAIN: &[u8] = b"arda-subseed-v1";

/// Which of the three tiers is drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Tier {
    /// The 1 km continent grid (`logic/01`).
    Continent = 1,
    /// A 51.2 km area tile (`logic/02`).
    Area = 2,
    /// A 64x64 tactical block (`logic/03`).
    Block = 3,
}

/// Which pipeline stage is drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Stage {
    /// Voronoi plate seeding (`logic/01` step 1).
    Plates = 1,
    /// Time-stepped tectonics (`logic/01` step 2).
    Tectonics = 2,
    /// 4 km to 1 km noise-refined upsample (`logic/01` step 4).
    Upsample = 3,
    /// Area relief detail (`logic/02`).
    Relief = 4,
    /// Area drainage (`logic/02`).
    Water = 5,
    /// Block WFC fill (`logic/03`).
    Blocks = 6,
    /// Per-tile input bundles (`logic/01` step 10).
    Bundles = 7,
    /// Landscape evolution: uplift, incision, creep, collapse, droplets.
    Erosion = 8,
}

/// The full key for one random stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SeedKey {
    tier: Tier,
    stage: Stage,
    x: i32,
    y: i32,
    attempt: u8,
}

impl SeedKey {
    /// Builds a stream key. `attempt` distinguishes rerolls
    /// (`logic/01` §Q9 continent rerolls, `logic/03` §Q12 WFC retries).
    #[must_use]
    pub const fn new(tier: Tier, stage: Stage, x: i32, y: i32, attempt: u8) -> Self {
        Self {
            tier,
            stage,
            x,
            y,
            attempt,
        }
    }

    /// Returns the same key at the next attempt number.
    #[must_use]
    pub const fn with_attempt(self, attempt: u8) -> Self {
        Self { attempt, ..self }
    }
}

/// Derives the 32-byte ChaCha8 key for one stream.
///
/// Fields are hashed in a fixed little-endian order so the derivation is
/// endianness-independent.
#[must_use]
pub fn derive(world_seed: u64, key: SeedKey) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(DOMAIN);
    hasher.update(&world_seed.to_le_bytes());
    hasher.update(&[key.tier as u8, key.stage as u8]);
    hasher.update(&key.x.to_le_bytes());
    hasher.update(&key.y.to_le_bytes());
    hasher.update(&[key.attempt]);
    *hasher.finalize().as_bytes()
}

/// Opens the deterministic stream for one key.
///
/// ChaCha8's output is defined bit-for-bit by the standard, so the same key
/// yields the same bytes on every platform — the §Q4 guarantee.
#[must_use]
pub fn rng(world_seed: u64, key: SeedKey) -> ChaCha8Rng {
    ChaCha8Rng::from_seed(derive(world_seed, key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::RngCore;

    fn draws(seed: u64, key: SeedKey) -> [u32; 4] {
        let mut r = rng(seed, key);
        [r.next_u32(), r.next_u32(), r.next_u32(), r.next_u32()]
    }

    #[test]
    fn same_key_yields_same_stream() {
        let k = SeedKey::new(Tier::Area, Stage::Relief, 3, 11, 0);
        assert_eq!(draws(42, k), draws(42, k));
    }

    #[test]
    fn every_field_changes_the_stream() {
        let base = SeedKey::new(Tier::Area, Stage::Relief, 3, 11, 0);
        let reference = draws(42, base);

        assert_ne!(draws(43, base), reference, "world seed must matter");
        assert_ne!(
            draws(42, SeedKey::new(Tier::Block, Stage::Relief, 3, 11, 0)),
            reference,
            "tier must matter"
        );
        assert_ne!(
            draws(42, SeedKey::new(Tier::Area, Stage::Water, 3, 11, 0)),
            reference,
            "stage must matter"
        );
        assert_ne!(
            draws(42, SeedKey::new(Tier::Area, Stage::Relief, 4, 11, 0)),
            reference,
            "x must matter"
        );
        assert_ne!(
            draws(42, SeedKey::new(Tier::Area, Stage::Relief, 3, 12, 0)),
            reference,
            "y must matter"
        );
        assert_ne!(
            draws(42, base.with_attempt(1)),
            reference,
            "attempt must matter"
        );
    }

    #[test]
    fn negative_coordinates_are_distinct_from_positive() {
        let a = draws(42, SeedKey::new(Tier::Continent, Stage::Plates, -3, -11, 0));
        let b = draws(42, SeedKey::new(Tier::Continent, Stage::Plates, 3, 11, 0));
        assert_ne!(a, b);
    }

    /// Reference vector: pins the byte layout of the derivation so a
    /// refactor cannot silently change every generated world.
    #[test]
    fn derivation_matches_reference_vector() {
        let key = derive(42, SeedKey::new(Tier::Area, Stage::Relief, 3, 11, 0));
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"arda-subseed-v1");
        hasher.update(&42u64.to_le_bytes());
        hasher.update(&[Tier::Area as u8, Stage::Relief as u8]);
        hasher.update(&3i32.to_le_bytes());
        hasher.update(&11i32.to_le_bytes());
        hasher.update(&[0u8]);
        assert_eq!(key, *hasher.finalize().as_bytes());
    }
}
