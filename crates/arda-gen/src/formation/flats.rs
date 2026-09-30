//! Wandering drainage across fill flats (logic/02 §fine-formation flats).
//!
//! Every depression fill leaves its flooded cells at the spill level plus a
//! millimetre gradient counted in grid steps to the outlet. Flow on such a
//! surface follows grid geodesics: dead-straight axis and 45° lines that
//! radiate from the outlet across a sediment-filled basin or a drowned
//! valley's infill. The seed-42 full-size world carried a 60 km plain at
//! 63 m whose rivers ran as straight diagonals and meridians.
//!
//! A real alluvial fill is almost as flat, but its rivers wander. Here the
//! flats are regraded as a cost-weighted geodesic from their outlets: each
//! step costs 1-5 mm (about 0.03-0.13‰) by smooth, rotated noise on a
//! 2.4 km and 900 m scale. Geodesics bend into the cheap corridors and
//! coalesce there, so drainage converges into sinuous trunks at the scale
//! of the plain, and the surface stays within millimetres per cell of the
//! flat it replaces. Only cells that already drain are touched, and every
//! regraded cell keeps a strictly lower parent, so the result drains.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use rayon::prelude::*;

use super::drainage::{neighbour, FIXED};
use super::lattice::{alloc, Lattice};
use super::FormationError;
use crate::noise::value_noise;

/// A land cell whose steepest drop to any neighbour is at most this is on
/// a flat, millimetres: fills grade 1-2 mm per step, estuarine infill
/// about 6 mm per 39 m step.
pub const FLAT_DROP_MM: i32 = 8;
/// Cheapest and dearest step across a flat, millimetres per cardinal step.
pub const STEP_MIN_MM: i64 = 1;
/// Dearest step across a flat, millimetres per cardinal step.
pub const STEP_MAX_MM: i64 = 5;

/// Step cost (mm per cardinal step) at world position `(xm, ym)` metres:
/// two rotated octaves of value noise, squared towards the cheap end so
/// cheap corridors are narrow and drainage gathers into them.
#[must_use]
pub fn step_cost_mm(seed: u64, xm: i64, ym: i64) -> i64 {
    let (ax, ay) = (
        ((xm * 3_271 - ym * 2_465) / 4_096) as i32,
        ((xm * 2_465 + ym * 3_271) / 4_096) as i32,
    );
    let (bx, by) = (
        ((xm * 3_770 + ym * 1_600) / 4_096) as i32,
        ((-xm * 1_600 + ym * 3_770) / 4_096) as i32,
    );
    let v = i64::from(value_noise(seed ^ 0xF1A7, ax, ay, 2_400)) * 2
        + i64::from(value_noise(seed ^ 0xF1A8, bx, by, 900));
    // v spans ±3 × 32768 but rarely leaves ±1.5 × 32768: map that to
    // 0..=1024, clamped.
    let u = ((v + 49_152) * 1_024 / 98_304).clamp(0, 1_024);
    STEP_MIN_MM + (STEP_MAX_MM - STEP_MIN_MM) * u * u / (1_024 * 1_024)
}

/// Regrades the flats of a drained lattice. `flags` marks fixed cells (the
/// open sea and protected sinks); they are never changed. Returns the
/// number of regraded cells.
///
/// # Errors
/// Allocation failure.
pub fn regrade(g: &mut Lattice, flags: &[u8], seed: u64) -> Result<usize, FormationError> {
    let (w, h) = (g.width, g.height);
    let n = w * h;
    let d_m = g.spacing_um / 1_000_000;
    // State: 0 not on a flat, 1 flat and waiting, 2 flat and final.
    let mut state: Vec<u8> = alloc(n)?;
    {
        let z = &g.z;
        state.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
            for (x, s) in row.iter_mut().enumerate() {
                let i = y * w + x;
                if flags[i] & FIXED != 0
                    || z[i] <= 0
                    || x == 0
                    || y == 0
                    || x + 1 == w
                    || y + 1 == h
                {
                    continue;
                }
                let drop = (0..8)
                    .filter_map(|k| neighbour(i, w, h, k))
                    .map(|j| z[i] - z[j])
                    .max()
                    .unwrap_or(0);
                if drop > 0 && drop <= FLAT_DROP_MM {
                    *s = 1;
                }
            }
        });
    }
    // Outlets: flat cells beside a lower cell off the flat keep their height.
    let mut heap = BinaryHeap::new();
    for i in 0..n {
        if state[i] != 1 {
            continue;
        }
        let outlet = (0..8)
            .filter_map(|k| neighbour(i, w, h, k))
            .any(|j| state[j] == 0 && g.z[j] < g.z[i]);
        if outlet {
            heap.push(Reverse((g.z[i], i)));
        }
    }
    let mut regraded = 0;
    while let Some(Reverse((zi, i))) = heap.pop() {
        if state[i] == 2 {
            continue;
        }
        state[i] = 2;
        g.z[i] = zi;
        regraded += 1;
        for k in 0..8 {
            let Some(j) = neighbour(i, w, h, k) else {
                continue;
            };
            if state[j] != 1 {
                continue;
            }
            let (xm, ym) = ((j % w) as i64 * d_m, (j / w) as i64 * d_m);
            let c = step_cost_mm(seed, xm, ym);
            // Diagonal steps cost √2 as much.
            let step = if k < 4 { c } else { c * 181 / 128 };
            let cand = i32::try_from(i64::from(zi) + step.max(1)).unwrap_or(i32::MAX);
            heap.push(Reverse((cand, j)));
        }
    }
    Ok(regraded)
}

#[cfg(test)]
mod tests {
    use super::super::drainage::{fill, receivers, upstream_order, SELF};
    use super::*;

    /// A 20 x 10 km flat valley floor at 60 m (a fill: 2 mm per step to an
    /// outlet at the east end), walled by hills.
    fn valley() -> (Lattice, Vec<u8>) {
        let (w, h) = (513, 257);
        let mut g = Lattice::new(w, h, 39_062_500).unwrap();
        for y in 0..h {
            for x in 0..w {
                let wall = (y as i32 - 128).abs() > 100;
                g.z[y * w + x] = if x + 1 == w {
                    -5_000
                } else if wall {
                    90_000 + (y as i32 - 128).abs() * 100
                } else {
                    60_000
                };
            }
        }
        let mut flags = vec![0; w * h];
        for y in 0..h {
            flags[y * w + w - 1] = FIXED;
        }
        let (mut next, mut closed) = (vec![0; w * h], vec![0; w * h]);
        // The floor is a closed flat: the fill grades it to the east.
        for y in 0..h {
            let i = y * w + w - 2;
            if (y as i32 - 128).abs() <= 100 {
                g.z[i] = 59_000;
            }
        }
        fill(&mut g.z, w, h, &flags, 2, &mut next, &mut closed).unwrap();
        (g, flags)
    }

    fn drains(g: &Lattice, flags: &[u8]) -> bool {
        let (w, h) = (g.width, g.height);
        (0..w * h).all(|i| {
            let (x, y) = (i % w, i / w);
            flags[i] != 0
                || x == 0
                || y == 0
                || x + 1 == w
                || y + 1 == h
                || (0..8).any(|k| neighbour(i, w, h, k).is_some_and(|j| g.z[j] < g.z[i]))
        })
    }

    /// Longest run of identical D8 steps along the largest trunk.
    fn straightest_run(g: &Lattice, flags: &[u8]) -> usize {
        let (w, h) = (g.width, g.height);
        let mut rcv = vec![0_u8; w * h];
        receivers(&g.z, w, h, flags, 0, 0, &mut rcv);
        let (mut indeg, mut order) = (vec![0_u8; w * h], vec![0_u32; w * h]);
        upstream_order(&rcv, w, h, &mut indeg, &mut order);
        let mut area = vec![1_u32; w * h];
        for &i in &order {
            let i = i as usize;
            if rcv[i] != SELF {
                let r = super::super::drainage::receiver_index(i, w, h, rcv[i]);
                area[r] += area[i];
            }
        }
        let mut best = 0;
        for i in 0..w * h {
            if area[i] < 2_000 || rcv[i] == SELF {
                continue;
            }
            let (mut run, mut cur) = (0, i);
            while rcv[cur] == rcv[i] && area[cur] >= 2_000 {
                run += 1;
                cur = super::super::drainage::receiver_index(cur, w, h, rcv[cur]);
            }
            best = best.max(run);
        }
        best
    }

    #[test]
    fn a_fill_flat_drains_along_wandering_trunks() {
        let (mut g, flags) = valley();
        assert!(drains(&g, &flags));
        let before = straightest_run(&g, &flags);
        let n = regrade(&mut g, &flags, 7).unwrap();
        assert!(n > 100_000, "the floor is regraded: {n}");
        assert!(drains(&g, &flags), "every cell still drains");
        let after = straightest_run(&g, &flags);
        // Before: the fill's geodesics run straight for most of the valley.
        assert!(before >= 200, "straight fill geodesics: {before}");
        assert!(after * 3 < before, "trunks wander: {after} vs {before}");
        // The floor stays a plain: under 3 m of relief over 20 km.
        let floor: Vec<i32> = (0..g.width * g.height)
            .filter(|&i| g.z[i] > 0 && g.z[i] < 80_000)
            .map(|i| g.z[i])
            .collect();
        let (lo, hi) = (floor.iter().min().unwrap(), floor.iter().max().unwrap());
        assert!(hi - lo < 3_000, "relief {} mm", hi - lo);
    }

    #[test]
    fn step_costs_stay_in_range_and_vary() {
        let costs: Vec<i64> = (0..400).map(|k| step_cost_mm(3, k * 97, k * 53)).collect();
        assert!(costs
            .iter()
            .all(|&c| (STEP_MIN_MM..=STEP_MAX_MM).contains(&c)));
        assert!(costs.iter().any(|&c| c <= 2) && costs.iter().any(|&c| c >= 4));
    }
}
