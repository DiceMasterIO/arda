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
fn maximum_coarse_catchment_does_not_saturate_at_u32() {
    assert_eq!(coarse_discharge_l_s(16_000_000 * 65_535), 16_624_809_741);
    assert!(coarse_discharge_l_s(16_000_000 * 19_584) > u64::from(u32::MAX));
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

/// Two separate pits divided by a tall wall, each enclosed by a
/// DIFFERENTLY-heighted rim — so if the two components were ever
/// wrongly merged (or a bug just took one grid-wide maximum instead of
/// a per-component one), their surfaces would wrongly agree; this shape
/// forces them to differ when grouping is correct. 11×5: the literal
/// grid edge is the rim (h=100 for x<5, h=300 for x>5 — seeded
/// directly, so it is never raised regardless of anything interior),
/// a h=1_000 wall fills the whole x=5 column (far above anything the
/// flood front reaches from either rim, so it is never raised either),
/// and a h=10 pit sits on each side of the wall.
fn two_pits() -> (Vec<i32>, i32, i32) {
    let (w, h) = (11, 5);
    let mut heights = vec![0i32; usize::try_from(w * h).unwrap_or(0)];
    for y in 0..h {
        for x in 0..w {
            let i = usize::try_from(y * w + x).unwrap_or(0);
            let is_rim = x == 0 || x == w - 1 || y == 0 || y == h - 1;
            heights[i] = if x == 5 {
                1_000 // dividing wall, far above either rim
            } else if is_rim {
                if x <= 4 {
                    100
                } else {
                    300
                }
            } else {
                10 // pit floor, both sides
            };
        }
    }
    (heights, w, h)
}

#[test]
fn basin_surface_is_constant_per_depression_and_sentinel_outside_it() {
    let (heights, w, h) = two_pits();
    let filled = fill(&heights, w, h);
    let basins = basin_components(&heights, &filled, w, h);
    let idx = |x: i32, y: i32| usize::try_from(y * w + x).unwrap_or(0);

    // The wall and the rim are seeded directly and never raised, so
    // they must read back the sentinel.
    for y in 0..h {
        for x in 0..w {
            let i = idx(x, y);
            let is_rim = x == 0 || x == w - 1 || y == 0 || y == h - 1;
            if x == 5 || is_rim {
                assert_eq!(
                    filled[i], heights[i],
                    "wall/rim cell {x},{y} was unexpectedly raised — fixture drifted"
                );
                assert_eq!(
                    basins[i], NO_BASIN,
                    "dry cell {x},{y} did not get the sentinel"
                );
            }
        }
    }

    let left_cells: Vec<usize> = (1..=3)
        .flat_map(|y| (1..=4).map(move |x| (x, y)))
        .map(|(x, y)| idx(x, y))
        .collect();
    let right_cells: Vec<usize> = (1..=3)
        .flat_map(|y| (6..=9).map(move |x| (x, y)))
        .map(|(x, y)| idx(x, y))
        .collect();

    for &i in left_cells.iter().chain(&right_cells) {
        assert!(filled[i] > heights[i], "pit cell {i} did not submerge");
    }

    // basin_surface must be the component's max `filled`, not
    // `filled[i]` itself: the priority-flood's 1 mm anti-tie ramp can
    // leave `filled` slightly different cell to cell within one
    // physical depression, and every cell of it must still report the
    // SAME basin_surface.
    let left_expected = left_cells.iter().map(|&i| filled[i]).max().unwrap();
    let right_expected = right_cells.iter().map(|&i| filled[i]).max().unwrap();
    for &i in &left_cells {
        assert_eq!(
            basins[i], left_expected,
            "left pit cell {i} off the component max"
        );
    }
    for &i in &right_cells {
        assert_eq!(
            basins[i], right_expected,
            "right pit cell {i} off the component max"
        );
    }

    // Differently-enclosed pits must produce different surfaces: proof
    // the two components were kept separate rather than merged into
    // one basin (or one grid-wide constant).
    assert_ne!(
        left_expected, right_expected,
        "two differently-enclosed pits produced the same surface — are they being merged \
         into one component (or one global maximum)?"
    );
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
