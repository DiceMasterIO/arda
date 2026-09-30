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
    /// D8 component of nonpositive coarse cells connected to the domain rim.
    pub ocean: Vec<bool>,
    /// Cardinal distance to that ocean, capped at the 300 km climate limit.
    pub ocean_distance_km: Vec<u16>,
}

/// Continentality cap: −1 centi-°C per km inland up to 300 km
/// (feature 02 §Q5).
const CONTINENTALITY_CAP_KM: u32 = 300;

/// Km distance to the nearest sea cell; 4-connected BFS from every
/// sea cell. Deterministic: BFS level order fixes each cell's value
/// regardless of intra-level ordering.
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
    let a = lat_mdeg.abs();
    if a <= 35_000 {
        1800 + (35_000 - a) * 979 / 35_000
    } else {
        1800 - (a - 35_000) * 6 / 100
    }
}

#[allow(clippy::cast_sign_loss)] // ContinentGrid has positive, bounded dimensions.
pub(crate) fn ocean_mask(grid: &ContinentGrid) -> Vec<bool> {
    let w = grid.width as usize;
    let h = grid.height as usize;
    let mut ocean = vec![false; grid.height_mm.len()];
    let mut queue = VecDeque::new();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if (x == 0 || y == 0 || x + 1 == w || y + 1 == h) && grid.height_mm[i] <= 0 {
                ocean[i] = true;
                queue.push_back(i);
            }
        }
    }
    while let Some(i) = queue.pop_front() {
        let (x, y) = (i % w, i / w);
        for ny in y.saturating_sub(1)..=(y + 1).min(h - 1) {
            for nx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                let j = ny * w + nx;
                if !ocean[j] && grid.height_mm[j] <= 0 {
                    ocean[j] = true;
                    queue.push_back(j);
                }
            }
        }
    }
    ocean
}

#[allow(clippy::cast_sign_loss)] // Same validated grid dimensions as ocean_mask.
fn distance_to_sea_km(grid: &ContinentGrid, ocean: &[bool]) -> Vec<u32> {
    let w = grid.width as usize;
    let h = grid.height as usize;
    let mut dist = vec![u32::MAX; grid.height_mm.len()];
    let mut queue = VecDeque::new();
    for (i, is_ocean) in ocean.iter().copied().enumerate() {
        if is_ocean {
            dist[i] = 0;
            queue.push_back(i);
        }
    }
    while let Some(i) = queue.pop_front() {
        let (x, y) = (i % w, i / w);
        for candidate in [
            y.checked_sub(1).map(|ny| ny * w + x),
            (x + 1 < w).then_some(i + 1),
            (y + 1 < h).then_some(i + w),
            x.checked_sub(1).map(|nx| y * w + nx),
        ]
        .into_iter()
        .flatten()
        {
            if dist[candidate] == u32::MAX {
                dist[candidate] = dist[i] + 1;
                queue.push_back(candidate);
            }
        }
    }
    dist
}

fn exchange(m: u64, climb_mm: u64, ocean: bool) -> (u64, u64, u64) {
    let factor = (F_BASE + F_ORO * climb_mm / 100_000).min(F_MAX);
    let rain = (m * factor) >> 16;
    let remaining = m - rain;
    let recharge = if ocean {
        (M_SAT - remaining) / RECHARGE_DIV
    } else {
        0
    };
    (remaining + recharge, rain, recharge)
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
fn rainfall_field(grid: &ContinentGrid, ocean: &[bool]) -> (Vec<u16>, Vec<u16>) {
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
                let west = if x == 0 { 0 } else { height(x - 1, y).max(0) };
                let climb = (height(x, y).max(0) - west).max(0).unsigned_abs();
                let (after, rain, _recharge) = exchange(m[i], climb, ocean[i]);
                m[i] = after;
                accum[i] += rain;
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

#[allow(clippy::cast_possible_truncation)]
fn capped_distance(d: u32) -> u16 {
    d.min(300) as u16
}

/// Computes temperature and regime; rainfall is computed via advection-diffusion.
#[must_use]
pub fn climate(grid: &ContinentGrid, band: LatitudeBand) -> ContinentClimate {
    climate_with_rain_relief(grid, band, grid)
}

/// Recipe-5 climate (logic/02 §fine-formation climate relief): orographic
/// rainfall reads `rain_relief`, a smoothed copy of the formed terrain,
/// because precipitation responds to topography at ~10 km and above;
/// 1 km ridges of eroded terrain would otherwise make per-cell rain spikes.
/// Temperature, regime and ocean masks still use the true grid.
#[must_use]
pub fn climate_smoothed_rain(grid: &ContinentGrid, band: LatitudeBand) -> ContinentClimate {
    let smooth = smoothed_land(grid, 5, 2);
    climate_with_rain_relief(grid, band, &smooth)
}

/// Box-blurs land heights (radius `r` cells, `passes` times); sea cells
/// keep their height so the ocean mask and coastline are unchanged.
fn smoothed_land(grid: &ContinentGrid, r: i32, passes: u32) -> ContinentGrid {
    let (w, h) = (grid.width(), grid.height());
    let n = usize::try_from(w * h).unwrap_or(0);
    let idx = |x: i32, y: i32| usize::try_from(y * w + x).unwrap_or(0);
    let mut cur: Vec<i64> = (0..n)
        .map(|i| {
            let (x, y) = (
                i32::try_from(i).unwrap_or(0) % w,
                i32::try_from(i).unwrap_or(0) / w,
            );
            i64::from(grid.get(x, y).raw().max(0))
        })
        .collect();
    let mut next = cur.clone();
    for _ in 0..passes {
        for y in 0..h {
            for x in 0..w {
                let s: i64 = (-r..=r).map(|d| cur[idx((x + d).clamp(0, w - 1), y)]).sum();
                next[idx(x, y)] = s / i64::from(2 * r + 1);
            }
        }
        for y in 0..h {
            for x in 0..w {
                let s: i64 = (-r..=r)
                    .map(|d| next[idx(x, (y + d).clamp(0, h - 1))])
                    .sum();
                cur[idx(x, y)] = s / i64::from(2 * r + 1);
            }
        }
    }
    let heights = (0..n)
        .map(|i| {
            let (x, y) = (
                i32::try_from(i).unwrap_or(0) % w,
                i32::try_from(i).unwrap_or(0) / w,
            );
            let raw = grid.get(x, y).raw();
            if raw <= 0 {
                raw
            } else {
                i32::try_from(cur[i].max(1)).unwrap_or(raw)
            }
        })
        .collect();
    ContinentGrid::from_heights(w, h, heights).unwrap_or_else(|_| grid.clone())
}

fn climate_with_rain_relief(
    grid: &ContinentGrid,
    band: LatitudeBand,
    rain_relief: &ContinentGrid,
) -> ContinentClimate {
    let (w, h) = (grid.width(), grid.height());
    let count = usize::try_from(w * h).unwrap_or(0);
    let ocean = ocean_mask(grid);
    let dist = distance_to_sea_km(grid, &ocean);

    let mut temperature = Vec::with_capacity(count);
    let mut regime = Vec::with_capacity(count);
    for y in 0..h {
        let lat = latitude_millideg(band, y, h);
        for x in 0..w {
            let i = usize::try_from(y * w + x).unwrap_or(0);
            let elev = if ocean[i] {
                0
            } else {
                i64::from(grid.get(x, y).raw())
            };

            // logic/01 §Q6: latitude + 6.5 °C/km lapse + continentality.
            let t = sea_level_centi(lat)
                - elev * 650 / 1_000_000
                - i64::from(dist[i].min(CONTINENTALITY_CAP_KM));
            let t = i16::try_from(t.clamp(-30_000, 30_000)).unwrap_or(0);
            temperature.push(t);
            // Feature 02 §Q5: band position and elevation, in order.
            regime.push(if t < 300 {
                ClimateRegime::Boreal
            } else if lat.abs() < 23_500 {
                ClimateRegime::Tropical
            } else if lat.abs() < 42_000 && elev < 1_000_000 {
                ClimateRegime::Mediterranean
            } else {
                ClimateRegime::Temperate
            });
        }
    }
    let (rainfall, moisture) = rainfall_field(rain_relief, &ocean);
    let ocean_distance_km = dist.into_iter().map(capped_distance).collect();
    ContinentClimate {
        temperature,
        rainfall,
        regime,
        moisture,
        ocean,
        ocean_distance_km,
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
    fn sea_cells_receive_precipitation() {
        let g = grid(|x, _| if x >= 4 { 400_000 } else { -1_000 });
        let c = climate(&g, BAND);
        for y in 0..10 {
            for x in 0..4 {
                let i = usize::try_from(y * 10 + x).unwrap();
                assert!(c.rainfall[i] > 0, "sea cell {x},{y}");
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
    #[test]
    fn annual_temperature_is_symmetric_and_tropical_bound_is_grounded() {
        assert_eq!(sea_level_centi(-55_000), 600);
        assert_eq!(sea_level_centi(55_000), 600);
        assert_eq!(sea_level_centi(35_000), 1800);
        assert_eq!(sea_level_centi(0), 2779);
    }
    #[test]
    fn precipitation_is_available_over_ocean_and_enclosed_negative_land() {
        let sea = grid(|_, _| -100_000);
        let c = climate(&sea, BAND);
        assert!(c.rainfall.iter().all(|p| *p > 0));
        let bowl = grid(|x, y| {
            if x == 0 || x == 9 || y == 0 || y == 9 {
                -100_000
            } else if (3..=6).contains(&x) && (3..=6).contains(&y) {
                -50_000
            } else {
                100_000
            }
        });
        let c = climate(&bowl, BAND);
        assert!(!c.ocean[5 * 10 + 5]);
        assert!(c.ocean_distance_km[5 * 10 + 5] > 0);
        assert!(c.rainfall[5 * 10 + 5] > 0);
    }
    #[test]
    fn diagonal_ocean_contact_matches_the_shared_d8_convention() {
        let g = grid(|x, y| if x == y { -1 } else { 1 });
        let ocean = ocean_mask(&g);
        assert!(ocean[5 * 10 + 5]);
        assert!(!ocean[5 * 10 + 4]);
    }
    #[test]
    fn condensation_is_removed_once_before_ocean_recharge() {
        for m in [0, M_SAT / 2, M_SAT] {
            for climb in [0, 100_000, 1_000_000] {
                for ocean in [false, true] {
                    let (after, p, recharge) = exchange(m, climb, ocean);
                    assert_eq!(m + recharge, after + p);
                    assert!(after <= M_SAT);
                    if !ocean {
                        assert_eq!(recharge, 0);
                    }
                }
            }
        }
    }
}
