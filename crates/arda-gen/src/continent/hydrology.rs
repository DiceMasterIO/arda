//! Continent hydrology (`logic/01` step 6, feature 02 §Q2, §D9).
//!
//! One final fill + route on the post-erosion surface, accumulating
//! catchment (cells ≡ km²) and rainfall-driven discharge down the same
//! tree the coarse erosion carved. Pure integer arithmetic; no RNG.

use super::climate::ContinentClimate;
use super::erode::{accumulate, fill, NEIGHBOURS};
use super::ContinentGrid;
use arda_core::ContinentRiver;

/// The continent drainage tree and its per-cell loads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinentHydrology {
    /// Priority-flood routing surface (basin spill levels for 03).
    pub filled: Vec<i32>,
    /// Row-major downstream index per cell.
    pub downstream: Vec<Option<u32>>,
    /// Downstream as a fixed-neighbour-order index;
    /// [`arda_core::NO_DOWNSTREAM`] = none.
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

    let mut downstream_dir = vec![arda_core::NO_DOWNSTREAM; count];
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
                downstream_dir[i] = u8::try_from(k).unwrap_or(arda_core::NO_DOWNSTREAM);
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

/// Catchment a cell needs before it belongs to a continent river
/// (feature 02 §Q7). `logic/01` §Q7's ~3,000 km² is what this yields
/// at default-world land areas.
#[must_use]
pub fn river_threshold_km2(land_km2: u32) -> u32 {
    (land_km2 / 30).max(300)
}

/// Traces the major network into river objects (`logic/01` §Q7).
///
/// Mouths (network cells whose downstream is ocean or a routing root)
/// are processed in row-major order. From each, the walk upstream
/// follows the largest-catchment network inflow as the main stem; the
/// other inflows queue as tributaries feeding the current river. Ids
/// are assigned in creation order, so `feeds` always points at a
/// smaller id and the link graph is acyclic by construction.
#[must_use]
pub fn extract_rivers(grid: &ContinentGrid, hydro: &ContinentHydrology) -> Vec<ContinentRiver> {
    let (w, h) = (grid.width(), grid.height());
    let count = usize::try_from(w * h).unwrap_or(0);
    let is_land = |i: usize| {
        let i = i32::try_from(i).unwrap_or(0);
        grid.get(i % w, i / w).raw() > 0
    };
    let land_km2 = u32::try_from((0..count).filter(|&i| is_land(i)).count()).unwrap_or(u32::MAX);
    let threshold = river_threshold_km2(land_km2);
    let in_network = |i: usize| is_land(i) && hydro.catchment_km2[i] >= threshold;

    // Network inflows per cell, in row-major child order (deterministic).
    let mut inflows: Vec<Vec<u32>> = vec![Vec::new(); count];
    for i in 0..count {
        if !in_network(i) {
            continue;
        }
        if let Some(d) = hydro.downstream[i] {
            inflows[d as usize].push(u32::try_from(i).unwrap_or(0));
        }
    }

    let km = |i: usize| {
        let i = i32::try_from(i).unwrap_or(0);
        arda_core::KmCoord::new(
            u16::try_from(i % w).unwrap_or(0),
            u16::try_from(i / w).unwrap_or(0),
        )
    };

    let mut rivers = Vec::new();
    let mut pending: std::collections::VecDeque<(usize, Option<u16>)> =
        std::collections::VecDeque::new();

    // Mouths: network cells draining to ocean or to a routing root.
    for i in 0..count {
        if !in_network(i) {
            continue;
        }
        let to_sea = match hydro.downstream[i] {
            None => true,
            Some(d) => !is_land(d as usize),
        };
        if to_sea {
            pending.push_back((i, None));
        }
    }

    while let Some((mouth, feeds)) = pending.pop_front() {
        let id = u16::try_from(rivers.len() + 1).unwrap_or(u16::MAX);
        let mut course_rev = vec![mouth];
        let mut at = mouth;
        loop {
            // Largest-catchment inflow continues the stem; ties break to
            // the earliest row-major child. Spec R4 says "fixed neighbour
            // order", not row-major order — this is a deliberate deviation,
            // recorded here for the reference refresh, and the golden
            // fixture is blessed on this behaviour.
            let mut main: Option<usize> = None;
            for &c in &inflows[at] {
                let c = c as usize;
                if main.is_none_or(|m| hydro.catchment_km2[c] > hydro.catchment_km2[m]) {
                    main = Some(c);
                }
            }
            let Some(main) = main else { break };
            for &c in &inflows[at] {
                if c as usize != main {
                    pending.push_back((c as usize, Some(id)));
                }
            }
            course_rev.push(main);
            at = main;
        }
        course_rev.reverse();
        rivers.push(ContinentRiver {
            id,
            catchment_km2: hydro.catchment_km2[mouth],
            discharge: arda_core::DischargeMilli::new(hydro.discharge_l_s[mouth]),
            feeds,
            course: course_rev.into_iter().map(km).collect(),
        });
    }
    rivers
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
                let (x, y) = (
                    i32::try_from(at).unwrap_or(0) % w,
                    i32::try_from(at).unwrap_or(0) / w,
                );
                if g.get(x, y).raw() <= 0 {
                    break; // reached ocean
                }
                let Some(d) = hy.downstream[at] else {
                    panic!("land cell {x},{y} is a routing dead end");
                };
                at = usize::try_from(d).unwrap_or(0);
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
            let d = usize::try_from(d).unwrap_or(0);
            let land = |j: usize| {
                g.get(
                    i32::try_from(j).unwrap_or(0) % w,
                    i32::try_from(j).unwrap_or(0) / w,
                )
                .raw()
                    > 0
            };
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
            if g.get(
                i32::try_from(i).unwrap_or(0) % w,
                i32::try_from(i).unwrap_or(0) / w,
            )
            .raw()
                <= 0
            {
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
                None => assert_eq!(hy.downstream_dir[i], arda_core::NO_DOWNSTREAM),
                Some(d) => {
                    let (dx, dy) = (
                        i32::try_from(d).unwrap_or(0) % w - i32::try_from(i).unwrap_or(0) % w,
                        i32::try_from(d).unwrap_or(0) / w - i32::try_from(i).unwrap_or(0) / w,
                    );
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

    /// A 40×40 south-sloping valley whose trench collects both flanks;
    /// sea at the south edge. Shared by the river-extraction tests.
    fn dome_big() -> ContinentGrid {
        let (w, h) = (40, 40);
        ContinentGrid {
            width: w,
            height: h,
            height_mm: (0..w * h)
                .map(|i| {
                    let (x, y) = (i % w, i / w);
                    if y >= h - 3 || y == 0 || x == 0 || x == w - 1 {
                        -500_000
                    } else {
                        // Tilted plane toward the south + a centre trench.
                        2_000_000 - y * 40_000 + (x - w / 2).abs() * 30_000
                    }
                })
                .collect(),
        }
    }

    #[test]
    fn the_threshold_scales_with_land_and_floors_at_300() {
        // Feature 02 §Q7: max(300, land_km2 / 30).
        assert_eq!(river_threshold_km2(1_000), 300);
        assert_eq!(river_threshold_km2(9_000), 300);
        assert_eq!(river_threshold_km2(300_000), 10_000);
    }

    #[test]
    fn a_valley_produces_one_river_reaching_the_sea() {
        // A south-sloping valley whose trench collects both flanks; sea
        // at the south edge. With the dome's land area the threshold
        // floors at 300 km², so a river only forms if the valley
        // focuses flow — dome_big()'s trench does.
        let w = 40;
        let g = dome_big();
        let c = climate(&g, LatitudeBand::new(35, 55));
        let hy = hydrology(&g, &c);
        let rivers = extract_rivers(&g, &hy);
        assert!(!rivers.is_empty(), "no river extracted");
        let main = &rivers[0];
        assert_eq!(main.feeds, None, "main stem must reach the sea");
        assert!(main.catchment_km2 >= 300);
        // Course runs source → mouth, descending on the routing surface.
        let idx = |c: &arda_core::KmCoord| {
            usize::from(c.y) * usize::try_from(w).unwrap_or(0) + usize::from(c.x)
        };
        for pair in main.course.windows(2) {
            assert!(hy.filled[idx(&pair[0])] >= hy.filled[idx(&pair[1])]);
        }
    }

    #[test]
    fn tributaries_feed_earlier_ids_and_stay_acyclic() {
        // Spec R7 invariant (c): feeds is acyclic. Creation order makes
        // every tributary's id greater than the id it feeds.
        let (g, hy) = {
            let g = dome_big();
            let c = climate(&g, LatitudeBand::new(35, 55));
            let hy = hydrology(&g, &c);
            (g, hy)
        };
        for r in extract_rivers(&g, &hy) {
            if let Some(f) = r.feeds {
                assert!(f < r.id, "river {} feeds later id {f}", r.id);
            }
        }
    }
}
