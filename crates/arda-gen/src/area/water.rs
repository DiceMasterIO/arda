//! Drainage over the filled surface (artifact, Water).
//!
//! "Every land cell drains to one of its eight neighbours, so the whole map
//! is a tree with the sea and the low edges at its roots." Walking that tree
//! gives drainage area; a cell becomes a watercourse once it carries about
//! 40 litres per second, "which in a temperate climate means roughly three
//! square kilometres of catchment above it".

use super::fill::{Filled, NEIGHBOURS};
use crate::continent::bundles::TileBundle;
use arda_core::{CellCoord, AREA_CELLS};

const N: i32 = AREA_CELLS as i32;

/// Catchment above a cell before it counts as a watercourse.
///
/// The artifact states the rule as ~40 L/s, and gives its temperate-climate
/// equivalent as ~3 km². At 100 m cells that is 300 cells. The discharge
/// form of the rule needs rainfall, which no stage produces yet; this is the
/// catchment stand-in the artifact itself supplies.
pub const CHANNEL_THRESHOLD_CELLS: u32 = 300;

/// Row-major offset. Invariant: every caller bounds-checks `0 <= x,y < N`
/// before calling, so the product is non-negative.
#[allow(clippy::cast_sign_loss)]
fn idx(x: i32, y: i32) -> usize {
    (y * N + x) as usize
}

fn coord(x: i32, y: i32) -> Option<CellCoord> {
    CellCoord::new(u16::try_from(x).ok()?, u16::try_from(y).ok()?)
}

/// Drainage directions, accumulation, and Strahler order for one tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaterGrid {
    drainage: Vec<u32>,
    order: Vec<u8>,
    downstream: Vec<Option<u32>>,
    outlets: Vec<bool>,
}

impl WaterGrid {
    /// Upstream cell count draining through a cell.
    #[must_use]
    pub fn drainage_at(&self, at: CellCoord) -> u32 {
        self.drainage[at.index()]
    }

    /// Strahler order at a cell; 0 below the channel threshold.
    #[must_use]
    pub fn order_at(&self, at: CellCoord) -> u8 {
        self.order[at.index()]
    }

    /// The cell this one drains into, when it has one in-tile.
    #[must_use]
    pub fn downstream_of(&self, at: CellCoord) -> Option<CellCoord> {
        let i = i32::try_from(self.downstream[at.index()]?).ok()?;
        coord(i % N, i / N)
    }

    /// Whether this cell's water leaves the tile.
    #[must_use]
    pub fn is_outlet(&self, at: CellCoord) -> bool {
        self.outlets[at.index()]
    }

    /// Whether a cell carries a watercourse.
    #[must_use]
    pub fn is_channel(&self, at: CellCoord) -> bool {
        self.order[at.index()] > 0
    }
}

/// Height just outside the tile, from the neighbour's own edge cells.
///
/// The bundle's arrays are the neighbouring tile's first row or column, so a
/// boundary cell can be compared against real terrain rather than dammed by
/// its own edge.
fn off_tile_height(bundle: &TileBundle, x: i32, y: i32) -> Option<i32> {
    let last = N - 1;
    let i = |v: i32| usize::try_from(v).ok();
    match (x, y) {
        (_, -1) => bundle.north.get(i(x)?).copied(),
        (_, yy) if yy > last => bundle.south.get(i(x)?).copied(),
        (-1, _) => bundle.west.get(i(y)?).copied(),
        (xx, _) if xx > last => bundle.east.get(i(y)?).copied(),
        _ => None,
    }
}

/// Routes water over `filled` and returns the drainage tree.
#[must_use]
pub fn water(filled: &Filled, bundle: &TileBundle) -> WaterGrid {
    let count = (N * N) as usize;
    let mut downstream: Vec<Option<u32>> = vec![None; count];
    let mut outlets = vec![false; count];
    let mut by_height: Vec<(i32, u32)> = Vec::with_capacity(count);

    for y in 0..N {
        for x in 0..N {
            let Some(at) = coord(x, y) else { continue };
            let h = filled.get(at);
            by_height.push((h, u32::try_from(idx(x, y)).unwrap_or(0)));

            // Steepest descent = drop ÷ distance. Compared by cross
            // multiplication so no division or float enters the choice:
            // an orthogonal step is 1000 units, a diagonal 1414, so scoring
            // by drop × (the *other* length) ranks them exactly.
            let mut best: Option<(i64, u32)> = None;
            let mut best_off: Option<i64> = None;

            for (dx, dy) in NEIGHBOURS {
                let (nx, ny) = (x + dx, y + dy);
                let inv = if dx != 0 && dy != 0 { 1000 } else { 1414 };

                if nx < 0 || ny < 0 || nx >= N || ny >= N {
                    // Off-tile: real neighbour height, so water leaves only
                    // when that is genuinely downhill.
                    if let Some(oh) = off_tile_height(bundle, nx, ny) {
                        let drop = i64::from(h - oh);
                        if drop > 0 {
                            let score = drop * inv;
                            if best_off.is_none_or(|b| score > b) {
                                best_off = Some(score);
                            }
                        }
                    }
                    continue;
                }

                let Some(nb) = coord(nx, ny) else { continue };
                let drop = i64::from(h - filled.get(nb));
                if drop <= 0 {
                    continue;
                }
                let score = drop * inv;
                if best.is_none_or(|(bs, _)| score > bs) {
                    best = Some((score, u32::try_from(idx(nx, ny)).unwrap_or(0)));
                }
            }

            let on_rim = x == 0 || y == 0 || x == N - 1 || y == N - 1;
            match (best, best_off) {
                // Leaving the tile wins: this cell is an outlet, not a sink.
                (Some((bs, _)), Some(off)) if off > bs => outlets[idx(x, y)] = true,
                (None, Some(_)) => outlets[idx(x, y)] = true,
                (Some((_, d)), _) => downstream[idx(x, y)] = Some(d),
                // The artifact roots the drainage tree at "the sea and the
                // low edges". A rim cell with no downhill neighbour in either
                // direction is such a root: water leaves the map there rather
                // than pooling against a pinned edge. Erosion lowers the
                // interior while the rim stays fixed, so without this the
                // whole catchment behind a rim cell would strand.
                (None, None) if on_rim => outlets[idx(x, y)] = true,
                (None, None) => {}
            }
        }
    }

    // Accumulate from high to low; ties by index keep the order total.
    by_height.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut drainage = vec![1u32; count];
    for &(_, i) in &by_height {
        if let Some(d) = downstream[i as usize] {
            drainage[d as usize] = drainage[d as usize].saturating_add(drainage[i as usize]);
        }
    }

    let order = strahler(&downstream, &drainage, &by_height);

    WaterGrid {
        drainage,
        order,
        downstream,
        outlets,
    }
}

/// Strahler order over the channel network (artifact, Water).
///
/// "Headwater streams are first order, two first-order streams meeting make
/// a second, two seconds make a third, while a smaller stream joining a
/// larger one does not change the larger one's order." On a grid a cell can
/// take up to seven upstream channels, so the rule generalises to: take the
/// maximum incoming order, and add one when two or more inflows share it.
///
/// Order is computed over the routing graph, which passes *through* filled
/// basins — so a river keeps its order across a lake instead of restarting
/// below it. Lake cells have their stored order zeroed later, at compose.
fn strahler(downstream: &[Option<u32>], drainage: &[u32], by_height: &[(i32, u32)]) -> Vec<u8> {
    let count = drainage.len();
    let mut order = vec![0u8; count];
    // Highest incoming order per cell, and how many inflows carry it.
    let mut max_in = vec![0u8; count];
    let mut max_count = vec![0u16; count];

    // by_height is high-to-low, so every upstream cell is settled first.
    for &(_, i) in by_height {
        let i = i as usize;
        if drainage[i] < CHANNEL_THRESHOLD_CELLS {
            continue;
        }
        order[i] = if max_in[i] == 0 {
            1 // a channel head
        } else if max_count[i] >= 2 {
            max_in[i].saturating_add(1)
        } else {
            max_in[i]
        };

        if let Some(d) = downstream[i] {
            let d = d as usize;
            match order[i].cmp(&max_in[d]) {
                std::cmp::Ordering::Greater => {
                    max_in[d] = order[i];
                    max_count[d] = 1;
                }
                std::cmp::Ordering::Equal => max_count[d] = max_count[d].saturating_add(1),
                std::cmp::Ordering::Less => {}
            }
        }
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::area::fill::fill;
    use crate::area::relief::relief;
    use crate::continent::bundles::bundle_for;
    use crate::continent::generate_continent;
    use arda_core::{AreaCoord, GenerateConfig};

    fn setup() -> (Vec<i32>, Filled, TileBundle) {
        let c = generate_continent(42, GenerateConfig::MICRO);
        let b = bundle_for(42, &c, AreaCoord::new(0, 1));
        let r = relief(42, &c, &b);
        let h: Vec<i32> = (0..(N * N) as usize)
            .filter_map(|i| {
                let i = i32::try_from(i).ok()?;
                Some(r.get(coord(i % N, i / N)?))
            })
            .collect();
        let f = fill(&h, &b);
        (h, f, b)
    }

    #[test]
    fn no_land_cell_is_a_sink() {
        let (h, f, b) = setup();
        let w = water(&f, &b);
        for y in 1..N - 1 {
            for x in 1..N - 1 {
                let Some(at) = coord(x, y) else { continue };
                if h[idx(x, y)] <= 0 {
                    continue;
                }
                assert!(
                    w.downstream_of(at).is_some() || w.is_outlet(at),
                    "cell {x},{y} has nowhere to drain"
                );
            }
        }
    }

    #[test]
    fn water_leaves_the_tile() {
        let (_, f, b) = setup();
        let w = water(&f, &b);
        let outlets = (0..N)
            .flat_map(|y| (0..N).map(move |x| (x, y)))
            .filter_map(|(x, y)| coord(x, y))
            .filter(|&c| w.is_outlet(c))
            .count();
        assert!(outlets > 0, "no cell drains off-tile");
    }

    #[test]
    fn diagonal_share_is_near_half() {
        // Steepest *descent* rather than steepest drop. Comparing raw drop
        // favours the longer diagonal step and pushed this to 85%.
        let (_, f, b) = setup();
        let w = water(&f, &b);
        let (mut diag, mut total) = (0usize, 0usize);
        for y in 1..N - 1 {
            for x in 1..N - 1 {
                let Some(at) = coord(x, y) else { continue };
                if let Some(d) = w.downstream_of(at) {
                    total += 1;
                    if d.x() != at.x() && d.y() != at.y() {
                        diag += 1;
                    }
                }
            }
        }
        let pct = diag * 100 / total.max(1);
        assert!((35..=60).contains(&pct), "diagonal share is {pct}%");
    }

    #[test]
    fn drainage_never_shrinks_downstream() {
        let (_, f, b) = setup();
        let w = water(&f, &b);
        for y in 1..N - 1 {
            for x in 1..N - 1 {
                let Some(at) = coord(x, y) else { continue };
                if let Some(d) = w.downstream_of(at) {
                    assert!(w.drainage_at(d) >= w.drainage_at(at));
                }
            }
        }
    }

    #[test]
    fn strahler_never_decreases_downstream() {
        let (_, f, b) = setup();
        let w = water(&f, &b);
        for y in 0..N {
            for x in 0..N {
                let Some(at) = coord(x, y) else { continue };
                if !w.is_channel(at) {
                    continue;
                }
                if let Some(d) = w.downstream_of(at) {
                    if w.is_channel(d) {
                        assert!(
                            w.order_at(d) >= w.order_at(at),
                            "order fell from {} to {} at {x},{y}",
                            w.order_at(at),
                            w.order_at(d)
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn channel_heads_are_first_order() {
        let (_, f, b) = setup();
        let w = water(&f, &b);
        let mut heads = 0;
        for y in 0..N {
            for x in 0..N {
                let Some(at) = coord(x, y) else { continue };
                if !w.is_channel(at) {
                    continue;
                }
                let has_channel_inflow = NEIGHBOURS.iter().any(|(dx, dy)| {
                    coord(x + dx, y + dy)
                        .filter(|&nb| w.is_channel(nb) && w.downstream_of(nb) == Some(at))
                        .is_some()
                });
                if !has_channel_inflow {
                    heads += 1;
                    assert_eq!(w.order_at(at), 1, "head at {x},{y} is not order 1");
                }
            }
        }
        assert!(heads > 0, "no channel heads found");
    }

    #[test]
    fn routing_is_deterministic() {
        let (_, f, b) = setup();
        assert_eq!(water(&f, &b), water(&f, &b));
    }
}
