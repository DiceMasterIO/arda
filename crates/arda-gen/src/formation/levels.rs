//! The multi-resolution level loop of fine formation (logic/02
//! §fine-formation levels, masks, rock strength, maturity, roughness and
//! seams), shared by recipes 5 and 6. Recipe 6 adds the maturity and
//! relief-scaling fields, pre-erosion roughness, a lower channel-initiation
//! area and a single seam pass; recipe 5 runs exactly as v0.1 did.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use rayon::prelude::*;

use super::drainage::FIXED;
use super::incision::{self, LevelFields, LevelParams, Scratch};
use super::lattice::{alloc, Lattice};
use super::macro_view::MacroView;
use super::{
    coastal, floodplain, surface, FormationError, FormationPlan, BASE_NOISE_MM, CHANNEL_AREA_Q8,
    CHANNEL_AREA_Q8_V5, COARSE_KAPPA_Q16, FINE_SPACING_UM, HILL_KAPPA_Q16, ITERATIONS, K_Q16,
    LAMBDA_Q16, LEVELS, LEVEL_NOISE_Q16, LOWLAND_CREEP_Q16, LOWLAND_RELIEF_M, LOWSTAND_MM,
    RELIEF_FULL_M, STOCHASTIC_Q16, TALUS_Q16, UPLIFT_FRACTION_Q16,
};
use crate::noise::{fbm, value_noise};

/// Runs every level, coarse to fine, and returns the finest lattice.
/// `is_basin` marks the fixed endorheic sink discs.
///
/// # Errors
/// Allocation failure.
pub(super) fn run(
    plan: &FormationPlan,
    view: &MacroView<'_>,
    base_seed: u64,
    is_basin: &(dyn Fn(i64, i64) -> bool + Sync),
) -> Result<Lattice, FormationError> {
    let v6 = view.v6;
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
        // Recipe-6 maturity and relief-scaling fields (empty for recipe 5).
        let (mut soft, mut relief_q8): (Vec<u8>, Vec<u8>) = if v6 {
            (alloc(n)?, alloc(n)?)
        } else {
            (Vec::new(), Vec::new())
        };
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
        if v6 && k + 1 == LEVELS {
            // logic/02 §fine-formation roughness: added before the finest
            // erosion pass (and into its envelope target) so channels cut
            // through it and drainage adapts, instead of fills flattening
            // post-hoc hollows into streaks. No channels exist yet, so the
            // channel mask is inactive here (no area: a zero vector of
            // u64 per finest cell, 2.6 GB at full size, was the peak's
            // largest single allocation).
            let open: Vec<u8> = alloc(n)?;
            surface::roughen(&mut g, &open, None, &relief_q8, base_seed ^ 0x2006)?;
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
            channel_area_q8: match (k >= 3, v6) {
                (false, _) => 0,
                (true, true) => CHANNEL_AREA_Q8,
                (true, false) => CHANNEL_AREA_Q8_V5,
            },
            convergent_area: k >= 3,
        };
        let fields = LevelFields {
            flags: &flags,
            uplift: &uplift,
            erodibility: &erod,
            runoff: None,
            creep: &lowland,
            rock: &rock,
            shape: v6.then_some(incision::ShapeFields {
                soft: &soft,
                relief: &relief_q8,
            }),
        };
        incision::run(&mut g, &fields, &params, &mut scratch)?;
        if k >= 2 {
            let mut chan = std::mem::take(&mut scratch.tmp);
            floodplain::apply(&mut g, &flags, &lowland, &scratch, &mut chan);
        }
        if k + 1 == LEVELS {
            // logic/02 §fine-formation seams: the D8 talus cap leaves
            // one-cell diagonal seams (metres deep) across every hillslope.
            // Binomial passes remove them (two for recipe 5, one for recipe 6,
            // whose pre-erosion roughness needs no second pass); features
            // wider than a few cells, including crests and valleys, keep
            // their shape.
            surface::smooth_seams(&mut g, &flags, if v6 { 1 } else { 2 })?;
        }
        prev = Some(g);
    }
    prev.ok_or(FormationError::ArithmeticOverflow)
}
