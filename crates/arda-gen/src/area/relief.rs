//! Area relief (`logic/02`), conditioned on the tile bundle.
//!
//! Every cell — edges included — comes from `boundary_height`, so the pinned
//! edges are not a repair step but the definition (`logic/02` amendment 3).

use crate::continent::bundles::{abs_cell, boundary_height, TileBundle};
use crate::continent::ContinentGrid;
use arda_core::{CellCoord, AREA_CELLS};

/// One tile's elevation field, in millimetres.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReliefGrid {
    heights: Vec<i32>,
}

impl ReliefGrid {
    /// Elevation at a cell.
    #[must_use]
    pub fn get(&self, at: CellCoord) -> i32 {
        self.heights[at.index()]
    }
}

/// Builds the tile's relief.
#[must_use]
pub fn relief(seed: u64, continent: &ContinentGrid, bundle: &TileBundle) -> ReliefGrid {
    let n = AREA_CELLS;
    let mut heights = Vec::with_capacity(n as usize * n as usize);
    for y in 0..n {
        for x in 0..n {
            let (ax, ay) = abs_cell(bundle.area, x, y);
            heights.push(boundary_height(seed, continent, ax, ay));
        }
    }
    ReliefGrid { heights }
}
