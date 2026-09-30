//! Shared modeled-domain landscape evolution (`logic/02`, shared terrain correction).
//!
//! Publication areas only partition storage. Uplift, contributing area, creep
//! and collapse cross every former 512-cell cut. The actual modeled outer rim
//! remains fixed; no tile-local taper or tile-local uplift maximum survives.

// Dimensions and total cells are checked before allocation; indices fit u32,
// coordinate arithmetic fits i64, and negative neighbours are rejected first.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use crate::hydrology::HydrologyError;
use std::{cmp::Reverse, collections::BinaryHeap, mem::size_of};

use super::{fill::NEIGHBOURS, mfd};

pub(crate) const SHARED_ITERATIONS: u32 = 160;

const UPLIFT_PEAK_MM: i64 = 900;
const CREEP_NUM: i64 = 3;
const CREEP_DEN: i64 = 100;
const TALUS_TAN_1000: i64 = 700;
const COLLAPSE_PASSES: u64 = 8;

type QueueEntry = Reverse<(i64, u32)>;

#[derive(Clone, Copy)]
struct Grid {
    width: usize,
    height: usize,
    count: usize,
}

impl Grid {
    fn checked(width: usize, height: usize) -> Result<Self, HydrologyError> {
        let count = width
            .checked_mul(height)
            .filter(|&n| width >= 2 && height >= 2 && n <= u32::MAX as usize)
            .ok_or(HydrologyError::TerrainPreparation(
                "shared terrain dimensions exceed the u32 cell domain",
            ))?;
        if count as u128 * size_of::<QueueEntry>() as u128 > isize::MAX as u128 {
            return Err(HydrologyError::TerrainPreparation(
                "shared terrain buffers exceed addressable allocation size",
            ));
        }
        Ok(Self {
            width,
            height,
            count,
        })
    }

    fn boundary(self, i: usize) -> bool {
        let (x, y) = (i % self.width, i / self.width);
        x == 0 || y == 0 || x + 1 == self.width || y + 1 == self.height
    }

    fn neighbours(self, i: usize) -> impl Iterator<Item = (usize, bool)> {
        let (x, y) = ((i % self.width) as i64, (i / self.width) as i64);
        NEIGHBOURS.into_iter().filter_map(move |(dx, dy)| {
            let (nx, ny) = (x + i64::from(dx), y + i64::from(dy));
            (nx >= 0 && ny >= 0 && nx < self.width as i64 && ny < self.height as i64)
                .then(|| (ny as usize * self.width + nx as usize, dx != 0 && dy != 0))
        })
    }
}

struct Scratch {
    next: Vec<i32>,
    physical: Vec<i32>,
    routing: Vec<i64>,
    order: Vec<u32>,
    area: Vec<u64>,
    seen: Vec<u8>,
    queue: BinaryHeap<QueueEntry>,
}

/// Exact requested owned payload, including scratch container headers. The
/// caller admits this plus its already-owned heights/coarse fields before
/// constructing output. Allocator metadata is outside the owned-payload budget.
/// Every vector reserves once and the heap can contain at most one entry per
/// cell, so no iteration grows or doubles any allocation.
pub(crate) fn scratch_bytes(width: usize, height: usize) -> Result<u128, HydrologyError> {
    let grid = Grid::checked(width, height)?;
    let per_cell = 2 * size_of::<i32>()
        + size_of::<i64>()
        + size_of::<u32>()
        + size_of::<u64>()
        + size_of::<u8>()
        + size_of::<QueueEntry>();
    Ok(grid.count as u128 * per_cell as u128 + size_of::<Scratch>() as u128)
}

/// Conservative declared work for sampling and evolving the shared rectangle.
///
/// Logic/02 shared terrain admission counts a bounded cell/neighbor transition or
/// heap comparison as one unit, not each arithmetic instruction or elapsed time.
/// Per step, two floods each enqueue/pop every cell once (at most three heap
/// comparisons per level per cell), MFD visits at most eight receivers in four
/// passes, and uplift/creep/incision visit each neighborhood once. Collapse makes
/// at most eight nine-visit cell/neighbor passes. The fixed 256-unit cell allowance
/// covers these scans and array passes; eight units per heap level cover both
/// floods. A separate 640-unit initial allowance covers regional/detail sampling,
/// normalization, scratch initialization and sequential bundle construction.
/// It includes 128 extra units for eight cached coarse reads and two integer
/// lattice-noise evaluations per cell for structural relief. Regional-detail
/// radius-ten fallbacks number at most 60*(width+height), or 0.1875 per cell
/// on admitted axes >=640. Structural-relief radius-fifty fallbacks number at
/// most 800*(width+height), or 2.5 per cell on those axes. Extra bundle
/// interpolation also fits this allowance (`logic/02`, Regional detail
/// correction). No additional dense field is retained. The loops have no
/// data-dependent retry, so admission requires no runtime model-changing cap.
pub(crate) fn work_operations(width: usize, height: usize) -> Result<u64, HydrologyError> {
    let grid = Grid::checked(width, height)?;
    let count = grid.count as u64;
    let heap_levels = u64::from(u64::BITS - (count - 1).leading_zeros());
    let per_step = 184 + COLLAPSE_PASSES * 9 + 8 * heap_levels;
    count
        .checked_mul(640 + u64::from(SHARED_ITERATIONS) * per_step)
        .ok_or(HydrologyError::Overflow("shared terrain work"))
}

fn allocated<T: Clone>(count: usize, initial: T) -> Result<Vec<T>, HydrologyError> {
    let mut values = Vec::new();
    values.try_reserve_exact(count).map_err(|_| {
        HydrologyError::TerrainPreparation("shared terrain scratch allocation failed")
    })?;
    values.resize(count, initial);
    Ok(values)
}

impl Scratch {
    fn new(grid: Grid) -> Result<Self, HydrologyError> {
        let mut queue = BinaryHeap::new();
        queue.try_reserve_exact(grid.count).map_err(|_| {
            HydrologyError::TerrainPreparation("shared terrain flood allocation failed")
        })?;
        Ok(Self {
            next: allocated(grid.count, 0)?,
            physical: allocated(grid.count, 0)?,
            routing: allocated(grid.count, 0)?,
            order: allocated(grid.count, 0)?,
            area: allocated(grid.count, 0)?,
            seen: allocated(grid.count, 0)?,
            queue,
        })
    }

    // A zero increment computes the exact physical spill. A one-millimetre
    // increment computes strictly descending temporary routing. Keeping those
    // surfaces separate prevents epsilon from exempting physical flats from
    // incision. No legacy basin objects or publication-area boundary are used.
    fn flood(&mut self, heights: &[i32], grid: Grid, increment: i64) {
        self.seen.fill(0);
        self.queue.clear();
        let mut seed = |i: usize| {
            self.seen[i] = 1;
            self.routing[i] = i64::from(heights[i]);
            self.queue.push(Reverse((self.routing[i], i as u32)));
        };
        for x in 0..grid.width {
            seed(x);
            seed((grid.height - 1) * grid.width + x);
        }
        for y in 1..grid.height - 1 {
            seed(y * grid.width);
            seed(y * grid.width + grid.width - 1);
        }
        let mut rank = 0;
        while let Some(Reverse((level, raw))) = self.queue.pop() {
            let i = raw as usize;
            self.order[rank] = raw;
            rank += 1;
            for (j, _) in grid.neighbours(i) {
                if self.seen[j] != 0 {
                    continue;
                }
                self.seen[j] = 1;
                let filled = i64::from(heights[j]).max(level + increment);
                self.routing[j] = filled;
                // Each cell is enqueued once. The fixed grid-sized capacity
                // therefore covers even the worst simultaneous frontier.
                self.queue.push(Reverse((filled, j as u32)));
            }
        }
        debug_assert_eq!(rank, grid.count, "rectangular flood visits every cell");
    }
}

pub(crate) fn evolve(
    heights: &mut [i32],
    coarse: &[i32],
    width: usize,
    height: usize,
) -> Result<(), HydrologyError> {
    evolve_steps(heights, coarse, width, height, SHARED_ITERATIONS)
}

fn evolve_steps(
    heights: &mut [i32],
    coarse: &[i32],
    width: usize,
    height: usize,
    iterations: u32,
) -> Result<(), HydrologyError> {
    let grid = Grid::checked(width, height)?;
    if heights.len() != grid.count || coarse.len() != grid.count {
        return Err(HydrologyError::TerrainPreparation(
            "shared terrain input length does not match the modeled rectangle",
        ));
    }
    let peak = coarse.iter().copied().max().unwrap_or(1).max(1);
    let mut scratch = Scratch::new(grid)?;
    for _ in 0..iterations {
        scratch.flood(heights, grid, 0);
        for (physical, &level) in scratch.physical.iter_mut().zip(&scratch.routing) {
            // Physical flood levels are maxima of original i32 heights only.
            *physical = level as i32;
        }
        scratch.flood(heights, grid, 1);
        mfd::accumulate_rectangular(
            &scratch.routing,
            width,
            height,
            &scratch.order,
            &mut scratch.area,
        )?;
        scratch.next.copy_from_slice(heights);
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let i = y * width + x;
                let h = heights[i];
                if h <= 0 {
                    continue;
                }
                let uplift = UPLIFT_PEAK_MM * i64::from(coarse[i]).max(0) / i64::from(peak);
                let orth = i64::from(heights[i - 1])
                    + i64::from(heights[i + 1])
                    + i64::from(heights[i - width])
                    + i64::from(heights[i + width]);
                let diagonal = i64::from(heights[i - width - 1])
                    + i64::from(heights[i - width + 1])
                    + i64::from(heights[i + width - 1])
                    + i64::from(heights[i + width + 1]);
                let laplacian = (4 * orth + diagonal - 20 * i64::from(h)) / 6;
                let creep = CREEP_NUM * laplacian / CREEP_DEN;
                scratch.next[i] =
                    (i64::from(h) + uplift + creep).clamp(1, i64::from(i32::MAX)) as i32;
            }
        }
        // logic/02 shared terrain correction: first form the uplift/creep bed,
        // then solve n=1 stream power downstream first against each receiver's
        // updated bed. An explicit cap against its old bed lets a rising
        // receiver strand a newly incised upstream cell in an artificial pit.
        // Every receiver has a lower temporary routing level, so flood order
        // ensures its bed is already final for this incision step.
        for &raw in &scratch.order {
            let i = raw as usize;
            if heights[i] <= 0 || grid.boundary(i) {
                continue;
            }
            let incision = physical_incision(
                heights,
                &scratch.next,
                &scratch.routing,
                scratch.physical[i],
                scratch.area[i],
                grid,
                i,
            );
            scratch.next[i] =
                (i64::from(scratch.next[i]) - incision).clamp(1, i64::from(i32::MAX)) as i32;
        }
        collapse(&mut scratch.next, grid);
        heights.copy_from_slice(&scratch.next);
    }
    Ok(())
}

fn physical_incision(
    heights: &[i32],
    provisional: &[i32],
    routing: &[i64],
    physical_spill: i32,
    area: u64,
    grid: Grid,
    i: usize,
) -> i64 {
    // logic/02 shared terrain correction: a genuine submerged bed cannot be
    // incised towards a higher spill; a routing-only flat receives no exemption.
    if physical_spill > heights[i] {
        return 0;
    }
    let mut downstream = None;
    let mut best_score = 0;
    for (j, diagonal) in grid.neighbours(i) {
        let drop = routing[i] - routing[j];
        let score = drop * if diagonal { 1000 } else { 1414 };
        if score > best_score {
            best_score = score;
            downstream = Some((j, diagonal));
        }
    }
    let Some((j, diagonal)) = downstream else {
        return 0;
    };
    let physical_drop = (i64::from(provisional[i]) - i64::from(provisional[j])).max(0);
    implicit_incision(area, physical_drop, diagonal)
}

fn implicit_incision(area_q32: u64, drop_mm: i64, diagonal: bool) -> i64 {
    // Same K, m=1/2, n=1 and one iteration of time as the retained stream-power
    // law: c = sqrt(A) * 1000 / run_mm. Backward Euler gives incision c*drop/(1+c).
    // Keep the full rational slope instead of flooring to per-mille first.
    // Flooring the removed material preserves any positive receiver separation
    // by at least 1 mm. All products fit u128 under the checked u32 cell domain.
    let coefficient = mfd::isqrt(u128::from(area_q32) << 32) * 1000;
    let run_mm = if diagonal { 141_400_u128 } else { 100_000 };
    let denominator = u128::from(mfd::ONE) * run_mm + coefficient;
    (coefficient * drop_mm.max(0) as u128 / denominator) as i64
}

fn collapse(heights: &mut [i32], grid: Grid) {
    for _ in 0..COLLAPSE_PASSES {
        let mut changed = false;
        for y in 1..grid.height - 1 {
            for x in 1..grid.width - 1 {
                let i = y * grid.width + x;
                if heights[i] <= 0 {
                    continue;
                }
                for (j, diagonal) in grid.neighbours(i) {
                    let drop = i64::from(heights[i]) - i64::from(heights[j]);
                    let run = if diagonal { 141_400 } else { 100_000 };
                    let excess = drop - run * TALUS_TAN_1000 / 1000;
                    if excess <= 0 {
                        continue;
                    }
                    if heights[j] <= 0 || grid.boundary(j) {
                        heights[i] =
                            (i64::from(heights[i]) - excess).clamp(1, i64::from(i32::MAX)) as i32;
                    } else {
                        let half = (excess / 2) as i32;
                        heights[i] -= half;
                        heights[j] += half;
                    }
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
}

#[cfg(test)]
#[path = "evolution_tests.rs"]
mod tests;
