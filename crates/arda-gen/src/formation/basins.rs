//! Basin and plateau audit on the 1 km macro lattice (logic/02
//! §fine-formation basin audit, goal 6).
//!
//! Every large closed depression and every high, low-relief plateau of the
//! macro surface must have a tectonic reason, read from the replayed plate
//! margins ([`crate::continent::margins`]):
//! - a basin within a **rift** belt is a rift basin;
//! - next to a **collision** belt, a foreland or intermontane basin;
//! - next to an **ocean-continent or island arc**, a fore-arc or back-arc
//!   basin.
//!
//! A plateau (above [`PLATEAU_MIN_MM`], local relief under
//! [`PLATEAU_RELIEF_M`]) is orogenic next to a collision belt, an arc
//! plateau next to an arc, a rift shoulder beside a rift.
//!
//! A basin without any of these causes is an artefact of the macro
//! surface, so it is filled to its spill level (with a gentle gradient to
//! the outlet) before formation: it then forms as ordinary drained terrain
//! and never becomes an endorheic sink.

use arda_core::{Landform, LandformCause, LandformKind};

use crate::continent::margins::{MarginContext, MarginSample};

use super::drainage::{fill, neighbour, FIXED};
use super::lattice::{alloc, Lattice};
use super::FormationError;

/// Basins at least this deep below their spill, millimetres (§basins).
pub const BASIN_DEPTH_MM: i32 = super::BASIN_DEPTH_MM;
/// ...and at least this large, km².
pub const BASIN_MIN_KM2: usize = 400;
/// Plateau floor, millimetres.
pub const PLATEAU_MIN_MM: i32 = 800_000;
/// Plateau local relief ceiling (25 km standard deviation), metres.
pub const PLATEAU_RELIEF_M: i32 = 150;
/// Margin proximity (0..=255) that counts as a tectonic cause.
const CAUSE_MIN: u8 = 48;
/// Gradient toward the outlet of a filled, unexplained basin, mm per cell.
const FILL_EPS_MM: i32 = 30;

/// The strongest boundary proximity over a component, per kind.
fn strongest(ctx: &MarginContext, cells: &[usize], w: usize, d_m: i64) -> MarginSample {
    let mut m = MarginSample::default();
    for &c in cells {
        let s = ctx.sample_m((c % w) as i64 * d_m, (c / w) as i64 * d_m);
        m.collision = m.collision.max(s.collision);
        m.arc = m.arc.max(s.arc).max(s.island_arc);
        m.rift = m.rift.max(s.rift);
    }
    m
}

/// Tectonic cause of a basin from the margins around it, if any.
#[must_use]
pub fn basin_cause(m: &MarginSample) -> Option<LandformCause> {
    if m.rift >= CAUSE_MIN {
        Some(LandformCause::Rift)
    } else if m.collision >= CAUSE_MIN {
        Some(LandformCause::Foreland)
    } else if m.arc.max(m.island_arc) >= CAUSE_MIN {
        Some(LandformCause::Arc)
    } else {
        None
    }
}

/// Tectonic cause of a plateau, if any.
#[must_use]
pub fn plateau_cause(m: &MarginSample) -> Option<LandformCause> {
    if m.collision >= CAUSE_MIN {
        Some(LandformCause::Orogenic)
    } else if m.arc.max(m.island_arc) >= CAUSE_MIN {
        Some(LandformCause::Arc)
    } else if m.rift >= CAUSE_MIN {
        Some(LandformCause::RiftShoulder)
    } else {
        None
    }
}

/// Connected components (8-neighbour) of cells where `inside` holds.
fn components(n: usize, w: usize, h: usize, inside: impl Fn(usize) -> bool) -> Vec<Vec<usize>> {
    let mut seen = vec![false; n];
    let mut out = Vec::new();
    let mut stack = Vec::new();
    for s0 in 0..n {
        if seen[s0] || !inside(s0) {
            continue;
        }
        let mut comp = Vec::new();
        seen[s0] = true;
        stack.push(s0);
        while let Some(c) = stack.pop() {
            comp.push(c);
            for k in 0..8 {
                if let Some(nb) = neighbour(c, w, h, k) {
                    if !seen[nb] && inside(nb) {
                        seen[nb] = true;
                        stack.push(nb);
                    }
                }
            }
        }
        out.push(comp);
    }
    out
}

fn record(
    comp: &[usize],
    w: usize,
    d_m: i64,
    kind: LandformKind,
    cause: LandformCause,
    relief_m: i64,
) -> Landform {
    let n = comp.len().max(1) as i64;
    let sx: i64 = comp.iter().map(|&c| (c % w) as i64).sum();
    let sy: i64 = comp.iter().map(|&c| (c / w) as i64).sum();
    Landform {
        x_um: sx * d_m * 1_000_000 / n,
        y_um: sy * d_m * 1_000_000 / n,
        area_km2: u32::try_from(comp.len() as i64 * d_m * d_m / 1_000_000).unwrap_or(u32::MAX),
        relief_m: i16::try_from(relief_m).unwrap_or(i16::MAX),
        kind,
        cause,
    }
}

/// Audits basins and plateaus of `macro_mm`, filling unexplained basins in
/// place. `relief_m` is the macro local relief (m). `lowstand_mm` is the
/// formation base level below which ocean is fixed. Returns the explained
/// landforms and the number of basins filled.
///
/// # Errors
/// Allocation failure.
pub fn audit(
    macro_mm: &mut Lattice,
    relief_m: &Lattice,
    ctx: &MarginContext,
    lowstand_mm: i32,
) -> Result<(Vec<Landform>, usize), FormationError> {
    let (w, h) = (macro_mm.width, macro_mm.height);
    let n = w * h;
    let d_m = (macro_mm.spacing_um / 1_000_000).max(1);
    let min_cells = usize::try_from(BASIN_MIN_KM2 as i64 * 1_000_000 / (d_m * d_m))
        .unwrap_or(usize::MAX)
        .max(1);
    // Only the lowstand ocean and the rim are base level: a closed
    // depression deeper than the lowstand is still a basin.
    let mut flags = lowstand_ocean(&macro_mm.z, w, h, lowstand_mm)?;
    for (i, f) in flags.iter_mut().enumerate() {
        let (x, y) = (i % w, i / w);
        if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
            *f = FIXED;
        }
    }
    let (mut next, mut closed): (Vec<u32>, Vec<u8>) = (alloc(n)?, alloc(n)?);
    let mut level = macro_mm.z.clone();
    fill(&mut level, w, h, &flags, 0, &mut next, &mut closed)?;
    let mut graded = macro_mm.z.clone();
    fill(
        &mut graded,
        w,
        h,
        &flags,
        FILL_EPS_MM,
        &mut next,
        &mut closed,
    )?;
    let mut landforms = Vec::new();
    let mut filled = 0;
    let deep = |i: usize| level[i] - macro_mm.z[i] >= BASIN_DEPTH_MM;
    for comp in components(n, w, h, deep) {
        if comp.len() < min_cells {
            continue;
        }
        let depth = comp
            .iter()
            .map(|&c| i64::from(level[c] - macro_mm.z[c]))
            .max()
            .unwrap_or(0);
        match basin_cause(&strongest(ctx, &comp, w, d_m)) {
            Some(cause) => landforms.push(record(
                &comp,
                w,
                d_m,
                LandformKind::Basin,
                cause,
                depth / 1_000,
            )),
            None => {
                // Fill the whole closed depression, not only its deep core.
                for (c, (z, &g)) in macro_mm.z.iter_mut().zip(&graded).enumerate() {
                    if g > *z && reaches(&comp, c, &level) {
                        *z = g;
                    }
                }
                filled += 1;
            }
        }
    }
    let high = |i: usize| macro_mm.z[i] >= PLATEAU_MIN_MM && relief_m.z[i] <= PLATEAU_RELIEF_M;
    for comp in components(n, w, h, high) {
        if comp.len() < min_cells {
            continue;
        }
        let mean =
            comp.iter().map(|&c| i64::from(macro_mm.z[c])).sum::<i64>() / comp.len() as i64 / 1_000;
        let cause =
            plateau_cause(&strongest(ctx, &comp, w, d_m)).unwrap_or(LandformCause::Unexplained);
        landforms.push(record(&comp, w, d_m, LandformKind::Plateau, cause, mean));
    }
    Ok((landforms, filled))
}

/// The lowstand ocean: `FIXED` on cells at or below `-lowstand_mm` that
/// connect (8-neighbour) to the domain rim through such cells. A closed
/// macro depression deeper than the lowstand is not ocean: formation fixed
/// it as sea floor, and the final fills then lifted it 150-200 m to its
/// spill as one planar plain (seed-42 full size: a 60 km plain at 63 m;
/// 250 x 500 km: 52 m), whose rivers ran along grid geodesics.
///
/// # Errors
/// Allocation failure.
pub fn lowstand_ocean(
    z: &[i32],
    w: usize,
    h: usize,
    lowstand_mm: i32,
) -> Result<Vec<u8>, FormationError> {
    let mut ocean: Vec<u8> = alloc(w * h)?;
    let deep = |i: usize| z[i] <= -lowstand_mm;
    let mut stack = Vec::new();
    for (i, o) in ocean.iter_mut().enumerate() {
        let (x, y) = (i % w, i / w);
        if (x == 0 || y == 0 || x == w - 1 || y == h - 1) && deep(i) {
            *o = FIXED;
            stack.push(i);
        }
    }
    while let Some(c) = stack.pop() {
        for k in 0..8 {
            if let Some(nb) = neighbour(c, w, h, k) {
                if ocean[nb] == 0 && deep(nb) {
                    ocean[nb] = FIXED;
                    stack.push(nb);
                }
            }
        }
    }
    Ok(ocean)
}

/// Whether cell `c` lies in the same filled lake as component `comp`: its
/// filled level equals the component's (one spill level per depression).
fn reaches(comp: &[usize], c: usize, level: &[i32]) -> bool {
    comp.first().is_some_and(|&k| level[k] == level[c])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continent::margins::margin_context_formed;

    #[test]
    fn causes_follow_the_nearest_active_margin() {
        let m = |collision, arc, rift| MarginSample {
            collision,
            arc,
            island_arc: 0,
            rift,
        };
        assert_eq!(basin_cause(&m(0, 0, 200)), Some(LandformCause::Rift));
        assert_eq!(basin_cause(&m(200, 0, 0)), Some(LandformCause::Foreland));
        assert_eq!(basin_cause(&m(0, 90, 0)), Some(LandformCause::Arc));
        assert_eq!(basin_cause(&m(10, 10, 10)), None);
        assert_eq!(plateau_cause(&m(200, 0, 0)), Some(LandformCause::Orogenic));
        assert_eq!(plateau_cause(&m(0, 0, 0)), None);
    }

    #[test]
    fn an_unexplained_basin_is_filled_and_an_explained_one_kept() {
        // A 60 x 60 km upland with a 30 km, 300 m deep bowl in the middle.
        let (w, h) = (60, 60);
        let mut m = Lattice::new(w, h, 1_000_000_000).unwrap();
        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = (x as i64 - 30, y as i64 - 30);
                let r2 = dx * dx + dy * dy;
                m.z[y * w + x] = if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                    -500_000
                } else if r2 < 15 * 15 {
                    100_000
                } else {
                    400_000
                };
            }
        }
        let relief = Lattice::new(w, h, 1_000_000_000).unwrap();
        // Seed 7 on a 60 km domain: find whichever outcome the plate replay
        // gives, then check the consequence matches it.
        let ctx = margin_context_formed(7, 60, 60, 0);
        let before = m.clone();
        let (forms, filled) = audit(&mut m, &relief, &ctx, 120_000).unwrap();
        let basins = forms
            .iter()
            .filter(|f| f.kind == LandformKind::Basin)
            .count();
        assert_eq!(basins + filled, 1, "the bowl is either explained or filled");
        let centre = 30 * w + 30;
        if filled == 1 {
            assert!(
                m.z[centre] >= 400_000,
                "filled to its spill: {}",
                m.z[centre]
            );
        } else {
            assert_eq!(m.z[centre], before.z[centre], "an explained basin is kept");
        }
    }
}
