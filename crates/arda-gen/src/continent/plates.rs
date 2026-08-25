//! Voronoi plate seeding (`logic/01` step 1).

use arda_core::{rng, SeedKey, Stage, Tier};
use rand_core::RngCore;

/// The 4 km simulation domain, roughly twice the visible continent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimExtent {
    /// Domain width in 4 km cells.
    pub width: i32,
    /// Domain height in 4 km cells.
    pub height: i32,
}

/// Whether a plate carries continental or oceanic crust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrustType {
    /// Thick, buoyant crust — sits above sea level once isostasy applies.
    Continental,
    /// Thin, dense crust — sits well below sea level.
    Oceanic,
}

/// One tectonic plate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plate {
    /// Plate identifier, used as the Voronoi label.
    pub id: u8,
    /// Voronoi site column on the 4 km grid.
    pub centre_x: i32,
    /// Voronoi site row on the 4 km grid.
    pub centre_y: i32,
    /// Crust type.
    pub crust: CrustType,
    /// Drift per step, columns.
    pub drift_x: i32,
    /// Drift per step, rows.
    pub drift_y: i32,
}

/// Seeds 8-14 plates as Voronoi sites, forcing the domain rim oceanic so map
/// edges are guaranteed ocean (`logic/01` step 1, mockup Q22).
///
/// `attempt` is the validation reroll counter (`logic/01` §Q9): a rejected
/// continent is regenerated from a derived subseed, not from a new seed.
#[must_use]
#[allow(clippy::cast_possible_wrap)]
pub fn seed_plates(seed: u64, sim: SimExtent, attempt: u8) -> Vec<Plate> {
    let mut r = rng(
        seed,
        SeedKey::new(Tier::Continent, Stage::Plates, 0, 0, attempt),
    );
    let count = 8 + u8::try_from(r.next_u32() % 7).unwrap_or(0); // 8..=14

    (0..count)
        .map(|id| {
            let centre_x = (r.next_u32() % sim.width.unsigned_abs()) as i32;
            let centre_y = (r.next_u32() % sim.height.unsigned_abs()) as i32;

            let near_rim = centre_x < sim.width / 5
                || centre_x > sim.width * 4 / 5
                || centre_y < sim.height / 5
                || centre_y > sim.height * 4 / 5;

            // Rim plates are always oceanic; interior plates are continental
            // about two times in three, which keeps land fraction inside the
            // step-9 gate without tuning.
            let crust = if near_rim || (r.next_u32()).is_multiple_of(3) {
                CrustType::Oceanic
            } else {
                CrustType::Continental
            };

            Plate {
                id,
                centre_x,
                centre_y,
                crust,
                drift_x: (r.next_u32() % 5) as i32 - 2,
                drift_y: (r.next_u32() % 5) as i32 - 2,
            }
        })
        .collect()
}

/// Nearest-site Voronoi assignment. Ties break toward the lower plate id, so
/// the result never depends on iteration order (§Q4).
#[must_use]
pub fn plate_of(plates: &[Plate], x: i32, y: i32) -> u8 {
    let mut best_id = 0u8;
    let mut best_d2 = i64::MAX;
    for p in plates {
        let dx = i64::from(x - p.centre_x);
        let dy = i64::from(y - p.centre_y);
        let d2 = dx * dx + dy * dy;
        if d2 < best_d2 {
            best_d2 = d2;
            best_id = p.id;
        }
    }
    best_id
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sim() -> SimExtent {
        SimExtent {
            width: 51,
            height: 102,
        }
    }

    #[test]
    fn plate_count_is_within_the_interviewed_range() {
        // logic/01 step 1: seed 8-14 plates.
        for seed in 0..20u64 {
            let n = seed_plates(seed, sim(), 0).len();
            assert!((8..=14).contains(&n), "seed {seed} produced {n} plates");
        }
    }

    #[test]
    fn plate_seeding_is_deterministic() {
        assert_eq!(seed_plates(42, sim(), 0), seed_plates(42, sim(), 0));
    }

    #[test]
    fn rim_plates_are_oceanic() {
        // logic/01 step 1: the domain rim is forced oceanic.
        let plates = seed_plates(42, sim(), 0);
        let s = sim();
        for p in &plates {
            let near_rim = p.centre_x < s.width / 5
                || p.centre_x > s.width * 4 / 5
                || p.centre_y < s.height / 5
                || p.centre_y > s.height * 4 / 5;
            if near_rim {
                assert_eq!(
                    p.crust,
                    CrustType::Oceanic,
                    "rim plate {} is continental",
                    p.id
                );
            }
        }
    }

    #[test]
    fn at_least_one_plate_is_continental() {
        let plates = seed_plates(42, sim(), 0);
        assert!(plates.iter().any(|p| p.crust == CrustType::Continental));
    }

    #[test]
    fn voronoi_assignment_is_total_and_stable() {
        let plates = seed_plates(42, sim(), 0);
        let s = sim();
        for y in 0..s.height {
            for x in 0..s.width {
                let id = plate_of(&plates, x, y);
                assert!(plates.iter().any(|p| p.id == id));
                assert_eq!(id, plate_of(&plates, x, y));
            }
        }
    }
}
