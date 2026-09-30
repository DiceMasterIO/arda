//! The stored 39 m field as a smooth base surface (logic/17 §base).
//!
//! A cubic B-spline is C2 and never shows the lattice, but on raw heights
//! it low-passes: crests sink and valleys fill by up to a third of the
//! local relief, which is the blur the maintainer saw. Three sharpening
//! iterations `C ← C + (H − S·C)` (S the spline's own node filter,
//! [1 4 1]/6 per axis) turn the heights into near-interpolating spline
//! coefficients, clamped to the 3×3 stored range so the surface never
//! rings into new pits or peaks. Every value is a fixed stencil of global
//! nodes, so any window reproduces it exactly.

use crate::source::Terrain;
use crate::MidzoomError;

/// Coarse halo (stored nodes) a caller needs around its refined span.
pub const COARSE_HALO: i64 = 10;
const SHARPEN_ITERATIONS: usize = 3;

/// A window of stored nodes with the per-node fields refinement reads.
#[derive(Debug, Clone)]
pub struct Coarse {
    /// First stored node column.
    pub kx0: i64,
    /// First stored node row.
    pub ky0: i64,
    /// Width in nodes.
    pub w: usize,
    /// Height in nodes.
    pub h: usize,
    /// Stored heights, mm.
    pub stored: Vec<i64>,
    /// Sharpened spline coefficients, mm.
    pub coef: Vec<i64>,
    /// Local roughness: mean |H − mean of 8 neighbours| over 3×3, mm.
    pub rough: Vec<i64>,
    /// Relief (max − min) over 5×5 nodes (≈ 156 m), mm.
    pub relief: Vec<i64>,
    /// Lowest stored height over 9×9 nodes (±156 m), mm.
    pub low: Vec<i64>,
    /// Highest stored height over 9×9 nodes (±156 m), mm.
    pub high: Vec<i64>,
}

impl Coarse {
    /// Reads and derives the window `[kx0, kx0 + w) × [ky0, ky0 + h)`.
    ///
    /// # Errors
    /// The stored layer failed to read.
    pub fn read(
        src: &dyn Terrain,
        kx0: i64,
        ky0: i64,
        w: usize,
        h: usize,
    ) -> Result<Self, MidzoomError> {
        let stored: Vec<i64> = src
            .read_nodes(kx0, ky0, w, h)?
            .into_iter()
            .map(i64::from)
            .collect();
        let at = |v: &[i64], x: isize, y: isize| -> i64 {
            let xi = x.clamp(0, isize::try_from(w).unwrap_or(1) - 1);
            let yi = y.clamp(0, isize::try_from(h).unwrap_or(1) - 1);
            v[usize::try_from(yi).unwrap_or(0) * w + usize::try_from(xi).unwrap_or(0)]
        };
        let map = |f: &dyn Fn(isize, isize) -> i64| -> Vec<i64> {
            let mut out = Vec::with_capacity(w * h);
            for y in 0..h {
                for x in 0..w {
                    out.push(f(
                        isize::try_from(x).unwrap_or(0),
                        isize::try_from(y).unwrap_or(0),
                    ));
                }
            }
            out
        };
        let coef = sharpen(&stored, w, h);
        let range = |x: isize, y: isize, r: isize| -> (i64, i64) {
            let mut lo = i64::MAX;
            let mut hi = i64::MIN;
            for dy in -r..=r {
                for dx in -r..=r {
                    let v = at(&stored, x + dx, y + dy);
                    lo = lo.min(v);
                    hi = hi.max(v);
                }
            }
            (lo, hi)
        };
        let coef = map(&|x, y| {
            let (lo, hi) = range(x, y, 1);
            at(&coef, x, y).clamp(lo, hi)
        });
        let dev = map(&|x, y| {
            let mut s = 0;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx != 0 || dy != 0 {
                        s += at(&stored, x + dx, y + dy);
                    }
                }
            }
            (at(&stored, x, y) * 8 - s).abs() / 8
        });
        let rough = map(&|x, y| {
            let mut s = 0;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    s += at(&dev, x + dx, y + dy);
                }
            }
            s / 9
        });
        let relief = map(&|x, y| {
            let (lo, hi) = range(x, y, 2);
            hi - lo
        });
        let low = map(&|x, y| range(x, y, 4).0);
        let high = map(&|x, y| range(x, y, 4).1);
        Ok(Self {
            kx0,
            ky0,
            w,
            h,
            stored,
            coef,
            rough,
            relief,
            low,
            high,
        })
    }

    /// Value of `field` at global node `(kx, ky)` (clamped into the window).
    #[must_use]
    pub fn at(&self, field: &[i64], kx: i64, ky: i64) -> i64 {
        let x = (kx - self.kx0).clamp(0, i64::try_from(self.w).unwrap_or(1) - 1);
        let y = (ky - self.ky0).clamp(0, i64::try_from(self.h).unwrap_or(1) - 1);
        usize::try_from(y)
            .ok()
            .zip(usize::try_from(x).ok())
            .and_then(|(y, x)| field.get(y * self.w + x))
            .copied()
            .unwrap_or(0)
    }
}

/// Near-interpolating spline coefficients of a `w × h` grid: three rounds
/// of `C ← C + (V − S·C)` with S the spline's [1 4 1]/6 node filter per
/// axis (edges repeat).
#[must_use]
pub fn sharpen(values: &[i64], w: usize, h: usize) -> Vec<i64> {
    let at = |v: &[i64], x: usize, y: usize, dx: isize, dy: isize| -> i64 {
        let xi = x.saturating_add_signed(dx).min(w - 1);
        let yi = y.saturating_add_signed(dy).min(h - 1);
        v[yi * w + xi]
    };
    let mut coef = values.to_vec();
    if w == 0 || h == 0 {
        return coef;
    }
    for _ in 0..SHARPEN_ITERATIONS {
        let mut next = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                let row = |dy| {
                    at(&coef, x, y, -1, dy) + 4 * at(&coef, x, y, 0, dy) + at(&coef, x, y, 1, dy)
                };
                let s = row(-1) + 4 * row(0) + row(1);
                let f = if s >= 0 { (s + 18) / 36 } else { (s - 18) / 36 };
                next.push(coef[y * w + x] + values[y * w + x] - f);
            }
        }
        coef = next;
    }
    coef
}

/// Cubic B-spline weights for `t = r / n`, scaled by `6 n³`, and their
/// `d/dt` scaled by `2 n²`.
#[must_use]
pub fn spline_weights(r: i64, n: i64) -> ([i64; 4], [i64; 4]) {
    let u = n - r;
    (
        [
            u * u * u,
            3 * r * r * r - 6 * r * r * n + 4 * n * n * n,
            -3 * r * r * r + 3 * r * r * n + 3 * r * n * n + n * n * n,
            r * r * r,
        ],
        [
            -(u * u),
            3 * r * r - 4 * r * n,
            -3 * r * r + 2 * r * n + n * n,
            r * r,
        ],
    )
}

/// Evaluates the spline of `field` at refined node `(i, j)` of a lattice
/// `n` times finer than the stored one: height (mm) and gradient (Q12).
#[must_use]
pub fn spline_at(coarse: &Coarse, field: &[i64], i: i64, j: i64, n: i64) -> (i64, i64, i64) {
    let (kx, rx) = (i.div_euclid(n), i.rem_euclid(n));
    let (ky, ry) = (j.div_euclid(n), j.rem_euclid(n));
    let (wx, dwx) = spline_weights(rx, n);
    let (wy, dwy) = spline_weights(ry, n);
    let (mut v, mut gx, mut gy) = (0_i128, 0_i128, 0_i128);
    for (b, dy) in (-1..=2_i64).enumerate() {
        let (mut row, mut drow) = (0_i128, 0_i128);
        for (a, dx) in (-1..=2_i64).enumerate() {
            let z = i128::from(coarse.at(field, kx + dx, ky + dy));
            row += i128::from(wx[a]) * z;
            drow += i128::from(dwx[a]) * z;
        }
        v += i128::from(wy[b]) * row;
        gx += i128::from(wy[b]) * drow;
        gy += i128::from(dwy[b]) * row;
    }
    let n = i128::from(n);
    let scale = 36 * n * n * n * n * n * n;
    // d/dx = (d/dt) / spacing; weights carry 6n³ · 2n² = 12 n⁵.
    let dscale = 12 * n * n * n * n * n * (crate::source::FINE_UM as i128 / 1_000);
    (
        i64::try_from(crate::fixed::round_div(v, scale)).unwrap_or(0),
        i64::try_from(crate::fixed::round_div(gx * 4_096, dscale)).unwrap_or(0),
        i64::try_from(crate::fixed::round_div(gy * 4_096, dscale)).unwrap_or(0),
    )
}

/// Bilinear value of a coarse `field` at refined node `(i, j)`.
#[must_use]
pub fn bilinear_at(coarse: &Coarse, field: &[i64], i: i64, j: i64, n: i64) -> i64 {
    let (kx, rx) = (i.div_euclid(n), i.rem_euclid(n));
    let (ky, ry) = (j.div_euclid(n), j.rem_euclid(n));
    let f = |x, y| i128::from(coarse.at(field, x, y));
    let top = f(kx, ky) * i128::from(n - rx) + f(kx + 1, ky) * i128::from(rx);
    let bottom = f(kx, ky + 1) * i128::from(n - rx) + f(kx + 1, ky + 1) * i128::from(rx);
    let v = top * i128::from(n - ry) + bottom * i128::from(ry);
    i64::try_from(crate::fixed::round_div(v, i128::from(n * n))).unwrap_or(0)
}
