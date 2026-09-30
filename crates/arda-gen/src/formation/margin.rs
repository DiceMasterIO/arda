//! Tectonic margins on the 1 km macro lattice (logic/02 §fine-formation
//! margins, goals 17 and 18).
//!
//! Reads the replayed plate history ([`crate::continent::margins`]) and
//! derives two things before any formation level runs:
//! - **margin activity** per macro cell (0 passive .. 255 active), from
//!   convergent boundaries nearby; the shelf profile narrows with it;
//! - **volcanic island arcs**: stratovolcano cones on open ocean along the
//!   final-step subduction axes, on the overriding plate. Formation then
//!   erodes them like any other high ground, so they gain drainage, cliffs
//!   and narrow shelves of their own.

use crate::continent::margins::MarginContext;
use crate::noise::hash_2d;

use super::incision::isqrt;
use super::lattice::Lattice;
use super::FormationError;

/// Minimum spacing of arc volcanoes, metres.
pub const VOLCANO_SPACING_M: i64 = 18_000;
/// Arc volcanoes rise only where the margin profile is at least this deep
/// (beyond the shelf and the upper slope), mm: open, oceanic crust.
const VOLCANO_MIN_DEPTH_MM: i32 = 1_000_000;
/// Keep-out from the domain edge, metres (the rim is forced ocean).
const VOLCANO_EDGE_M: i64 = 20_000;
/// Summit height above sea level, metres: 300 + up to 1100.
const SUMMIT_BASE_M: i64 = 300;
const SUMMIT_SPAN_M: i64 = 1_100;
/// Cone base radius, metres: 9 km + up to 6 km.
const CONE_BASE_M: i64 = 9_000;
const CONE_SPAN_M: i64 = 6_000;

/// One volcanic cone raised on the macro lattice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Volcano {
    /// Summit position, micrometres.
    pub x_um: i64,
    /// Summit position, micrometres.
    pub y_um: i64,
    /// Base radius, metres.
    pub radius_m: i64,
}

/// Tectonic activity per macro cell, 0 (passive) ..= 255 (active margin):
/// the strongest nearby convergence of any kind.
#[must_use]
pub fn activity(ctx: &MarginContext, w: usize, h: usize, spacing_m: i64) -> Vec<u8> {
    (0..w * h)
        .map(|i| {
            let s = ctx.sample_m((i % w) as i64 * spacing_m, (i / w) as i64 * spacing_m);
            s.arc.max(s.island_arc).max(s.collision)
        })
        .collect()
}

/// Raises stratovolcano cones along the final-step volcanic axes where the
/// macro surface and its margin profile are deep open ocean (goal 17: volcanic island arcs only where the
/// tectonics support them). Marks their flanks as high relief so formation
/// shapes them as mountains. Returns the cones built.
///
/// # Errors
/// Arithmetic overflow on degenerate lattices.
pub fn volcanic_arcs(
    seed: u64,
    ctx: &MarginContext,
    shelf_target: &Lattice,
    macro_mm: &mut Lattice,
    relief_m: &mut Lattice,
) -> Result<Vec<Volcano>, FormationError> {
    let (w, h) = (macro_mm.width, macro_mm.height);
    let d_m = (macro_mm.spacing_um / 1_000_000).max(1);
    let (ext_x, ext_y) = ((w as i64 - 1) * d_m, (h as i64 - 1) * d_m);
    let mut candidates: Vec<(u32, i64, i64)> = ctx
        .volcanic_axis_m()
        .into_iter()
        .filter(|&(x, y)| {
            x >= VOLCANO_EDGE_M
                && y >= VOLCANO_EDGE_M
                && x <= ext_x - VOLCANO_EDGE_M
                && y <= ext_y - VOLCANO_EDGE_M
        })
        .filter(|&(x, y)| {
            let i = (y / d_m) as usize * w + (x / d_m) as usize;
            macro_mm
                .z
                .get(i)
                .is_some_and(|&z| z <= -VOLCANO_MIN_DEPTH_MM)
                && shelf_target
                    .z
                    .get(i)
                    .is_some_and(|&t| t <= -VOLCANO_MIN_DEPTH_MM)
        })
        .map(|(x, y)| {
            let hx = i32::try_from(x / 1_000).unwrap_or(0);
            let hy = i32::try_from(y / 1_000).unwrap_or(0);
            (hash_2d(seed ^ 0x7011_CA00, hx, hy), x, y)
        })
        .collect();
    // Strongest hash first, ties by position: a deterministic Poisson-disc
    // thinning along the arc. About a third of axis cells are skipped
    // outright so arcs have gaps, as real ones do.
    candidates.sort_unstable_by(|a, b| b.cmp(a));
    let mut cones: Vec<Volcano> = Vec::new();
    for (hv, x, y) in candidates {
        if hv < u32::MAX / 3 {
            continue;
        }
        let spaced = cones.iter().all(|c| {
            let (dx, dy) = (c.x_um / 1_000_000 - x, c.y_um / 1_000_000 - y);
            dx * dx + dy * dy >= VOLCANO_SPACING_M * VOLCANO_SPACING_M
        });
        if !spaced {
            continue;
        }
        let summit_m = SUMMIT_BASE_M + i64::from(hv % 1_024) * SUMMIT_SPAN_M / 1_024;
        let radius_m = CONE_BASE_M + i64::from((hv >> 10) % 1_024) * CONE_SPAN_M / 1_024;
        stamp_cone(macro_mm, relief_m, x, y, summit_m * 1_000, radius_m)?;
        cones.push(Volcano {
            x_um: x * 1_000_000,
            y_um: y * 1_000_000,
            radius_m,
        });
    }
    Ok(cones)
}

/// A volcanic edifice over the local floor, never lowering the surface: a
/// straight subaerial cone from the summit to the shore at 0.35 R, and
/// submarine flanks `floor × (1 − (1 − s)²)` over the rest of the radius
/// (steep below the shore, flattening onto the floor).
fn stamp_cone(
    macro_mm: &mut Lattice,
    relief_m: &mut Lattice,
    x_m: i64,
    y_m: i64,
    summit_mm: i64,
    radius_m: i64,
) -> Result<(), FormationError> {
    let (w, h) = (macro_mm.width as i64, macro_mm.height as i64);
    let d_m = (macro_mm.spacing_um / 1_000_000).max(1);
    let (cx, cy) = (x_m / d_m, y_m / d_m);
    let floor = i64::from(macro_mm.z[usize::try_from(cy * w + cx).unwrap_or(0)]);
    let r_cells = radius_m / d_m + 1;
    for oy in -r_cells..=r_cells {
        for ox in -r_cells..=r_cells {
            let (x, y) = (cx + ox, cy + oy);
            if x < 1 || y < 1 || x >= w - 1 || y >= h - 1 {
                continue;
            }
            let r_m = i64::try_from(isqrt(
                u64::try_from((ox * ox + oy * oy) * d_m * d_m)
                    .map_err(|_| FormationError::ArithmeticOverflow)?,
            ))
            .map_err(|_| FormationError::ArithmeticOverflow)?;
            if r_m >= radius_m {
                continue;
            }
            // The subaerial cone rises straight from the shore at 35% of the
            // base radius; the submarine flanks fall steeply from it and
            // flatten onto the floor. A concave (1 - r/R)² cone from a
            // 1-4 km deep floor stood above sea within only 0.4 km of its
            // summit, and formation's envelope averaged every full-size
            // seed-42 summit under water.
            let isle_m = radius_m * 35 / 100;
            let z = if r_m < isle_m {
                summit_mm * (isle_m - r_m) / isle_m.max(1)
            } else {
                let s = (r_m - isle_m) * 4_096 / (radius_m - isle_m).max(1);
                let u = 4_096 - s;
                floor.min(0) * (4_096 * 4_096 - u * u) / (4_096 * 4_096)
            };
            let t = (radius_m - r_m) * 4_096 / radius_m;
            let i = usize::try_from(y * w + x).map_err(|_| FormationError::ArithmeticOverflow)?;
            let zi = i32::try_from(z).map_err(|_| FormationError::ArithmeticOverflow)?;
            macro_mm.z[i] = macro_mm.z[i].max(zi);
            // Volcanic edifices are high-relief ground: steep, talus-capped.
            let rel = i32::try_from(400 * t / 4_096).unwrap_or(400);
            relief_m.z[i] = relief_m.z[i].max(rel);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cone_rises_above_sea_from_the_abyss_and_marks_relief() {
        let (w, h) = (60, 60);
        let mut m = Lattice::new(w, h, 1_000_000_000).unwrap();
        m.z.fill(-2_000_000);
        let mut r = Lattice::new(w, h, 1_000_000_000).unwrap();
        stamp_cone(&mut m, &mut r, 30_000, 30_000, 900_000, 12_000).unwrap();
        assert!(m.z[30 * w + 30] > 800_000, "summit {}", m.z[30 * w + 30]);
        assert!(m.z[30 * w + 36] < 0, "flank 6 km out is under water");
        assert_eq!(m.z[30 * w + 50], -2_000_000, "abyss beyond the cone");
        assert!(r.z[30 * w + 30] >= 390);
    }

    #[test]
    fn an_arc_cone_stands_above_sea_over_kilometres_not_a_point() {
        // Regression (seed-42 full size: every arc summit ended under
        // water): from a 4 km deep floor, a 400 m cone of 12 km base must
        // stand above sea for 3 km around its summit.
        let (w, h) = (60, 60);
        let mut m = Lattice::new(w, h, 1_000_000_000).unwrap();
        m.z.fill(-4_000_000);
        let mut r = Lattice::new(w, h, 1_000_000_000).unwrap();
        stamp_cone(&mut m, &mut r, 30_000, 30_000, 400_000, 12_000).unwrap();
        for off in 0..=3 {
            let z = m.z[30 * w + 30 + off];
            assert!(z > 0, "{off} km out: {z}");
        }
        assert!(m.z[30 * w + 38] < -1_000_000, "steep submarine flank");
    }
}
