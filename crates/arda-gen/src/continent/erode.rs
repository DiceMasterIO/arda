//! Continent-scale hillslope creep on the 1 km working grid (`logic/01` step 2).
//!
//! The 1 km stage softens hillslopes across the whole continent. Stream-power
//! incision belongs to the shared 100 m terrain evolution: cutting the coarse
//! grid first creates artificial D8 trenches that survive fine refinement.

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

/// Hillslope creep iterations at continent scale.
pub const ITERATIONS: u32 = 25;

/// Hillslope creep, as a fraction of the nine-point Laplacian.
const CREEP_NUM: i64 = 4;
const CREEP_DEN: i64 = 100;

/// Runs global hillslope creep over the 1 km grid, in place.
pub fn erode_continent(heights: &mut [i32], w: i32, h: i32) {
    let count = usize::try_from(w * h).unwrap_or(0);
    if count == 0 || heights.len() != count {
        return;
    }
    let idx = |x: i32, y: i32| usize::try_from(y * w + x).unwrap_or(0);

    for _ in 0..ITERATIONS {
        let mut next = heights.to_vec();

        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = idx(x, y);
                let here = heights[i];
                if here <= 0 {
                    continue; // the sea is the floor
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
                let dz = CREEP_NUM * lap / CREEP_DEN;

                next[i] = i32::try_from((i64::from(here) + dz).clamp(1, i64::from(i32::MAX)))
                    .unwrap_or(i32::MAX);
            }
        }
        heights.copy_from_slice(&next);
    }
}

/// Priority-flood to a routing surface. The domain rim is ocean, so seeding
/// from it needs no bundle.
pub(crate) fn fill(heights: &[i32], w: i32, h: i32) -> Vec<i32> {
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
pub(crate) fn accumulate(filled: &[i32], w: i32, h: i32) -> (Vec<Option<u32>>, Vec<u32>) {
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
        let mut base = ramp(w, h);
        for y in 25..35 {
            for x in 25..35 {
                base[usize::try_from(y * w + x).unwrap()] = -12_000;
            }
        }
        let mut e = base.clone();
        erode_continent(&mut e, w, h);
        let mut sea_cells = 0;
        for (i, &raw) in base.iter().enumerate() {
            if raw <= 0 {
                sea_cells += 1;
                assert_eq!(e[i], raw, "sea cell {i} moved");
            }
        }
        assert_eq!(sea_cells, 100);
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
    fn affine_slope_has_no_coarse_incised_channel() {
        let (w, h) = (60, 60);
        let base: Vec<i32> = (0..h)
            .flat_map(|y| (0..w).map(move |x| 2_000_000 - 12_000 * x - 8_000 * y))
            .collect();
        let mut e = base.clone();
        erode_continent(&mut e, w, h);
        assert_eq!(e, base, "a planar slope has no curvature to diffuse");
    }

    #[test]
    fn isolated_peak_diffuses_symmetrically() {
        let (w, h) = (21, 21);
        let mut heights = vec![100_000; usize::try_from(w * h).unwrap()];
        let center = usize::try_from(10 * w + 10).unwrap();
        heights[center] = 200_000;
        erode_continent(&mut heights, w, h);
        assert!(heights[center] < 200_000);
        assert!(heights[center - 1] > 100_000);
        assert_eq!(heights[center - 1], heights[center + 1]);
        assert_eq!(
            heights[center - 1],
            heights[center - usize::try_from(w).unwrap()]
        );
    }
}
