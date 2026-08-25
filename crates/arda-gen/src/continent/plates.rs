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

/// Sim cells per plate. Calibrated so a 250x500 km continent gets the 8-14
/// plates that produce Earth-like hypsometry, then held constant per unit
/// area for every other size.
const CELLS_PER_PLATE: i64 = 2_800;

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
    // Plate count scales with domain area, so boundary density — and with it
    // the spacing of mountain belts — is the same on a small continent and a
    // large one. A fixed count made the default 500x1000 km world a flat
    // plain while a quarter-size one came out properly mountainous.
    let cells = i64::from(sim.width) * i64::from(sim.height);
    let base = (cells / CELLS_PER_PLATE).clamp(8, 220);
    let spread = (base / 3).max(1);
    let count = u16::try_from(base + i64::from(r.next_u32() % u32::try_from(spread).unwrap_or(1)))
        .unwrap_or(8);

    (0..u8::try_from(count).unwrap_or(8))
        .map(|id| {
            let centre_x = (r.next_u32() % sim.width.unsigned_abs()) as i32;
            let centre_y = (r.next_u32() % sim.height.unsigned_abs()) as i32;

            // Crust follows position: the core of the domain is continental,
            // the rim oceanic, and the band between them is mixed. Leaving it
            // to chance meant the central plates were sometimes all oceanic
            // and the continent simply failed to exist — attempt 0 of the
            // micro world came out 37 per mille land.
            let nx = (i64::from(centre_x) * 2 - i64::from(sim.width)).abs() * 1024
                / i64::from(sim.width.max(1));
            let ny = (i64::from(centre_y) * 2 - i64::from(sim.height)).abs() * 1024
                / i64::from(sim.height.max(1));
            let radius = nx.max(ny);
            let crust = if radius < 420 {
                CrustType::Continental
            } else if radius > 820 {
                CrustType::Oceanic
            } else {
                // Mixed margin: continental about two times in three, which
                // gives the ragged coast and the offshore islands.
                if (r.next_u32()).is_multiple_of(3) {
                    CrustType::Oceanic
                } else {
                    CrustType::Continental
                }
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

/// Nearest-site Voronoi assignment over a *warped* coordinate.
///
/// Raw Voronoi gives straight-line plate boundaries, and since the coast
/// follows those boundaries the continent comes out as a polygon. Warping the
/// lookup position by low-frequency noise before the nearest-site search
/// bends the boundaries without changing which plates exist.
#[must_use]
pub fn plate_of_warped(seed: u64, plates: &[Plate], x: i32, y: i32) -> u8 {
    let wx = x + crate::noise::fbm(seed ^ 0x0057_A9F1, x, y, 24, 3) * 7 / 32_768;
    let wy = y + crate::noise::fbm(seed ^ 0x00B4_11E3, x, y, 24, 3) * 7 / 32_768;
    plate_of(plates, wx, wy)
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
    fn crust_follows_position() {
        // Core continental, rim oceanic, margin mixed — so a continent exists
        // whatever the plate draw does.
        let s = sim();
        for p in &seed_plates(42, s, 0) {
            let nx =
                (i64::from(p.centre_x) * 2 - i64::from(s.width)).abs() * 1024 / i64::from(s.width);
            let ny = (i64::from(p.centre_y) * 2 - i64::from(s.height)).abs() * 1024
                / i64::from(s.height);
            let radius = nx.max(ny);
            if radius < 420 {
                assert_eq!(
                    p.crust,
                    CrustType::Continental,
                    "core plate {} is oceanic",
                    p.id
                );
            }
            if radius > 820 {
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
