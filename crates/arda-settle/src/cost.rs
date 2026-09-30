//! The artifact's road-cost table (artifact "Roads").
//!
//! Costs are metre-equivalents: a 100 m step over flat grass costs 100.
//! Fields, scrub and open woodland cost a little more, dense forest and
//! alpine ground more again, marsh several times, rock and snow many times.
//! Slope multiplies everything, mildly at first and steeply beyond a 30 %
//! grade, measured along the step so roads climb obliquely. Crossing a
//! watercourse costs in proportion to its width (a ten-metre stream is a
//! kilometre of detour, a fifty-metre river four). Existing roads are cheap.

use crate::error::SettleError;
use crate::grid::{filled, Grid};
use crate::landuse::{code, LandUse};
use crate::num::sat_u16;
use crate::rng::hash;
use crate::roads::RoadClass;
use crate::tags::{self, Sites};
use arda::Cover;

/// Per-metre cost on an existing road, per-cent of flat grass.
pub const ROAD_PCT: u64 = 30;
/// Per-metre cost of a ferry passage over open water, per-cent.
pub const FERRY_PCT: u64 = 400;
/// Fixed cost of a ferry landing, metre-equivalents.
pub const FERRY_LANDING: u64 = 40_000;
/// Lowest per-metre cost anywhere, per-cent (the A* heuristic's bound).
pub const MIN_PCT: u64 = ROAD_PCT;

/// Base per-cell costs.
#[derive(Debug, Clone)]
pub struct CostSurface {
    /// Per-cent of flat grass, per cell; 0 on open water.
    pub base: Vec<u16>,
}

/// Cover cost, per-cent of flat grass.
fn cover_pct(g: &Grid, i: usize) -> i64 {
    let alpine = g.temp_cc[i] < 200;
    match g.cover[i] {
        Cover::Bare | Cover::Grass if alpine => 160,
        Cover::Scrub if alpine => 180,
        Cover::Bare | Cover::Grass => 100,
        Cover::Scrub => 120,
        Cover::Forest => 125 + i64::from(g.forest[i]) * 75 / 255,
        Cover::Marsh => 450,
        Cover::Rock => 900,
        Cover::Ice => 1400,
    }
}

impl CostSurface {
    /// Builds the surface from the grid, the site tags and the land use.
    ///
    /// A gentle two-octave hash texture (hedges, strips, holdings) adds up
    /// to three fifths, so routes over open plains wander like real lanes
    /// instead of drawing ruler lines. Saddles are a little cheaper, so
    /// routes prefer passes.
    ///
    /// # Errors
    /// [`SettleError::Reserve`] when the raster cannot be allocated.
    pub fn build(g: &Grid, sites: &Sites, lu: &LandUse, seed: u64) -> Result<Self, SettleError> {
        let mut base = filled(g.len(), 0_u16, "cost surface")?;
        for (i, b) in base.iter_mut().enumerate() {
            if !g.is_land(i) {
                continue;
            }
            let mut c = match lu.codes[i] {
                code::BUILT => 90,
                code::ARABLE | code::FALLOW | code::ORCHARD => 110,
                code::PASTURE | code::MEADOW => 105,
                code::FARMSTEAD => 95,
                _ => cover_pct(g, i),
            };
            let (x, y) = g.xy(i);
            c += texture(seed, x, y) * c / 1000;
            if sites.tags[i] & tags::PASS != 0 {
                c = c * 85 / 100;
            }
            *b = sat_u16(c.max(1));
        }
        Ok(Self { base })
    }
}

/// Smooth hash texture in per-mille: a 1.2 km octave up to 450 and a 400 m
/// octave up to 150, each bilinear on its own lattice.
fn texture(seed: u64, x: i64, y: i64) -> i64 {
    octave(seed, "lanes-wide", x, y, 12, 451) + octave(seed, "lanes", x, y, 4, 151)
}

fn octave(seed: u64, tag: &str, x: i64, y: i64, l: i64, span: u64) -> i64 {
    let (lx, ly) = (x.div_euclid(l), y.div_euclid(l));
    let (fx, fy) = (x.rem_euclid(l), y.rem_euclid(l));
    let v = |a: i64, b: i64| {
        let k = hash(seed, tag, a.cast_unsigned(), b.cast_unsigned());
        i64::try_from(k % span).unwrap_or(0)
    };
    let top = v(lx, ly) * (l - fx) + v(lx + 1, ly) * fx;
    let bot = v(lx, ly + 1) * (l - fx) + v(lx + 1, ly + 1) * fx;
    (top * (l - fy) + bot * fy) / (l * l)
}

/// Slope multiplier, per-cent, for a grade in whole per-cent.
#[must_use]
pub fn slope_pct(grade: i64) -> i64 {
    let mut m = 100 + 3 * grade;
    if grade > 12 {
        m += 2 * (grade - 12).pow(2);
    }
    if grade > 30 {
        m += 20 * (grade - 30).pow(2);
    }
    m
}

/// Extra cost of crossing a watercourse `w` metres wide.
#[must_use]
pub fn crossing_cost(w: u32) -> u64 {
    let w = u64::from(w);
    if w <= 10 {
        100 * w
    } else {
        1000 + 75 * (w - 10)
    }
}

/// Grade a road of each class sustains, per cent (spec §roads): the slope
/// multiplier reads a grade as `g × 10 / max_grade`, so a footpath climbs
/// straight up what a highway must wind across.
#[must_use]
pub const fn max_grade_pct(class: RoadClass) -> i64 {
    match class {
        RoadClass::Highway => 8,
        RoadClass::Road => 10,
        RoadClass::Track => 14,
        RoadClass::Footpath | RoadClass::None => 25,
    }
}

/// Cost of stepping from `a` to its neighbour `b` (offset `d`) for a road
/// of `class`, or `None` when the step is not allowed. `on_road(i)` says
/// whether a road already occupies a cell.
#[must_use]
pub fn step(
    g: &Grid,
    cs: &CostSurface,
    on_road: &impl Fn(usize) -> bool,
    a: usize,
    b: usize,
    d: (i64, i64),
    class: RoadClass,
) -> Option<u64> {
    let (dx, dy) = d;
    let diagonal = dx != 0 && dy != 0;
    let len: u64 = if diagonal { 141 } else { 100 };
    let wet = |i: usize| g.is_water(i) || g.is_watercourse(i);
    if diagonal {
        // Water is crossed orthogonally, never slipped through at a corner.
        let (ax, ay) = g.xy(a);
        let side1 = g.at(ax + dx, ay)?;
        let side2 = g.at(ax, ay + dy)?;
        if wet(a) || wet(b) || wet(side1) || wet(side2) {
            return None;
        }
    }
    let (wa, wb) = (g.is_water(a), g.is_water(b));
    if wa || wb {
        let landing = if wb && !wa { FERRY_LANDING } else { 0 };
        return Some(landing + len * FERRY_PCT / 100);
    }
    let grade = (i64::from(g.height_mm[b]) - i64::from(g.height_mm[a])).abs() * 100
        / (i64::try_from(len).unwrap_or(100) * 1000);
    if on_road(a) && on_road(b) {
        let m = u64::try_from(slope_pct(grade).min(150)).unwrap_or(150);
        return Some(len * ROAD_PCT * m / 10_000);
    }
    let avg = (u64::from(cs.base[a]) + u64::from(cs.base[b])) / 2;
    let felt = grade * 10 / max_grade_pct(class);
    let m = u64::try_from(slope_pct(felt)).unwrap_or(u64::MAX / 4);
    let mut c = len * avg * m / 10_000;
    if g.is_watercourse(b) && !on_road(b) {
        c += crossing_cost(g.width_m(b));
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossing_follows_the_artifact_detours() {
        assert_eq!(crossing_cost(10), 1000);
        assert_eq!(crossing_cost(50), 4000);
        assert!(crossing_cost(1) < 200);
    }

    #[test]
    fn slope_is_mild_then_steep() {
        assert_eq!(slope_pct(0), 100);
        assert!(slope_pct(10) < 140);
        assert!(slope_pct(35) > 8 * slope_pct(5));
    }
}
