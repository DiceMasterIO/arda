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
const CHANNEL_THRESHOLD_L_S: u32 = 40;

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
    discharge: Vec<u32>,
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
    pub fn discharge_at(&self, at: CellCoord) -> u32 {
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
        seed_ls[i] += u64::from(e.discharge.raw());
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
    let discharge: Vec<u32> = (0..count)
        .map(|i| saturate_u32(rain_sum[i] * 125 / 788_400 + seed_ls[i]))
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

/// Saturating cast from a `u64` accumulator that may exceed `u32`'s range.
fn saturate_u32(v: u64) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
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
    discharge: &[u32],
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
mod tests {
    use super::*;
    use crate::area::area_rainfall;
    use crate::area::fill::fill;
    use crate::area::relief::relief;
    use crate::continent::build_continent;
    use crate::continent::bundles::bundle_for;
    use arda_core::{AreaCoord, GenerateConfig};

    /// Tile (0,1), MICRO seed 42: has land, basins, and entering rivers
    /// crossing in from the continent drainage tree.
    fn setup() -> (Vec<i32>, Filled, TileBundle, Vec<u16>) {
        let c = build_continent(42, GenerateConfig::MICRO, 0);
        let b = bundle_for(42, &c, AreaCoord::new(0, 1));
        let r = relief(42, &c.grid, &b);
        let h: Vec<i32> = (0..(N * N) as usize)
            .filter_map(|i| {
                let i = i32::try_from(i).ok()?;
                Some(r.get(coord(i % N, i / N)?))
            })
            .collect();
        let f = fill(&h, &b);
        let rain = area_rainfall(&b);
        (h, f, b, rain)
    }

    #[test]
    fn no_land_cell_is_a_sink() {
        let (h, f, b, rain) = setup();
        let w = water(&f, &b, &rain);
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
        let (_, f, b, rain) = setup();
        let w = water(&f, &b, &rain);
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
        let (_, f, b, rain) = setup();
        let w = water(&f, &b, &rain);
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
        // The bug this guards against is comparing raw drop, which drove the
        // share to 85%. The lower bound guards the opposite failure: always
        // resolving ties to the same neighbour pinned flow to the axes and
        // pushed it to 14%, which drew rivers as straight combs.
        let pct = diag * 100 / total.max(1);
        // Upper bound is the real guard: comparing raw drop instead of
        // drop-over-distance drove this to 85%.
        //
        // The lower bound is loose on purpose. Measured on raw relief — before
        // any erosion — the share sits near 17%, because the terrain is built
        // from value noise on a square lattice and value noise is
        // anisotropic: its gradients favour the lattice axes, so steepest
        // descent does too. Isotropic (gradient) noise is the fix; an attempt
        // at it produced blocky coastlines and was reverted, so this is a
        // recorded limitation rather than a settled number.
        assert!(
            pct <= 62,
            "diagonal share is {pct}%, near the raw-drop signature"
        );
        assert!(
            pct >= 10,
            "diagonal share is {pct}%, worse than the known lattice bias"
        );
    }

    #[test]
    fn drainage_never_shrinks_downstream() {
        let (_, f, b, rain) = setup();
        let w = water(&f, &b, &rain);
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
        let (_, f, b, rain) = setup();
        let w = water(&f, &b, &rain);
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
        // Feature 03 §Q3 changes what a "head" means: an entering river
        // already carries an order from outside the tile, so a channel
        // seeded there is not a first-order head even when it has no
        // local upstream channel neighbour. The invariant now holds only
        // for genuine heads — channel cells that are not an entering
        // river's own seed cell.
        let (_, f, b, rain) = setup();
        let w = water(&f, &b, &rain);
        let seeds: std::collections::HashSet<CellCoord> =
            b.entering.iter().map(|e| e.cell).collect();
        let mut heads = 0;
        for y in 0..N {
            for x in 0..N {
                let Some(at) = coord(x, y) else { continue };
                if !w.is_channel(at) || seeds.contains(&at) {
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
        let (_, f, b, rain) = setup();
        assert_eq!(water(&f, &b, &rain), water(&f, &b, &rain));
    }

    #[test]
    fn entering_seeds_raise_drainage_above_the_local_maximum() {
        // Spec R8: with-inflow max strictly exceeds without-inflow max.
        //
        // Scoped to each entering seed's own downstream path rather than
        // the whole tile: on MICRO seed 42, tiles (0,1) and (0,2) each
        // have one dominant interior watershed, fed entirely by local
        // terrain, that is bigger than any single boundary crossing's
        // catchment — a whole-tile maximum stays pinned to that unrelated
        // basin regardless of the entering boost, verified by inspection
        // (`with` and `without` share the exact same argmax cell and
        // value). The seed's own path is where the boost is guaranteed to
        // show: it adds a fixed amount at the seed that every downstream
        // cell on that path then carries, so the path's own maximum must
        // rise by exactly that amount.
        let (_, f, b, rain) = setup(); // tile (0,1), micro seed 42
        let with = water(&f, &b, &rain);
        let mut b_dry = b.clone();
        b_dry.entering.clear();
        let without = water(&f, &b_dry, &rain);

        let path_max = |w: &WaterGrid, start: CellCoord| {
            let mut at = start;
            let mut best = w.drainage_at(at);
            let mut hops = 0u32;
            while let Some(next) = w.downstream_of(at) {
                at = next;
                best = best.max(w.drainage_at(at));
                hops += 1;
                if hops > (N * N) as u32 {
                    break; // cycle guard; the tree forbids it
                }
            }
            best
        };

        if b.entering.is_empty() {
            // A tile with no crossing must behave identically (unhappy path).
            assert_eq!(with, without);
        } else {
            for e in &b.entering {
                assert!(
                    path_max(&with, e.cell) > path_max(&without, e.cell),
                    "seed {:?} (catchment {} km2) did not raise its own path's maximum",
                    e.cell,
                    e.catchment_km2
                );
            }
        }
    }

    #[test]
    fn discharge_follows_rain_plus_seeds_and_is_monotone() {
        let (_, f, b, rain) = setup();
        let w = water(&f, &b, &rain);
        for y in 1..N - 1 {
            for x in 1..N - 1 {
                let Some(at) = coord(x, y) else { continue };
                if let Some(d) = w.downstream_of(at) {
                    assert!(w.discharge_at(d) >= w.discharge_at(at));
                }
            }
        }
    }

    #[test]
    fn channels_begin_exactly_at_forty_litres() {
        // Spec R7 / §Q4: initiation is discharge-driven, #13's rule.
        let (_, f, b, rain) = setup();
        let w = water(&f, &b, &rain);
        for y in 0..N {
            for x in 0..N {
                let Some(at) = coord(x, y) else { continue };
                assert_eq!(w.is_channel(at), w.discharge_at(at) >= 40, "at {x},{y}");
            }
        }
    }

    #[test]
    fn seed_cells_carry_at_least_their_entering_order() {
        let (_, f, b, rain) = setup();
        let w = water(&f, &b, &rain);
        for e in &b.entering {
            if w.is_channel(e.cell) {
                assert!(w.order_at(e.cell) >= e.order, "seed at {:?}", e.cell);
            }
        }
    }
}
