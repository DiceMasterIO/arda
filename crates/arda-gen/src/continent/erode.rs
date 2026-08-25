//! Continent-scale erosion on the 1 km working grid (`logic/01` step 2).
//!
//! "Coarse stream-power erosion and drainage respond, so rivers and valleys
//! co-evolve with the ranges."
//!
//! This runs **globally**, with no tiles, which is what makes it different
//! from the area stage in two ways that matter:
//!
//! - No pinned rim, so no seams. Per-tile erosion has to freeze tile edges to
//!   keep neighbours agreeing, and the frozen strips show as straight lines
//!   across the map once relief is strong.
//! - Drainage organises across the whole continent, so valleys run for
//!   hundreds of kilometres instead of stopping at a 51 km tile boundary.

/// The eight neighbour offsets, fixed order.
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

/// Coupled iterations at continent scale.
pub const ITERATIONS: u32 = 25;

/// Incision coefficient, scaled for 1 km cells.
const K_NUM: i64 = 24;
const K_DEN: i64 = 1;

/// Hillslope creep, as a fraction of the five-point Laplacian.
const CREEP_NUM: i64 = 4;
const CREEP_DEN: i64 = 100;

/// Runs the coupled loop over the 1 km grid, in place.
pub fn erode_continent(heights: &mut [i32], w: i32, h: i32) {
    let count = usize::try_from(w * h).unwrap_or(0);
    if count == 0 || heights.len() != count {
        return;
    }
    let idx = |x: i32, y: i32| usize::try_from(y * w + x).unwrap_or(0);

    for _ in 0..ITERATIONS {
        let filled = fill(heights, w, h);
        let (downstream, area) = accumulate(&filled, w, h);
        let mut next = heights.to_vec();

        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = idx(x, y);
                let here = heights[i];
                if here <= 0 {
                    continue; // the sea is the floor
                }
                let mut dz: i64 = 0;

                // Stream-power incision, skipped inside standing water so
                // lakes are not drained from below (artifact, Relief).
                if filled[i] <= here {
                    if let Some(d) = downstream[i] {
                        let d = d as usize;
                        let drop = i64::from(here - heights[d]).max(0);
                        let diagonal = (i % usize::try_from(w).unwrap_or(1))
                            != (d % usize::try_from(w).unwrap_or(1))
                            && (i / usize::try_from(w).unwrap_or(1))
                                != (d / usize::try_from(w).unwrap_or(1));
                        let dist = if diagonal { 1414 } else { 1000 };
                        let slope = drop * 1000 / dist;
                        let incision = K_NUM * isqrt(i64::from(area[i])) * slope / (K_DEN * 1000);
                        // Never cut below what this cell drains into.
                        dz -= incision.min(drop);
                    }
                }

                // Hillslope creep, isotropic nine-point Laplacian: the
                // five-point form is anisotropic and channels flow onto the
                // grid axes.
                let stride = usize::try_from(w).unwrap_or(1);
                let orth = i64::from(heights[i - 1])
                    + i64::from(heights[i + 1])
                    + i64::from(heights[i - stride])
                    + i64::from(heights[i + stride]);
                let diag = i64::from(heights[i - stride - 1])
                    + i64::from(heights[i - stride + 1])
                    + i64::from(heights[i + stride - 1])
                    + i64::from(heights[i + stride + 1]);
                let lap = (4 * orth + diag - 20 * i64::from(here)) / 6;
                dz += CREEP_NUM * lap / CREEP_DEN;

                next[i] = i32::try_from((i64::from(here) + dz).clamp(1, i64::from(i32::MAX)))
                    .unwrap_or(i32::MAX);
            }
        }
        heights.copy_from_slice(&next);
    }
}

fn isqrt(v: i64) -> i64 {
    if v <= 0 {
        return 0;
    }
    let mut x = v;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + v / x) / 2;
    }
    x
}

/// Priority-flood to a routing surface. The domain rim is ocean, so seeding
/// from it needs no bundle.
fn fill(heights: &[i32], w: i32, h: i32) -> Vec<i32> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let count = usize::try_from(w * h).unwrap_or(0);
    let idx = |x: i32, y: i32| usize::try_from(y * w + x).unwrap_or(0);
    let mut surface = vec![i32::MIN; count];
    let mut heap: BinaryHeap<Reverse<(i32, i32, i32)>> = BinaryHeap::new();

    for y in 0..h {
        for x in 0..w {
            if x != 0 && y != 0 && x != w - 1 && y != h - 1 {
                continue;
            }
            let i = idx(x, y);
            surface[i] = heights[i];
            heap.push(Reverse((heights[i], y, x)));
        }
    }
    // Key on (height, y, x): BinaryHeap leaves equal keys unordered.
    while let Some(Reverse((hh, y, x))) = heap.pop() {
        for (dx, dy) in NEIGHBOURS {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                continue;
            }
            let ni = idx(nx, ny);
            if surface[ni] != i32::MIN {
                continue;
            }
            let v = heights[ni].max(hh.saturating_add(1));
            surface[ni] = v;
            heap.push(Reverse((v, ny, nx)));
        }
    }
    surface
}

/// Steepest-descent directions and drainage area over a filled surface.
fn accumulate(filled: &[i32], w: i32, h: i32) -> (Vec<Option<u32>>, Vec<u32>) {
    let count = usize::try_from(w * h).unwrap_or(0);
    let idx = |x: i32, y: i32| usize::try_from(y * w + x).unwrap_or(0);
    let mut downstream: Vec<Option<u32>> = vec![None; count];
    let mut order: Vec<(i32, u32)> = Vec::with_capacity(count);

    for y in 0..h {
        for x in 0..w {
            let i = idx(x, y);
            order.push((filled[i], u32::try_from(i).unwrap_or(0)));
            let mut best: Option<(i64, u32)> = None;
            for (dx, dy) in NEIGHBOURS {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let ni = idx(nx, ny);
                let drop = i64::from(filled[i] - filled[ni]);
                if drop <= 0 {
                    continue;
                }
                // Steepest descent: drop / distance, by cross multiplication.
                let inv = if dx != 0 && dy != 0 { 1000 } else { 1414 };
                let score = drop * inv;
                if best.is_none_or(|(bs, _)| score > bs) {
                    best = Some((score, u32::try_from(ni).unwrap_or(0)));
                }
            }
            downstream[i] = best.map(|(_, n)| n);
        }
    }

    order.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut area = vec![1u32; count];
    for &(_, i) in &order {
        if let Some(d) = downstream[i as usize] {
            area[d as usize] = area[d as usize].saturating_add(area[i as usize]);
        }
    }
    (downstream, area)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(w: i32, h: i32) -> Vec<i32> {
        (0..w * h)
            .map(|i| {
                let y = i / w;
                // A dome, so there is somewhere for water to go.
                let dx = (i % w - w / 2).abs();
                let dy = (y - h / 2).abs();
                2_000_000 - (dx + dy) * 12_000
            })
            .collect()
    }

    #[test]
    fn erosion_is_deterministic() {
        let (w, h) = (60, 60);
        let mut a = ramp(w, h);
        let mut b = a.clone();
        erode_continent(&mut a, w, h);
        erode_continent(&mut b, w, h);
        assert_eq!(a, b);
    }

    #[test]
    fn the_sea_is_the_floor() {
        let (w, h) = (60, 60);
        let base = ramp(w, h);
        let mut e = base.clone();
        erode_continent(&mut e, w, h);
        for (i, &raw) in base.iter().enumerate() {
            if raw <= 0 {
                assert_eq!(e[i], raw, "sea cell {i} moved");
            }
        }
    }

    #[test]
    fn filling_leaves_no_interior_pit() {
        let (w, h) = (60, 60);
        let mut e = ramp(w, h);
        erode_continent(&mut e, w, h);
        let s = fill(&e, w, h);
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = usize::try_from(y * w + x).unwrap_or(0);
                let lower = NEIGHBOURS.iter().any(|(dx, dy)| {
                    let (nx, ny) = (x + dx, y + dy);
                    nx >= 0
                        && ny >= 0
                        && nx < w
                        && ny < h
                        && s[usize::try_from(ny * w + nx).unwrap_or(0)] < s[i]
                });
                assert!(lower, "pit at {x},{y}");
            }
        }
    }

    #[test]
    fn valleys_are_cut() {
        let (w, h) = (60, 60);
        let base = ramp(w, h);
        let mut e = base.clone();
        erode_continent(&mut e, w, h);
        let cut = base
            .iter()
            .zip(e.iter())
            .filter(|(b, a)| **a < **b - 1_000)
            .count();
        assert!(cut > 100, "only {cut} cells were incised");
    }
}
