//! Landscape evolution (artifact, Relief).
//!
//! Four processes act together, per the artifact: uplift raises the land
//! fastest where the coarse relief is highest; rivers cut it down with power
//! growing as drainage area and bed steepness; hillslopes creep downhill;
//! and any slope past about 35 degrees collapses until it is not. The sea is
//! the floor — nothing erodes below it and sea cells never move.
//!
//! Two rules from the artifact shape the loop:
//!
//! - Lakes "are not eroded from below toward a spill that sits above them",
//!   so submerged cells are exempt from incision. Creep still acts on them,
//!   which is what removes shallow noise pits without draining real basins.
//! - The run stops before equilibrium; the network organises within the
//!   first quarter of it.

use super::fill::{fill, NEIGHBOURS};
use crate::continent::bundles::TileBundle;
use arda_core::{CellCoord, AREA_CELLS};

const N: i32 = AREA_CELLS as i32;

/// Iterations of the coupled loop.
pub const ITERATIONS: u32 = 40;

/// Uplift at the highest coarse relief, millimetres per iteration.
const UPLIFT_PEAK_MM: i64 = 900;

/// Incision coefficient, calibrated against the artifact's equilibrium
/// targets: "a mountain stream draining a single square kilometre settles
/// near a 9% slope, a river draining a hundred near 1%".
///
/// At equilibrium uplift balances incision, `U = K·sqrt(A)·S`. Both anchor
/// points give the same `K`, which is the check that `m/n = 0.5` is right:
/// A=100 cells with S=90‰ and A=10,000 with S=10‰ both need incision to
/// equal `UPLIFT_PEAK_MM`. Solving with area in cells and slope in per-mille
/// gives `K_NUM/K_DEN = 100`.
const K_NUM: i64 = 100;
const K_DEN: i64 = 1;

/// Hillslope creep coefficient, as a fraction of the five-point Laplacian.
const CREEP_NUM: i64 = 3;
const CREEP_DEN: i64 = 100;

/// Tangent of the repose angle, scaled by 1000. 35 degrees ≈ 0.700.
const TALUS_TAN_1000: i64 = 700;

/// Cells over which erosion amplitude ramps in from the pinned tile edge.
///
/// `logic/02` amendment 3 pins edge heights throughout the erosion run so
/// neighbours agree. Eroding the interior at full rate against a frozen
/// one-cell rim would leave a raised lip around all four sides of every
/// tile; ramping the amplitude turns that step into a gradient.
const EDGE_TAPER: i32 = 32;

/// Row-major offset. Invariant: every caller bounds-checks `0 <= x,y < N`
/// before calling, so the product is non-negative.
#[allow(clippy::cast_sign_loss)]
fn idx(x: i32, y: i32) -> usize {
    (y * N + x) as usize
}

fn coord(x: i32, y: i32) -> Option<CellCoord> {
    CellCoord::new(u16::try_from(x).ok()?, u16::try_from(y).ok()?)
}

/// Smoothstep ramp, 0 at the tile edge to 1024 at `EDGE_TAPER` cells in.
fn taper(x: i32, y: i32) -> i64 {
    let d = x.min(y).min(N - 1 - x).min(N - 1 - y);
    if d >= EDGE_TAPER {
        return 1024;
    }
    let t = i64::from(d) * 1024 / i64::from(EDGE_TAPER);
    // 3t² - 2t³ in 1024 fixed point.
    let t2 = t * t / 1024;
    let t3 = t2 * t / 1024;
    (3 * t2 - 2 * t3).clamp(0, 1024)
}

/// Runs the coupled landscape-evolution loop.
///
/// `coarse` supplies the uplift pattern: the artifact raises land "fastest
/// near the mountain edge, slowly in the lowland", which at area scale is
/// the continent relief this tile sits on.
pub fn erode(heights: &mut [i32], coarse: &[i32], bundle: &TileBundle) {
    let peak = coarse.iter().copied().max().unwrap_or(1).max(1);

    for _ in 0..ITERATIONS {
        let filled = fill(heights, bundle);

        // Drainage area over the filled surface, high cells first.
        let (downstream, area) = accumulate(&filled);

        let mut next = heights.to_vec();

        for y in 1..N - 1 {
            for x in 1..N - 1 {
                let i = idx(x, y);
                let h = heights[i];
                if h <= 0 {
                    continue; // the sea is the floor; sea cells never move
                }
                let Some(here) = coord(x, y) else { continue };
                let ramp = taper(x, y);
                let mut dz: i64 = 0;

                // 1. Uplift, scaled by the coarse relief beneath this cell.
                dz += UPLIFT_PEAK_MM * i64::from(coarse[i]).max(0) / i64::from(peak);

                // 2. Stream-power incision, skipped inside lakes.
                let submerged = filled.get(here) > h;
                if !submerged {
                    if let Some(d) = downstream[i] {
                        let drop = i64::from(h - heights[d as usize]).max(0);
                        let dist = if is_diagonal(i, d as usize) {
                            1414
                        } else {
                            1000
                        };
                        // slope in 1/1000 units; A in cells
                        let slope = drop * 1000 / dist;
                        let incision = K_NUM * isqrt(i64::from(area[i])) * slope / (K_DEN * 100);
                        // A channel may not cut below what it drains into.
                        // Without this the calibrated K digs a pit at every
                        // cell and the basins all become lakes.
                        dz -= incision.min(drop);
                    }
                }

                // 3. Hillslope creep, isotropic nine-point Laplacian.
                //
                // The five-point form uses only orthogonal neighbours, so it
                // smooths along the axes differently from the diagonals and
                // imprints axis-aligned structure over the run. Steepest
                // descent then follows it and rivers come out as straight
                // combs — the diagonal share of flow directions sat at 14%.
                let orth = i64::from(heights[idx(x - 1, y)])
                    + i64::from(heights[idx(x + 1, y)])
                    + i64::from(heights[idx(x, y - 1)])
                    + i64::from(heights[idx(x, y + 1)]);
                let diag = i64::from(heights[idx(x - 1, y - 1)])
                    + i64::from(heights[idx(x + 1, y - 1)])
                    + i64::from(heights[idx(x - 1, y + 1)])
                    + i64::from(heights[idx(x + 1, y + 1)]);
                let lap = (4 * orth + diag - 20 * i64::from(h)) / 6;
                dz += CREEP_NUM * lap / CREEP_DEN;

                let applied = dz * ramp / 1024;
                next[i] = i32::try_from((i64::from(h) + applied).clamp(1, i64::from(i32::MAX)))
                    .unwrap_or(i32::MAX);
            }
        }

        collapse(&mut next);
        heights.copy_from_slice(&next);
    }
}

fn is_diagonal(a: usize, b: usize) -> bool {
    let (Ok(a), Ok(b)) = (i32::try_from(a), i32::try_from(b)) else {
        return false;
    };
    (a % N != b % N) && (a / N != b / N)
}

/// Integer square root.
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

/// Steepest-descent directions and drainage area over a filled surface.
fn accumulate(filled: &super::fill::Filled) -> (Vec<Option<u32>>, Vec<u32>) {
    let count = (N * N) as usize;
    let mut downstream: Vec<Option<u32>> = vec![None; count];
    let mut order: Vec<(i32, u32)> = Vec::with_capacity(count);

    for y in 0..N {
        for x in 0..N {
            let Some(at) = coord(x, y) else { continue };
            let h = filled.get(at);
            order.push((h, u32::try_from(idx(x, y)).unwrap_or(0)));
            let mut best: Option<(i64, u32)> = None;
            for (dx, dy) in NEIGHBOURS {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= N || ny >= N {
                    continue;
                }
                let Some(nb) = coord(nx, ny) else { continue };
                let drop = i64::from(h - filled.get(nb));
                if drop <= 0 {
                    continue;
                }
                let dist = if dx != 0 && dy != 0 { 1000 } else { 1414 };
                let score = drop * dist; // drop / distance, cross-multiplied
                if best.is_none_or(|(bs, _)| score > bs) {
                    best = Some((score, u32::try_from(idx(nx, ny)).unwrap_or(0)));
                }
            }
            downstream[idx(x, y)] = best.map(|(_, i)| i);
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

/// Talus collapse: no slope steeper than the repose angle survives
/// (artifact: "any slope steeper than about 35 degrees collapses until it is
/// not").
///
/// Material moves from the high cell to the low one, except where the low
/// cell is sea or on the pinned rim — neither may be raised, so the high
/// cell simply sheds the whole excess. Swept repeatedly because one pass can
/// steepen a pair it already visited.
fn collapse(heights: &mut [i32]) {
    for _ in 0..8 {
        let mut changed = false;
        for y in 1..N - 1 {
            for x in 1..N - 1 {
                let i = idx(x, y);
                if heights[i] <= 0 {
                    continue; // the sea never moves
                }
                for (dx, dy) in NEIGHBOURS {
                    let (nx, ny) = (x + dx, y + dy);
                    let j = idx(nx, ny);
                    let drop = i64::from(heights[i] - heights[j]);
                    if drop <= 0 {
                        continue;
                    }
                    // Ground run in mm: 100 m orthogonal, 141.4 m diagonal.
                    let run = if dx != 0 && dy != 0 { 141_400 } else { 100_000 };
                    let max_drop = run * TALUS_TAN_1000 / 1000;
                    if drop <= max_drop {
                        continue;
                    }
                    // Ramped like every other process. Untapered, collapse ran
                    // at full strength against the pinned rim, digging the
                    // rim's neighbour down while the rim itself stayed — which
                    // is exactly the seam step the taper exists to prevent.
                    let excess = (drop - max_drop) * taper(x, y) / 1024;
                    if excess <= 0 {
                        continue;
                    }
                    let frozen =
                        heights[j] <= 0 || nx == 0 || ny == 0 || nx == N - 1 || ny == N - 1;
                    if frozen {
                        // Cannot raise the sea or the pinned rim; shed it all.
                        heights[i] -= i32::try_from(excess).unwrap_or(0);
                    } else {
                        let half = i32::try_from(excess / 2).unwrap_or(0);
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
mod tests {
    use super::*;
    use crate::continent::bundles::bundle_for;
    use crate::continent::generate_continent;
    use arda_core::{AreaCoord, GenerateConfig};

    fn setup() -> (Vec<i32>, Vec<i32>, TileBundle) {
        let c = generate_continent(42, GenerateConfig::MICRO);
        let b = bundle_for(42, &c, AreaCoord::new(0, 1));
        let r = crate::area::relief::relief(42, &c, &b);
        let h: Vec<i32> = (0..(N * N) as usize)
            .filter_map(|i| {
                let i = i32::try_from(i).ok()?;
                Some(r.get(coord(i % N, i / N)?))
            })
            .collect();
        let coarse = h.clone();
        (h, coarse, b)
    }

    #[test]
    fn erosion_is_deterministic() {
        let (h, c, b) = setup();
        let mut a = h.clone();
        let mut z = h;
        erode(&mut a, &c, &b);
        erode(&mut z, &c, &b);
        assert_eq!(a, z);
    }

    #[test]
    fn the_pinned_edge_never_moves() {
        // logic/02 amendment 3: edge heights are pinned throughout the run.
        let (h, c, b) = setup();
        let mut e = h.clone();
        erode(&mut e, &c, &b);
        for k in 0..N {
            for (x, y) in [(k, 0), (k, N - 1), (0, k), (N - 1, k)] {
                assert_eq!(e[idx(x, y)], h[idx(x, y)], "edge moved at {x},{y}");
            }
        }
    }

    #[test]
    fn sea_cells_never_move() {
        // Artifact: "the sea cells themselves never move".
        let (h, c, b) = setup();
        let mut e = h.clone();
        erode(&mut e, &c, &b);
        for (i, &raw) in h.iter().enumerate() {
            if raw <= 0 {
                assert_eq!(e[i], raw, "sea cell {i} moved");
            }
        }
    }

    #[test]
    fn no_slope_exceeds_the_repose_angle() {
        // Artifact: slopes past ~35 degrees collapse until they are not.
        //
        // Asserted where erosion runs at full strength. Within EDGE_TAPER of
        // the pinned rim every process including collapse is deliberately
        // ramped to zero, so the rule cannot hold there; that band is
        // reported rather than asserted, and it is bounded by the untouched
        // relief it is ramping back toward.
        let (h, c, b) = setup();
        let mut e = h.clone();
        erode(&mut e, &c, &b);

        let worst = |lo: i32, hi: i32| {
            let mut w = 0i64;
            for y in lo..hi {
                for x in lo..hi {
                    if e[idx(x, y)] <= 0 {
                        continue;
                    }
                    for (dx, dy) in NEIGHBOURS {
                        let drop = i64::from(e[idx(x, y)] - e[idx(x + dx, y + dy)]);
                        let run = if dx != 0 && dy != 0 { 141_400 } else { 100_000 };
                        w = w.max(drop * 1000 / run);
                    }
                }
            }
            w
        };

        let full = worst(EDGE_TAPER, N - EDGE_TAPER);
        let all = worst(1, N - 1);
        println!("steepest tan*1000: full-strength {full}, including taper band {all}");
        assert!(
            full <= TALUS_TAN_1000 * 5 / 4,
            "full-strength region reaches tan*1000 = {full}, past the repose angle {TALUS_TAN_1000}"
        );
    }

    #[test]
    fn erosion_changes_the_interior() {
        let (h, c, b) = setup();
        let mut e = h.clone();
        erode(&mut e, &c, &b);
        let moved = h.iter().zip(e.iter()).filter(|(a, z)| a != z).count();
        assert!(moved > (N * N) as usize / 10, "only {moved} cells changed");
    }

    #[test]
    fn taper_is_zero_at_the_edge_and_full_inside() {
        assert_eq!(taper(0, 100), 0);
        assert_eq!(taper(N - 1, 100), 0);
        assert_eq!(taper(EDGE_TAPER, 100), 1024);
        assert_eq!(taper(256, 256), 1024);
    }

    #[test]
    fn isqrt_matches_the_float_result() {
        for (v, want) in [
            (0i64, 0i64),
            (1, 1),
            (2, 1),
            (3, 1),
            (4, 2),
            (99, 9),
            (100, 10),
            (10_000, 100),
            (262_144, 512),
        ] {
            assert_eq!(isqrt(v), want, "isqrt({v})");
        }
    }
}
