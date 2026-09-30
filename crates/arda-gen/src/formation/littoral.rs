//! Littoral landforms built by waves and sediment (logic/02
//! §fine-formation littoral, goals 15-17).
//!
//! Runs on the finest lattice after wave reworking, before the final
//! drainage guarantees, and reads the [`CoastSetting`] of each shore:
//! 1. **Headlands and cliffs**: on exposed coasts backed by high ground,
//!    waves cut convex shores back into a steep face above a shallow
//!    wave-cut platform; headland tips and weak rock retreat furthest, so
//!    lobate interfluves become cliffed headlands between bays.
//! 2. **Bay beaches**: on sediment-rich, low to moderate coasts, narrow
//!    coves and drowned-valley mouths are sealed by beach berms, so bays
//!    end in sand instead of a notch. Mouths of larger rivers stay open as
//!    estuaries.
//! 3. **Barrier islands and spits**: on low, sediment-rich, exposed coasts
//!    with a shallow nearshore, a sand ridge builds 0.9-1.6 km offshore,
//!    broken by tidal inlets and at river mouths. Where the offshore
//!    distance pinches to the shore the ridge attaches as a spit.

use super::coastal::{CoastSetting, Setting};
use super::distance::chamfer_m;
use super::drainage::{open_sea_flags, FIXED};
use super::lattice::{alloc, Lattice};
use super::FormationError;
use crate::noise::value_noise;

/// Wave-cut platform depth at the cliff foot, millimetres below sea level.
const PLATFORM_MM: i32 = 1_500;
/// Width of the cliff face ramp from platform to the old surface, metres.
const FACE_M: i64 = 90;
/// Radius over which shore convexity (the open-sea share) is measured, m.
const CONVEX_RADIUS_M: i64 = 1_500;
/// Headland retreat at full convexity, exposure and weakness, metres.
const RETREAT_SPAN_M: i64 = 2_500;
/// Retreat of the weakest rock on a straight exposed shore, metres.
const RETREAT_WEAK_M: i64 = 1_500;
/// No land further than this from the sea is cut, metres.
const RETREAT_MAX_M: i64 = 2_500;
/// Beach berm height, millimetres.
const BERM_MM: i32 = 800;
/// Barrier crest above sea level, millimetres (edges at 1 m).
const BARRIER_CREST_MM: i64 = 3_500;
/// Closing radius of bay-head beaches, metres.
const CLOSE_M: i64 = 200;
/// Rivers at least this large keep an open mouth (estuary or inlet), km².
const OPEN_MOUTH_KM2: u64 = 20;

/// What the littoral pass built.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Littoral {
    /// Fine cells turned into barrier or spit sand.
    pub barrier_cells: Vec<u32>,
    /// Sea cells sealed by bay-head beach berms.
    pub berm_cells: usize,
    /// Land cells cut back into wave-cut platform below cliffs.
    pub cliff_cut_cells: usize,
}

/// Cliff coast: exposed to waves and backed by high ground.
fn cliff_coast(s: &Setting) -> bool {
    s.hinterland_m >= 15 && s.exposure >= 40
}

/// Sandy coast: enough sediment, not backed by mountains.
fn sandy_coast(s: &Setting) -> bool {
    s.sediment >= 90 && s.hinterland_m <= 80
}

/// Barrier coast: low, sandy, shallow and exposed to waves.
fn barrier_coast(s: &Setting) -> bool {
    s.hinterland_m <= 25 && s.sediment >= 120 && s.exposure >= 60 && s.nearshore_depth_mm <= 15_000
}

/// Protection discs around the mouths of larger rivers.
fn protected_mouths(
    g: &Lattice,
    mouths: &[(usize, u64)],
    radius_m: i64,
) -> Result<Vec<bool>, FormationError> {
    let (w, h) = (g.width as i64, g.height as i64);
    let d_m = (g.spacing_um / 1_000_000).max(1);
    let r = radius_m / d_m;
    let mut mask: Vec<bool> = alloc(g.z.len())?;
    for &(i, km2) in mouths {
        if km2 < OPEN_MOUTH_KM2 {
            continue;
        }
        let (mx, my) = ((i % g.width) as i64, (i / g.width) as i64);
        for oy in -r..=r {
            for ox in -r..=r {
                let (x, y) = (mx + ox, my + oy);
                if x >= 0 && y >= 0 && x < w && y < h && ox * ox + oy * oy <= r * r {
                    mask[(y * w + x) as usize] = true;
                }
            }
        }
    }
    Ok(mask)
}

/// Applies cliffs, bay beaches and barriers. `mouths` are the open-sea
/// river mouths of `g` (fine index, km²); `rock_seed` is the formation
/// seed of the rock-strength field; `volcanoes` are the arc cones
/// (§margins), whose young basalt retreats only the 30 m minimum.
///
/// # Errors
/// Allocation failure.
pub fn apply(
    g: &mut Lattice,
    setting: &CoastSetting,
    mouths: &[(usize, u64)],
    seed: u64,
    rock_seed: u64,
    volcanoes: &[super::margin::Volcano],
) -> Result<Littoral, FormationError> {
    let (w, h) = (g.width, g.height);
    let n = w * h;
    let d_m = (g.spacing_um / 1_000_000).max(1);
    let mut out = Littoral::default();
    let mut open: Vec<u8> = alloc(n)?;
    open_sea_flags(&g.z, w, h, &mut open);
    // 1. Headland retreat and cliffs: waves focus on convex, exposed
    // shores backed by high ground. The shore retreats by up to ~1 km,
    // most at headland tips and in weak rock, leaving a cliff face above a
    // gently sloping wave-cut platform; bays barely retreat.
    {
        let to_sea = chamfer_m(|i| open[i] & FIXED != 0 && g.z[i] <= 0, w, h, d_m)?;
        let sea_frac = {
            let mask: Vec<i32> = (0..n)
                .map(|i| {
                    if open[i] & FIXED != 0 && g.z[i] <= 0 {
                        1_024
                    } else {
                        0
                    }
                })
                .collect();
            let r = usize::try_from(CONVEX_RADIUS_M / d_m).unwrap_or(1).max(1);
            let (mut tmp, mut out) = (alloc(n)?, alloc(n)?);
            super::lattice::blur_into(&mask, w, h, r, &mut tmp, &mut out);
            out
        };
        for i in 0..n {
            if g.z[i] <= 0 || i64::from(to_sea[i]) > RETREAT_MAX_M {
                continue;
            }
            let Some(s) = setting.at_fine(i) else {
                continue;
            };
            if !cliff_coast(&s) {
                continue;
            }
            // Arc cones are young, strong basalt: a small island is convex
            // all round, and the full attack would plane it to a shoal.
            let (xu, yu) = ((i % w) as i64 * g.spacing_um, (i / w) as i64 * g.spacing_um);
            if volcanoes.iter().any(|v| {
                let (dx, dy) = (
                    i128::from((xu - v.x_um) / 1_000_000),
                    i128::from((yu - v.y_um) / 1_000_000),
                );
                dx * dx + dy * dy <= i128::from(v.radius_m) * i128::from(v.radius_m)
            }) {
                if i64::from(to_sea[i]) <= 30 {
                    let t = ((30 - i64::from(to_sea[i])) * 1_024 / FACE_M).min(1_024);
                    let z = i64::from(g.z[i]);
                    let nz = z - (z + i64::from(PLATFORM_MM)) * t / 1_024;
                    g.z[i] = i32::try_from(nz).unwrap_or(g.z[i]);
                    out.cliff_cut_cells += usize::from(nz <= 0);
                }
                continue;
            }
            let convex = (i64::from(sea_frac[i]) - 460).max(0) * 1_024 / (1_024 - 460);
            // Weakness 0 (strong) ..= 1024 (weak): differential retreat cuts
            // coves into weak rock and leaves strong rock as points; convex
            // shores (headlands) are attacked hardest.
            let (xm, ym) = ((i % w) as i64 * d_m, (i / w) as i64 * d_m);
            let rock = super::coastal::rock_strength_q8(rock_seed, xm, ym);
            let weak = i64::from(176_u8.saturating_sub(rock)) * 1_024 / 96;
            // Exposure saturates at 160 (a straight open coast sees ~half the
            // compass as open water).
            let exposure = i64::from(s.exposure).min(160);
            let attack = (RETREAT_WEAK_M * weak / 1_024 * weak / 1_024
                + RETREAT_SPAN_M * convex / 1_024 * (256 + weak) / 1_280)
                * exposure
                / 160;
            let retreat = 30 + attack;
            let d = i64::from(to_sea[i]);
            if d <= retreat {
                // Platform deepens seaward: 1.5 m at the cliff foot plus
                // 15 m per km out. The last [`FACE_M`] rise linearly to the
                // old surface, so the cliff line is a smooth contour rather
                // than a 39 m staircase.
                let depth = i64::from(PLATFORM_MM) + (retreat - d) * 15;
                let t = ((retreat - d) * 1_024 / FACE_M).min(1_024);
                let z = i64::from(g.z[i]);
                let nz = z - (z + depth) * t / 1_024;
                g.z[i] = i32::try_from(nz).unwrap_or(g.z[i]);
                out.cliff_cut_cells += usize::from(nz <= 0);
            }
        }
    }
    open_sea_flags(&g.z, w, h, &mut open);
    let protect = protected_mouths(g, mouths, 600)?;
    // 2. Bay beaches: a morphological closing of the land mask (radius
    // [`CLOSE_M`]) seals coves and drowned-valley mouths narrower than
    // twice that with a berm; straight shores are unchanged.
    {
        let to_land = chamfer_m(|i| g.z[i] > 0, w, h, d_m)?;
        let wide = chamfer_m(|i| i64::from(to_land[i]) > CLOSE_M, w, h, d_m)?;
        for i in 0..n {
            if g.z[i] > 0
                || open[i] & FIXED == 0
                || protect[i]
                || i64::from(to_land[i]) > CLOSE_M
                || i64::from(wide[i]) <= CLOSE_M
            {
                continue;
            }
            if setting.at_fine(i).is_some_and(|s| sandy_coast(&s)) {
                g.z[i] = BERM_MM;
                out.berm_cells += 1;
            }
        }
    }
    // 3. Barrier islands and spits along an offshore contour.
    {
        let to_land = chamfer_m(|i| g.z[i] > 0, w, h, d_m)?;
        // Inlets at every larger river mouth: beyond the ridge's reach.
        let inlet_mask = protected_mouths(g, mouths, 2_000)?;
        for i in 0..n {
            if g.z[i] > 0 || g.z[i] < -10_000 || inlet_mask[i] {
                continue;
            }
            let Some(s) = setting.at_fine(i) else {
                continue;
            };
            if !barrier_coast(&s) {
                continue;
            }
            let (xm, ym) = (((i % w) as i64 * d_m) as i32, ((i / w) as i64 * d_m) as i32);
            // Offshore distance 0.9-1.6 km, varying over ~12 km; half-width
            // 60-100 m; tidal inlets where a 7 km noise dips low.
            let n1 = i64::from(value_noise(seed ^ 0xBA11, xm, ym, 12_000)) + 32_768;
            let n2 = i64::from(value_noise(seed ^ 0xBA12, xm, ym, 3_000)) + 32_768;
            let gap = value_noise(seed ^ 0xBA13, xm, ym, 7_000);
            if gap < -15_000 {
                continue;
            }
            let d0 = 900 + 700 * n1 / 65_536;
            let hw = 60 + 40 * n2 / 65_536;
            let dd = (i64::from(to_land[i]) - d0).abs();
            if dd > hw {
                continue;
            }
            let crest = 1_000 + (BARRIER_CREST_MM - 1_000) * (hw - dd) / hw.max(1);
            g.z[i] = i32::try_from(crest).unwrap_or(BERM_MM);
            out.barrier_cells.push(u32::try_from(i).unwrap_or(u32::MAX));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::super::coastal;
    use super::*;

    /// A low sandy coast facing open sea to the south, with a shallow
    /// nearshore and a large river mouth.
    fn low_coast() -> (Lattice, Vec<(usize, u64)>) {
        let (w, h) = (1_024, 400);
        let mut g = Lattice::new(w, h, 39_062_500).unwrap();
        for y in 0..h {
            for x in 0..w {
                g.z[y * w + x] = if y < 100 {
                    5_000
                } else {
                    -(((y - 100) * 25) as i32).min(8_000) - 500
                };
            }
        }
        let mouths = vec![(99 * w + 500, 2_000)];
        (g, mouths)
    }

    #[test]
    fn low_sandy_exposed_coasts_grow_barriers_with_inlets() {
        let (mut g, mouths) = low_coast();
        let setting = coastal::compute(&g, 5, &mouths).unwrap();
        let out = apply(&mut g, &setting, &mouths, 5, 5, &[]).unwrap();
        assert!(!out.barrier_cells.is_empty(), "no barrier built");
        let w = g.width;
        // Offshore ridge: land ~1-1.6 km (25-41 cells) out, sea inside it.
        let row = |y: usize| (0..w).filter(|&x| g.z[y * w + x] > 0).count();
        assert!((125..145).any(|y| row(y) > 0), "barrier row missing");
        assert_eq!(row(110), 0, "the lagoon stays water");
        // The river mouth keeps its inlet.
        assert!((100..160).all(|y| g.z[y * w + 500] <= 0));
    }

    #[test]
    fn exposed_high_coasts_are_cut_into_cliffs() {
        let (w, h) = (512, 512);
        let mut g = Lattice::new(w, h, 39_062_500).unwrap();
        for y in 0..h {
            for x in 0..w {
                // A 60 m plateau ending in a gentle 1 km ramp to the sea.
                g.z[y * w + x] = if y < 200 {
                    60_000
                } else {
                    60_000 - ((y - 200) as i32) * 2_500
                };
            }
        }
        let setting = coastal::compute(&g, 5, &[]).unwrap();
        let before = g.clone();
        let out = apply(&mut g, &setting, &[], 5, 5, &[]).unwrap();
        assert!(out.cliff_cut_cells > 0);
        // The new shore is steeper: the first land cell stands higher.
        let first = |g: &Lattice| (0..h).rev().find(|&y| g.z[y * w + 256] > 0).unwrap();
        let (a, b) = (first(&before), first(&g));
        assert!(b < a, "shore retreated");
        assert!(g.z[b * w + 256] > before.z[a * w + 256]);
    }

    #[test]
    fn volcanic_arc_islands_are_not_planed_to_shoals() {
        // A 2 km-radius cone summit 600 m above an open sea, 1 km deep.
        // Unprotected, its all-round convex shore retreats (here ~0.4 km,
        // with the short fetch of a 16 km domain). On the full-size seed-42
        // world every arc summit ended 40-80 m deep: the platform formula
        // at 3-4 km of retreat.
        let (w, h) = (400, 400);
        let mut g = Lattice::new(w, h, 39_062_500).unwrap();
        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = (x as i64 - 200, y as i64 - 200);
                let r_m = ((dx * dx + dy * dy) as f64).sqrt() * 39.0625;
                g.z[y * w + x] = (600_000.0 - r_m * 300.0).max(-1_000_000.0) as i32;
            }
        }
        let setting = coastal::compute(&g, 5, &[]).unwrap();
        let cone = super::super::margin::Volcano {
            x_um: 200 * 39_062_500,
            y_um: 200 * 39_062_500,
            radius_m: 9_000,
        };
        let mut bare = g.clone();
        apply(&mut bare, &setting, &[], 5, 5, &[]).unwrap();
        apply(&mut g, &setting, &[], 5, 5, &[cone]).unwrap();
        let land = |g: &Lattice| g.z.iter().filter(|&&z| z > 0).count();
        assert!(
            land(&bare) * 10 < land(&g) * 7,
            "unprotected cone keeps its land: {} vs {}",
            land(&bare),
            land(&g)
        );
        assert!(g.z[200 * w + 200] > 500_000, "summit stands");
        assert!(land(&g) > 7_000, "island kept: {}", land(&g));
    }
}
