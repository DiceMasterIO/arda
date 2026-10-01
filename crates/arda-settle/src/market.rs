//! A coarse travel lattice for weighing realm market areas (`logic/08`
//! §realm-seats step 1).
//!
//! The land is cut into square blocks of about 20,000 in all (1 km on a
//! MICRO world). A block is passable when any of its cells is land, and a
//! step between neighbouring blocks costs its length plus
//! [`CLIMB_PER_M`] metres for every metre of rise or fall between their mean
//! heights, so a mountain range parts two market areas the way it parts two
//! valleys' trade. Floods over it are multi-source Dijkstra with ties to
//! the lower source slot, so they are deterministic.

use crate::num::{isqrt, ui};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Land sample blocks aimed for.
const BLOCK_TARGET: u64 = 20_000;
/// Metres of travel one metre of climb is worth (assumed, tunable: a
/// 500 m ridge is worth a 4 km detour, as a loaded cart reckons it).
pub const CLIMB_PER_M: u64 = 8;
/// Unreached block.
pub const NONE: u16 = u16::MAX;

/// The coarse lattice.
#[derive(Debug, Clone)]
pub struct Lattice {
    /// Block edge, cells.
    pub k: usize,
    /// Blocks per row.
    pub bw: usize,
    /// Block rows.
    pub bh: usize,
    /// Land cells per block.
    pub land: Vec<u64>,
    /// Mean land height per block, metres.
    pub height_m: Vec<i64>,
}

/// A flood's outcome: owning source slot (or [`NONE`]) and cost per block.
#[derive(Debug, Clone)]
pub struct Flood {
    /// Owning slot per block.
    pub owner: Vec<u16>,
    /// Least cost per block, metres (`u64::MAX` when unreached).
    pub cost: Vec<u64>,
}

impl Lattice {
    /// Builds the lattice of a `width × height` cell grid from its land mask
    /// and cell heights (millimetres).
    #[must_use]
    pub fn new(
        width: usize,
        height: usize,
        is_land: impl Fn(usize) -> bool,
        height_mm: impl Fn(usize) -> i32,
    ) -> Self {
        let cells = u64::try_from(width * height).unwrap_or(u64::MAX);
        let k = usize::try_from(isqrt(cells / BLOCK_TARGET))
            .unwrap_or(1)
            .max(1);
        let (bw, bh) = (width.div_ceil(k), height.div_ceil(k));
        let mut land = vec![0_u64; bw * bh];
        let mut sum_mm = vec![0_i64; bw * bh];
        for y in 0..height {
            for x in 0..width {
                let i = y * width + x;
                if is_land(i) {
                    let b = (y / k) * bw + x / k;
                    land[b] += 1;
                    sum_mm[b] += i64::from(height_mm(i));
                }
            }
        }
        let height_m = land
            .iter()
            .zip(&sum_mm)
            .map(|(&n, &s)| s / i64::try_from(n.max(1)).unwrap_or(1) / 1000)
            .collect();
        Self {
            k,
            bw,
            bh,
            land,
            height_m,
        }
    }

    /// Block holding cell `(x, y)`.
    #[must_use]
    pub fn block(&self, x: u32, y: u32) -> usize {
        let k = u32::try_from(self.k).unwrap_or(1);
        usize::try_from(y / k).unwrap_or(0) * self.bw + usize::try_from(x / k).unwrap_or(0)
    }

    /// Block count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.land.len()
    }

    /// Whether the lattice has no blocks.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.land.is_empty()
    }

    /// Multi-source flood from `sources` (blocks, one per slot). Only
    /// blocks cheaper than `bound` are entered, so a single-source flood
    /// against a running minimum stays local.
    #[must_use]
    pub fn flood(&self, sources: &[usize], bound: Option<&[u64]>) -> Flood {
        let n = self.len();
        let mut owner = vec![NONE; n];
        let mut cost = vec![u64::MAX; n];
        let mut heap = BinaryHeap::new();
        for (slot, &b) in sources.iter().enumerate() {
            let slot = u16::try_from(slot).unwrap_or(NONE - 1);
            if b < n && slot < owner[b] {
                owner[b] = slot;
                cost[b] = 0;
                heap.push(Reverse((0_u64, slot, b)));
            }
        }
        let step_m = ui(self.k) * 100;
        while let Some(Reverse((d, slot, a))) = heap.pop() {
            if d > cost[a] || owner[a] != slot {
                continue;
            }
            let (ax, ay) = (ui(a % self.bw), ui(a / self.bw));
            for (dx, dy) in [
                (-1, -1),
                (0, -1),
                (1, -1),
                (-1, 0),
                (1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
            ] {
                let (bx, by) = (ax + dx, ay + dy);
                if bx < 0 || by < 0 || bx >= ui(self.bw) || by >= ui(self.bh) {
                    continue;
                }
                let b = usize::try_from(by * ui(self.bw) + bx).unwrap_or(0);
                if self.land[b] == 0 {
                    continue;
                }
                let len = if dx != 0 && dy != 0 {
                    step_m * 1414 / 1000
                } else {
                    step_m
                };
                let climb = (self.height_m[a] - self.height_m[b]).unsigned_abs();
                let nd = d + u64::try_from(len).unwrap_or(0) + climb * CLIMB_PER_M;
                if bound.is_some_and(|m| nd >= m[b]) {
                    continue;
                }
                if nd < cost[b] || (nd == cost[b] && slot < owner[b]) {
                    cost[b] = nd;
                    owner[b] = slot;
                    heap.push(Reverse((nd, slot, b)));
                }
            }
        }
        Flood { owner, cost }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ridge_parts_two_market_areas() {
        // 100 × 20 land with a 600 m ridge at x = 30: the western source's
        // area stops at the ridge although the ridge is nearer to it.
        let lat = Lattice::new(
            100,
            20,
            |_| true,
            |i| if i % 100 == 30 { 600_000 } else { 0 },
        );
        assert_eq!(lat.k, 1);
        let f = lat.flood(&[lat.block(10, 10), lat.block(70, 10)], None);
        assert_eq!(f.owner[lat.block(25, 10)], 0);
        assert_eq!(f.owner[lat.block(35, 10)], 1);
        let g = lat.flood(&[lat.block(10, 10), lat.block(70, 10)], None);
        assert_eq!(f.owner, g.owner);
    }
}
