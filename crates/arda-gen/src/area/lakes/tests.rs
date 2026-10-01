//! Lake recording at the tile rim: the continent spill-level clamp, seam
//! trims and outlet recomputation (feature 03 §Q5).

use super::*;
use crate::area::fill::Basin;
use crate::continent::build_continent;
use crate::continent::bundles::bundle_for;
use crate::continent::hydrology::NO_BASIN;
use arda_core::{AreaCoord, GenerateConfig};

/// `build_continent(123, MICRO, 0)`, memoized: the seam-lake tests
/// below scan every MICRO tile, and continent generation is too
/// expensive to redo per tile (mirrors
/// `continent::bundles::tests::fixture_ctx`).
///
/// Seed 123, not the usual 42: a survey of twelve seeds under MICRO
/// (feature 03 §Q5 / open-items #12) found exactly one near-rim lake
/// in 96 tile-generations, on seed 123 tile (1, 0); seed 42 has none.
/// See task-5-report.md for the full survey.
const SEAM_LAKE_SEED: u64 = 123;

fn fixture_ctx() -> crate::continent::Continent {
    static CTX: std::sync::OnceLock<crate::continent::Continent> = std::sync::OnceLock::new();
    CTX.get_or_init(|| build_continent(SEAM_LAKE_SEED, GenerateConfig::MICRO, 0))
        .clone()
}

/// Feature 02 §Q1 / feature 03 §Q5 test oracle: the near-rim cells
/// among `cells`, first checked for continent-tier lake identity
/// (nearest-cell `bundle.basin_km`, `NO_BASIN` excluded) and, only if
/// none maps to a depression, falling back to the max smoothstep-bilinear
/// sample of `bundle.filled_km` — reimplemented here independently of
/// `nearest_km_patch` / `sample_km_patch` / `clamp_near_rim` so this
/// check cannot pass merely by calling back into the code under test.
/// `None` when `cells` has no near-rim cell.
fn continent_surface_at(b: &TileBundle, cells: &[CellCoord]) -> Option<i32> {
    let km0x = (b.area.x * 512).div_euclid(10);
    let km0y = (b.area.y * 512).div_euclid(10);
    let side = i32::try_from(PATCH_KM).unwrap_or(0);

    let rim: Vec<&CellCoord> = cells
        .iter()
        .filter(|c| c.x() <= 1 || c.y() <= 1 || c.x() >= 510 || c.y() >= 510)
        .collect();
    if rim.is_empty() {
        return None;
    }

    let basin_best = rim
        .iter()
        .map(|c| {
            let (ax, ay) = abs_cell(b.area, c.x(), c.y());
            let (kx, ky) = (ax.div_euclid(10) - km0x, ay.div_euclid(10) - km0y);
            let px = usize::try_from(kx.clamp(0, side - 1)).unwrap_or(0);
            let py = usize::try_from(ky.clamp(0, side - 1)).unwrap_or(0);
            b.basin_km[py * PATCH_KM + px]
        })
        .filter(|&v| v != crate::continent::hydrology::NO_BASIN)
        .max();
    if let Some(v) = basin_best {
        return Some(v);
    }

    let smooth = |v: i32| -> i64 {
        let t = i64::from(v) * 65536 / 10;
        let t2 = (t * t) >> 16;
        let t3 = (t2 * t) >> 16;
        (3 * t2 - 2 * t3).clamp(0, 65536)
    };

    let best = rim
        .iter()
        .map(|c| {
            let (ax, ay) = abs_cell(b.area, c.x(), c.y());
            let (kx, ky) = (ax.div_euclid(10) - km0x, ay.div_euclid(10) - km0y);
            let (fx, fy) = (smooth(ax.rem_euclid(10)), smooth(ay.rem_euclid(10)));
            let at = |dx: i32, dy: i32| -> i64 {
                let px = usize::try_from((kx + dx).clamp(0, side - 1)).unwrap_or(0);
                let py = usize::try_from((ky + dy).clamp(0, side - 1)).unwrap_or(0);
                i64::from(b.filled_km[py * PATCH_KM + px])
            };
            let top = at(0, 0) + (((at(1, 0) - at(0, 0)) * fx) >> 16);
            let bottom = at(0, 1) + (((at(1, 1) - at(0, 1)) * fx) >> 16);
            top + (((bottom - top) * fy) >> 16)
        })
        .max()?;
    Some(i32::try_from(best).unwrap_or(i32::MAX))
}

/// A rim-touching basin takes the continent's spill level, and its
/// membership stays exactly what `fill::fill` found (feature 03 §Q5,
/// closes open-items #12).
///
/// Built from a synthetic pit rather than generated relief. It used to
/// scan seed 123 tile (1, 0), the single near-rim lake a 12-seed
/// survey turned up; raising `LAKE_MIN_CELLS`/`LAKE_MIN_DEPTH_MM`
/// leaves no near-rim lake anywhere in a MICRO world (re-surveyed 12
/// seeds x 8 tiles with `survey_near_rim_lakes`: zero hits — a MICRO
/// tile is too small to hold a 3 km2 lake against its own rim). A test
/// that quietly finds nothing to check is worse than one that builds
/// its own case, so it now builds one.
#[test]
fn edge_touching_basins_take_the_continent_spill_level() {
    let ctx = fixture_ctx();
    let mut b = bundle_for(SEAM_LAKE_SEED, &ctx, AreaCoord::new(0, 1));
    b.filled_km = vec![DIAG_CLAMP_MM; PATCH_KM * PATCH_KM];

    let heights = rim_touching_pit();
    let filled = fill::fill(&heights, &b);
    let lakes = collect_lakes(&heights, &filled, &b);

    let mut checked = 0u32;
    for lake in &lakes {
        if !lake.cells.iter().any(|&c| near_rim(c)) {
            continue;
        }
        checked += 1;

        let expected = continent_surface_at(&b, &lake.cells)
            .expect("a near-rim lake must have at least one near-rim cell");
        assert_eq!(
            lake.surface.raw(),
            expected,
            "lake {} did not take the continent spill",
            lake.id
        );

        // Basins are disjoint and non-empty, so the first cell
        // (ascending row-major, per `fill::fill`) identifies the basin
        // this lake came from.
        let basin = filled
            .basins
            .iter()
            .find(|basin| basin.cells.first() == lake.cells.first())
            .unwrap_or_else(|| {
                panic!(
                    "lake {} has no fill::fill basin starting at the same cell",
                    lake.id
                )
            });
        assert_eq!(
            &basin.cells, &lake.cells,
            "lake {} cell set drifted from fill::fill's basin \u{2014} membership must \
             stay local to the tile (\u{a7}Q5)",
            lake.id
        );
    }
    assert!(
        checked > 0,
        "the synthetic rim basin must yield a near-rim lake for this to check anything"
    );
}

/// A pit pressed against the tile's west rim, sized to clear both lake
/// floors.
///
/// Column 0 stays wall: `fill::fill` seeds the border at its own
/// height, so a cell ON the edge is never submerged and could not form
/// a depression. `near_rim` reaches x <= 1, so column 1 is what makes
/// this basin near-rim.
fn rim_touching_pit() -> Vec<i32> {
    const WALL_BASE: i32 = 10_000;
    const WALL_STEP: i32 = 100;
    let mut heights = vec![0i32; usize::try_from(N * N).unwrap_or(0)];
    for y in 0..N {
        for x in 0..N {
            let dist = x.min(y).min(N - 1 - x).min(N - 1 - y);
            let i = usize::try_from(y * N + x).unwrap_or(0);
            heights[i] = WALL_BASE + dist * WALL_STEP;
        }
    }
    // 4 columns x 100 rows = 400 cells, over LAKE_MIN_CELLS, and
    // DIAG_CLAMP_MM - DIAG_LOW_MM = 4.5 m clears LAKE_MIN_DEPTH_MM.
    for y in 100..=199 {
        for x in 1..=4 {
            let i = usize::try_from(y * N + x).unwrap_or(0);
            heights[i] = DIAG_LOW_MM;
        }
    }
    heights
}

#[test]
fn clamp_near_rim_replaces_local_spill_but_not_membership() {
    // Feature 03 §Q5, non-vacuous regardless of whether any generated
    // MICRO tile happens to carry a near-rim lake: a synthetic basin
    // touching the rim, clamped against a `filled_km` patch flattened
    // to one known value so the expected clamp is exact without
    // re-deriving the smoothstep-bilinear weights a third time.
    let ctx = fixture_ctx();
    let mut b = bundle_for(SEAM_LAKE_SEED, &ctx, AreaCoord::new(0, 1));
    let flat = 12_345i32;
    b.filled_km = vec![flat; PATCH_KM * PATCH_KM];

    let rim_cell = coord(0, 200).unwrap();
    let inner_cell = coord(5, 200).unwrap();
    assert!(near_rim(rim_cell) && !near_rim(inner_cell));
    let mut heights = vec![0i32; (N * N) as usize];
    heights[rim_cell.index()] = 100;
    heights[inner_cell.index()] = 300;

    let basin = Basin {
        cells: vec![inner_cell, rim_cell],
        surface_mm: 500, // local spill, deliberately far from `flat`
        depth_mm: 400,
    };
    assert_eq!(
        clamp_near_rim(&basin, &heights, &b),
        (flat, u32::try_from(flat - 100).unwrap()),
        "clamp must take the continent surface and recompute depth against \
         it and the basin's own floor, without touching membership"
    );

    // An interior basin (no near-rim cell) must pass through exactly
    // as `fill::fill` reported it, regardless of `filled_km`.
    let interior = Basin {
        cells: vec![coord(200, 200).unwrap()],
        surface_mm: 500,
        depth_mm: 400,
    };
    assert_eq!(clamp_near_rim(&interior, &heights, &b), (500, 400));
}

/// A west-rim physical basin with 100 high bank cells and 300 low cells.
/// Both controls pass this terrain through the real local priority flood.
/// A constant coarse clamp makes trim expectations analytic; no natural
/// erosion pit or sampled continent-depression location is required.
fn explicit_seam_basin() -> (Vec<i32>, TileBundle, Filled) {
    let ctx = fixture_ctx();
    let mut b = bundle_for(SEAM_LAKE_SEED, &ctx, AreaCoord::new(0, 1));
    b.basin_km.fill(NO_BASIN);
    b.filled_km.fill(DIAG_CLAMP_MM);
    let mut heights = rim_touching_pit();
    for y in 100..=199 {
        heights[coord(1, y).unwrap().index()] = DIAG_HIGH_MM;
    }
    let filled = fill::fill(&heights, &b);
    assert_eq!(
        filled.basins.len(),
        1,
        "only the physical pit may be submerged"
    );
    (heights, b, filled)
}

#[test]
fn seam_clamp_trims_cells_the_shared_surface_leaves_dry() {
    // The former seed 99 pit vanished under the approved physical erosion
    // correction. Preserve the down-clamp/trim/outlet contract with a real
    // explicit 400-cell pit: its 100 high bank cells become dry at 5.5 m,
    // while 300 cells at 1 m remain submerged with 4.5 m depth.
    let (heights, b, filled) = explicit_seam_basin();
    let target = coord(1, 100).unwrap();
    let basin = filled
        .basins
        .iter()
        .find(|basin| basin.cells.first() == Some(&target))
        .expect("the physical west-rim pit must be found by the real fill");
    // Four cardinal steps from the 10 m physical rim give the legacy
    // routing flood's 10,004 mm maximum; walls already descend without fill.
    assert_eq!((basin.cells.len(), basin.surface_mm), (400, 10_004));

    let (surface_mm, depth_mm, cells) =
        clamp_and_trim(basin, &heights, &b).expect("the clamp does not empty this basin");
    assert!(
        surface_mm < basin.surface_mm,
        "this case is meant to pin a DOWN clamp"
    );
    assert_eq!((surface_mm, depth_mm, cells.len()), (5_500, 4_500, 300));

    for &c in &cells {
        assert!(
            heights[c.index()] < surface_mm,
            "cell {c:?} survived the trim but sits at or above the clamped surface"
        );
    }
    for &c in &basin.cells {
        if !cells.contains(&c) {
            assert!(
                heights[c.index()] >= surface_mm,
                "cell {c:?} was trimmed despite sitting below the clamped surface"
            );
        }
    }

    // Round-2 review fix: the outlet now comes from a second pass over
    // the tile's final lake set (`collect_lakes`), not from
    // `clamp_and_trim` itself. Mirror that pass here for this single
    // basin: a `submerged` grid marking exactly its surviving `cells`,
    // as `collect_lakes` would build if this were the tile's only
    // lake.
    let mut submerged = vec![false; (N * N) as usize];
    for &c in &cells {
        submerged[c.index()] = true;
    }
    let outlet = recompute_outlet(&cells, &submerged, &filled);

    // The outlet moves to a released bank cell, remains outside surviving
    // membership, and still physically touches the trimmed lake.
    let out = outlet.expect("the 300 surviving cells have a released adjacent bank");
    assert!(
        basin.cells.contains(&out),
        "the outlet uses the released bank"
    );
    assert!(
        !cells.contains(&out),
        "outlet {out:?} must not be one of the surviving lake's own cells"
    );
    assert!(
        cells.iter().any(|&c| {
            let dx = i32::from(c.x()) - i32::from(out.x());
            let dy = i32::from(c.y()) - i32::from(out.y());
            dx.abs() <= 1 && dy.abs() <= 1
        }),
        "outlet {out:?} is not 4/8-adjacent to any surviving cell"
    );
}

#[test]
fn seam_clamp_drops_a_basin_the_trim_empties() {
    // At the 1 m floor, every cell in the explicit physical basin becomes
    // dry (equality is not submerged). This replaces the vanished seed 42
    // two-cell natural pit without weakening the empty-trim contract.
    let (heights, mut b, filled) = explicit_seam_basin();
    b.filled_km.fill(DIAG_LOW_MM);
    let target = coord(1, 100).unwrap();
    let basin = filled
        .basins
        .iter()
        .find(|basin| basin.cells.first() == Some(&target))
        .expect("the physical west-rim pit must be found by the real fill");
    assert_eq!((basin.cells.len(), basin.surface_mm), (400, 10_004));
    assert!(basin
        .cells
        .iter()
        .all(|c| heights[c.index()] >= DIAG_LOW_MM));

    assert_eq!(
        clamp_and_trim(basin, &heights, &b),
        None,
        "every cell in this basin sits at or above the clamped surface, so trimming \
         must empty it and the basin must not become a lake"
    );
    assert!(collect_lakes(&heights, &filled, &b).is_empty());
}

/// Heights of the rim-pit and diagonal-basin fixtures (see [`diagonal`]).
const DIAG_LOW_MM: i32 = 1_000;
const DIAG_HIGH_MM: i32 = 9_000;
const DIAG_CLAMP_MM: i32 = 5_500;

mod diagonal;
