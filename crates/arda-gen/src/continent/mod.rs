//! Continent generation (`logic/01`).
//!
//! Skeleton scope: plates, kinematic-lite tectonics, coast, and the 4 km to
//! 1 km upsample. Climate, hydrology, human geography, and naming arrive at
//! build-order step 4.

mod area_detail;
pub mod bundles;
pub mod climate;
pub mod coast;
pub mod erode;
pub mod hydrology;
pub mod plates;
pub mod tectonics;

#[cfg(test)]
mod terrain_tests;

use crate::noise::fbm;
use arda_core::{GenerateConfig, HeightMm};
use coast::{
    continental_mask, graded_base_mm, rim_forced_ocean, shelf_gradient, CONTINENTALITY_FULL,
};
use plates::SimExtent;

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

/// Everything the continent stage knows, threaded to bundles and areas
/// (feature 03 §Q2 — closes the computes-and-discards seam).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Continent {
    /// The accepted 1 km continent grid.
    pub grid: ContinentGrid,
    /// Per-cell climate fields for [`Continent::grid`].
    pub climate: climate::ContinentClimate,
    /// The drainage tree and per-cell loads routed over [`Continent::grid`].
    pub hydrology: hydrology::ContinentHydrology,
}

/// Runs the continent stage for one attempt and packages the context.
///
/// Runs exactly the sequence `generate_world` runs on acceptance:
/// [`generate_continent_attempt`], then [`climate::climate`], then
/// [`hydrology::hydrology`] over the result — so callers get one continent
/// context instead of computing climate and hydrology again themselves.
#[must_use]
pub fn build_continent(seed: u64, config: GenerateConfig, attempt: u8) -> Continent {
    let grid = generate_continent_attempt(seed, config, attempt);
    let clim = climate::climate(&grid, config.latitude_band());
    let hydro = hydrology::hydrology(&grid, &clim);
    Continent {
        grid,
        climate: clim,
        hydrology: hydro,
    }
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
    let uplift = tectonics::run_tectonics(seed, &plates, sim, SKELETON_STEPS);

    // Step 3: isostasy plus accumulated uplift, on the 4 km grid.
    // Crust is sampled through a warped Voronoi so plate boundaries are not
    // straight lines, then blurred into a shelf gradient so the continent
    // meets the ocean over tens of kilometres rather than one 4 km cell.
    let binary: Vec<i32> = (0..sim.height)
        .flat_map(|y| (0..sim.width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let id = plates::plate_of_warped(seed, &plates, x, y);
            let continental = plates
                .iter()
                .find(|p| p.id == id)
                .is_some_and(|p| p.crust == plates::CrustType::Continental);
            i32::from(continental) * CONTINENTALITY_FULL
        })
        .collect();
    let mut continentality = shelf_gradient(&binary, sim.width, sim.height);

    // Blend the plate field with a centred mask, weighting the mask twice.
    //
    // Multiplying by the mask instead leaves the result at the mercy of which
    // plates happen to sit in the middle: with only eight plates on a small
    // domain, often none does, and attempt 0 of the micro world came out 169
    // per mille land. Blending guarantees the core is above sea level while
    // the plates still shape the margins, which is where the ragged coast and
    // the offshore islands come from.
    for y in 0..sim.height {
        for x in 0..sim.width {
            let i = usize::try_from(y * sim.width + x).unwrap_or(0);
            let m = continental_mask(x, y, sim.width, sim.height);
            continentality[i] = (continentality[i] + 2 * m) / 3;
        }
    }

    let coarse: Vec<i32> = (0..sim.height)
        .flat_map(|y| (0..sim.width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let i = usize::try_from(y * sim.width + x).unwrap_or(0);
            graded_base_mm(continentality[i]).saturating_add(uplift[i])
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

            // Noise refinement, applied on both sides of sea level.
            //
            // Restricting detail to land left the sea floor as pure bilinear
            // interpolation, so the sea-level contour followed the 4 km grid
            // and the coast came out as rectangular steps. Perturbing the
            // shelf too is what produces bays and headlands.
            //
            // Amplitude tapers with depth: the abyssal floor stays smooth
            // while the shelf and the coast get the full swing.
            // Amplitude must stay well under the coastal plain's own height.
            // At 60-200 m it swamped a 107 m plain and drowned half of it;
            // the point of this noise is a ragged coast, not new terrain.
            let shelf = 1_024 - (h.abs() / 400).clamp(0, 1_024);
            let amp = 12_000 + 38_000 * i64::from(shelf) / 1_024;
            let detail = i64::from(fbm(
                seed ^ 0x00DE_7A11 ^ (u64::from(attempt) << 48),
                x,
                y,
                24,
                5,
            ));
            #[allow(clippy::cast_possible_truncation)]
            let detail_mm = (detail * amp / 32_768) as i32;
            h = h.saturating_add(detail_mm);

            if rim_forced_ocean(x, y, vis_w, vis_h, RIM_MARGIN) {
                h = h.min(coast::OCEANIC_BASE_MM / 2);
            }
            height_mm.push(h);
        }
    }

    // Step 2 (continued): coarse erosion and drainage respond on the 1 km
    // grid. Running it here rather than only per-tile is what lets valleys
    // cross tile boundaries, and it leaves no pinned-rim seams.
    erode::erode_continent(&mut height_mm, vis_w, vis_h);

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
    fn some_attempt_passes_the_land_fraction_gate() {
        // `logic/01` §Q9 gates land fraction at 25-90% and rerolls up to five
        // times on failure, so the property is that the ladder finds an
        // acceptable continent — not that attempt 0 always does. Asserting the
        // latter made this fail on a seed the batch handles fine.
        for seed in [1u64, 7, 42, 99] {
            let ok = (0..5).any(|attempt| {
                let g = generate_continent_attempt(seed, GenerateConfig::MICRO, attempt);
                (250..=900).contains(&g.land_fraction_permille())
            });
            assert!(ok, "seed {seed} failed the gate on all five attempts");
        }
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

#[cfg(test)]
#[path = "../orchestrator/annual_source_tests.rs"]
mod annual_source_tests;
#[cfg(test)]
#[path = "../orchestrator/shared_solve_tests.rs"]
mod shared_solve_tests;
