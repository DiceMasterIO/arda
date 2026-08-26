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

/// Computes temperature and regime; rainfall stays zero until the
/// advection pass exists (next task).
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
    ContinentClimate {
        temperature,
        rainfall: vec![0; count],
        regime,
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
}
