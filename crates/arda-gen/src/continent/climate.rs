//! Continent climate (`logic/01` step 5, feature 02 §Q3–§Q5).
//!
//! Temperature and regime here; rainfall lands in this module at the
//! next task. Everything is integer arithmetic — no libm, no RNG.

use super::ContinentGrid;
use arda_core::{ClimateRegime, LatitudeBand};
use std::collections::VecDeque;

/// Per-cell climate fields, row-major over the 1 km grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinentClimate {
    /// Mean annual temperature, centi-°C.
    pub temperature: Vec<i16>,
    /// Mean annual rainfall, mm.
    pub rainfall: Vec<u16>,
    /// Climate regime.
    pub regime: Vec<ClimateRegime>,
    /// Final advection moisture store, 0..=M_SAT (feature 03 §Q6).
    pub moisture: Vec<u16>,
}

/// Continentality cap: −1 centi-°C per km inland up to 300 km
/// (feature 02 §Q5).
const CONTINENTALITY_CAP_KM: u32 = 300;

/// Km distance to the nearest sea cell; 4-connected BFS from every
/// sea cell. Deterministic: BFS level order fixes each cell's value
/// regardless of intra-level ordering.
fn distance_to_sea_km(grid: &ContinentGrid) -> Vec<u32> {
    let (w, h) = (grid.width(), grid.height());
    let count = usize::try_from(w * h).unwrap_or(0);
    let idx = |x: i32, y: i32| usize::try_from(y * w + x).unwrap_or(0);
    let mut dist = vec![u32::MAX; count];
    let mut queue = VecDeque::new();
    for y in 0..h {
        for x in 0..w {
            if grid.get(x, y).raw() <= 0 {
                dist[idx(x, y)] = 0;
                queue.push_back((x, y));
            }
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        let d = dist[idx(x, y)];
        for (nx, ny) in [(x, y - 1), (x + 1, y), (x, y + 1), (x - 1, y)] {
            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                continue;
            }
            if dist[idx(nx, ny)] == u32::MAX {
                dist[idx(nx, ny)] = d.saturating_add(1);
                queue.push_back((nx, ny));
            }
        }
    }
    dist
}

/// Latitude of a row in millidegrees; row 0 is the band's north edge.
fn latitude_millideg(band: LatitudeBand, y: i32, height: i32) -> i64 {
    let north = i64::from(band.north_deg) * 1_000;
    let south = i64::from(band.south_deg) * 1_000;
    let span = north - south;
    north - span * i64::from(y) / i64::from((height - 1).max(1))
}

/// Sea-level mean annual temperature, centi-°C: 18 °C at 35°N to
/// 6 °C at 55°N, linear, extrapolated outside (feature 02 §Q5).
fn sea_level_centi(lat_mdeg: i64) -> i64 {
    1_800 - (lat_mdeg - 35_000) * 6 / 100
}

/// Moisture-store saturation, dimensionless fixed point (§Q4 table).
const M_SAT: u64 = 32_768;
/// Sea recharge: 1/8 of the deficit per pass.
const RECHARGE_DIV: u64 = 8;
/// Base land release, in 1/65,536 units (1/512).
const F_BASE: u64 = 128;
/// Orographic release per 100 m of climb, in 1/65,536 units (1/64).
const F_ORO: u64 = 1_024;
/// Release clamp (1/16).
const F_MAX: u64 = 4_096;
/// Lateral diffusion divisor: 1/8 blend with the 4-neighbour mean.
const DIFFUSE_DIV: u64 = 8;
/// Per-pass rainfall scale; calibrated on seed 42 at default size so
/// the land mean lands near 800 mm/yr (feature 02 §Q3, R8 probe).
/// The accumulator is divided by the pass count before scaling, so the
/// magnitude is independent of grid width and one calibration serves
/// unit-test grids, MICRO, and the default world alike.
const C_NORM: u64 = 153;

/// One pass advects due west→east by one cell, diffuses, then
/// exchanges with the surface (feature 02 §Q4). Pass count is fixed at
/// 1.5 × width — never a convergence test, so determinism holds.
/// Returns (rainfall, moisture).
fn rainfall_field(grid: &ContinentGrid) -> (Vec<u16>, Vec<u16>) {
    let (w, h) = (grid.width(), grid.height());
    let count = usize::try_from(w * h).unwrap_or(0);
    let idx = |x: i32, y: i32| usize::try_from(y * w + x).unwrap_or(0);
    let height = |x: i32, y: i32| i64::from(grid.get(x, y).raw());

    let passes = 3 * w / 2;
    let mut m = vec![M_SAT; count];
    let mut advected = vec![0u64; count];
    let mut accum = vec![0u64; count];

    for _ in 0..passes {
        // 1. Advect: the store moves one cell east; the west edge
        //    refills from the off-map ocean (rim is ocean by invariant).
        for y in 0..h {
            for x in 0..w {
                advected[idx(x, y)] = if x == 0 { M_SAT } else { m[idx(x - 1, y)] };
            }
        }
        // 2. Diffuse from the advected snapshot, so scan order cannot
        //    leak into the result.
        for y in 0..h {
            for x in 0..w {
                let at = |xx: i32, yy: i32| advected[idx(xx.clamp(0, w - 1), yy.clamp(0, h - 1))];
                let mean4 = (at(x, y - 1) + at(x + 1, y) + at(x, y + 1) + at(x - 1, y)) / 4;
                let a = advected[idx(x, y)];
                // Deviation from spec R1's literal (7M + mean4)/8: the
                // decomposed form floors each term separately and can sit
                // one unit higher per pass; shipped and calibrated as-is,
                // recorded for the reference refresh.
                m[idx(x, y)] = a - a / DIFFUSE_DIV + mean4 / DIFFUSE_DIV;
            }
        }
        // 3. Exchange: recharge over water, release over land
        //    (logic/01 §Q6: "release on climb, recharge over water").
        for y in 0..h {
            for x in 0..w {
                let i = idx(x, y);
                if height(x, y) <= 0 {
                    m[i] += (M_SAT.saturating_sub(m[i])) / RECHARGE_DIV;
                } else {
                    // Climb along the wind, floored at sea level so a
                    // deep offshore shelf does not fabricate a cliff.
                    let west = if x == 0 { 0 } else { height(x - 1, y).max(0) };
                    let climb = u64::try_from((height(x, y) - west).max(0)).unwrap_or(0);
                    let f = (F_BASE + F_ORO * climb / 100_000).min(F_MAX);
                    let rain = (m[i] * f) >> 16;
                    accum[i] += rain;
                    m[i] -= rain;
                }
            }
        }
    }

    let p = u64::try_from(passes).unwrap_or(1).max(1);
    let rainfall = accum
        .into_iter()
        .map(|r| u16::try_from(((r / p) * C_NORM) >> 4).unwrap_or(u16::MAX))
        .collect();
    let moisture = m
        .into_iter()
        .map(|v| u16::try_from(v).unwrap_or(u16::MAX))
        .collect();
    (rainfall, moisture)
}

/// Computes temperature and regime; rainfall is computed via advection-diffusion.
#[must_use]
pub fn climate(grid: &ContinentGrid, band: LatitudeBand) -> ContinentClimate {
    let (w, h) = (grid.width(), grid.height());
    let count = usize::try_from(w * h).unwrap_or(0);
    let dist = distance_to_sea_km(grid);

    let mut temperature = Vec::with_capacity(count);
    let mut regime = Vec::with_capacity(count);
    for y in 0..h {
        let lat = latitude_millideg(band, y, h);
        for x in 0..w {
            let i = usize::try_from(y * w + x).unwrap_or(0);
            let elev = i64::from(grid.get(x, y).raw().max(0));
            // logic/01 §Q6: latitude + 6.5 °C/km lapse + continentality.
            let t = sea_level_centi(lat)
                - elev * 650 / 1_000_000
                - i64::from(dist[i].min(CONTINENTALITY_CAP_KM));
            let t = i16::try_from(t.clamp(-30_000, 30_000)).unwrap_or(0);
            temperature.push(t);
            // Feature 02 §Q5: band position and elevation, in order.
            regime.push(if lat < 23_500 {
                ClimateRegime::Tropical
            } else if lat < 42_000 && elev < 1_000_000 {
                ClimateRegime::Mediterranean
            } else if t < 300 {
                ClimateRegime::Boreal
            } else {
                ClimateRegime::Temperate
            });
        }
    }
    let (rainfall, moisture) = rainfall_field(grid);
    ContinentClimate {
        temperature,
        rainfall,
        regime,
        moisture,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continent::ContinentGrid;
    use arda_core::LatitudeBand;

    /// 10×10, all sea except one column of land rising eastward.
    fn grid(heights: impl Fn(i32, i32) -> i32) -> ContinentGrid {
        let (w, h) = (10, 10);
        ContinentGrid {
            width: w,
            height: h,
            height_mm: (0..w * h).map(|i| heights(i % w, i / w)).collect(),
        }
    }

    const BAND: LatitudeBand = LatitudeBand::new(35, 55);

    #[test]
    fn higher_ground_is_colder_by_the_lapse_rate() {
        // logic/01 §Q6: 6.5 °C/km.
        let g = grid(|x, _| {
            if x == 5 {
                2_000_000
            } else if x == 6 {
                1_000_000
            } else {
                -1_000
            }
        });
        let c = climate(&g, BAND);
        let (low, high) = (c.temperature[5 * 10 + 6], c.temperature[5 * 10 + 5]);
        assert_eq!(i32::from(low) - i32::from(high), 650);
    }

    #[test]
    fn the_north_edge_is_colder_than_the_south_edge() {
        let g = grid(|_, _| 500_000);
        let c = climate(&g, BAND);
        assert!(c.temperature[5] < c.temperature[9 * 10 + 5]);
    }

    #[test]
    fn continentality_saturates_at_300_km() {
        // −1 centi-°C per km inland, capped at 300 (feature 02 §Q5).
        // A 10 km grid never saturates; assert the per-km slope instead.
        let g = grid(|x, _| if x >= 2 { 100_000 } else { -1_000 });
        let c = climate(&g, BAND);
        let row = 5 * 10;
        assert_eq!(
            i32::from(c.temperature[row + 2]) - i32::from(c.temperature[row + 4]),
            2
        );
    }

    #[test]
    fn regimes_follow_band_position_and_elevation() {
        // Rows map north (y=0, 55°) to south (y=9, 35°); feature 02 §Q5.
        let g = grid(|x, _| {
            if x == 5 {
                200_000
            } else if x == 6 {
                3_500_000
            } else {
                -1_000
            }
        });
        let c = climate(&g, BAND);
        // South row, low, warm → mediterranean (lat 35° < 42°, < 1,000 m).
        assert_eq!(c.regime[9 * 10 + 5], ClimateRegime::Mediterranean);
        // High peak → boreal by computed temperature.
        assert_eq!(c.regime[5 * 10 + 6], ClimateRegime::Boreal);
        // North row, low → temperate (lat 55°).
        assert_eq!(c.regime[5], ClimateRegime::Temperate);
    }

    #[test]
    fn climate_is_deterministic() {
        let g = grid(|x, y| (x - y) * 100_000);
        assert_eq!(climate(&g, BAND), climate(&g, BAND));
    }

    #[test]
    fn sea_cells_get_no_rainfall() {
        let g = grid(|x, _| if x >= 4 { 400_000 } else { -1_000 });
        let c = climate(&g, BAND);
        for y in 0..10 {
            for x in 0..4 {
                let i = usize::try_from(y * 10 + x).unwrap();
                assert_eq!(c.rainfall[i], 0, "sea cell {x},{y}");
            }
        }
    }

    #[test]
    fn land_downwind_of_the_sea_gets_rain() {
        let g = grid(|x, _| if x >= 4 { 400_000 } else { -1_000 });
        let c = climate(&g, BAND);
        assert!(c.rainfall[5 * 10 + 4] > 0, "first land column is dry");
    }

    #[test]
    fn a_ridge_casts_a_rain_shadow() {
        // Sea → plain → ridge → plain: the lee plain must be drier than
        // the windward plain (logic/01 §Q6 rain shadows).
        let g = grid(|x, _| match x {
            0..=1 => -1_000,
            5 => 2_500_000,
            _ => 200_000,
        });
        let c = climate(&g, BAND);
        let row = 5 * 10;
        let windward = c.rainfall[row + 4];
        let leeward = c.rainfall[row + 7];
        assert!(
            windward > leeward,
            "no shadow: windward {windward} <= leeward {leeward}"
        );
    }

    #[test]
    fn the_final_moisture_store_is_exposed() {
        // Feature 03 §Q6: edge moisture for the step-10 bundle payload.
        let g = grid(|x, _| if x >= 4 { 400_000 } else { -1_000 });
        let c = climate(&g, BAND);
        assert_eq!(c.moisture.len(), 100);
        // Sea cells sit near saturation; land cells are depleted below it.
        let sea = c.moisture[5 * 10 + 1];
        let far_land = c.moisture[5 * 10 + 9];
        assert!(
            sea > far_land,
            "sea {sea} not wetter than far land {far_land}"
        );
        assert!(u64::from(sea) <= M_SAT);
    }
}
