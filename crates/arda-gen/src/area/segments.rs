//! River segments: the channel network broken at sources and confluences,
//! each knowing what it feeds and how it ends (logic/02 Water).

use super::*;
use crate::hydrology::budget::BudgetError;
use arda_core::{AreaCells, CellCoord, Terminus, TerrainKind};
use local_objects::LocalRiverSegment as RiverSegment;

/// Whether a channel cell begins a segment: a head, or just below a
/// confluence (artifact: segments run "from a source or a junction").
pub(super) fn is_segment_start(cells: &AreaCells, water: &WaterGrid, at: CellCoord) -> bool {
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
#[allow(clippy::cast_possible_truncation)] // At most512² channel starts.
pub(super) fn collect_segments(
    cells: &AreaCells,
    water: &WaterGrid,
    lake_cell: &[bool],
) -> Result<Vec<RiverSegment>, BudgetError> {
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

    let mut owner = vec![0u32; (N * N) as usize]; // segment id per channel cell
    let mut drafts = Vec::new();

    for (i, &start) in starts.iter().enumerate() {
        let id = (i + 1) as u32;
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
            Ok(RiverSegment {
                id,
                order,
                width_dm: channel_width_dm(discharge)?,
                discharge,
                feeds,
                ends,
                course,
            })
        })
        .collect()
}
