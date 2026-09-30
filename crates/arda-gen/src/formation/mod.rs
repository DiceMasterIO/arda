//! Fine terrain formation, recipe 5 (logic/02 §fine-formation).
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
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
// Index casts in this module are bounded by the admitted lattice size,
// which [`plan`] limits to fewer than 2^31 cells per level.

pub mod basins;
pub mod bathymetry;
pub mod canyon;
pub mod coast;
pub mod coastal;
pub mod distance;
pub mod drainage;
pub mod flats;
pub mod floodplain;
pub mod flow;
pub mod glacial;
pub mod incision;
pub mod lattice;
pub mod littoral;
pub mod margin;
pub mod relief;
pub mod sampled;
pub mod shore;
pub mod terrace;
pub mod water;

use rayon::prelude::*;
use thiserror::Error;

use crate::continent::ContinentGrid;
use crate::noise::{fbm, value_noise};
use drainage::FIXED;
use incision::{LevelFields, LevelParams, Scratch};
use lattice::{alloc, blur_into, Lattice};

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
/// Channel-initiation area, 0.25 km² in Q8 finest-cell units.
pub const CHANNEL_AREA_Q8: u64 = 33_554;
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
    let per_cell = 4 + Scratch::BYTES_PER_CELL + 3;
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

/// Macro surface and its local relief as 1 km lattices (mm and m).
fn macro_lattices(grid: &ContinentGrid) -> Result<(Lattice, Lattice), FormationError> {
    let (w, h) = (grid.width() as usize, grid.height() as usize);
    let mut m = Lattice::new(w, h, 1_000_000_000)?;
    for y in 0..h {
        for x in 0..w {
            m.z[y * w + x] = grid.get(x as i32, y as i32).raw();
        }
    }
    // logic/02 §fine-formation masks: local standard deviation of land
    // height over a ~25 km window measures tectonic relief.
    let land_m: Vec<i32> = m.z.iter().map(|&v| v.max(0) / 1000).collect();
    let sq: Vec<i32> = land_m.iter().map(|&v| v * v).collect();
    let mut tmp = alloc(w * h)?;
    let mut mean = alloc(w * h)?;
    let mut mean_sq = alloc(w * h)?;
    blur_into(&land_m, w, h, 12, &mut tmp, &mut mean);
    blur_into(&sq, w, h, 12, &mut tmp, &mut mean_sq);
    let mut rel = Lattice::new(w, h, 1_000_000_000)?;
    for i in 0..w * h {
        let var = i64::from(mean_sq[i]) - i64::from(mean[i]) * i64::from(mean[i]);
        rel.z[i] = incision::isqrt(var.max(0) as u64) as i32;
    }
    Ok((m, rel))
}

/// The macro surface and its relief as formation sees them: domain-warped
/// by two octaves of value noise (±20 km at 120 km, ±6 km at 45 km), tapered
/// to zero within 40 km of the domain edge so the forced ocean rim holds.
/// The warp bends range crests, basin walls and macro coasts that the
/// plate model draws along near-straight Voronoi edges (logic/02
/// §fine-formation macro warp). Its peak slope stays below one.
struct MacroView<'a> {
    height: &'a Lattice,
    relief: &'a Lattice,
    belt: &'a Lattice,
    /// Lowstand ocean on the macro lattice ([`basins::lowstand_ocean`]).
    ocean: &'a [u8],
    seed: u64,
    extent_um: (i64, i64),
}

impl MacroView<'_> {
    fn warp(&self, x_um: i64, y_um: i64) -> (i64, i64) {
        let edge = x_um
            .min(y_um)
            .min(self.extent_um.0 - x_um)
            .min(self.extent_um.1 - y_um)
            .max(0);
        let taper = (edge * 4096 / 40_000_000_000).min(4096);
        if taper == 0 {
            return (x_um, y_um);
        }
        let (xm, ym) = (
            i32::try_from(x_um / 1_000_000).unwrap_or(0),
            i32::try_from(y_um / 1_000_000).unwrap_or(0),
        );
        let n = |salt: u64| {
            i64::from(value_noise(self.seed ^ salt, xm, ym, 120_000)) * 20_000_000_000 / 32_768
                + i64::from(value_noise(self.seed ^ salt ^ 0x55, xm, ym, 45_000)) * 6_000_000_000
                    / 32_768
        };
        (
            x_um + n(0x3A11) * taper / 4096,
            y_um + n(0x3A12) * taper / 4096,
        )
    }

    fn height(&self, x_um: i64, y_um: i64) -> i32 {
        let (x, y) = self.warp(x_um, y_um);
        self.height.sample_um_bounded(x, y)
    }

    fn relief_m(&self, x_um: i64, y_um: i64) -> i64 {
        let (x, y) = self.warp(x_um, y_um);
        i64::from(self.relief.sample_um(x, y)).max(0)
    }

    /// Whether the macro cell nearest a point is lowstand ocean.
    fn is_ocean(&self, x_um: i64, y_um: i64) -> bool {
        let (x, y) = self.warp(x_um, y_um);
        let s = self.height.spacing_um;
        let (w, h) = (self.height.width as i64, self.height.height as i64);
        let cx = ((x + s / 2) / s).clamp(0, w - 1);
        let cy = ((y + s / 2) / s).clamp(0, h - 1);
        self.ocean
            .get((cy * w + cx) as usize)
            .is_some_and(|&o| o != 0)
    }

    /// Belt relief (see [`relief::belt_relief`]) for the level masks.
    fn belt_m(&self, x_um: i64, y_um: i64) -> i64 {
        let (x, y) = self.warp(x_um, y_um);
        i64::from(self.belt.sample_um(x, y)).max(0)
    }
}

/// Sink points (µm) of large closed macro depressions, found on the base
/// lattice (logic/02 §fine-formation basins). Depths of small or shallow
/// depressions are zero: those are ordinary relief that formation drains.
fn macro_basins(
    view: &MacroView<'_>,
    w: usize,
    h: usize,
    d: i64,
) -> Result<Vec<(i64, i64)>, FormationError> {
    let mut z = Lattice::new(w, h, d)?;
    z.z.par_iter_mut().enumerate().for_each(|(i, v)| {
        let (x, y) = ((i % w) as i64 * d, (i / w) as i64 * d);
        *v = view.height(x, y);
    });
    let n = w * h;
    let mut flags: Vec<u8> = alloc(n)?;
    for (i, (f, &zi)) in flags.iter_mut().zip(&z.z).enumerate() {
        let (x, y) = (i % w, i / w);
        let ocean = zi <= -LOWSTAND_MM && view.is_ocean(x as i64 * d, y as i64 * d);
        if ocean || x == 0 || y == 0 || x == w - 1 || y == h - 1 {
            *f = FIXED;
        }
    }
    let mut filled = z.z.clone();
    let mut next: Vec<u32> = alloc(n)?;
    let mut closed: Vec<u8> = alloc(n)?;
    drainage::fill(&mut filled, w, h, &flags, 0, &mut next, &mut closed)?;
    let mut depth = Lattice::new(w, h, d)?;
    for ((dz, &f), &zi) in depth.z.iter_mut().zip(&filled).zip(&z.z) {
        *dz = (f - zi).max(0);
    }
    // Large deep components each get one sink.
    let mut sinks = Vec::new();
    let mut seen: Vec<u8> = alloc(n)?;
    let mut stack = Vec::new();
    let mut comp = Vec::new();
    for s0 in 0..n {
        if seen[s0] != 0 || depth.z[s0] < BASIN_DEPTH_MM {
            continue;
        }
        comp.clear();
        stack.push(s0);
        seen[s0] = 1;
        while let Some(c) = stack.pop() {
            comp.push(c);
            for k in 0..8 {
                if let Some(nb) = drainage::neighbour(c, w, h, k) {
                    if seen[nb] == 0 && depth.z[nb] >= BASIN_DEPTH_MM {
                        seen[nb] = 1;
                        stack.push(nb);
                    }
                }
            }
        }
        if comp.len() < BASIN_MIN_CELLS {
            continue;
        }
        // The sink: the deepest cell, ties broken toward the component
        // centroid (flat floors sink in their middle).
        let n_c = comp.len() as i64;
        let cx = comp.iter().map(|&c| (c % w) as i64).sum::<i64>() / n_c;
        let cy = comp.iter().map(|&c| (c / w) as i64).sum::<i64>() / n_c;
        let key = |c: usize| {
            let (x, y) = ((c % w) as i64, (c / w) as i64);
            (
                std::cmp::Reverse(depth.z[c]),
                (x - cx) * (x - cx) + (y - cy) * (y - cy),
                c,
            )
        };
        if let Some(&c) = comp.iter().min_by_key(|&&c| key(c)) {
            sinks.push(((c % w) as i64 * d, (c / w) as i64 * d));
        }
    }
    Ok(sinks)
}

/// logic/02 §fine-formation roughness: multi-scale (600-75 m) rugged
/// texture added on top of the formed surface, so slopes are not glassy.
/// Amplitude grows with local slope (rock faces) and macro relief, and
/// fades to zero near channels (area ≥ 0.05 km²) so drainage stays clean.
/// Octaves are sampled in two rotated domains, never axis-aligned.
fn roughen(
    g: &mut Lattice,
    flags: &[u8],
    area: Option<&[u64]>,
    relief: &[u8],
    seed: u64,
) -> Result<(), FormationError> {
    let (w, h) = (g.width, g.height);
    let d_mm = g.spacing_um / 1000;
    let mut out: Vec<i32> = alloc(w * h)?;
    let z = &g.z;
    // 0.05 km^2 in Q8 finest-cell units.
    let channel_q8: u64 = 8_389;
    out.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (x, o) in row.iter_mut().enumerate() {
            let i = y * w + x;
            *o = z[i];
            if flags[i] & FIXED != 0 || z[i] <= 0 || x == 0 || y == 0 || x + 1 == w || y + 1 == h {
                continue;
            }
            let gx = i64::from(z[i + 1]) - i64::from(z[i - 1]);
            let gy = i64::from(z[i + w]) - i64::from(z[i - w]);
            let slope_q12 = i64::try_from(
                (i128::from(gx) * i128::from(gx) + i128::from(gy) * i128::from(gy))
                    .unsigned_abs()
                    .isqrt(),
            )
            .unwrap_or(0)
                * 4_096
                / (2 * d_mm).max(1);
            // 0.25 at flat ground to 1 at slope >= 0.6.
            let steep = 1_024 + 3_072 * slope_q12.min(2_458) / 2_458;
            let rel = 64 + i64::from(relief[i]) * 192 / 255;
            let a = area.map_or(0, |a| a[i]);
            let away = if a >= channel_q8 {
                0
            } else {
                4_096 - i64::try_from(a * 4_096 / channel_q8).unwrap_or(4_096)
            };
            let amp_mm = 70_000 * steep / 4_096 * rel / 256 * away / 4_096;
            if amp_mm == 0 {
                continue;
            }
            let xm = x as i64 * g.spacing_um / 1_000_000;
            let ym = y as i64 * g.spacing_um / 1_000_000;
            let (ax, ay) = (
                (xm * 3_271 - ym * 2_465) / 4_096,
                (xm * 2_465 + ym * 3_271) / 4_096,
            );
            let (bx, by) = (
                (xm * 3_770 + ym * 1_600) / 4_096,
                (-xm * 1_600 + ym * 3_770) / 4_096,
            );
            // Octaves no finer than ~5 cells (finer ones alias against the
            // 39 m lattice); each is the mean of two rotated domains so no
            // lattice direction survives.
            let mut sum = 0_i64;
            for (k, (period, weight)) in [(800, 4), (400, 2), (200, 1)].into_iter().enumerate() {
                let salt = seed ^ ((k as u64 + 1) << 48);
                let va = i64::from(value_noise(salt, ax as i32, ay as i32, period));
                let vb = i64::from(value_noise(salt ^ 0x5A5A, bx as i32, by as i32, period));
                sum += weight * (va + vb);
            }
            // sum within about ±14 * 32768; two-domain mean halves variance,
            // so scale back up by ~sqrt(2).
            let dz = amp_mm * sum * 181 / (128 * 7 * 32_768);
            *o = i32::try_from(i64::from(z[i]) + dz).unwrap_or(z[i]).max(1);
        }
    });
    g.z = out;
    Ok(())
}

/// Two passes of a 3x3 binomial (1-2-1) filter over non-fixed land cells.
fn smooth_seams(g: &mut Lattice, flags: &[u8]) -> Result<(), FormationError> {
    let (w, h) = (g.width, g.height);
    let mut tmp: Vec<i32> = alloc(w * h)?;
    for _ in 0..1 {
        let z = &g.z;
        tmp.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
            for (x, out) in row.iter_mut().enumerate() {
                let i = y * w + x;
                if flags[i] & FIXED != 0
                    || z[i] <= 0
                    || x == 0
                    || y == 0
                    || x + 1 == w
                    || y + 1 == h
                {
                    *out = z[i];
                    continue;
                }
                let at = |dx: i64, dy: i64| {
                    i64::from(z[((y as i64 + dy) as usize) * w + (x as i64 + dx) as usize])
                };
                let sum = 4 * at(0, 0)
                    + 2 * (at(1, 0) + at(-1, 0) + at(0, 1) + at(0, -1))
                    + at(1, 1)
                    + at(-1, 1)
                    + at(1, -1)
                    + at(-1, -1);
                *out = i32::try_from(sum.div_euclid(16)).unwrap_or(z[i]).max(1);
            }
        });
        g.z.copy_from_slice(&tmp);
    }
    Ok(())
}

/// Fixed flags for the final fills: the open sea and protected sinks.
fn sea_and_sinks(
    g: &Lattice,
    is_sink: &(dyn Fn(i64, i64) -> bool + Sync),
) -> Result<Vec<u8>, FormationError> {
    let (w, h) = (g.width, g.height);
    let mut flags: Vec<u8> = alloc(w * h)?;
    drainage::open_sea_flags(&g.z, w, h, &mut flags);
    let d = g.spacing_um;
    flags.par_iter_mut().enumerate().for_each(|(i, f)| {
        if is_sink((i % w) as i64 * d, (i / w) as i64 * d) {
            *f = FIXED;
        }
    });
    Ok(flags)
}

/// Forms the recipe-5 fine surface for one world seed and attempt.
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
    form_world(seed, attempt, grid, fine_width, fine_height, max_ram_bytes).map(|f| f.lattice)
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
    form_world(seed, attempt, grid, fine_width, fine_height, max_ram_bytes)
        .map(|f| (f.lattice, f.water))
}

/// A formed recipe-5 world: the finest lattice, its shore classes and
/// island census on the 100 m prepared grid (logic/02 §fine-formation shore
/// classes), and the water forms shaped into it (logic/02 §world-water).
#[derive(Debug)]
pub struct Formed {
    /// Finest lattice, heights in millimetres.
    pub lattice: Lattice,
    /// Shore classes, islands and audited landforms.
    pub shore: arda_core::ShoreLayer,
    /// Rivers and lakes formation shaped.
    pub water: water::WaterFeatures,
}

/// [`form`] with every published by-product ([`Formed`]). Stages run in
/// physical order: tectonic margins, bathymetry, arcs and the basin audit
/// on the macro surface; belt-relief level masks; multi-level formation;
/// the coast (infill, terraces, shelf, shore rework, littoral, canyons);
/// then water (deltas, channels, basins); drainage guarantees; the shore
/// survey last, so it describes the published surface.
///
/// # Errors
/// Admission or allocation failure.
pub fn form_world(
    seed: u64,
    attempt: u8,
    grid: &ContinentGrid,
    fine_width: u32,
    fine_height: u32,
    max_ram_bytes: u128,
) -> Result<Formed, FormationError> {
    let plan = plan(fine_width, fine_height, max_ram_bytes)?;
    let (mut macro_mm, mut relief_m) = macro_lattices(grid)?;
    let base_seed = seed ^ (u64::from(attempt) << 56) ^ 0x0F0E_5A11_0000_0005;
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
    let (mut landforms, _filled) = basins::audit(&mut macro_mm, &relief_m, &margins, LOWSTAND_MM)?;
    // Masks read the macro surface after the tectonic margin stages, so
    // volcanic arc cones count as high relief too.
    // logic/02 §fine-formation masks: level masks read belt relief, so
    // broad crests and valley axes are not classed as low hills.
    let belt_m = relief::belt_relief(&relief_m)?;
    let ocean = basins::lowstand_ocean(&macro_mm.z, macro_mm.width, macro_mm.height, LOWSTAND_MM)?;
    let view = MacroView {
        height: &macro_mm,
        relief: &relief_m,
        belt: &belt_m,
        ocean: &ocean,
        seed: base_seed,
        extent_um: (
            i64::from(fine_width - 1) * FINE_SPACING_UM,
            i64::from(fine_height - 1) * FINE_SPACING_UM,
        ),
    };
    let basins = macro_basins(
        &view,
        plan.dims[0].0,
        plan.dims[0].1,
        FINE_SPACING_UM << (LEVELS - 1),
    )?;
    // logic/02 §fine-formation basins: endorheic sinks. Only a 1.5 km disc
    // at each basin's deepest point is fixed at the macro floor; the rest
    // of the basin forms normally and drains to it, and the annual water
    // balance sets the lake (or playa) around the sink.
    let is_basin = |x_um: i64, y_um: i64| {
        basins.iter().any(|&(sx, sy)| {
            let (dx, dy) = (i128::from(x_um - sx), i128::from(y_um - sy));
            dx * dx + dy * dy <= SINK_RADIUS_UM * SINK_RADIUS_UM
        })
    };
    let mut prev: Option<Lattice> = None;
    for k in 0..LEVELS {
        let (w, h) = plan.dims[k];
        let d = FINE_SPACING_UM << (LEVELS - 1 - k);
        let n = w * h;
        let level_seed = base_seed.wrapping_add((k as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mut scratch = Scratch::new(n)?;
        let mut flags: Vec<u8> = alloc(n)?;
        let mut uplift: Vec<u8> = alloc(n)?;
        let mut erod: Vec<u8> = alloc(n)?;
        let mut lowland: Vec<u8> = alloc(n)?;
        let mut rock: Vec<u8> = alloc(n)?;
        let mut soft: Vec<u8> = alloc(n)?;
        let mut relief_q8: Vec<u8> = alloc(n)?;
        // Masks from macro relief (logic/02 §fine-formation masks).
        let xy_m = |i: usize| {
            (
                ((i % w) as i64 * d) / 1_000_000,
                ((i / w) as i64 * d) / 1_000_000,
            )
        };
        uplift
            .par_iter_mut()
            .zip(erod.par_iter_mut())
            .zip(lowland.par_iter_mut())
            .enumerate()
            .for_each(|(i, ((u, e), l))| {
                let (xm, ym) = xy_m(i);
                let rel = view.belt_m(xm * 1_000_000, ym * 1_000_000);
                let (xi, yi) = (xm as i32, ym as i32);
                // Along-range variation of uplift (0.2..1.6) and erodibility.
                let md = 36_045
                    + (i64::from(value_noise(base_seed ^ 0x4242, xi, yi, 45_000))
                        + i64::from(value_noise(base_seed ^ 0x4243, xi, yi, 18_000)) / 2)
                        * 29_491
                        / 32_768;
                // Low relative uplift on flat macro terrain (2% floor, ramping
                // smoothly with relief): plains keep gentle drainage relief
                // without being built up into talus-steep gullies.
                let mask = 1_311 + 64_225 * rel.min(RELIEF_FULL_M) / RELIEF_FULL_M;
                let um = (mask * md.clamp(13_107, 104_858) / 65_536) * 128 / 65_536;
                *u = um.clamp(0, 255) as u8;
                let ev = 64
                    + (i64::from(value_noise(base_seed ^ 0x9191, xi, yi, 14_000)) * 32
                        + i64::from(value_noise(base_seed ^ 0x9192, xi, yi, 5_000)) * 16)
                        / 32_768;
                *e = ev.clamp(16, 160) as u8;
                *l = (256 - 256 * rel.min(LOWLAND_RELIEF_M) / LOWLAND_RELIEF_M).clamp(0, 255) as u8;
            });
        // logic/02 §fine-formation rock strength: talus slope varies ±30%
        // with lithology-like noise fixed in world coordinates, so strong
        // rock stands as steeper ribs and faces instead of uniform planes.
        // logic/02 §fine-formation maturity: slowly varying (8-25 km,
        // rotated) landscape maturity; mature patches keep rounded crests
        // and rolling uplands, young ones are fully dissected.
        relief_q8.par_iter_mut().enumerate().for_each(|(i, o)| {
            let (xm, ym) = xy_m(i);
            let rel = view
                .belt_m(xm * 1_000_000, ym * 1_000_000)
                .min(RELIEF_FULL_M);
            *o = (rel * 255 / RELIEF_FULL_M) as u8;
        });
        soft.par_iter_mut().enumerate().for_each(|(i, o)| {
            let (xm, ym) = xy_m(i);
            let rx = ((xm * 3_770 + ym * 1_600) / 4_096) as i32;
            let ry = ((-xm * 1_600 + ym * 3_770) / 4_096) as i32;
            let v = i64::from(value_noise(base_seed ^ 0x50F7, rx, ry, 25_000)) * 2
                + i64::from(value_noise(base_seed ^ 0x50F8, rx, ry, 8_000));
            // v in about ±1.5 * 32768; map to 0..255 with a soft centre.
            *o = (128 + v * 200 / (3 * 32_768)).clamp(0, 255) as u8;
        });
        rock.par_iter_mut().enumerate().for_each(|(i, r)| {
            let (xm, ym) = xy_m(i);
            // Rotated 37° so lattice cells do not imprint axis-aligned
            // lithology; three octaves down to 150 m vary gully initiation
            // and facet steepness locally (breaks parallel combs). The shore
            // pass reads the same field as its rock proxy.
            *r = coastal::rock_strength_q8(base_seed, xm, ym);
        });
        // Level surface.
        let mut g = match prev.take() {
            None => {
                let mut g = Lattice::new(w, h, d)?;
                g.z.par_iter_mut().enumerate().for_each(|(i, z)| {
                    let (xm, ym) = xy_m(i);
                    *z = view.height(xm * 1_000_000, ym * 1_000_000);
                });
                g
            }
            Some(p) => p.resample_warped(
                w,
                h,
                d,
                p.spacing_um * 3 / 5,
                p.spacing_um * 5,
                level_seed ^ 0x3A2F,
            )?,
        };
        // The envelope target: macro at level 0, the inherited parent after.
        if k + 1 == LEVELS {
            // logic/02 §fine-formation roughness: added before the finest
            // erosion pass (and into its envelope target) so channels cut
            // through it and drainage adapts, instead of fills flattening
            // post-hoc hollows into streaks. No channels exist yet, so the
            // channel mask is inactive here (no area: a zero vector of
            // u64 per finest cell, 2.6 GB at full size, was the peak's
            // largest single allocation).
            let open: Vec<u8> = alloc(n)?;
            roughen(&mut g, &open, None, &relief_q8, base_seed ^ 0x2006)?;
        }
        scratch.raw_target().copy_from_slice(&g.z);
        // Ocean (non-positive macro) and the domain rim are fixed.
        let talus = TALUS_Q16[k];
        let level_amp = if k >= 3 {
            0
        } else {
            LEVEL_NOISE_Q16 * talus / 65_536 * d / 1000 / 65_536
        };
        g.z.par_iter_mut()
            .zip(flags.par_iter_mut())
            .enumerate()
            .for_each(|(i, (z, f))| {
                let (x, y) = (i % w, i / w);
                let (xm, ym) = xy_m(i);
                let target = view.height(xm * 1_000_000, ym * 1_000_000);
                let rim = x == 0 || y == 0 || x == w - 1 || y == h - 1;
                if (target <= -LOWSTAND_MM && view.is_ocean(xm * 1_000_000, ym * 1_000_000)) || rim
                {
                    // Open ocean below the lowstand, and the domain rim, are
                    // fixed base level; the rim is always sea. A closed
                    // depression deeper than the lowstand forms as land
                    // (logic/02 §fine-formation basins).
                    *f = FIXED;
                    *z = target.min(-LOWSTAND_MM);
                    return;
                }
                if is_basin(xm * 1_000_000, ym * 1_000_000) {
                    // Tectonic basin floor: fixed at the macro surface, a
                    // base level for rivers; hydrology decides its lake.
                    *f = FIXED;
                    *z = target;
                    return;
                }
                let rel = view.belt_m(xm * 1_000_000, ym * 1_000_000);
                let (xi, yi) = (xm as i32, ym as i32);
                let dz = if k == 0 {
                    let mask = 9_830 + 55_706 * rel.min(RELIEF_FULL_M) / RELIEF_FULL_M;
                    BASE_NOISE_MM * mask / 65_536
                        * i64::from(fbm(base_seed ^ 77, xi, yi, 24_000, 4))
                        / 32_768
                } else {
                    let mask = 6_554 + 58_982 * rel.min(RELIEF_FULL_M) / RELIEF_FULL_M;
                    let period = ((d * 27 / 10) / 1_000_000).max(2) as i32;
                    level_amp * mask / 65_536 * i64::from(value_noise(level_seed, xi, yi, period))
                        / 32_768
                };
                *z = i32::try_from(i64::from(*z) + dz).unwrap_or(*z);
            });
        {
            let raw = scratch.raw_target();
            for i in 0..n {
                if flags[i] & FIXED != 0 {
                    raw[i] = g.z[i];
                }
            }
        }
        let blur_cells = if k == 0 { 8 } else { 4 };
        incision::prepare_target(w, h, blur_cells, &mut scratch);
        let params = LevelParams {
            k_q16: K_Q16,
            talus_q16: talus,
            lambda_q16: LAMBDA_Q16[usize::from(k > 0)],
            blur_cells,
            iterations: ITERATIONS[k],
            kappa_q16: if k < 3 { COARSE_KAPPA_Q16 } else { 0 },
            stoch_q16: STOCHASTIC_Q16,
            seed: level_seed,
            uplift_mm: UPLIFT_FRACTION_Q16 * talus / 65_536 * d / 1000 / 65_536,
            eps_mm: i32::try_from((d / 19_531_250).max(1)).unwrap_or(1),
            area_unit: 1 << (2 * (LEVELS - 1 - k)),
            floor_mm: incision::LAND_FLOOR_MM - LOWSTAND_MM,
            creep_kappa_q16: LOWLAND_CREEP_Q16,
            hill_kappa_q16: if k >= 3 { HILL_KAPPA_Q16 } else { 0 },
            channel_area_q8: if k >= 3 { CHANNEL_AREA_Q8 } else { 0 },
            convergent_area: k >= 3,
        };
        let fields = LevelFields {
            flags: &flags,
            uplift: &uplift,
            erodibility: &erod,
            runoff: None,
            creep: &lowland,
            rock: &rock,
            soft: &soft,
            relief: &relief_q8,
        };
        incision::run(&mut g, &fields, &params, &mut scratch)?;
        if k >= 2 {
            let mut chan = std::mem::take(&mut scratch.tmp);
            floodplain::apply(&mut g, &flags, &lowland, &scratch, &mut chan);
        }
        if k + 1 == LEVELS {
            // logic/02 §fine-formation seams: the D8 talus cap leaves
            // one-cell diagonal seams (metres deep) across every hillslope.
            // Two binomial passes remove them; features wider than a few
            // cells, including crests and valleys, keep their shape.
            smooth_seams(&mut g, &flags)?;
        }
        prev = Some(g);
    }
    let mut g = prev.ok_or(FormationError::ArithmeticOverflow)?;
    // logic/02 §fine-formation drowned coasts: estuarine infill beyond the
    // relief-dependent ria reach.
    let relief_q8 = {
        let (w, d) = (g.width, g.spacing_um);
        let n = g.z.len();
        let mut macro_fine: Vec<i32> = alloc(n)?;
        let mut relief_q8: Vec<u8> = alloc(n)?;
        macro_fine
            .par_iter_mut()
            .zip(relief_q8.par_iter_mut())
            .enumerate()
            .for_each(|(i, (m, r))| {
                let (x_um, y_um) = ((i % w) as i64 * d, (i / w) as i64 * d);
                *m = view.height(x_um, y_um);
                let rel = view.relief_m(x_um, y_um).min(RELIEF_FULL_M);
                *r = (rel * 255 / RELIEF_FULL_M) as u8;
            });
        coast::infill(&mut g, &macro_fine, &relief_q8)?;
        drop(macro_fine);
        // logic/02 §fine-formation terraces: lowland terraces, bluffs and
        // flat interfluves on the infilled coastal lowland.
        terrace::apply(&mut g, &relief_q8, base_seed ^ 0x7E44_0000)?;
        // logic/02 §fine-formation bathymetry: passive shelves shallow
        // toward the margin profile, seen through the same macro warp.
        let shelf_view = MacroView {
            height: &shelf.target,
            relief: &shelf.activity,
            ..view
        };
        let (mut target, mut activity): (Vec<i32>, Vec<u8>) = (alloc(n)?, alloc(n)?);
        target
            .par_iter_mut()
            .zip(activity.par_iter_mut())
            .enumerate()
            .for_each(|(i, (t, a))| {
                let (x_um, y_um) = ((i % w) as i64 * d, (i / w) as i64 * d);
                *t = shelf_view.height(x_um, y_um);
                *a = u8::try_from(shelf_view.relief_m(x_um, y_um).min(255)).unwrap_or(255);
            });
        bathymetry::shelf_fill(&mut g, &target, &activity)?;
        drop(target);
        drop(activity);
        relief_q8
    };
    coast::rework_shore(&mut g, -LOWSTAND_MM)?;
    // logic/02 §fine-formation littoral: cliffs, bay beaches, barriers;
    // then §canyons: submarine canyons off the major river mouths.
    let littoral = {
        let mouths = coastal::river_mouths(&g, &flow::route(&g)?);
        let setting = coastal::compute(&g, base_seed, &mouths)?;
        let built = littoral::apply(
            &mut g,
            &setting,
            &mouths,
            base_seed ^ 0x0117,
            base_seed,
            &volcanoes,
        )?;
        drop(setting);
        canyon::carve(&mut g, &mouths)?;
        built
    };
    let fine_fill = |g: &mut Lattice,
                     is_basin: &(dyn Fn(i64, i64) -> bool + Sync)|
     -> Result<(), FormationError> {
        let flags = sea_and_sinks(g, is_basin)?;
        let (w, h, n) = (g.width, g.height, g.z.len());
        let mut next: Vec<u32> = alloc(n)?;
        let mut closed: Vec<u8> = alloc(n)?;
        drainage::fill(&mut g.z, w, h, &flags, 1, &mut next, &mut closed)
    };
    // logic/02 §fine-formation flats: fill flats (sediment-filled basins,
    // estuarine infill) are regraded as cost-weighted geodesics, so rivers
    // wander across them instead of running along grid geodesics.
    fine_fill(&mut g, &is_basin)?;
    {
        let flags = sea_and_sinks(&g, &is_basin)?;
        flats::regrade(&mut g, &flags, base_seed ^ 0xF1A7_5000)?;
    }
    // logic/02 §world-water deltas: the one delta rule. River-led,
    // volume-limited lobes, split into delta islands on large rivers. Built
    // after the shore rework and the littoral headland retreat, which would
    // otherwise smooth the lobes away or cut them back into cliffs.
    let mut features = water::WaterFeatures::default();
    // The macro surface is sampled again here rather than held through the
    // littoral stages, to keep the peak allocation down.
    let mut macro_fine: Vec<i32> = alloc(g.z.len())?;
    {
        let (w, d) = (g.width, g.spacing_um);
        macro_fine.par_iter_mut().enumerate().for_each(|(i, m)| {
            *m = view.height((i % w) as i64 * d, (i / w) as i64 * d);
        });
    }
    water::delta::build(
        &mut g,
        &macro_fine,
        &relief_q8,
        base_seed ^ 0xDE17A,
        &mut features,
    )?;
    drop(macro_fine);
    // Final drainage (logic/02 §fine-formation sampled drainage): every fine
    // land cell drains to the sea, and so does the 100 m point-sampled bed.
    // The sampled pass runs last: the published 100 m bed is the authority.
    // Raising its support nodes can leave a few isolated fine pits, which
    // no downstream stage reads.
    fine_fill(&mut g, &is_basin)?;
    // logic/02 §world-water: braided belts and meanders, drained again,
    // then oxbows and karst poljes; their closed basins become protected
    // sinks for every later drainage guarantee.
    let shaped = water::shape_channels(&mut g, &relief_q8, base_seed ^ 0x3A7E_5000, &mut features)?;
    drop(relief_q8);
    fine_fill(&mut g, &is_basin)?;
    water::shape_basins(&mut g, &shaped, &mut features)?;
    drop(shaped);
    features.index();
    let is_shaped = |x_um: i64, y_um: i64| is_basin(x_um, y_um) || features.is_sink(x_um, y_um);
    fine_fill(&mut g, &is_shaped)?;
    sampled::drain_sampled(&mut g, PREPARED_SPACING_UM, &is_shaped)?;
    // logic/02 §fine-formation glacial lakes: deliberately closed trough
    // basins. Overlapping carve discs also leave secondary hollows, so the
    // drainage guarantees run again with each trough's sink protected.
    let troughs = glacial::trough_lakes(&mut g)?;
    // The guarantees run once more, with each trough's sink protected: the
    // fine fill also grades the pockets the sampled pass levelled, which it
    // leaves exactly flat, and the last sampled pass keeps the published
    // 100 m bed the authority.
    let is_sink = |x_um: i64, y_um: i64| {
        is_shaped(x_um, y_um)
            || troughs.iter().any(|&(sx, sy)| {
                let (dx, dy) = (i128::from(x_um - sx), i128::from(y_um - sy));
                dx * dx + dy * dy <= TROUGH_SINK_RADIUS_UM * TROUGH_SINK_RADIUS_UM
            })
    };
    fine_fill(&mut g, &is_sink)?;
    sampled::drain_sampled(&mut g, PREPARED_SPACING_UM, &is_sink)?;
    water::record_basins(
        &mut features,
        &basins,
        &troughs,
        SINK_RADIUS_UM,
        TROUGH_SINK_RADIUS_UM,
    );
    // logic/02 §fine-formation shore: classes and island census, last, so
    // they describe the published surface.
    let built = shore::Builders {
        volcanoes: &volcanoes,
        deltas: &features.deltas,
        barrier_cells: &littoral.barrier_cells,
    };
    let mut layer = shore::survey(&g, PREPARED_SPACING_UM, base_seed, &built)?;
    landforms.extend(troughs.iter().map(|&(x_um, y_um)| arda_core::Landform {
        x_um,
        y_um,
        area_km2: 0,
        relief_m: 0,
        kind: arda_core::LandformKind::GlacialTrough,
        cause: arda_core::LandformCause::Glacial,
    }));
    layer.landforms = landforms;
    Ok(Formed {
        lattice: g,
        shore: layer,
        water: features,
    })
}

#[cfg(test)]
mod band_tests;

#[cfg(test)]
mod form_tests;
