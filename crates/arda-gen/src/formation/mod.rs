//! Fine terrain formation, recipes 5 and 6 (logic/02 §fine-formation).
//!
//! Multi-resolution stream-power landscape evolution from the tectonic
//! macro surface down to the 39.0625 m canonical lattice:
//!
//! 1. At 1250 m the whole domain evolves towards a quasi-steady state whose
//!    blurred envelope tracks the macro surface. This establishes the
//!    continental drainage skeleton: main divides, trunk valleys, basins.
//! 2. Each finer level warps and upsamples its parent, adds band-limited
//!    relief, and evolves again with its envelope pinned to the parent at
//!    scales above two parent cells. Every level therefore adds valleys and
//!    ridges at its own scale inside the inherited network.
//! 3. A relative uplift keeps hillslopes rising to the talus slope around
//!    incising channels, giving V-shaped valleys and sharp crests rather
//!    than diffusive, "plastic" relief. Diffusion runs only at the three
//!    coarse levels, where it sets a physical valley spacing.
//! 4. Lowland valley floors are filled into alluvial floodplains.
//!
//! All arithmetic is integer; the result is identical for any thread count.
//!
//! Recipe 5 is formation as v0.1 shipped it and replays byte for byte.
//! Recipe 6 adds tectonic margins, the basin audit, belt relief, maturity
//! and relief scaling, pre-erosion roughness, the coast stages and the
//! water forms. Recipe 7 adds climate: runoff-weighted area and arid
//! endorheic basins with playas ([`arid`]). Every such rule is gated on
//! [`Recipe`].
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
// Index casts in this module are bounded by the admitted lattice size,
// which [`plan`] limits to fewer than 2^31 cells per level.

pub mod arid;
pub mod basins;
pub mod bathymetry;
pub mod canyon;
pub mod coast;
pub mod coastal;
pub mod distance;
pub mod drainage;
mod finish;
pub mod flats;
pub mod floodplain;
pub mod flow;
pub mod glacial;
pub mod incision;
pub mod lattice;
mod levels;
pub mod littoral;
mod macro_view;
pub mod margin;
mod recipe5;
pub mod relief;
pub mod sampled;
pub mod shore;
mod surface;
pub mod terrace;
pub mod water;

use arda_core::LatitudeBand;
use thiserror::Error;

use crate::continent::ContinentGrid;
use incision::Scratch;
use lattice::Lattice;
use macro_view::{macro_basins, macro_lattices, MacroView};

/// Finest lattice spacing, identical to the canonical fine terrain file.
pub const FINE_SPACING_UM: i64 = 39_062_500;
/// Glacial lowstand: formation erodes against a base level this far below
/// today's sea, then the sea returns to 0. Lower valleys graded to the
/// lowstand drown into rias and bays; interfluves end as headlands and
/// islets (logic/02 §fine-formation drowned coasts, goal 15).
pub const LOWSTAND_MM: i32 = 120_000;
/// Macro depressions at least this deep below their spill are tectonic
/// basins (logic/02 §fine-formation basins), millimetres.
pub const BASIN_DEPTH_MM: i32 = 50_000;
/// ...and at least this large, in base-level (1250 m) cells (~400 km²).
pub const BASIN_MIN_CELLS: usize = 256;
/// Radius of the fixed endorheic sink at a basin's deepest point, µm.
const SINK_RADIUS_UM: i128 = 1_500_000_000;
/// Radius of the protected sink at a glacial trough's lowest point, µm.
const TROUGH_SINK_RADIUS_UM: i128 = 200_000_000;
/// Spacing of the downstream prepared bed that must drain.
pub const PREPARED_SPACING_UM: i64 = 100_000_000;
/// Number of levels; level `k` has spacing `FINE_SPACING_UM << (5 - k)`.
pub const LEVELS: usize = 6;
/// Iterations per level, coarse to fine.
pub const ITERATIONS: [u32; LEVELS] = [300, 120, 80, 50, 40, 30];
/// Talus slope per level, Q16: `0.85 * (39.0625 m / d)^0.2`.
pub const TALUS_Q16: [i64; LEVELS] = [27_852, 31_993, 36_750, 42_216, 48_493, 55_706];
/// Stream-power `K dt` in Q16 (0.02 per iteration, metre units).
pub const K_Q16: i64 = 1_311;
/// Relative uplift per iteration as a fraction of `talus * d`, Q16 (0.02).
pub const UPLIFT_FRACTION_Q16: i64 = 1_311;
/// Receiver randomisation probability, Q16 (0.7).
pub const STOCHASTIC_Q16: u32 = 45_875;
/// Coarse-level diffusion number, Q16 (0.02); zero from level 3 on.
pub const COARSE_KAPPA_Q16: i64 = 1_311;
/// Envelope correction gain at level 0 (0.1) and finer levels (0.3), Q16.
pub const LAMBDA_Q16: [i64; 2] = [6_554, 19_661];
/// Base-level perturbation amplitude, millimetres at full relief mask.
pub const BASE_NOISE_MM: i64 = 250_000;
/// Per-level band-limited relief as a fraction of `talus * d`, Q16 (0.3).
pub const LEVEL_NOISE_Q16: i64 = 19_661;
/// Hillslope diffusion number below channel initiation, fine levels, Q16.
pub const HILL_KAPPA_Q16: i64 = 2_621;
/// Channel-initiation area, 0.2 km² in Q8 finest-cell units (recipe 6).
pub const CHANNEL_AREA_Q8: u64 = 33_554;
/// Recipe-5 channel-initiation area, 0.25 km² in Q8 finest-cell units.
pub const CHANNEL_AREA_Q8_V5: u64 = 41_943;
/// Lowland soil-creep diffusion number at full lowland, Q16 (0.05).
pub const LOWLAND_CREEP_Q16: i64 = 3_277;
/// Macro relief (m, local standard deviation) at which masks saturate.
pub const RELIEF_FULL_M: i64 = 400;
/// Macro relief below which a setting counts as full lowland.
pub const LOWLAND_RELIEF_M: i64 = 250;

/// Formation failure.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FormationError {
    /// A size or index computation overflowed.
    #[error("fine formation arithmetic overflow")]
    ArithmeticOverflow,
    /// A fallible allocation failed after admission.
    #[error("fine formation allocation failed")]
    AllocationFailed,
    /// The requested lattice exceeds the admitted RAM envelope.
    #[error("fine formation requires {required} bytes, admitted {limit} bytes")]
    ResourceLimit {
        /// Required bytes.
        required: u128,
        /// Caller ceiling.
        limit: u128,
    },
}

/// Lattice dimensions of every level and the admitted peak.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormationPlan {
    /// `(width, height)` per level, coarse to fine.
    pub dims: [(usize, usize); LEVELS],
    /// Conservative peak RAM in bytes.
    pub peak_bytes: u128,
}

/// Plans all levels for a fine lattice of `fine_width x fine_height` nodes.
///
/// # Errors
/// Overflow, or a peak beyond `max_ram_bytes`.
pub fn plan(
    fine_width: u32,
    fine_height: u32,
    max_ram_bytes: u128,
) -> Result<FormationPlan, FormationError> {
    plan_recipe(fine_width, fine_height, max_ram_bytes, Recipe::V6)
}

/// [`plan`] for `recipe`: recipe 7 also holds a runoff weight per node.
///
/// # Errors
/// Overflow, or a peak beyond `max_ram_bytes`.
pub fn plan_recipe(
    fine_width: u32,
    fine_height: u32,
    max_ram_bytes: u128,
    recipe: Recipe,
) -> Result<FormationPlan, FormationError> {
    let mut dims = [(0, 0); LEVELS];
    for (k, dim) in dims.iter_mut().enumerate() {
        let f = 1_u64 << (LEVELS - 1 - k);
        let w = (u64::from(fine_width) - 1).div_ceil(f) + 1;
        let h = (u64::from(fine_height) - 1).div_ceil(f) + 1;
        if w * h >= 1 << 31 {
            return Err(FormationError::ArithmeticOverflow);
        }
        *dim = (
            usize::try_from(w).map_err(|_| FormationError::ArithmeticOverflow)?,
            usize::try_from(h).map_err(|_| FormationError::ArithmeticOverflow)?,
        );
    }
    let (fw, fh) = dims[LEVELS - 1];
    let fine = (fw * fh) as u128;
    let (pw, ph) = dims[LEVELS - 2];
    // Level lattice, scratch, three u8 fields, the parent during upsampling,
    // and a 64 MiB allowance for bucket heads, masks and row buffers.
    let per_cell = 4 + Scratch::BYTES_PER_CELL + 3 + if recipe >= Recipe::V7 { 2 } else { 0 };
    let peak = fine * per_cell + (pw * ph) as u128 * 4 + (64 << 20);
    if peak > max_ram_bytes {
        return Err(FormationError::ResourceLimit {
            required: peak,
            limit: max_ram_bytes,
        });
    }
    Ok(FormationPlan {
        dims,
        peak_bytes: peak,
    })
}

/// A fine-formation rule set (logic/02 §fine-formation recipes). The
/// number is the `recipe_version` a world's manifest records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Recipe {
    /// v0.1 formation, replayed byte for byte.
    V5,
    /// v0.2 formation: margins, belt relief, maturity, roughness, coasts
    /// and water forms.
    V6,
    /// Recipe 6 plus climate: runoff-weighted channels and incision, and
    /// arid endorheic basins with terminal lakes and playas.
    V7,
}

impl Recipe {
    /// The recipe new worlds use: recipe 7.
    pub const DEFAULT: Self = Self::V7;

    /// The newest recipe this build forms.
    pub const LATEST: Self = Self::V7;

    /// The manifest `recipe_version` of this recipe.
    #[must_use]
    pub const fn version(self) -> u16 {
        match self {
            Self::V5 => 5,
            Self::V6 => 6,
            Self::V7 => 7,
        }
    }

    /// The formation recipe with manifest number `version`, if any.
    #[must_use]
    pub const fn from_version(version: u16) -> Option<Self> {
        match version {
            5 => Some(Self::V5),
            6 => Some(Self::V6),
            7 => Some(Self::V7),
            _ => None,
        }
    }
}

/// Forms the default-recipe fine surface for one world seed and attempt.
/// Returns the finest lattice, `fine_width x fine_height` nodes at
/// [`FINE_SPACING_UM`], heights in millimetres.
///
/// # Errors
/// Admission or allocation failure.
pub fn form(
    seed: u64,
    attempt: u8,
    grid: &ContinentGrid,
    fine_width: u32,
    fine_height: u32,
    max_ram_bytes: u128,
) -> Result<Lattice, FormationError> {
    form_world(
        seed,
        attempt,
        grid,
        (fine_width, fine_height),
        max_ram_bytes,
        Recipe::DEFAULT,
        arda_core::GenerateConfig::MICRO.latitude_band(),
    )
    .map(|f| f.lattice)
}

/// [`form`], also returning the rivers and lakes formation shaped
/// (logic/02 §world-water): protected sinks by origin, braided belts,
/// deltas and dolines.
///
/// # Errors
/// Admission or allocation failure.
pub fn form_with_water(
    seed: u64,
    attempt: u8,
    grid: &ContinentGrid,
    fine_width: u32,
    fine_height: u32,
    max_ram_bytes: u128,
) -> Result<(Lattice, water::WaterFeatures), FormationError> {
    form_world(
        seed,
        attempt,
        grid,
        (fine_width, fine_height),
        max_ram_bytes,
        Recipe::V6,
        arda_core::GenerateConfig::MICRO.latitude_band(),
    )
    .map(|f| (f.lattice, f.water.unwrap_or_default()))
}

/// A formed world: the finest lattice and, for recipe 6, its shore classes
/// and island census on the 100 m prepared grid (logic/02 §fine-formation
/// shore classes) and the water forms shaped into it (logic/02
/// §world-water). Recipe 5 publishes neither.
#[derive(Debug)]
pub struct Formed {
    /// Finest lattice, heights in millimetres.
    pub lattice: Lattice,
    /// Shore classes, islands and audited landforms (recipe 6).
    pub shore: Option<arda_core::ShoreLayer>,
    /// Rivers and lakes formation shaped (recipe 6).
    pub water: Option<water::WaterFeatures>,
}

/// Forms a `size` (`width x height` nodes) fine surface with `recipe`
/// and every by-product it publishes ([`Formed`]). Recipe-6 stages run in
/// physical order: tectonic margins, bathymetry, arcs and the basin audit
/// on the macro surface; belt-relief level masks; multi-level formation;
/// the coast (infill, terraces, shelf, shore rework, littoral, canyons);
/// then water (deltas, channels, basins); drainage guarantees; the shore
/// survey last, so it describes the published surface. Recipe 5 runs the
/// v0.1 stages ([`recipe5`]). Recipe 7 reads the climate of `band`
/// ([`arid`]); earlier recipes ignore it.
///
/// # Errors
/// Admission or allocation failure.
pub fn form_world(
    seed: u64,
    attempt: u8,
    grid: &ContinentGrid,
    (fine_width, fine_height): (u32, u32),
    max_ram_bytes: u128,
    recipe: Recipe,
    band: LatitudeBand,
) -> Result<Formed, FormationError> {
    let plan = plan_recipe(fine_width, fine_height, max_ram_bytes, recipe)?;
    let (mut macro_mm, mut relief_m) = macro_lattices(grid)?;
    let base_seed = seed ^ (u64::from(attempt) << 56) ^ 0x0F0E_5A11_0000_0005;
    let extent_um = (
        i64::from(fine_width - 1) * FINE_SPACING_UM,
        i64::from(fine_height - 1) * FINE_SPACING_UM,
    );
    let base = (
        plan.dims[0].0,
        plan.dims[0].1,
        FINE_SPACING_UM << (LEVELS - 1),
    );
    if recipe == Recipe::V5 {
        // logic/02 §fine-formation bathymetry (recipe 5): relief-only
        // margins; masks read the local relief; no lowstand-ocean mask.
        bathymetry::apply_v5(&mut macro_mm, &relief_m)?;
        let view = MacroView {
            height: &macro_mm,
            relief: &relief_m,
            belt: &relief_m,
            ocean: &[],
            seed: base_seed,
            extent_um,
            v6: false,
            water: None,
        };
        let sinks = macro_basins(&view, base.0, base.1, base.2)?;
        let is_basin = |x_um: i64, y_um: i64| in_sink(&sinks, x_um, y_um);
        let g = levels::run(&plan, &view, base_seed, &is_basin)?;
        let lattice = recipe5::finish(g, &view, base_seed, &is_basin)?;
        return Ok(Formed {
            lattice,
            shore: None,
            water: None,
        });
    }
    // logic/02 §fine-formation margins: replayed plate margins decide
    // active versus passive shelves and where volcanic island arcs rise.
    let margins = crate::continent::margins::margin_context_formed(
        seed,
        i32::try_from(macro_mm.width).map_err(|_| FormationError::ArithmeticOverflow)?,
        i32::try_from(macro_mm.height).map_err(|_| FormationError::ArithmeticOverflow)?,
        attempt,
    );
    let macro_activity = margin::activity(&margins, macro_mm.width, macro_mm.height, 1_000);
    // logic/02 §fine-formation bathymetry: margin-aware open ocean.
    let shelf = bathymetry::apply(&mut macro_mm, &relief_m, &macro_activity)?;
    let volcanoes = margin::volcanic_arcs(
        base_seed ^ 0x0A2C,
        &margins,
        &shelf.target,
        &mut macro_mm,
        &mut relief_m,
    )?;
    // logic/02 §fine-formation basin audit: every large closed basin and
    // plateau gets a tectonic cause; unexplained basins are filled.
    let (landforms, _filled) = basins::audit(&mut macro_mm, &relief_m, &margins, LOWSTAND_MM)?;
    // Masks read the macro surface after the tectonic margin stages, so
    // volcanic arc cones count as high relief too.
    // logic/02 §fine-formation masks: level masks read belt relief, so
    // broad crests and valley axes are not classed as low hills.
    let belt_m = relief::belt_relief(&relief_m)?;
    let ocean = basins::lowstand_ocean(&macro_mm.z, macro_mm.width, macro_mm.height, LOWSTAND_MM)?;
    // logic/02 §fine-formation climate runoff (recipe 7): the continent's
    // annual water balance drives channels, incision and basin lakes.
    let water = match recipe {
        Recipe::V7 => Some(arid::macro_water(grid, band)?),
        _ => None,
    };
    let view = MacroView {
        height: &macro_mm,
        relief: &relief_m,
        belt: &belt_m,
        ocean: &ocean,
        seed: base_seed,
        extent_um,
        v6: true,
        water: water.as_ref().map(|m| (&m.runoff, &m.deficit)),
    };
    let sinks = macro_basins(&view, base.0, base.1, base.2)?;
    let arid_basins = arid::classify(&view, base, &sinks)?;
    let is_basin = |x_um: i64, y_um: i64| in_sink(&sinks, x_um, y_um);
    let g = levels::run(&plan, &view, base_seed, &is_basin)?;
    finish::run(
        g,
        finish::Context {
            view: &view,
            base_seed,
            basins: &sinks,
            is_basin: &is_basin,
            shelf: &shelf,
            volcanoes: &volcanoes,
            landforms,
            arid: &arid_basins,
        },
    )
}

/// logic/02 §fine-formation basins: endorheic sinks. Only a 1.5 km disc at
/// each basin's deepest point is fixed at the macro floor; the rest of the
/// basin forms normally and drains to it, and the annual water balance sets
/// the lake (or playa) around the sink.
fn in_sink(sinks: &[(i64, i64)], x_um: i64, y_um: i64) -> bool {
    sinks.iter().any(|&(sx, sy)| {
        let (dx, dy) = (i128::from(x_um - sx), i128::from(y_um - sy));
        dx * dx + dy * dy <= SINK_RADIUS_UM * SINK_RADIUS_UM
    })
}

#[cfg(test)]
mod band_tests;

#[cfg(test)]
mod form_tests;

#[cfg(test)]
mod recipe_tests;
