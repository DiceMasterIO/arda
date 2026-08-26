//! Continent hydrology (`logic/01` step 6, feature 02 §Q2, §D9).
//!
//! One final fill + route on the post-erosion surface, accumulating
//! catchment (cells ≡ km²) and rainfall-driven discharge down the same
//! tree the coarse erosion carved. Pure integer arithmetic; no RNG.

use super::climate::ContinentClimate;
use super::erode::{accumulate, fill, NEIGHBOURS};
use super::ContinentGrid;

/// The continent drainage tree and its per-cell loads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinentHydrology {
    /// Priority-flood routing surface (basin spill levels for 03).
    pub filled: Vec<i32>,
    /// Row-major downstream index per cell.
    pub downstream: Vec<Option<u32>>,
    /// Downstream as a fixed-neighbour-order index; 255 = none.
    pub downstream_dir: Vec<u8>,
    /// Drainage area, km²; zero on sea cells (§D9 hygiene).
    pub catchment_km2: Vec<u32>,
    /// Discharge, L/s; zero on sea cells.
    pub discharge_l_s: Vec<u32>,
}

/// Runoff conversion: `rainfall_mm × 0.5` on 1 km² is
/// `rainfall × 500,000` L/yr; divided by 31,536,000 s/yr that is
/// `Σrain × 125 / 7,884` L/s (§D9 — the artifact's "roughly half").
const RUNOFF_NUM: u64 = 125;
const RUNOFF_DEN: u64 = 7_884;

/// Routes the final surface and accumulates both loads.
#[must_use]
pub fn hydrology(grid: &ContinentGrid, climate: &ContinentClimate) -> ContinentHydrology {
    let (w, h) = (grid.width(), grid.height());
    let count = usize::try_from(w * h).unwrap_or(0);
    let heights: Vec<i32> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| grid.get(x, y).raw())
        .collect();

    let filled = fill(&heights, w, h);
    let (downstream, area) = accumulate(&filled, w, h);

    // High-to-low walk, the same order accumulate() uses internally.
    let mut order: Vec<(i32, u32)> = filled
        .iter()
        .enumerate()
        .map(|(i, &f)| (f, u32::try_from(i).unwrap_or(0)))
        .collect();
    order.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

    // Rain lands on land cells only; sea rain belongs to the sea.
    let mut rain_sum: Vec<u64> = (0..count)
        .map(|i| {
            if heights[i] > 0 {
                u64::from(climate.rainfall[i])
            } else {
                0
            }
        })
        .collect();
    for &(_, i) in &order {
        if let Some(d) = downstream[i as usize] {
            rain_sum[d as usize] += rain_sum[i as usize];
        }
    }

    let mut downstream_dir = vec![255u8; count];
    let mut catchment_km2 = vec![0u32; count];
    let mut discharge_l_s = vec![0u32; count];
    let w_usize = usize::try_from(w).unwrap_or(1);
    for i in 0..count {
        if let Some(d) = downstream[i] {
            let di = usize::try_from(d).unwrap_or(0);
            let xi = i32::try_from(i % w_usize).unwrap_or(0);
            let yi = i32::try_from(i / w_usize).unwrap_or(0);
            let xd = i32::try_from(di % w_usize).unwrap_or(0);
            let yd = i32::try_from(di / w_usize).unwrap_or(0);
            let (dx, dy) = (xd - xi, yd - yi);
            if let Some(k) = NEIGHBOURS.iter().position(|&n| n == (dx, dy)) {
                downstream_dir[i] = u8::try_from(k).unwrap_or(255);
            }
        }
        if heights[i] > 0 {
            catchment_km2[i] = area[i];
            discharge_l_s[i] =
                u32::try_from(rain_sum[i] * RUNOFF_NUM / RUNOFF_DEN).unwrap_or(u32::MAX);
        }
    }

    ContinentHydrology {
        filled,
        downstream,
        downstream_dir,
        catchment_km2,
        discharge_l_s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continent::climate::climate;
    use crate::continent::ContinentGrid;
    use arda_core::LatitudeBand;

    /// A 20×20 island dome on an ocean rim.
    fn dome() -> ContinentGrid {
        let (w, h) = (20, 20);
        ContinentGrid {
            width: w,
            height: h,
            height_mm: (0..w * h)
                .map(|i| {
                    let (x, y) = (i % w, i / w);
                    let d = (x - 10).abs().max((y - 10).abs());
                    if d >= 8 {
                        -500_000
                    } else {
                        1_600_000 - d * 200_000
                    }
                })
                .collect(),
        }
    }

    fn hydro() -> (ContinentGrid, ContinentHydrology) {
        let g = dome();
        let c = climate(&g, LatitudeBand::new(35, 55));
        let hy = hydrology(&g, &c);
        (g, hy)
    }

    #[test]
    fn every_land_cell_drains_to_the_ocean() {
        // Spec R7 invariant (a), at unit scale.
        let (g, hy) = hydro();
        let (w, h) = (g.width(), g.height());
        for start in 0..(w * h) {
            let i = usize::try_from(start).unwrap();
            if g.get(start % w, start / w).raw() <= 0 {
                continue;
            }
            let mut at = i;
            let mut steps = 0;
            loop {
                let (x, y) = (at as i32 % w, at as i32 / w);
                if g.get(x, y).raw() <= 0 {
                    break; // reached ocean
                }
                let Some(d) = hy.downstream[at] else {
                    panic!("land cell {x},{y} is a routing dead end");
                };
                at = d as usize;
                steps += 1;
                assert!(steps <= w * h, "cycle from cell {i}");
            }
        }
    }

    #[test]
    fn catchment_and_discharge_never_shrink_downstream() {
        // Spec R7 invariant (b). Compare raw sums before sea-zeroing:
        // land cell → land downstream only.
        let (g, hy) = hydro();
        let w = g.width();
        for (i, d) in hy.downstream.iter().enumerate() {
            let Some(d) = *d else { continue };
            let d = d as usize;
            let land = |j: usize| g.get(j as i32 % w, j as i32 / w).raw() > 0;
            if land(i) && land(d) {
                assert!(hy.catchment_km2[d] >= hy.catchment_km2[i]);
                assert!(hy.discharge_l_s[d] >= hy.discharge_l_s[i]);
            }
        }
    }

    #[test]
    fn sea_cells_store_zero_catchment_and_discharge() {
        // Spec R7 invariant (d); mirrors the area tier's §Q13 hygiene.
        let (g, hy) = hydro();
        let w = g.width();
        for i in 0..hy.catchment_km2.len() {
            if g.get(i as i32 % w, i as i32 / w).raw() <= 0 {
                assert_eq!(hy.catchment_km2[i], 0);
                assert_eq!(hy.discharge_l_s[i], 0);
            }
        }
    }

    #[test]
    fn downstream_dir_agrees_with_downstream() {
        let (g, hy) = hydro();
        let w = g.width();
        for (i, d) in hy.downstream.iter().enumerate() {
            match *d {
                None => assert_eq!(hy.downstream_dir[i], 255),
                Some(d) => {
                    let (dx, dy) = (d as i32 % w - i as i32 % w, d as i32 / w - i as i32 / w);
                    let k = hy.downstream_dir[i] as usize;
                    assert_eq!(super::super::erode::NEIGHBOURS[k], (dx, dy));
                }
            }
        }
    }

    #[test]
    fn hydrology_is_deterministic() {
        let g = dome();
        let c = climate(&g, LatitudeBand::new(35, 55));
        assert_eq!(hydrology(&g, &c), hydrology(&g, &c));
    }
}
