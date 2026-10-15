//! Recipe-9 continent climate (logic/01 §recipe-9 climate; review 3.2-2,
//! 3.2-3, 3.2-4, 3.2-17).
//!
//! Temperature and regime as recipe 7. Rainfall keeps the recipe-7
//! moisture model (release on climb, recharge over water, 1/8 lateral
//! mixing, the subtropical-high factor) but solves its steady state
//! directly: one sweep per row from the windward edge. Recipe 7 started
//! every cell saturated and averaged all `1.5 × width` passes, warm-up
//! included, so the leeward edge was too wet (+48% at 500 km, +133% at
//! 1,000 km) and the solve cost O(w²h).

use super::aridity::subtropical_permille;
use super::climate::{
    capped_distance, distance_to_sea_km, exchange_capped, latitude_millideg, ocean_mask,
    release_factor, sea_level_centi, ContinentClimate, CONTINENTALITY_CAP_KM, C_NORM, DIFFUSE_DIV,
    M_SAT,
};
use super::ContinentGrid;
use arda_core::{ClimateRegime, LatitudeBand};

/// Recipe-9 climate of `grid` at `band`.
#[must_use]
pub fn climate_recipe9(grid: &ContinentGrid, band: LatitudeBand) -> ContinentClimate {
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
            let t = sea_level_centi(lat)
                - elev * 650 / 1_000_000
                - i64::from(dist[i].min(CONTINENTALITY_CAP_KM));
            let t = i16::try_from(t.clamp(-30_000, 30_000)).unwrap_or(0);
            temperature.push(t);
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
    let relief = rain_relief(grid);
    let (rainfall, moisture) = steady_rainfall(&relief, &ocean, band);
    ContinentClimate {
        temperature,
        rainfall,
        regime,
        moisture,
        ocean,
        ocean_distance_km: dist.into_iter().map(capped_distance).collect(),
    }
}

/// Release cap of recipe 9 (1/32 of the store per cell): even unsmoothed,
/// the recipe-7 cap of 1/16 let a cliff coast reach 13 m/yr (review 3.2-2).
const F_MAX_V9: u64 = 2_048;

/// The relief rainfall climbs (logic/02 §fine-formation climate relief):
/// land heights box-blurred over 5 km twice, with the sea counted as sea
/// level, and kept on the sea cells too. Recipe 7 restored the sea's own
/// heights after blurring, so the first land cell climbed the whole
/// smoothed coastal height at once: a rain spike on every windward coast
/// (review 3.2-2). Here the ramp starts offshore.
fn rain_relief(grid: &ContinentGrid) -> ContinentGrid {
    let land = ContinentGrid::from_heights(
        grid.width(),
        grid.height(),
        grid.height_mm.iter().map(|&v| v.max(0)).collect(),
    )
    .unwrap_or_else(|_| grid.clone());
    let mut out = land.clone();
    let (w, h) = (grid.width(), grid.height());
    let r = 5;
    let mut col = vec![0_i64; usize::try_from(w * h).unwrap_or(0)];
    let idx = |x: i32, y: i32| usize::try_from(y * w + x).unwrap_or(0);
    let mut cur: Vec<i64> = land.height_mm.iter().map(|&v| i64::from(v)).collect();
    for _ in 0..2 {
        for y in 0..h {
            for x in 0..w {
                let s: i64 = (-r..=r).map(|d| cur[idx((x + d).clamp(0, w - 1), y)]).sum();
                col[idx(x, y)] = s / i64::from(2 * r + 1);
            }
        }
        for y in 0..h {
            for x in 0..w {
                let s: i64 = (-r..=r).map(|d| col[idx(x, (y + d).clamp(0, h - 1))]).sum();
                cur[idx(x, y)] = s / i64::from(2 * r + 1);
            }
        }
    }
    for (o, &v) in out.height_mm.iter_mut().zip(&cur) {
        *o = i32::try_from(v).unwrap_or(*o);
    }
    out
}

/// Steady-state rainfall and moisture: one sweep per row in its wind
/// direction ([`easterly`]), with the recipe-7 exchange and 1/8 mixing with the rows
/// above and below. The rain released at a cell in the steady state is the
/// per-pass rain recipe 7's average converges to, so the same conversion
/// (`× C_NORM / 16`) applies, here to the unfloored release. Rows are scaled by the subtropical-high
/// factor (logic/01 §Q6).
fn steady_rainfall(
    relief: &ContinentGrid,
    ocean: &[bool],
    band: LatitudeBand,
) -> (Vec<u16>, Vec<u16>) {
    let (w, h) = (relief.width(), relief.height());
    let (wu, hu) = (
        usize::try_from(w).unwrap_or(0),
        usize::try_from(h).unwrap_or(0),
    );
    let height = |x: i32, y: i32| i64::from(relief.get(x, y).raw()).max(0);
    // Prevailing wind by latitude (review 3.2-3): easterly trades within
    // 30° of the equator and polar easterlies beyond 60°, westerlies
    // between. Recipe 7 blew from the west at every latitude, so rain
    // shadows in the trade-wind belt fell on the wrong side of ranges.
    let east: Vec<bool> = (0..h)
        .map(|y| easterly(latitude_millideg(band, y, h)))
        .collect();
    let mut rain = vec![0_u64; wu * hu];
    let mut moisture = vec![0_u64; wu * hu];
    // The air arriving at each row's current column.
    let mut air = vec![M_SAT; hu];
    let mut mixed = vec![0_u64; hu];
    for t in 0..w {
        for y in 0..hu {
            // Rows mix only with neighbours under the same wind.
            let a = air[y];
            let same = |k: usize| if east[k] == east[y] { air[k] } else { a };
            let (n, s) = (same(y.saturating_sub(1)), same((y + 1).min(hu - 1)));
            let mean4 = (n + s + 2 * a) / 4;
            mixed[y] = a - a / DIFFUSE_DIV + mean4 / DIFFUSE_DIV;
        }
        for (y, m) in mixed.iter().enumerate() {
            let yi = i32::try_from(y).unwrap_or(0);
            let (x, upwind_x) = if east[y] {
                (w - 1 - t, (t > 0).then_some(w - t))
            } else {
                (t, (t > 0).then(|| t - 1))
            };
            let i = y * wu + usize::try_from(x).unwrap_or(0);
            let upwind = upwind_x.map_or(0, |u| height(u, yi));
            let climb = (height(x, yi) - upwind).max(0).unsigned_abs();
            let (after, _, _) = exchange_capped(*m, climb, ocean[i], F_MAX_V9);
            // The release in Q16, unfloored: recipe 7's per-pass floor
            // averaged out over its passes; a single sweep keeps it exact.
            rain[i] = *m * release_factor(climb, F_MAX_V9);
            moisture[i] = after;
            air[y] = after;
        }
    }
    let mut rainfall = vec![0_u16; wu * hu];
    for y in 0..hu {
        let f = subtropical_permille(latitude_millideg(band, i32::try_from(y).unwrap_or(0), h));
        for x in 0..wu {
            let i = y * wu + x;
            let mm = (rain[i] * C_NORM) >> 20;
            rainfall[i] = u16::try_from(i64::try_from(mm).unwrap_or(i64::MAX) * f / 1_000)
                .unwrap_or(u16::MAX);
        }
    }
    let moisture = moisture
        .into_iter()
        .map(|v| u16::try_from(v).unwrap_or(u16::MAX))
        .collect();
    (rainfall, moisture)
}

/// Whether the prevailing wind at `lat_mdeg` blows from the east: the
/// trades within 30° of the equator and the polar easterlies beyond 60°.
#[must_use]
pub fn easterly(lat_mdeg: i64) -> bool {
    let a = lat_mdeg.abs();
    !(30_000..=60_000).contains(&a)
}

#[cfg(test)]
#[path = "climate9_tests.rs"]
mod tests;
