//! Drainage over the filled surface (artifact, Water).
//!
//! "Every land cell drains to one of its eight neighbours, so the whole map
//! is a tree with the sea and the low edges at its roots." Walking that tree
//! gives drainage area; a cell becomes a watercourse once it carries about
//! 40 litres per second, "which in a temperate climate means roughly three
//! square kilometres of catchment above it".

use super::fill::{Filled, NEIGHBOURS};
use crate::continent::bundles::{EnteringRiver, TileBundle};
use crate::noise::hash_2d;
use arda_core::{CellCoord, AREA_CELLS};

const N: i32 = AREA_CELLS as i32;

/// Discharge a cell needs before it counts as a watercourse (artifact,
/// Water): "about 40 litres per second, which in a temperate climate
/// means roughly three square kilometres of catchment" (feature 03 §Q4).
const CHANNEL_THRESHOLD_L_S: u64 = 40;

/// Row-major offset. Invariant: every caller bounds-checks `0 <= x,y < N`
/// before calling, so the product is non-negative.
#[allow(clippy::cast_sign_loss)]
fn idx(x: i32, y: i32) -> usize {
    (y * N + x) as usize
}

fn coord(x: i32, y: i32) -> Option<CellCoord> {
    CellCoord::new(u16::try_from(x).ok()?, u16::try_from(y).ok()?)
}

/// Drainage directions, accumulation, discharge, and Strahler order for
/// one tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaterGrid {
    drainage: Vec<u32>,
    discharge: Vec<u64>,
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

    /// Discharge at a cell, litres per second (feature 03 §Q4).
    #[must_use]
    pub fn discharge_at(&self, at: CellCoord) -> u64 {
        self.discharge[at.index()]
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

    /// Whether a cell carries a watercourse: discharge at or above the
    /// artifact's ~40 L/s initiation rule (feature 03 §Q4).
    #[must_use]
    pub fn is_channel(&self, at: CellCoord) -> bool {
        self.discharge[at.index()] >= CHANNEL_THRESHOLD_L_S
    }
}

/// Height just outside the tile, from the neighbour's own edge cells.
///
/// Logic/02 "Legacy outside-neighbor correction": north/west own-edge
/// samples retain their seam contract; routing uses the separate actual
/// outside samples, including all four D8 corners. Only the one-cell ring
/// is an adjacent neighbor.
fn off_tile_height(bundle: &TileBundle, x: i32, y: i32) -> Option<i32> {
    let i = |v: i32| usize::try_from(v).ok();
    match (x, y) {
        (-1, -1) => Some(bundle.outside_corners[0]),
        (N, -1) => Some(bundle.outside_corners[1]),
        (-1, N) => Some(bundle.outside_corners[2]),
        (N, N) => Some(bundle.outside_corners[3]),
        (xx, -1) if (0..N).contains(&xx) => bundle.north_outside.get(i(x)?).copied(),
        (xx, N) if (0..N).contains(&xx) => bundle.south.get(i(x)?).copied(),
        (-1, yy) if (0..N).contains(&yy) => bundle.west_outside.get(i(y)?).copied(),
        (N, yy) if (0..N).contains(&yy) => bundle.east.get(i(y)?).copied(),
        _ => None,
    }
}

/// Routes water over `filled` and returns the drainage tree.
///
/// `rain` is the tile's rainfall, millimetres per cell, row-major over
/// [`AREA_CELLS`] × [`AREA_CELLS`] (`super::area_rainfall`'s output).
#[must_use]
pub fn water(filled: &Filled, bundle: &TileBundle, rain: &[u16]) -> WaterGrid {
    let count = (N * N) as usize;
    let mut downstream: Vec<Option<u32>> = vec![None; count];
    let mut outlets = vec![false; count];
    let mut by_height: Vec<(i32, u32)> = Vec::with_capacity(count);
    // Rain lands on land cells only; sea rain belongs to the sea (mirrors
    // continent hydrology's own gate, here read off the filled surface
    // since raw heights are not available at this stage).
    let mut rain_sum = vec![0u64; count];

    for y in 0..N {
        for x in 0..N {
            let Some(at) = coord(x, y) else { continue };
            let h = filled.get(at);
            if h > 0 {
                rain_sum[idx(x, y)] = u64::from(rain[idx(x, y)]);
            }
            by_height.push((h, u32::try_from(idx(x, y)).unwrap_or(0)));

            // Steepest descent = drop ÷ distance. Compared by cross
            // multiplication so no division or float enters the choice:
            // an orthogonal step is 1000 units, a diagonal 1414, so scoring
            // by drop × (the *other* length) ranks them exactly.
            let mut best: Option<(i64, i64, u32)> = None;
            let mut best_off: Option<i64> = None;
            // Tie-break order, varied per cell.
            //
            // On smooth ground the drop to a diagonal neighbour is about
            // sqrt(2) times the drop to an orthogonal one, so their
            // drop-over-distance scores tie, and always resolving to the same
            // neighbour aligned flow to the axes: the diagonal share fell to
            // 14% and rivers came out as straight combs. Choosing among tied
            // candidates by a hash of the cell keeps the choice deterministic
            // while removing the bias.
            let jitter = hash_2d(0x00D8_71E5, x, y);

            for (k, (dx, dy)) in NEIGHBOURS.into_iter().enumerate() {
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
                // Rank ties by a per-cell rotation of the neighbour order.
                let rank = i64::from((u32::try_from(k).unwrap_or(0) + jitter) % 8);
                if best.is_none_or(|(bs, br, _)| score > bs || (score == bs && rank > br)) {
                    best = Some((score, rank, u32::try_from(idx(nx, ny)).unwrap_or(0)));
                }
            }

            let on_rim = x == 0 || y == 0 || x == N - 1 || y == N - 1;
            match (best, best_off) {
                // Leaving the tile wins: this cell is an outlet, not a sink.
                (Some((bs, _, _)), Some(off)) if off > bs => outlets[idx(x, y)] = true,
                (None, Some(_)) => outlets[idx(x, y)] = true,
                (Some((_, _, d)), _) => downstream[idx(x, y)] = Some(d),
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
    let mut seed_ls = vec![0u64; count];
    // Entering rivers seed both loads at their crossing cell before the
    // walk: the upstream catchment they carried joins drainage, and the
    // discharge they carried joins the load that becomes this cell's own
    // discharge below (feature 03 §Q4).
    for e in &bundle.entering {
        let i = e.cell.index();
        drainage[i] = drainage[i].saturating_add(e.catchment_km2.saturating_mul(100));
        seed_ls[i] += e.discharge.raw();
    }
    for &(_, i) in &by_height {
        if let Some(d) = downstream[i as usize] {
            let (i, d) = (i as usize, d as usize);
            drainage[d] = drainage[d].saturating_add(drainage[i]);
            rain_sum[d] += rain_sum[i];
            seed_ls[d] += seed_ls[i];
        }
    }

    // feature 03 §Q4: 0.5 runoff on 0.01 km² (100 m cells), plus whatever
    // discharge entering rivers already carried in from outside the tile.
    let discharge: Vec<u64> = (0..count)
        .map(|i| rain_sum[i] * 125 / 788_400 + seed_ls[i])
        .collect();

    let order = strahler(&downstream, &discharge, &by_height, &bundle.entering);

    WaterGrid {
        drainage,
        discharge,
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
///
/// Entering rivers seed their own order at their crossing cell before the
/// walk, as one upstream inflow already carrying that order: a river
/// keeps its order across tiles rather than restarting as a first-order
/// head at every seam it crosses (feature 03 §Q3).
fn strahler(
    downstream: &[Option<u32>],
    discharge: &[u64],
    by_height: &[(i32, u32)],
    entering: &[EnteringRiver],
) -> Vec<u8> {
    let count = discharge.len();
    let mut order = vec![0u8; count];
    // Highest incoming order per cell, and how many inflows carry it.
    let mut max_in = vec![0u8; count];
    let mut max_count = vec![0u16; count];

    // Round-1 review note (no behavior change): this floor only ever takes
    // effect below, where a cell becomes a channel — the loop `continue`s
    // past any cell under `CHANNEL_THRESHOLD_L_S` without ever reading
    // `max_in`/`max_count`. So when an entering river's own seed cell
    // carries < 40 L/s, the seed here is written but never consumed: the
    // seed cell is not a channel, order stays 0 there, and if the river
    // regains enough discharge further downstream it restarts at order 1
    // rather than continuing from `e.order`. That is an accepted
    // consequence of discharge-driven initiation, not a bug to route
    // around — an arid crossing genuinely is not a channel at the seed
    // (feature 03 §Q4), and §Q4's climate-driven initiation is deliberately
    // allowed to override §Q3's order-continuity guarantee when the two
    // disagree.
    for e in entering {
        let i = e.cell.index();
        max_in[i] = max_in[i].max(e.order);
        max_count[i] = max_count[i].max(1);
    }

    // by_height is high-to-low, so every upstream cell is settled first.
    for &(_, i) in by_height {
        let i = i as usize;
        if discharge[i] < CHANNEL_THRESHOLD_L_S {
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
#[path = "water_neighbor_tests.rs"]
mod neighbor_tests;

#[cfg(test)]
mod tests;
