//! Low-relief drainage on relief tiles (logic/17 §low-relief): on plains
//! the stored relief is a few metres over kilometres, so the formed
//! shader's light (tuned for hills) leaves valley floors, terraces and
//! bluffs invisible. Two gentle terms bring them out where — and only
//! where — local relief is small, both read from the stored field itself
//! (no relief is added):
//!
//! - **position**: the height above or below the ~200 m neighbourhood mean
//!   (topographic position), normalised by the local relief, tints valley
//!   floors a cooler, greener meadow tone and interfluves a warmer, paler
//!   one;
//! - **slope light**: the refined surface's north-west light term, scaled
//!   up by up to 2.5× as local relief shrinks below 6 m, so terrace risers
//!   and bluffs read as lines while flat treads stay flat.
//!
//! Every value is an integer function of global stored nodes; the window
//! carries a margin wider than the kernels, so tiles join exactly.

use crate::fixed::{isqrt128, smooth, ONE};
use crate::source::{Terrain, FINE_UM};
use crate::{MidzoomError, SurfacePoint};

/// Box radius of the position mean, nodes (three passes ≈ Gaussian σ 214 m).
const MEAN_R: i64 = 5;
/// Box radius of the local-relief mean, nodes (two passes ≈ 650 m across).
const RELIEF_R: i64 = 8;
/// Nodes the kernels read beyond the window.
const MARGIN: i64 = 3 * MEAN_R + 2 * RELIEF_R + 2;
/// Light from the north-west at 42° (logic/04): per-axis horizontal and
/// vertical components, Q12.
const LIGHT_AXIS: i64 = 2_152;
const LIGHT_UP: i64 = 2_741;

/// Smoothed fields over a window of stored nodes.
#[derive(Debug, Clone)]
pub struct LowRelief {
    k0: (i64, i64),
    w: i64,
    h: i64,
    /// Neighbourhood mean height, mm.
    mean: Vec<i64>,
    /// Local relief (mean absolute deviation from the mean), mm.
    relief: Vec<i64>,
}

/// One pass of a box mean of radius `r` along rows (`horizontal`) or
/// columns, edges repeating.
fn box_pass(src: &[i64], w: i64, h: i64, r: i64, horizontal: bool) -> Vec<i64> {
    let at = |x: i64, y: i64| {
        let (x, y) = (x.clamp(0, w - 1), y.clamp(0, h - 1));
        src[usize::try_from(y * w + x).unwrap_or(0)]
    };
    let n = 2 * r + 1;
    let mut out = vec![0; src.len()];
    let (outer, inner) = if horizontal { (h, w) } else { (w, h) };
    for o in 0..outer {
        let get = |i: i64| if horizontal { at(i, o) } else { at(o, i) };
        let mut sum: i64 = (-r..=r).map(get).sum();
        for i in 0..inner {
            let k = if horizontal { o * w + i } else { i * w + o };
            out[usize::try_from(k).unwrap_or(0)] = sum / n;
            sum += get(i + r + 1) - get(i - r);
        }
    }
    out
}

fn blur(src: Vec<i64>, w: i64, h: i64, r: i64, passes: u32) -> Vec<i64> {
    let mut v = src;
    for _ in 0..passes {
        v = box_pass(&v, w, h, r, true);
        v = box_pass(&v, w, h, r, false);
    }
    v
}

impl LowRelief {
    /// The fields for queries in fine-lattice micrometres
    /// `[lx0, lx1] × [ly0, ly1]`.
    ///
    /// # Errors
    /// The stored layer failed to read.
    pub fn gather(
        terrain: &dyn Terrain,
        (lx0, ly0, lx1, ly1): (i64, i64, i64, i64),
    ) -> Result<Self, MidzoomError> {
        let kx0 = lx0.div_euclid(FINE_UM) - MARGIN;
        let ky0 = ly0.div_euclid(FINE_UM) - MARGIN;
        let w = lx1.div_euclid(FINE_UM) + MARGIN + 2 - kx0;
        let h = ly1.div_euclid(FINE_UM) + MARGIN + 2 - ky0;
        let (uw, uh) = (
            usize::try_from(w).map_err(|_| MidzoomError::Window("low relief".into()))?,
            usize::try_from(h).map_err(|_| MidzoomError::Window("low relief".into()))?,
        );
        let nodes: Vec<i64> = terrain
            .read_nodes(kx0, ky0, uw, uh)?
            .into_iter()
            .map(i64::from)
            .collect();
        let mean = blur(nodes.clone(), w, h, MEAN_R, 3);
        let dev: Vec<i64> = nodes
            .iter()
            .zip(&mean)
            .map(|(a, b)| (a - b).abs())
            .collect();
        let relief = blur(dev, w, h, RELIEF_R, 2);
        Ok(Self {
            k0: (kx0, ky0),
            w,
            h,
            mean,
            relief,
        })
    }

    /// Bilinear `(mean, relief)` at fine-lattice micrometres, mm.
    #[must_use]
    pub fn sample(&self, lx: i64, ly: i64) -> (i64, i64) {
        let (i, fx) = (
            lx.div_euclid(FINE_UM),
            lx.rem_euclid(FINE_UM) * ONE / FINE_UM,
        );
        let (j, fy) = (
            ly.div_euclid(FINE_UM),
            ly.rem_euclid(FINE_UM) * ONE / FINE_UM,
        );
        let at = |v: &[i64], di: i64, dj: i64| {
            let x = (i + di - self.k0.0).clamp(0, self.w - 1);
            let y = (j + dj - self.k0.1).clamp(0, self.h - 1);
            v[usize::try_from(y * self.w + x).unwrap_or(0)]
        };
        let bil = |v: &[i64]| {
            let top = at(v, 0, 0) + (at(v, 1, 0) - at(v, 0, 0)) * fx / ONE;
            let bot = at(v, 0, 1) + (at(v, 1, 1) - at(v, 0, 1)) * fx / ONE;
            top + (bot - top) * fy / ONE
        };
        (bil(&self.mean), bil(&self.relief))
    }

    /// Shades a land pixel at fine-lattice micrometres with the refined
    /// surface point `c`: returns per-channel Q12 multipliers, and how
    /// strongly (Q12) the point reads as a valley floor (for floodplain
    /// meadows).
    #[must_use]
    pub fn shade(&self, lx: i64, ly: i64, c: SurfacePoint) -> ([i64; 3], i64) {
        let (mean, relief) = self.sample(lx, ly);
        // Only low country: full below 3 m of local relief (mean absolute
        // deviation), none above 10 m.
        let flat = ONE - smooth(3_000, 10_000, relief);
        if flat == 0 {
            return ([ONE; 3], 0);
        }
        // Topographic position, in units of the local relief (Q12, ±1.5).
        let t = ((c.height_mm - mean) * ONE / relief.max(700)).clamp(-ONE * 6 / 5, ONE * 6 / 5);
        let t = t * flat / ONE;
        let pos: [i64; 3] = if t < 0 {
            // Valley floor: cooler and greener, slightly darker.
            let a = -t;
            [ONE - a * 4 / 100, ONE - a / 100, ONE - a * 35 / 1000]
        } else {
            [ONE + t * 3 / 100, ONE + t * 25 / 1000, ONE + t / 100]
        };
        // Slope light, amplified as relief shrinks below 6 m (up to 2.5×).
        let gain = (6_000 * ONE / relief.max(2_400)).clamp(ONE, 5 * ONE / 2) - ONE;
        let light = relief_term(c.gradient_q12) * gain / ONE * flat / ONE;
        let l = ONE + light * 9 / 10;
        (pos.map(|p| p * l / ONE), (-t).clamp(0, ONE))
    }
}

/// Lambert term minus flat ground for a Q12 gradient, light from the
/// north-west at 42° (the formed shader's light, logic/04).
fn relief_term((gx, gy): (i64, i64)) -> i64 {
    let (gx, gy, one) = (i128::from(gx), i128::from(gy), i128::from(ONE));
    let len = i128::from(isqrt128(gx * gx + gy * gy + one * one)).max(1);
    let dot = (i128::from(LIGHT_AXIS) * (gx + gy) + i128::from(LIGHT_UP) * one) / len;
    i64::try_from(dot).unwrap_or(0) - LIGHT_UP
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GridTerrain;

    #[test]
    fn hills_are_left_alone_and_flat_plains_stay_flat() {
        // A 60 m ridge every 1.25 km: high relief, no change.
        let hills = GridTerrain::from_fn(1, 96, 96, |i, j| {
            i32::try_from(((i + j) % 32 - 16).abs() * 4_000).unwrap()
        });
        let f = LowRelief::gather(
            &hills,
            (30 * FINE_UM, 30 * FINE_UM, 60 * FINE_UM, 60 * FINE_UM),
        )
        .unwrap();
        let c = SurfacePoint {
            height_mm: 30_000,
            gradient_q12: (400, 400),
        };
        assert_eq!(f.shade(40 * FINE_UM, 40 * FINE_UM, c).0, [ONE; 3]);
        // A dead-flat plain: no tint, no light.
        let flat = GridTerrain::from_fn(1, 96, 96, |_, _| 5_000);
        let f = LowRelief::gather(
            &flat,
            (30 * FINE_UM, 30 * FINE_UM, 60 * FINE_UM, 60 * FINE_UM),
        )
        .unwrap();
        let c = SurfacePoint {
            height_mm: 5_000,
            gradient_q12: (0, 0),
        };
        assert_eq!(f.shade(40 * FINE_UM, 40 * FINE_UM, c).0, [ONE; 3]);
    }

    #[test]
    fn a_shallow_valley_floor_reads_as_floor() {
        // A 3 m deep, 300 m wide valley in a plain.
        let v = GridTerrain::from_fn(1, 128, 128, |i, _| {
            let d = (i - 64).abs();
            if d < 4 {
                10_000
            } else {
                13_000
            }
        });
        let f = LowRelief::gather(&v, (40 * FINE_UM, 40 * FINE_UM, 90 * FINE_UM, 90 * FINE_UM))
            .unwrap();
        let floor = SurfacePoint {
            height_mm: 10_000,
            gradient_q12: (0, 0),
        };
        let (mul, valley) = f.shade(64 * FINE_UM, 64 * FINE_UM, floor);
        assert!(valley > ONE / 2, "{valley}");
        assert!(mul[0] < ONE && mul[1] > mul[0], "{mul:?}");
    }
}
