//! Recipe-7 climate aridity and the macro water balance (logic/01 §Q6
//! subtropical highs; logic/02 §fine-formation climate runoff).
//!
//! The advected rainfall of [`super::climate`] has no latitude term, so a
//! continent at any latitude is as wet as one at 45°. Recipe 7 scales it by
//! the subsiding branch of the Hadley cell: full rain poleward of 38°, a dry
//! subtropical belt (30%) between 15° and 26°, and a wetter equatorial
//! trough (120% at the equator). Earth's great deserts (Sahara, Arabia,
//! Kalahari, Atacama, the Australian interior) lie in that belt.
//!
//! The annual water balance per 1 km cell uses the same terms as the shared
//! hydrology (logic/02 Steps 4, `annual_aggregation::cell_budget`): `P` is
//! annual rainfall, `E` the sum of twelve monthly Hamon depths, the dry-land
//! loss is `A = min(P/2, E)`, runoff `R = P − A`, and a permanently wet
//! cell costs `D = E − A` more. Formation reads `R` for channel sizing and
//! incision and `D` for the lake balance of endorheic basins, so the bed it
//! shapes and the lakes the shared hydrology later finds agree.

use super::climate::{climate_smoothed_rain, ContinentClimate};
use super::ContinentGrid;
use crate::hydrology::forcing::{profile_year, LatitudeMilli};
use arda_core::{LatitudeBand, RainfallMm, TempCentiC};

/// Rainfall multiplier of the Hadley circulation at a latitude, ‰.
#[must_use]
pub fn subtropical_permille(lat_mdeg: i64) -> i64 {
    let a = lat_mdeg.abs();
    if a >= 38_000 {
        1_000
    } else if a >= 26_000 {
        300 + 700 * (a - 26_000) / 12_000
    } else if a >= 15_000 {
        300
    } else if a >= 5_000 {
        300 + 900 * (15_000 - a) / 10_000
    } else {
        1_200
    }
}

/// Latitude of row `y` of `height` rows, millidegrees (row 0 is north),
/// matching the shared hydrology's forcing rows.
fn row_latitude(band: LatitudeBand, y: i32, height: i32) -> i64 {
    let north = i64::from(band.north_deg) * 1_000;
    let span = i64::from(band.north_deg - band.south_deg) * 1_000;
    north - span * i64::from(y) / i64::from((height - 1).max(1))
}

/// Recipe-7 climate: the recipe-5 smoothed-relief climate with rainfall
/// scaled by [`subtropical_permille`] per row.
#[must_use]
pub fn climate_recipe7(grid: &ContinentGrid, band: LatitudeBand) -> ContinentClimate {
    let mut c = climate_smoothed_rain(grid, band);
    let (w, h) = (grid.width(), grid.height());
    for y in 0..h {
        let f = subtropical_permille(row_latitude(band, y, h));
        for x in 0..w {
            let i = usize::try_from(y * w + x).unwrap_or(0);
            if let Some(p) = c.rainfall.get_mut(i) {
                *p = u16::try_from(i64::from(*p) * f / 1_000).unwrap_or(u16::MAX);
            }
        }
    }
    c
}

/// Annual water balance per 1 km cell, millimetres per year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaterBalance {
    /// Grid width.
    pub width: usize,
    /// Grid height.
    pub height: usize,
    /// Runoff `R = P − min(P/2, E)`.
    pub runoff_mm: Vec<u16>,
    /// Extra loss of standing water `D = E − min(P/2, E)`.
    pub deficit_mm: Vec<u16>,
}

/// One cell's `(runoff, deficit)` in mm/yr from rain `p` and Hamon `e`.
#[must_use]
pub fn budget_mm(p: u32, e: u32) -> (u32, u32) {
    let a = (p / 2).min(e);
    (p - a, e - a)
}

/// The recipe-7 water balance of `grid` at `band`.
#[must_use]
pub fn water_balance(grid: &ContinentGrid, band: LatitudeBand) -> WaterBalance {
    let c = climate_recipe7(grid, band);
    let (w, h) = (grid.width(), grid.height());
    let n = usize::try_from(w * h).unwrap_or(0);
    let (mut runoff_mm, mut deficit_mm) = (vec![0; n], vec![0; n]);
    for y in 0..h {
        let lat = i32::try_from(row_latitude(band, y, h))
            .ok()
            .and_then(|l| LatitudeMilli::new(l).ok());
        for x in 0..w {
            let i = usize::try_from(y * w + x).unwrap_or(0);
            let p = c.rainfall[i];
            // Hamon is defined from −100 °C to 50 °C; the continent stays
            // well inside, and a refused year counts as no evaporation.
            let e_um: u64 = lat
                .and_then(|lat| {
                    profile_year(
                        lat,
                        c.ocean_distance_km[i],
                        RainfallMm::new(p),
                        TempCentiC::new(c.temperature[i]),
                    )
                    .ok()
                })
                .map_or(0, |year| {
                    year.periods
                        .iter()
                        .map(|m| u64::from(m.evaporation_um))
                        .sum()
                });
            let e = u32::try_from(e_um / 1_000).unwrap_or(u32::MAX);
            let (r, d) = budget_mm(u32::from(p), e);
            runoff_mm[i] = u16::try_from(r).unwrap_or(u16::MAX);
            deficit_mm[i] = u16::try_from(d).unwrap_or(u16::MAX);
        }
    }
    WaterBalance {
        width: usize::try_from(w).unwrap_or(0),
        height: usize::try_from(h).unwrap_or(0),
        runoff_mm,
        deficit_mm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_subtropical_belt_is_dry_and_mid_latitudes_unchanged() {
        assert_eq!(subtropical_permille(45_000), 1_000);
        assert_eq!(subtropical_permille(-40_000), 1_000);
        assert_eq!(subtropical_permille(38_000), 1_000);
        assert_eq!(subtropical_permille(20_000), 300);
        assert_eq!(subtropical_permille(-22_000), 300);
        assert_eq!(subtropical_permille(32_000), 650);
        assert_eq!(subtropical_permille(0), 1_200);
        // Continuous at every knot.
        for knot in [5_000, 15_000, 26_000, 38_000] {
            let d = subtropical_permille(knot) - subtropical_permille(knot - 1);
            assert!(d.abs() <= 1, "{knot}: {d}");
        }
    }

    #[test]
    fn the_budget_matches_the_shared_hydrology_terms() {
        // Humid: runoff is half the rain, a lake costs little.
        assert_eq!(budget_mm(1_200, 700), (600, 100));
        // Arid: the same half-rain runoff, but a lake costs E − P/2.
        assert_eq!(budget_mm(300, 1_100), (150, 950));
        // Evaporation below half the rain: all of E is the land loss.
        assert_eq!(budget_mm(2_000, 600), (1_400, 0));
    }

    #[test]
    fn a_subtropical_continent_is_drier_than_a_temperate_one() {
        let (w, h) = (40, 40);
        let heights = (0..w * h)
            .map(|i| if i % w < 3 { -1_000_000 } else { 200_000 })
            .collect();
        let grid = ContinentGrid::from_heights(w, h, heights).unwrap();
        let temperate = water_balance(&grid, LatitudeBand::new(40, 50));
        let arid = water_balance(&grid, LatitudeBand::new(18, 24));
        let at = |b: &WaterBalance, v: fn(&WaterBalance) -> &Vec<u16>| v(b)[20 * 40 + 30];
        assert!(at(&arid, |b| &b.runoff_mm) < at(&temperate, |b| &b.runoff_mm));
        assert!(at(&arid, |b| &b.deficit_mm) > at(&temperate, |b| &b.deficit_mm));
        assert!(at(&arid, |b| &b.deficit_mm) > 2 * at(&arid, |b| &b.runoff_mm));
    }
}
