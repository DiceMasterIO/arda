//! Area drainage (`logic/02` water).
//!
//! D8 flow with a fixed neighbour order, so ties break identically on every
//! platform and the result never depends on iteration order (§Q4).

use super::relief::ReliefGrid;
use arda_core::{CellCoord, AREA_CELLS};

/// Upstream cells needed before a channel is cut.
pub const CHANNEL_THRESHOLD_CELLS: u32 = 240;

/// The eight neighbour offsets, in a fixed order. Ties in the steepest-descent
/// search resolve to the first entry, which makes D8 deterministic.
const NEIGHBOURS: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

/// Drainage directions, accumulation, and Strahler order for one tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaterGrid {
    drainage: Vec<u32>,
    order: Vec<u8>,
    downstream: Vec<Option<u32>>,
}

impl WaterGrid {
    /// Upstream cell count draining through a cell.
    #[must_use]
    pub fn drainage_at(&self, at: CellCoord) -> u32 {
        self.drainage[at.index()]
    }

    /// Strahler order at a cell; 0 when below the channel threshold.
    #[must_use]
    pub fn order_at(&self, at: CellCoord) -> u8 {
        self.order[at.index()]
    }

    /// The cell this one drains into, when it has one.
    #[must_use]
    pub fn downstream_of(&self, at: CellCoord) -> Option<CellCoord> {
        let idx = self.downstream[at.index()]?;
        let n = u32::from(AREA_CELLS);
        CellCoord::new(u16::try_from(idx % n).ok()?, u16::try_from(idx / n).ok()?)
    }
}

/// Computes drainage from relief alone — the causal rule made a signature
/// (`logic/02`: a stage reads only prior stages' outputs).
#[must_use]
pub fn water(relief: &ReliefGrid) -> WaterGrid {
    let n = i32::from(AREA_CELLS);
    let count = usize::try_from(n * n).unwrap_or(0);

    // Step 1: steepest-descent direction per cell.
    let mut downstream: Vec<Option<u32>> = vec![None; count];
    for y in 0..n {
        for x in 0..n {
            let (Ok(ux), Ok(uy)) = (u16::try_from(x), u16::try_from(y)) else {
                continue;
            };
            let Some(at) = CellCoord::new(ux, uy) else {
                continue;
            };
            let here = relief.get(at);
            let mut best_drop = 0i32;
            let mut best: Option<u32> = None;

            for (dx, dy) in NEIGHBOURS {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= n || ny >= n {
                    // Off-tile: water leaves the area, which is an outlet
                    // rather than a sink.
                    continue;
                }
                let (Ok(unx), Ok(uny)) = (u16::try_from(nx), u16::try_from(ny)) else {
                    continue;
                };
                let Some(nb) = CellCoord::new(unx, uny) else {
                    continue;
                };
                let drop = here - relief.get(nb);
                if drop > best_drop {
                    best_drop = drop;
                    best = u32::try_from(ny * n + nx).ok();
                }
            }
            downstream[at.index()] = best;
        }
    }

    // Step 2: flow accumulation, processing cells from high to low so every
    // upstream contribution is already counted. Sorting by (height, index)
    // keeps the order total and therefore deterministic.
    let mut by_height: Vec<(i32, u32)> = (0..count)
        .filter_map(|i| {
            let idx = u32::try_from(i).ok()?;
            let x = u16::try_from(idx % u32::from(AREA_CELLS)).ok()?;
            let y = u16::try_from(idx / u32::from(AREA_CELLS)).ok()?;
            let at = CellCoord::new(x, y)?;
            Some((relief.get(at), idx))
        })
        .collect();
    by_height.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

    let mut drainage = vec![1u32; count];
    for &(_, idx) in &by_height {
        if let Some(down) = downstream[idx as usize] {
            drainage[down as usize] =
                drainage[down as usize].saturating_add(drainage[idx as usize]);
        }
    }

    // Step 3: Strahler order from the accumulation, one band per doubling
    // above the channel threshold.
    let order: Vec<u8> = drainage
        .iter()
        .map(|&d| {
            if d < CHANNEL_THRESHOLD_CELLS {
                0
            } else {
                let bands = (d / CHANNEL_THRESHOLD_CELLS).ilog2();
                u8::try_from(bands)
                    .unwrap_or(u8::MAX)
                    .saturating_add(1)
                    .min(9)
            }
        })
        .collect();

    WaterGrid {
        drainage,
        order,
        downstream,
    }
}
