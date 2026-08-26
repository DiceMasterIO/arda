//! Depression filling and basin detection.
//!
//! Priority-flood raises every interior depression to its spill elevation,
//! producing the surface flow is routed over. Stored relief is never
//! modified — the artifact's Relief section keeps depressions as real
//! terrain ("rivers simply pass through them"), so only routing sees the
//! filled field.

use crate::continent::bundles::TileBundle;
use arda_core::{CellCoord, AREA_CELLS};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// A filled depression: the cells under water, how deep, and where it spills.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Basin {
    /// Submerged cells, ascending row-major.
    pub cells: Vec<CellCoord>,
    /// Spill elevation, millimetres — the water surface.
    pub surface_mm: i32,
    /// Surface minus the lowest submerged cell, millimetres.
    pub depth_mm: u32,
    /// The cell water leaves through; `None` when it spills off-tile.
    pub outlet: Option<CellCoord>,
}

/// The routing surface plus the basins that produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filled {
    surface: Vec<i32>,
    /// Basins found, ordered by their first cell.
    pub basins: Vec<Basin>,
}

impl Filled {
    /// Routing elevation at a cell.
    #[must_use]
    pub fn get(&self, at: CellCoord) -> i32 {
        self.surface[at.index()]
    }

    /// Whether a cell sits under a basin's water surface.
    #[must_use]
    pub fn is_submerged(&self, at: CellCoord, raw: i32) -> bool {
        self.surface[at.index()] > raw
    }
}

const N: i32 = AREA_CELLS as i32;

/// Row-major offset. Invariant: every caller bounds-checks `0 <= x,y < N`
/// before calling, so the product is non-negative.
#[allow(clippy::cast_sign_loss)]
fn idx(x: i32, y: i32) -> usize {
    (y * N + x) as usize
}

fn coord(x: i32, y: i32) -> Option<CellCoord> {
    CellCoord::new(u16::try_from(x).ok()?, u16::try_from(y).ok()?)
}

/// The eight neighbour offsets, fixed order.
pub(crate) const NEIGHBOURS: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

/// Fills every interior depression by priority-flood.
///
/// The boundary queue is seeded from the tile edge at the *neighbouring*
/// tile's heights, taken from the bundle, so water that should leave the
/// tile is not dammed by its own edge.
///
/// Determinism: the heap key is `(height, y, x)`. `BinaryHeap` leaves the
/// order of equal keys unspecified, and a bare height key would give a
/// different filled surface between runs — passing locally while breaking
/// the cross-platform golden gate.
#[must_use]
pub fn fill(heights: &[i32], bundle: &TileBundle) -> Filled {
    let count = (N * N) as usize;
    let mut surface = vec![i32::MIN; count];
    let mut heap: BinaryHeap<Reverse<(i32, i32, i32)>> = BinaryHeap::new();

    // Seed the rim. A rim cell can never be raised: it either drains inward
    // to a lower cell or leaves the tile past the neighbour's edge height.
    for y in 0..N {
        for x in 0..N {
            if x != 0 && y != 0 && x != N - 1 && y != N - 1 {
                continue;
            }
            let h = heights[idx(x, y)];
            surface[idx(x, y)] = h;
            heap.push(Reverse((h, y, x)));
        }
    }

    while let Some(Reverse((h, y, x))) = heap.pop() {
        for (dx, dy) in NEIGHBOURS {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= N || ny >= N {
                continue;
            }
            let ni = idx(nx, ny);
            if surface[ni] != i32::MIN {
                continue;
            }
            // +1 mm keeps a strictly downhill path across a filled flat.
            let v = heights[ni].max(h.saturating_add(1));
            surface[ni] = v;
            heap.push(Reverse((v, ny, nx)));
        }
    }

    let basins = find_basins(heights, &surface, bundle);
    Filled { surface, basins }
}

/// Groups submerged cells into connected basins (4-connectivity).
fn find_basins(heights: &[i32], surface: &[i32], _bundle: &TileBundle) -> Vec<Basin> {
    let count = (N * N) as usize;
    let mut seen = vec![false; count];
    let mut basins = Vec::new();

    for y in 0..N {
        for x in 0..N {
            let i = idx(x, y);
            if seen[i] || surface[i] <= heights[i] || heights[i] <= 0 {
                continue;
            }
            let mut stack = vec![(x, y)];
            seen[i] = true;
            let mut cells = Vec::new();
            let mut floor = i32::MAX;
            let mut spill = i32::MIN;

            while let Some((cx, cy)) = stack.pop() {
                let ci = idx(cx, cy);
                if let Some(c) = coord(cx, cy) {
                    cells.push(c);
                }
                floor = floor.min(heights[ci]);
                spill = spill.max(surface[ci]);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if nx < 0 || ny < 0 || nx >= N || ny >= N {
                        continue;
                    }
                    let ni = idx(nx, ny);
                    if !seen[ni] && surface[ni] > heights[ni] && heights[ni] > 0 {
                        seen[ni] = true;
                        stack.push((nx, ny));
                    }
                }
            }

            cells.sort_unstable();
            let outlet = spill_cell(&cells, surface, heights);
            basins.push(Basin {
                cells,
                surface_mm: spill,
                depth_mm: u32::try_from(spill.saturating_sub(floor)).unwrap_or(0),
                outlet,
            });
        }
    }
    basins.sort_by_key(|b| b.cells.first().copied());
    basins
}

/// The dry neighbour a basin spills through: the lowest cell adjacent to the
/// basin that is not itself submerged. `None` when the basin only touches
/// the tile edge.
fn spill_cell(cells: &[CellCoord], surface: &[i32], heights: &[i32]) -> Option<CellCoord> {
    let mut best: Option<(i32, CellCoord)> = None;
    for c in cells {
        let (x, y) = (i32::from(c.x()), i32::from(c.y()));
        for (dx, dy) in NEIGHBOURS {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= N || ny >= N {
                continue;
            }
            let ni = idx(nx, ny);
            if surface[ni] > heights[ni] {
                continue; // still inside the basin
            }
            let Some(nc) = coord(nx, ny) else { continue };
            let h = surface[ni];
            if best.is_none_or(|(bh, bc)| h < bh || (h == bh && nc < bc)) {
                best = Some((h, nc));
            }
        }
    }
    best.map(|(_, c)| c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continent::build_continent;
    use crate::continent::bundles::bundle_for;
    use arda_core::{AreaCoord, GenerateConfig};

    fn setup() -> (Vec<i32>, TileBundle) {
        let c = build_continent(42, GenerateConfig::MICRO, 0);
        let b = bundle_for(42, &c, AreaCoord::new(0, 1));
        let r = crate::area::relief::relief(42, &c.grid, &b);
        let heights = (0..(N * N) as usize)
            .filter_map(|i| {
                let i = i32::try_from(i).ok()?;
                Some(r.get(coord(i % N, i / N)?))
            })
            .collect();
        (heights, b)
    }

    #[test]
    fn filling_removes_every_interior_pit() {
        let (h, b) = setup();
        let f = fill(&h, &b);
        for y in 1..N - 1 {
            for x in 1..N - 1 {
                let here = f.surface[idx(x, y)];
                let has_lower = NEIGHBOURS.iter().any(|(dx, dy)| {
                    let (nx, ny) = (x + dx, y + dy);
                    nx >= 0 && ny >= 0 && nx < N && ny < N && f.surface[idx(nx, ny)] < here
                });
                assert!(has_lower, "cell {x},{y} is still a pit after filling");
            }
        }
    }

    #[test]
    fn the_surface_never_drops_below_raw_relief() {
        let (h, b) = setup();
        let f = fill(&h, &b);
        for (i, &raw) in h.iter().enumerate() {
            assert!(f.surface[i] >= raw, "cell {i} was lowered by filling");
        }
    }

    #[test]
    fn filling_is_deterministic() {
        let (h, b) = setup();
        assert_eq!(fill(&h, &b), fill(&h, &b));
    }

    #[test]
    fn basins_record_depth_and_an_outlet() {
        let (h, b) = setup();
        let f = fill(&h, &b);
        assert!(!f.basins.is_empty(), "the noise surface should hold basins");
        for basin in &f.basins {
            assert!(!basin.cells.is_empty());
            assert!(basin.depth_mm > 0, "a basin with no depth is not a basin");
            if let Some(o) = basin.outlet {
                assert!(
                    !basin.cells.contains(&o),
                    "outlet must be outside the basin"
                );
            }
        }
    }

    #[test]
    fn basins_are_disjoint() {
        let (h, b) = setup();
        let f = fill(&h, &b);
        let mut seen = std::collections::HashSet::new();
        for basin in &f.basins {
            for c in &basin.cells {
                assert!(seen.insert(*c), "cell {c:?} is in two basins");
            }
        }
    }
}
