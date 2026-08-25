//! Area generation (`logic/02`).
//!
//! The normative rules are the artifact's own sections; this module
//! implements Relief and Water. Climate, vegetation, settlement, land use,
//! and roads arrive at build-order step 5.

pub mod erosion;
pub mod fields;
pub mod fill;
pub mod relief;
pub mod water;

use crate::continent::bundles::TileBundle;
use crate::continent::ContinentGrid;
use arda_core::{
    AreaCells, AreaObjects, Cell, CellCoord, Cover, DischargeMilli, HeightMm, Lake, RiverSegment,
    Terminus, TerrainKind, AREA_CELLS,
};
use fields::Floodplain;
use fill::Filled;
pub use relief::{relief, ReliefGrid};
pub use water::{water, WaterGrid, CHANNEL_THRESHOLD_CELLS};

const N: i32 = AREA_CELLS as i32;

/// Smallest submerged extent that is recorded as a lake, in cells.
pub const LAKE_MIN_CELLS: usize = 100;
/// Smallest maximum depth that is recorded as a lake, in millimetres.
pub const LAKE_MIN_DEPTH_MM: u32 = 2_000;

/// Discharge in thousandth-cumecs contributed per upstream cell.
///
/// `ponytail:` the artifact's rule is rainfall-driven — "the rain that fell
/// upstream, less the roughly half that evaporates or soaks in". No stage
/// produces rainfall yet, so this stands in until the climate stage exists.
const DISCHARGE_PER_CELL_MILLI: u32 = 90;

fn coord(x: i32, y: i32) -> Option<CellCoord> {
    CellCoord::new(u16::try_from(x).ok()?, u16::try_from(y).ok()?)
}

/// Channel width from discharge (artifact, Water).
///
/// "Width grows with the square root of discharge — a stream carrying one
/// cubic metre a second is about four metres wide, a river carrying
/// twenty-five is twenty." So `w = 4 * sqrt(Q)` metres, returned in
/// decimetres.
#[must_use]
pub fn channel_width_dm(discharge: DischargeMilli) -> u16 {
    let q_milli = i64::from(discharge.raw());
    if q_milli == 0 {
        return 0;
    }
    // w_dm = 40 * sqrt(Q_m3s) = 40 * sqrt(q_milli / 1000)
    let scaled = isqrt(q_milli * 1000); // sqrt(q_milli)*1000 in milli units
    u16::try_from(40 * scaled / 1000).unwrap_or(u16::MAX)
}

fn isqrt(v: i64) -> i64 {
    if v <= 0 {
        return 0;
    }
    let mut x = v;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + v / x) / 2;
    }
    x
}

/// Builds the stored cell grid and object lists.
#[must_use]
pub fn compose(heights: &[i32], filled: &Filled, water: &WaterGrid) -> (AreaCells, AreaObjects) {
    let mut cells = AreaCells::flat(Cell::default());

    // Which cells belong to a lake big enough to record.
    let lakes = collect_lakes(filled);
    let mut lake_cell = vec![false; (N * N) as usize];
    for l in &lakes {
        for c in &l.cells {
            lake_cell[c.index()] = true;
        }
    }

    let hand = fields::hand(heights, water);

    for y in 0..N {
        for x in 0..N {
            let Some(at) = coord(x, y) else { continue };
            let h = heights[at.index()];
            let is_lake = lake_cell[at.index()];
            let terrain = if is_lake {
                TerrainKind::Lake
            } else if h > 0 {
                TerrainKind::Land
            } else {
                TerrainKind::Sea
            };

            let (slope_milli_deg, aspect_deg) = fields::slope_and_aspect(heights, x, y);
            let land = terrain == TerrainKind::Land;

            // Non-land cells carry no flow of their own; routing still
            // crosses them so land upstream reaches an outlet.
            let drainage = if land { water.drainage_at(at) } else { 0 };
            let discharge = DischargeMilli::new(drainage.saturating_mul(DISCHARGE_PER_CELL_MILLI));
            let order = if land { water.order_at(at) } else { 0 };
            let hand_mm = if land { hand[at.index()] } else { 0 };

            let cover = match (terrain, fields::floodplain(hand_mm)) {
                (TerrainKind::Land, Floodplain::Marsh) if order == 0 => Cover::Marsh,
                (TerrainKind::Land, _) => Cover::Grass,
                _ => Cover::Bare,
            };

            cells.set(
                at,
                Cell {
                    height: HeightMm::new(h),
                    terrain,
                    cover,
                    slope_milli_deg,
                    aspect_deg,
                    drainage_area_cells: drainage,
                    discharge,
                    watercourse_order: order,
                    watercourse_width_dm: if order > 0 {
                        channel_width_dm(discharge)
                    } else {
                        0
                    },
                    height_above_river_dm: u16::try_from(hand_mm / 100).unwrap_or(u16::MAX),
                    wetness: if land {
                        fields::wetness(drainage, slope_milli_deg)
                    } else {
                        255
                    },
                    ..Cell::default()
                },
            );
        }
    }

    let rivers = collect_segments(&cells, water, &lake_cell);
    (cells, AreaObjects { rivers, lakes })
}

/// Basins large and deep enough to record as lakes.
fn collect_lakes(filled: &Filled) -> Vec<Lake> {
    filled
        .basins
        .iter()
        .filter(|b| b.cells.len() >= LAKE_MIN_CELLS && b.depth_mm >= LAKE_MIN_DEPTH_MM)
        .enumerate()
        .map(|(i, b)| Lake {
            id: u16::try_from(i + 1).unwrap_or(u16::MAX),
            surface: HeightMm::new(b.surface_mm),
            depth_mm: b.depth_mm,
            outlet: b.outlet,
            cells: b.cells.clone(),
        })
        .collect()
}

/// Whether a channel cell begins a segment: a head, or just below a
/// confluence (artifact: segments run "from a source or a junction").
fn is_segment_start(cells: &AreaCells, water: &WaterGrid, at: CellCoord) -> bool {
    let (x, y) = (i32::from(at.x()), i32::from(at.y()));
    let inflows = fill::NEIGHBOURS
        .iter()
        .filter_map(|(dx, dy)| coord(x + dx, y + dy))
        .filter(|&nb| cells.get(nb).watercourse_order > 0 && water.downstream_of(nb) == Some(at))
        .count();
    inflows != 1
}

/// Breaks the network into segments, each knowing what it feeds and how it
/// ends (artifact, Water).
fn collect_segments(cells: &AreaCells, water: &WaterGrid, lake_cell: &[bool]) -> Vec<RiverSegment> {
    // Pass 1: walk each segment, recording its cells and where it stopped.
    let mut starts = Vec::new();
    for y in 0..N {
        for x in 0..N {
            let Some(at) = coord(x, y) else { continue };
            if cells.get(at).watercourse_order > 0 && is_segment_start(cells, water, at) {
                starts.push(at);
            }
        }
    }

    let mut owner = vec![0u16; (N * N) as usize]; // segment id per channel cell
    let mut drafts = Vec::new();

    for (i, &start) in starts.iter().enumerate() {
        let id = u16::try_from(i + 1).unwrap_or(u16::MAX);
        let mut course = vec![start];
        owner[start.index()] = id;
        let mut cursor = start;

        let ends = loop {
            let Some(next) = water.downstream_of(cursor) else {
                break if water.is_outlet(cursor) {
                    Terminus::OffTile
                } else {
                    Terminus::Sea
                };
            };
            if lake_cell[next.index()] {
                break Terminus::Lake;
            }
            let c = cells.get(next);
            if c.terrain == TerrainKind::Sea {
                break Terminus::Sea;
            }
            if c.watercourse_order == 0 {
                break Terminus::Sea;
            }
            if is_segment_start(cells, water, next) {
                break Terminus::Junction;
            }
            course.push(next);
            owner[next.index()] = id;
            cursor = next;
        };

        let tail = cells.get(cursor);
        drafts.push((
            id,
            ends,
            cursor,
            course,
            tail.watercourse_order,
            tail.discharge,
        ));
    }

    // Pass 2: resolve which segment each one feeds.
    drafts
        .into_iter()
        .map(|(id, ends, tail_cell, course, order, discharge)| {
            let feeds = if ends == Terminus::Junction {
                water
                    .downstream_of(tail_cell)
                    .map(|d| owner[d.index()])
                    .filter(|&f| f != 0)
            } else {
                None
            };
            RiverSegment {
                id,
                order,
                width_dm: channel_width_dm(discharge),
                discharge,
                feeds,
                ends,
                course,
            }
        })
        .collect()
}

/// Runs the area stage for one tile: relief, erosion, filling, routing.
#[must_use]
pub fn generate_area(
    seed: u64,
    continent: &ContinentGrid,
    bundle: &TileBundle,
) -> (AreaCells, AreaObjects) {
    let r = relief(seed, continent, bundle);
    let mut heights: Vec<i32> = (0..(N * N) as usize)
        .filter_map(|i| {
            let i = i32::try_from(i).ok()?;
            Some(r.get(coord(i % N, i / N)?))
        })
        .collect();
    let coarse = heights.clone();

    erosion::erode(&mut heights, &coarse, bundle);

    let filled = fill::fill(&heights, bundle);
    let w = water::water(&filled, bundle);
    compose(&heights, &filled, &w)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continent::bundles::bundle_for;
    use crate::continent::generate_continent;
    use arda_core::{AreaCoord, GenerateConfig};

    fn world(area: AreaCoord) -> (AreaCells, AreaObjects) {
        let c = generate_continent(42, GenerateConfig::MICRO);
        let b = bundle_for(42, &c, area);
        generate_area(42, &c, &b)
    }

    #[test]
    fn width_follows_the_artifact_relation() {
        // "one cubic metre a second is about four metres wide, a river
        // carrying twenty-five is twenty"
        let four_m = channel_width_dm(DischargeMilli::new(1_000));
        let twenty_m = channel_width_dm(DischargeMilli::new(25_000));
        assert!((38..=42).contains(&four_m), "1 m3/s gave {four_m} dm");
        assert!(
            (190..=210).contains(&twenty_m),
            "25 m3/s gave {twenty_m} dm"
        );
    }

    #[test]
    fn area_generation_is_deterministic() {
        assert_eq!(world(AreaCoord::new(1, 1)), world(AreaCoord::new(1, 1)));
    }

    #[test]
    fn segments_know_how_they_end() {
        let (_, o) = world(AreaCoord::new(0, 1));
        assert!(!o.rivers.is_empty(), "no segments emitted");
        for s in &o.rivers {
            if s.ends == Terminus::Junction {
                assert!(
                    s.feeds.is_some(),
                    "segment {} ends at a junction but feeds nothing",
                    s.id
                );
            } else {
                assert_eq!(
                    s.feeds, None,
                    "segment {} ends at {:?} yet feeds",
                    s.id, s.ends
                );
            }
        }
    }

    #[test]
    fn segments_partition_the_channel_network() {
        let (c, o) = world(AreaCoord::new(0, 1));
        let mut seen = std::collections::HashSet::new();
        for s in &o.rivers {
            for cell in &s.course {
                assert!(seen.insert(*cell), "cell {cell:?} is in two segments");
            }
        }
        for y in 0..N {
            for x in 0..N {
                let Some(at) = coord(x, y) else { continue };
                if c.get(at).watercourse_order > 0 {
                    assert!(seen.contains(&at), "channel cell {x},{y} is in no segment");
                }
            }
        }
    }

    #[test]
    fn non_land_cells_carry_no_flow() {
        let (c, _) = world(AreaCoord::new(0, 0));
        for y in 0..N {
            for x in 0..N {
                let Some(at) = coord(x, y) else { continue };
                let cell = c.get(at);
                if cell.terrain != TerrainKind::Land {
                    assert_eq!(cell.drainage_area_cells, 0);
                    assert_eq!(cell.discharge.raw(), 0);
                    assert_eq!(cell.watercourse_order, 0);
                }
            }
        }
    }

    #[test]
    fn slope_and_aspect_are_populated() {
        let (c, _) = world(AreaCoord::new(0, 1));
        let sloped = (0..N)
            .flat_map(|y| (0..N).map(move |x| (x, y)))
            .filter_map(|(x, y)| coord(x, y))
            .filter(|&at| c.get(at).slope_milli_deg > 0)
            .count();
        assert!(sloped > 1_000, "only {sloped} cells have a slope");
    }
}
