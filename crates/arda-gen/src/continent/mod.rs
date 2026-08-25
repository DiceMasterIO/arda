//! Continent generation (`logic/01`).
//!
//! Skeleton scope: plates, kinematic-lite tectonics, coast, and the 4 km to
//! 1 km upsample. Climate, hydrology, human geography, and naming arrive at
//! build-order step 4.

pub mod bundles;
pub mod coast;
pub mod plates;
pub mod tectonics;

use crate::noise::fbm;
use arda_core::{GenerateConfig, HeightMm};
use coast::{base_elevation_mm, rim_forced_ocean};
use plates::{plate_of, SimExtent};

/// Simulation cell size in kilometres (`logic/01` step 2).
const SIM_CELL_KM: i32 = 4;
/// Tectonic steps in the skeleton pass; the full stage runs 100.
const SKELETON_STEPS: u16 = 20;
/// Cells of guaranteed ocean at the visible map edge.
const RIM_MARGIN: i32 = 2;

/// The 1 km continent working grid (`logic/01` step 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinentGrid {
    width: i32,
    height: i32,
    height_mm: Vec<i32>,
}

impl ContinentGrid {
    /// Grid width in 1 km cells.
    #[must_use]
    pub const fn width(&self) -> i32 {
        self.width
    }

    /// Grid height in 1 km cells.
    #[must_use]
    pub const fn height(&self) -> i32 {
        self.height
    }

    /// Elevation at a cell; out-of-range coordinates clamp to the edge, which
    /// is always ocean.
    #[must_use]
    pub fn get(&self, x: i32, y: i32) -> HeightMm {
        let cx = x.clamp(0, self.width - 1);
        let cy = y.clamp(0, self.height - 1);
        HeightMm::new(self.height_mm[usize::try_from(cy * self.width + cx).unwrap_or(0)])
    }

    /// Land cells per thousand (`logic/01` step 9 gate).
    #[must_use]
    pub fn land_fraction_permille(&self) -> u16 {
        let land = self.height_mm.iter().filter(|&&h| h > 0).count();
        let total = self.height_mm.len().max(1);
        u16::try_from(land * 1000 / total).unwrap_or(1000)
    }
}

/// Runs the skeleton continent stage at attempt 0.
///
/// Steps, in the `logic/01` order: seed plates on a 2x domain, run the
/// coupled tectonics loop, apply isostatic base elevation, cut the coastline
/// at sea level, then upsample 4 km to 1 km with noise refinement.
#[must_use]
pub fn generate_continent(seed: u64, config: GenerateConfig) -> ContinentGrid {
    generate_continent_attempt(seed, config, 0)
}

/// Runs the continent stage for one validation attempt.
///
/// `attempt` feeds the subseed derivation, so a continent rejected by the
/// step-9 gate is regenerated deterministically rather than randomly
/// (`logic/01` §Q9).
#[must_use]
#[allow(clippy::cast_possible_wrap)]
pub fn generate_continent_attempt(seed: u64, config: GenerateConfig, attempt: u8) -> ContinentGrid {
    let vis_w = config.size_km().width as i32;
    let vis_h = config.size_km().height as i32;

    // Step 1: plates on a domain twice the visible continent.
    let sim = SimExtent {
        width: (vis_w * 2 / SIM_CELL_KM).max(8),
        height: (vis_h * 2 / SIM_CELL_KM).max(8),
    };
    let plates = plates::seed_plates(seed, sim, attempt);

    // Step 2: coupled tectonics.
    let uplift = tectonics::run_tectonics(&plates, sim, SKELETON_STEPS);

    // Step 3: isostasy plus accumulated uplift, on the 4 km grid.
    let coarse: Vec<i32> = (0..sim.height)
        .flat_map(|y| (0..sim.width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let id = plate_of(&plates, x, y);
            let base = plates
                .iter()
                .find(|p| p.id == id)
                .map_or(coast::OCEANIC_BASE_MM, base_elevation_mm);
            base.saturating_add(uplift[usize::try_from(y * sim.width + x).unwrap_or(0)])
        })
        .collect();

    // Step 4: upsample to the 1 km working grid, noise-refined. The visible
    // continent is the centre of the domain.
    let off_x = (sim.width * SIM_CELL_KM - vis_w) / 2;
    let off_y = (sim.height * SIM_CELL_KM - vis_h) / 2;

    let mut height_mm = Vec::with_capacity(usize::try_from(vis_w * vis_h).unwrap_or(0));
    for y in 0..vis_h {
        for x in 0..vis_w {
            let mut h = sample_coarse(&coarse, sim, x + off_x, y + off_y);

            // Noise refinement: about +/-180 m of detail, only on land, so
            // the coastline stays where tectonics put it (§Q8: area detail
            // refines coarse features but never relocates them).
            if h > 0 {
                // i64 intermediate: the product overflows i32 at full swing.
                let detail = i64::from(fbm(
                    seed ^ 0x00DE_7A11 ^ (u64::from(attempt) << 48),
                    x,
                    y,
                    24,
                    4,
                ));
                #[allow(clippy::cast_possible_truncation)]
                let detail_mm = (detail * 180_000 / 32_768) as i32;
                h = h.saturating_add(detail_mm);
            }

            if rim_forced_ocean(x, y, vis_w, vis_h, RIM_MARGIN) {
                h = h.min(coast::OCEANIC_BASE_MM / 2);
            }
            height_mm.push(h);
        }
    }

    ContinentGrid {
        width: vis_w,
        height: vis_h,
        height_mm,
    }
}

/// Bilinear sample of the 4 km grid at 1 km resolution, in fixed point.
fn sample_coarse(coarse: &[i32], sim: SimExtent, km_x: i32, km_y: i32) -> i32 {
    let gx = (km_x / SIM_CELL_KM).clamp(0, sim.width - 1);
    let gy = (km_y / SIM_CELL_KM).clamp(0, sim.height - 1);
    let gx1 = (gx + 1).min(sim.width - 1);
    let gy1 = (gy + 1).min(sim.height - 1);

    let fx = i64::from(km_x.rem_euclid(SIM_CELL_KM)) * 65536 / i64::from(SIM_CELL_KM);
    let fy = i64::from(km_y.rem_euclid(SIM_CELL_KM)) * 65536 / i64::from(SIM_CELL_KM);

    let at = |x: i32, y: i32| i64::from(coarse[usize::try_from(y * sim.width + x).unwrap_or(0)]);
    let top = at(gx, gy) + (((at(gx1, gy) - at(gx, gy)) * fx) >> 16);
    let bottom = at(gx, gy1) + (((at(gx1, gy1) - at(gx, gy1)) * fx) >> 16);

    #[allow(clippy::cast_possible_truncation)]
    {
        (top + (((bottom - top) * fy) >> 16)) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn micro_continent_grid_is_the_configured_size() {
        let grid = generate_continent(42, GenerateConfig::MICRO);
        assert_eq!(grid.width(), 102);
        assert_eq!(grid.height(), 204);
    }

    #[test]
    fn generation_is_deterministic() {
        let a = generate_continent(42, GenerateConfig::MICRO);
        let b = generate_continent(42, GenerateConfig::MICRO);
        assert_eq!(a, b);
    }

    #[test]
    fn different_seeds_give_different_continents() {
        let a = generate_continent(42, GenerateConfig::MICRO);
        let b = generate_continent(43, GenerateConfig::MICRO);
        assert_ne!(a, b);
    }

    #[test]
    fn every_edge_cell_is_ocean() {
        // logic/01 invariant: every map-edge cell is ocean (mockup Q22).
        let grid = generate_continent(42, GenerateConfig::MICRO);
        let (w, h) = (grid.width(), grid.height());
        for x in 0..w {
            assert!(grid.get(x, 0).raw() < 0, "top edge at {x} is land");
            assert!(grid.get(x, h - 1).raw() < 0, "bottom edge at {x} is land");
        }
        for y in 0..h {
            assert!(grid.get(0, y).raw() < 0, "left edge at {y} is land");
            assert!(grid.get(w - 1, y).raw() < 0, "right edge at {y} is land");
        }
    }

    #[test]
    fn land_fraction_is_within_the_validation_gate() {
        // logic/01 step 9: land fraction within 25-90%.
        let grid = generate_continent(42, GenerateConfig::MICRO);
        let permille = grid.land_fraction_permille();
        assert!(
            (250..=900).contains(&permille),
            "land fraction {permille} per mille is outside 250..=900"
        );
    }

    #[test]
    fn relief_has_real_range() {
        // A flat plate would pass the other tests; assert the continent
        // actually has mountains.
        let grid = generate_continent(42, GenerateConfig::MICRO);
        let max = (0..grid.height())
            .flat_map(|y| (0..grid.width()).map(move |x| (x, y)))
            .map(|(x, y)| grid.get(x, y).raw())
            .max()
            .unwrap_or(0);
        assert!(max > 400_000, "highest point is only {max} mm");
    }
}
