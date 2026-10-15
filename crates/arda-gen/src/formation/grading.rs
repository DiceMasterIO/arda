//! Recipe-9 fill grading (logic/02 §fine-formation drainage; review 3.1-3).
//!
//! A priority-flood fill raises every closed depression to its spill and
//! grades the flooded cells by a small step per cell so they drain. Earlier
//! recipes count that step in breadth-first grid steps from the spill:
//! square (Chebyshev) rings of equal height, across which steepest descent
//! picks the cardinal neighbour, so flow over every fill flat runs as
//! straight axis-parallel lines into the spill's row or column. The same
//! rings come from every fill at every level, and they are the likeliest
//! shared root of open item OI-03 (near-straight reaches leaning to the
//! axes and diagonals).
//!
//! Recipe 9 grades by a weighted geodesic instead: a step costs `eps` on a
//! cardinal and `eps·√2` on a diagonal, jittered ±40% per cell by a hash,
//! with fractional millimetres rounded up or down by the same hash. The
//! flooded cells then drain along wandering, roughly radial paths. Every
//! raised cell still sits above the neighbour that raised it, so the
//! result drains exactly as before.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::drainage::{neighbour, FIXED};
use super::FormationError;
use crate::noise::hash_2d;

/// Cost in millimetres of grading cell `i` (of a `width`-wide lattice) one
/// step from its neighbour in direction `k` (0..4 cardinal, 4..8 diagonal),
/// at a nominal `eps_mm` per cardinal step. At least 1 mm.
#[must_use]
pub fn step_mm(seed: u64, eps_mm: i32, i: usize, width: usize, k: usize) -> i32 {
    let (x, y) = (
        i32::try_from(i % width).unwrap_or(0),
        i32::try_from(i / width).unwrap_or(0),
    );
    let hash = hash_2d(seed ^ 0x6EAD_E009, x, y);
    // Q8: √2 is 362/256; the jitter spans 0.6..1.4 (154..358).
    let base_q8 = i64::from(eps_mm.max(1)) * if k < 4 { 256 } else { 362 };
    let jitter_q8 = 154 + i64::from(hash % 205);
    let cost_q8 = base_q8 * jitter_q8 / 256;
    let round_up = i64::from((hash >> 16) % 256) < cost_q8 % 256;
    i32::try_from(cost_q8 / 256 + i64::from(round_up))
        .unwrap_or(i32::MAX)
        .max(1)
}

/// [`super::drainage::fill`] with the recipe-9 grading: every closed
/// depression rises to its spill, and its flooded cells are graded by
/// [`step_mm`] along a weighted geodesic from the spill (module docs).
/// `next` and `closed` are scratch.
///
/// # Errors
/// Allocation failure for the bucket heads.
#[allow(clippy::too_many_arguments)]
pub fn fill_graded(
    z: &mut [i32],
    width: usize,
    height: usize,
    flags: &[u8],
    eps_mm: i32,
    seed: u64,
    next: &mut [u32],
    closed: &mut [u8],
) -> Result<(), FormationError> {
    const NONE: u32 = u32::MAX;
    let n = z.len();
    if n == 0 {
        return Ok(());
    }
    let (lo, hi) = z
        .iter()
        .fold((i32::MAX, i32::MIN), |(a, b), &v| (a.min(v), b.max(v)));
    let range = usize::try_from(i64::from(hi) - i64::from(lo) + 1)
        .map_err(|_| FormationError::ArithmeticOverflow)?;
    let mut head: Vec<u32> = super::lattice::alloc(range)?;
    let mut tail: Vec<u32> = super::lattice::alloc(range)?;
    head.fill(NONE);
    closed.fill(0);
    let push_bucket = |head: &mut [u32], tail: &mut [u32], next: &mut [u32], b: usize, i: u32| {
        next[i as usize] = NONE;
        if head[b] == NONE {
            head[b] = i;
        } else {
            next[tail[b] as usize] = i;
        }
        tail[b] = i;
    };
    let bucket = |v: i32| usize::try_from(i64::from(v) - i64::from(lo)).unwrap_or(0);
    for i in 0..n {
        if flags[i] & FIXED != 0 {
            closed[i] = 1;
            push_bucket(&mut head, &mut tail, next, bucket(z[i]), i as u32);
        }
    }
    // Raised cells wait in a min-heap on their graded height, so the flood
    // over a depression advances as a weighted wavefront, not in rings.
    let mut pit: BinaryHeap<Reverse<(i32, u32)>> = BinaryHeap::new();
    let mut b = 0_usize;
    loop {
        while b < range && head[b] == NONE {
            b += 1;
        }
        let from_pit = match (pit.peek(), b < range) {
            (Some(&Reverse((pz, _))), true) => i64::from(pz) < i64::from(lo) + b as i64,
            (Some(_), false) => true,
            (None, true) => false,
            (None, false) => break,
        };
        let c = if from_pit {
            match pit.pop() {
                Some(Reverse((_, c))) => c as usize,
                None => break,
            }
        } else {
            let c = head[b] as usize;
            head[b] = next[c];
            c
        };
        let zc = z[c];
        for k in 0..8 {
            let Some(ni) = neighbour(c, width, height, k) else {
                continue;
            };
            if closed[ni] != 0 {
                continue;
            }
            closed[ni] = 1;
            if z[ni] <= zc {
                z[ni] = zc.saturating_add(step_mm(seed, eps_mm, ni, width, k));
                pit.push(Reverse((z[ni], ni as u32)));
            } else {
                let nb = bucket(z[ni]);
                push_bucket(&mut head, &mut tail, next, nb, ni as u32);
                if nb < b {
                    b = nb;
                }
            }
        }
    }
    Ok(())
}

/// Grades one locally flooded region (recipe 9; [`super::drainage::fill_local`]):
/// from `outlet`, every cell stamped `popped` is raised to at least `level`
/// plus its weighted geodesic cost, and stamped `done`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn grade_region(
    z: &mut [i32],
    width: usize,
    height: usize,
    stamp: &mut [u32],
    (outlet, level): (usize, i32),
    (popped, done): (u32, u32),
    eps_mm: i32,
    seed: u64,
) {
    let mut heap: BinaryHeap<Reverse<(i32, u32)>> = BinaryHeap::new();
    heap.push(Reverse((level, outlet as u32)));
    let mut first = true;
    while let Some(Reverse((target, c))) = heap.pop() {
        let c = c as usize;
        if !first {
            if stamp[c] == done {
                continue;
            }
            stamp[c] = done;
            if z[c] < target {
                z[c] = target;
            }
        }
        first = false;
        for k in 0..8 {
            let Some(nb) = neighbour(c, width, height, k) else {
                continue;
            };
            if stamp[nb] == popped {
                let cand = target.saturating_add(step_mm(seed, eps_mm, nb, width, k));
                heap.push(Reverse((cand, nb as u32)));
            }
        }
    }
}

#[cfg(test)]
#[path = "grading_tests.rs"]
mod tests;
