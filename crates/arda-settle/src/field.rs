//! Neighbourhood fields: box sums over integral images and chamfer distances.

use crate::error::SettleError;
use crate::grid::{filled, Grid};
use crate::num::{iu, ui};

/// Summed-area table for O(1) box sums.
#[derive(Debug, Clone)]
pub struct Integral {
    width: usize,
    height: usize,
    sums: Vec<i64>,
}

impl Integral {
    /// Builds the table from per-cell values.
    ///
    /// # Errors
    /// [`SettleError::Reserve`] when the table cannot be allocated.
    pub fn new(
        width: usize,
        height: usize,
        value: impl Fn(usize) -> i64,
    ) -> Result<Self, SettleError> {
        let w1 = width + 1;
        let mut sums = filled(w1 * (height + 1), 0_i64, "integral image")?;
        for y in 0..height {
            let mut row = 0_i64;
            for x in 0..width {
                row += value(y * width + x);
                sums[(y + 1) * w1 + x + 1] = sums[y * w1 + x + 1] + row;
            }
        }
        Ok(Self {
            width,
            height,
            sums,
        })
    }

    /// Sum and cell count over the square of radius `r` cells around `(x, y)`,
    /// clipped to the grid.
    #[must_use]
    pub fn sum(&self, x: i64, y: i64, r: i64) -> (i64, i64) {
        let x0 = iu(x - r);
        let y0 = iu(y - r);
        let x1 = iu((x + r + 1).min(ui(self.width)));
        let y1 = iu((y + r + 1).min(ui(self.height)));
        if x1 <= x0 || y1 <= y0 {
            return (0, 0);
        }
        let w1 = self.width + 1;
        let s = self.sums[y1 * w1 + x1] - self.sums[y0 * w1 + x1] - self.sums[y1 * w1 + x0]
            + self.sums[y0 * w1 + x0];
        (s, ui((x1 - x0) * (y1 - y0)))
    }

    /// Box sum scaled to per-mille of the box's cell count.
    #[must_use]
    pub fn permille(&self, x: i64, y: i64, r: i64) -> i64 {
        let (s, n) = self.sum(x, y, r);
        if n == 0 {
            0
        } else {
            s * 1000 / n
        }
    }

    /// Box mean.
    #[must_use]
    pub fn mean(&self, x: i64, y: i64, r: i64) -> i64 {
        let (s, n) = self.sum(x, y, r);
        if n == 0 {
            0
        } else {
            s / n
        }
    }
}

/// Chamfer (100 / 141) distance in metres from every cell to the nearest
/// cell where `source` holds, capped at `cap` metres.
///
/// # Errors
/// [`SettleError::Reserve`] when the raster cannot be allocated.
pub fn distance_m(
    grid: &Grid,
    source: impl Fn(usize) -> bool,
    cap: u32,
) -> Result<Vec<u32>, SettleError> {
    let (w, h) = (grid.width, grid.height);
    let mut d = filled(w * h, cap, "distance field")?;
    for (i, v) in d.iter_mut().enumerate() {
        if source(i) {
            *v = 0;
        }
    }
    let relax = |d: &mut Vec<u32>, i: usize, j: usize, step: u32| {
        let c = d[j].saturating_add(step);
        if c < d[i] {
            d[i] = c;
        }
    };
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if x > 0 {
                relax(&mut d, i, i - 1, 100);
            }
            if y > 0 {
                relax(&mut d, i, i - w, 100);
                if x > 0 {
                    relax(&mut d, i, i - w - 1, 141);
                }
                if x + 1 < w {
                    relax(&mut d, i, i - w + 1, 141);
                }
            }
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            if x + 1 < w {
                relax(&mut d, i, i + 1, 100);
            }
            if y + 1 < h {
                relax(&mut d, i, i + w, 100);
                if x + 1 < w {
                    relax(&mut d, i, i + w + 1, 141);
                }
                if x > 0 {
                    relax(&mut d, i, i + w - 1, 141);
                }
            }
        }
    }
    Ok(d)
}

/// Linear ramp: `full` per-mille at or below `lo`, nothing at or above `hi`.
#[must_use]
pub fn falloff(v: i64, lo: i64, hi: i64) -> i64 {
    if v <= lo {
        1000
    } else if v >= hi {
        0
    } else {
        (hi - v) * 1000 / (hi - lo)
    }
}

/// Linear ramp: nothing at or below `lo`, full per-mille at or above `hi`.
#[must_use]
pub fn rise(v: i64, lo: i64, hi: i64) -> i64 {
    1000 - falloff(v, lo, hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_sums_match_brute_force() {
        let f = |i: usize| i64::try_from(i % 7).unwrap();
        let t = Integral::new(9, 5, f).unwrap();
        let (s, n) = t.sum(4, 2, 1);
        let mut want = 0;
        for y in 1..=3 {
            for x in 3..=5 {
                want += f(y * 9 + x);
            }
        }
        assert_eq!((s, n), (want, 9));
        assert_eq!(t.sum(0, 0, 1).1, 4);
    }

    #[test]
    fn ramps() {
        assert_eq!(falloff(150, 150, 750), 1000);
        assert_eq!(falloff(450, 150, 750), 500);
        assert_eq!(rise(30, 0, 30), 1000);
    }
}
