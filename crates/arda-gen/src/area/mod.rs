//! Area generation (`logic/02`).
//!
//! Skeleton scope: relief and water. Climate, vegetation, settlement,
//! land-use, and roads arrive at build-order step 5, each as another pure
//! stage taking prior outputs.

pub mod relief;
pub mod water;

use crate::continent::bundles::TileBundle;
use crate::continent::ContinentGrid;
use arda_core::{
    AreaCells, AreaObjects, Cell, CellCoord, Cover, DischargeMilli, HeightMm, RiverSegment,
    TerrainKind, AREA_CELLS,
};
pub use relief::{relief, ReliefGrid};
pub use water::{water, WaterGrid, CHANNEL_THRESHOLD_CELLS};

/// Discharge in thousandth-cumecs contributed per upstream cell.
///
/// `ponytail:` linear rating curve; replace with the rainfall-driven
/// relationship once the climate stage exists (build-order step 5).
const DISCHARGE_PER_CELL_MILLI: u32 = 90;

/// Turns relief and water into the stored cell grid and object lists.
#[must_use]
pub fn compose(relief: &ReliefGrid, water: &WaterGrid) -> (AreaCells, AreaObjects) {
    let mut cells = AreaCells::flat(Cell::default());
    let n = AREA_CELLS;

    for y in 0..n {
        for x in 0..n {
            let Some(at) = CellCoord::new(x, y) else {
                continue;
            };
            let h = relief.get(at);
            let is_land = h > 0;
            let order = if is_land { water.order_at(at) } else { 0 };
            let drainage = water.drainage_at(at);

            cells.set(
                at,
                Cell {
                    height: HeightMm::new(h),
                    terrain: if is_land {
                        TerrainKind::Land
                    } else {
                        TerrainKind::Sea
                    },
                    cover: if is_land { Cover::Grass } else { Cover::Bare },
                    drainage_area_cells: drainage,
                    discharge: DischargeMilli::new(
                        drainage.saturating_mul(DISCHARGE_PER_CELL_MILLI),
                    ),
                    watercourse_order: order,
                    watercourse_width_dm: u16::from(order) * 12,
                    ..Cell::default()
                },
            );
        }
    }

    // One segment per channel run, walked downstream from each channel head.
    // Build-order step 5 merges these into named reaches.
    let mut rivers = Vec::new();
    let mut next_id = 1u16;
    for y in 0..n {
        for x in 0..n {
            let Some(at) = CellCoord::new(x, y) else {
                continue;
            };
            if cells.get(at).watercourse_order == 0 || has_channel_inflow(&cells, water, at) {
                continue;
            }

            let mut course = vec![at];
            let mut cursor = at;
            while let Some(down) = water.downstream_of(cursor) {
                if cells.get(down).watercourse_order == 0 {
                    break;
                }
                course.push(down);
                cursor = down;
                // Invariant guard: a course cannot exceed the tile.
                if course.len() >= usize::from(AREA_CELLS) * 2 {
                    break;
                }
            }

            let tail = cells.get(cursor);
            rivers.push(RiverSegment {
                id: next_id,
                order: tail.watercourse_order,
                width_dm: tail.watercourse_width_dm,
                discharge: tail.discharge,
                course,
            });
            next_id = next_id.saturating_add(1);
        }
    }

    (
        cells,
        AreaObjects {
            rivers,
            lakes: Vec::new(),
        },
    )
}

/// Whether any neighbouring channel cell drains into `at`.
fn has_channel_inflow(cells: &AreaCells, water: &WaterGrid, at: CellCoord) -> bool {
    let n = i32::from(AREA_CELLS);
    let (x, y) = (i32::from(at.x()), i32::from(at.y()));
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= n || ny >= n {
                continue;
            }
            let (Ok(unx), Ok(uny)) = (u16::try_from(nx), u16::try_from(ny)) else {
                continue;
            };
            let Some(nb) = CellCoord::new(unx, uny) else {
                continue;
            };
            if cells.get(nb).watercourse_order > 0 && water.downstream_of(nb) == Some(at) {
                return true;
            }
        }
    }
    false
}

/// Runs the skeleton area stage for one tile.
#[must_use]
pub fn generate_area(
    seed: u64,
    continent: &ContinentGrid,
    bundle: &TileBundle,
) -> (AreaCells, AreaObjects) {
    let r = relief(seed, continent, bundle);
    let w = water(&r);
    compose(&r, &w)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continent::bundles::bundle_for;
    use crate::continent::generate_continent;
    use arda_core::{AreaCoord, GenerateConfig};

    fn setup(area: AreaCoord) -> (ContinentGrid, TileBundle) {
        let c = generate_continent(42, GenerateConfig::MICRO);
        let b = bundle_for(42, &c, area);
        (c, b)
    }

    fn cc(x: u16, y: u16) -> CellCoord {
        CellCoord::new(x, y).unwrap()
    }

    #[test]
    fn area_generation_is_deterministic() {
        let (c, b) = setup(AreaCoord::new(1, 1));
        assert_eq!(generate_area(42, &c, &b), generate_area(42, &c, &b));
    }

    #[test]
    fn relief_matches_the_bundle_on_the_northern_edge() {
        // logic/02 amendment 3: edge heights are pinned to the bundle.
        let (c, b) = setup(AreaCoord::new(1, 1));
        let r = relief(42, &c, &b);
        for x in 0..AREA_CELLS {
            assert_eq!(
                r.get(cc(x, 0)),
                b.north[x as usize],
                "north edge mismatch at x={x}"
            );
        }
    }

    #[test]
    fn relief_matches_the_bundle_on_the_western_edge() {
        let (c, b) = setup(AreaCoord::new(1, 1));
        let r = relief(42, &c, &b);
        for y in 0..AREA_CELLS {
            assert_eq!(
                r.get(cc(0, y)),
                b.west[y as usize],
                "west edge mismatch at y={y}"
            );
        }
    }

    /// The cross-tile agreement gate (`implementation.md` step 5
    /// verification, brought forward because the skeleton must not bake in a
    /// seam it cannot later remove).
    #[test]
    fn adjacent_tiles_agree_on_their_shared_edge_cells() {
        let c = generate_continent(42, GenerateConfig::MICRO);
        let left = bundle_for(42, &c, AreaCoord::new(0, 1));
        let right = bundle_for(42, &c, AreaCoord::new(1, 1));
        let lr = relief(42, &c, &left);
        let rr = relief(42, &c, &right);
        // Tile (0,1)'s last column is absolute 511; tile (1,1)'s first is 512.
        // They are neighbours, so the bundle edge is the shared line: left's
        // `east` (absolute 512) must equal right's column 0.
        for y in 0..AREA_CELLS {
            assert_eq!(
                left.east[y as usize],
                rr.get(cc(0, y)),
                "tiles disagree at shared row y={y}"
            );
        }
        // And the interior columns are genuinely different data.
        assert_ne!(lr.get(cc(0, 0)), rr.get(cc(0, 0)));
    }

    #[test]
    fn every_landlocked_cell_has_a_downstream() {
        // logic/01 invariant, at area scale: water always has somewhere to go.
        let (c, b) = setup(AreaCoord::new(1, 1));
        let r = relief(42, &c, &b);
        let w = water(&r);
        let mut sinks = 0;
        for y in 1..AREA_CELLS - 1 {
            for x in 1..AREA_CELLS - 1 {
                let at = cc(x, y);
                if r.get(at) > 0 && w.downstream_of(at).is_none() {
                    sinks += 1;
                }
            }
        }
        // Pit cells are legitimate (they become lakes at step 5), but they
        // must be rare — a flat field would make every cell a sink.
        let total = f64::from(u32::from(AREA_CELLS - 2)).powi(2);
        assert!(
            f64::from(sinks) / total < 0.05,
            "{sinks} interior sinks is more than 5% of the tile"
        );
    }

    #[test]
    fn drainage_area_is_at_least_one_everywhere() {
        let (c, b) = setup(AreaCoord::new(1, 1));
        let w = water(&relief(42, &c, &b));
        for y in 0..AREA_CELLS {
            for x in 0..AREA_CELLS {
                assert!(w.drainage_at(cc(x, y)) >= 1);
            }
        }
    }

    #[test]
    fn drainage_increases_downstream() {
        // The defining property of flow accumulation.
        let (c, b) = setup(AreaCoord::new(1, 1));
        let r = relief(42, &c, &b);
        let w = water(&r);
        for y in 1..AREA_CELLS - 1 {
            for x in 1..AREA_CELLS - 1 {
                let at = cc(x, y);
                if let Some(down) = w.downstream_of(at) {
                    assert!(
                        w.drainage_at(down) >= w.drainage_at(at),
                        "drainage shrank downstream of {x},{y}"
                    );
                }
            }
        }
    }

    #[test]
    fn channels_appear_and_carry_strahler_order() {
        let (c, b) = setup(AreaCoord::new(1, 1));
        let (cells, objects) = generate_area(42, &c, &b);
        let channel_cells = (0..AREA_CELLS)
            .flat_map(|y| (0..AREA_CELLS).map(move |x| cc(x, y)))
            .filter(|&at| cells.get(at).watercourse_order > 0)
            .count();
        assert!(channel_cells > 0, "no channels were cut");
        assert!(!objects.rivers.is_empty(), "no river segments were emitted");
        assert!(objects.rivers.iter().all(|r| r.order >= 1));
    }

    #[test]
    fn sea_cells_are_marked_and_carry_no_channel() {
        let (c, b) = setup(AreaCoord::new(0, 0));
        let (cells, _) = generate_area(42, &c, &b);
        let mut saw_sea = false;
        for y in 0..AREA_CELLS {
            for x in 0..AREA_CELLS {
                let cell = cells.get(cc(x, y));
                if cell.terrain == TerrainKind::Sea {
                    saw_sea = true;
                    assert_eq!(cell.watercourse_order, 0);
                }
            }
        }
        assert!(saw_sea, "the corner tile should contain ocean");
    }
}
