//! Recipe-9 slope, aspect and wetness, read across area edges (review
//! 3.2-5, 3.2-7, 3.2-16; logic/02 §recipe 9 fields).
//!
//! Up to recipe 8 an area's slope and aspect come from its own 512² tile:
//! the steepest of the eight neighbours, skipping those outside the tile, so
//! every edge cell had a one-sided slope (36.5 % of seam land cells wrong,
//! up to 44°), an aspect of one of eight directions and `0` for both flat
//! and north. Wetness was a D8 index: contributing area along single
//! receivers over the steepest-neighbour slope, so marsh followed routing
//! lines on flat plains.
//!
//! Recipe 9 reads a three-cell halo from the neighbouring areas' prepared
//! heights and solved drainage. Slope and aspect are Horn's continuous
//! gradient over the 3 × 3 neighbourhood, the aspect a whole-degree bearing
//! (`arda_core::biome::aspect`, flat `0`, north `360`). Wetness is the
//! world's topographic index on a continuous base-2 logarithm of area and
//! slope, smoothed over a 5 × 5 binomial kernel of land cells, so single D8
//! lines and log steps no longer print into it. Integer only.

use arda_core::biome::aspect;

/// Cells of halo read beyond each edge of the area.
pub const HALO: i32 = 3;
/// Side of the halo grid in cells.
pub const SIDE: i32 = 512 + 2 * HALO;
/// Cell size, millimetres.
const CELL_MM: i64 = 100_000;

/// Heights, drainage and land membership over an area and its halo,
/// row-major over [`SIDE`]², local cell `(x, y)` at `(x + HALO, y + HALO)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Halo {
    /// Physical heights, millimetres.
    pub heights: Vec<i32>,
    /// Distinct upstream cells on the final dry flow graph (zero on water).
    pub drainage: Vec<u32>,
    /// Whether the cell is dry land (neither sea nor lake).
    pub land: Vec<bool>,
}

impl Halo {
    /// An all-sea halo at height zero, to be filled by the caller.
    #[must_use]
    pub fn new() -> Self {
        let n = usize::try_from(SIDE * SIDE).unwrap_or(0);
        Self {
            heights: vec![0; n],
            drainage: vec![0; n],
            land: vec![false; n],
        }
    }

    /// Index of local cell `(x, y)`, `-HALO..512 + HALO`, clamped into the
    /// halo.
    #[must_use]
    pub fn at(x: i32, y: i32) -> usize {
        let (hx, hy) = ((x + HALO).clamp(0, SIDE - 1), (y + HALO).clamp(0, SIDE - 1));
        usize::try_from(hy * SIDE + hx).unwrap_or(0)
    }

    fn h(&self, x: i32, y: i32) -> i64 {
        i64::from(self.heights[Self::at(x, y)])
    }
}

impl Default for Halo {
    fn default() -> Self {
        Self::new()
    }
}

/// `atan` of a gradient scaled by 10,000, in milli-degrees (the shared
/// table of [`super::fields`]).
fn atan_milli_deg(grad_10k: i64) -> u16 {
    super::fields::atan_milli_deg(grad_10k)
}

/// Slope (milli-degrees) and stored aspect of local cell `(x, y)` from
/// Horn's gradient over its 3 × 3 neighbourhood, halo included.
#[must_use]
pub fn slope_aspect(h: &Halo, x: i32, y: i32) -> (u16, u16) {
    let z = |dx: i32, dy: i32| h.h(x + dx, y + dy);
    // Rise per cell eastward and southward, times 8 cells of run.
    let gx = (z(1, -1) + 2 * z(1, 0) + z(1, 1)) - (z(-1, -1) + 2 * z(-1, 0) + z(-1, 1));
    let gy = (z(-1, 1) + 2 * z(0, 1) + z(1, 1)) - (z(-1, -1) + 2 * z(0, -1) + z(1, -1));
    if gx == 0 && gy == 0 {
        return (0, aspect::FLAT);
    }
    let len = (i128::from(gx) * i128::from(gx) + i128::from(gy) * i128::from(gy)).isqrt();
    let grad_10k = i64::try_from(len * 10_000 / i128::from(8 * CELL_MM)).unwrap_or(i64::MAX);
    // Downhill is against the gradient.
    let bearing = aspect::compass_bearing(-gx, -gy).map_or(aspect::FLAT, aspect::encode);
    (atan_milli_deg(grad_10k), bearing)
}

/// `log2(v)` in Q8 for `v >= 1`: the integer part from the highest bit,
/// the fraction linear in the mantissa (within 0.09 of the true log), so
/// the index has no unit steps.
#[must_use]
pub fn log2_q8(v: u64) -> i64 {
    let v = v.max(1);
    let k = v.ilog2();
    let base = 1_u64 << k;
    let frac = i64::try_from((v - base) * 256 / base).unwrap_or(255);
    i64::from(k) * 256 + frac
}

/// The world's topographic wetness index (`28 + 15·log2 A + 16·(10 −
/// log2 slope)`, slope in milli-degrees floored at 50) in Q8, on continuous
/// logarithms.
#[must_use]
pub fn twi_q8(drainage: u32, slope_milli_deg: u16) -> i64 {
    28 * 256
        + 15 * log2_q8(u64::from(drainage.max(1)))
        + 16 * (10 * 256 - log2_q8(u64::from(slope_milli_deg.max(50))))
}

/// Binomial weights of the 5 × 5 smoothing kernel.
const KERNEL: [i64; 5] = [1, 4, 6, 4, 1];

/// Wetness `0..=255` of every cell of the area (row-major 512²): the index
/// smoothed over the land cells of a 5 × 5 binomial kernel; water cells
/// take 255 as before.
#[must_use]
pub fn wetness(h: &Halo) -> Vec<u8> {
    // The index over the area and a two-cell ring, from halo slopes.
    let r = 2;
    let w = 512 + 2 * r;
    let mut twi = vec![0_i64; usize::try_from(w * w).unwrap_or(0)];
    for y in -r..512 + r {
        for x in -r..512 + r {
            let i = usize::try_from((y + r) * w + x + r).unwrap_or(0);
            let (slope, _) = slope_aspect(h, x, y);
            twi[i] = twi_q8(h.drainage[Halo::at(x, y)], slope);
        }
    }
    let mut out = vec![255_u8; 512 * 512];
    for y in 0..512 {
        for x in 0..512 {
            if !h.land[Halo::at(x, y)] {
                continue;
            }
            let (mut sum, mut weight) = (0_i64, 0_i64);
            for (j, wy) in KERNEL.iter().enumerate() {
                for (i, wx) in KERNEL.iter().enumerate() {
                    let (dx, dy) = (
                        i32::try_from(i).unwrap_or(0) - r,
                        i32::try_from(j).unwrap_or(0) - r,
                    );
                    if !h.land[Halo::at(x + dx, y + dy)] {
                        continue;
                    }
                    let k = usize::try_from((y + dy + r) * w + x + dx + r).unwrap_or(0);
                    sum += twi[k] * wx * wy;
                    weight += wx * wy;
                }
            }
            let v = sum / weight.max(1) / 256;
            out[usize::try_from(y * 512 + x).unwrap_or(0)] =
                u8::try_from(v.clamp(0, 255)).unwrap_or(u8::MAX);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tilted(f: impl Fn(i32, i32) -> i32) -> Halo {
        let mut h = Halo::new();
        for y in -HALO..512 + HALO {
            for x in -HALO..512 + HALO {
                let i = Halo::at(x, y);
                h.heights[i] = f(x, y);
                h.land[i] = true;
                h.drainage[i] = 1;
            }
        }
        h
    }

    #[test]
    fn edge_cells_read_the_neighbouring_area() {
        // A plane rising 10 m per cell eastward: every cell, the edges
        // included, has the same slope and faces west (review 3.2-5).
        let h = tilted(|x, _| x * 10_000);
        let inner = slope_aspect(&h, 200, 200);
        assert_eq!(slope_aspect(&h, 0, 200), inner);
        assert_eq!(slope_aspect(&h, 511, 0), inner);
        assert_eq!(inner.1, 270);
        assert!((5_600..5_800).contains(&inner.0), "{inner:?}");
    }

    #[test]
    fn aspect_is_continuous_and_north_is_not_flat() {
        // Rising southward: downhill is north, stored as 360 (review 3.2-7).
        let north = tilted(|_, y| y * 5_000);
        assert_eq!(slope_aspect(&north, 10, 10).1, 360);
        let flat = tilted(|_, _| 7_000);
        assert_eq!(slope_aspect(&flat, 10, 10), (0, 0));
        // A plane falling toward 30° east of north is not snapped to 45°.
        let ne = tilted(|x, y| -x * 5_000 + y * 8_660);
        assert_eq!(slope_aspect(&ne, 10, 10).1, 30);
    }

    #[test]
    fn wetness_has_no_single_cell_routing_lines() {
        // A flat plain with one D8 line of large drainage: the raw index
        // jumps by its whole log range across the line; the smoothed index
        // spreads it so no cell differs from its neighbour by more than a
        // third of that jump (review 3.2-16).
        let mut h = tilted(|_, _| 50_000);
        for y in -HALO..512 + HALO {
            h.drainage[Halo::at(100, y)] = 1 << 16;
        }
        let wet = wetness(&h);
        let at = |x: usize| i64::from(wet[200 * 512 + x]);
        let raw = (twi_q8(1 << 16, 50) - twi_q8(1, 50)) / 256;
        for x in 95..105 {
            assert!(
                (at(x + 1) - at(x)).abs() * 3 <= raw,
                "{x}: {} {}",
                at(x),
                at(x + 1)
            );
        }
        assert!(at(100) > at(90));
    }

    #[test]
    fn continuous_logs_track_the_integer_ones() {
        assert_eq!(log2_q8(1), 0);
        assert_eq!(log2_q8(1024), 10 * 256);
        assert_eq!(log2_q8(1536), 10 * 256 + 128);
    }
}
