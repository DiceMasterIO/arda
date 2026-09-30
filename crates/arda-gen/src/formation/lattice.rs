//! Integer level lattices: Catmull-Rom resampling, smooth domain warp and
//! separable box blur (logic/02 §fine-formation lattices).
//!
//! Every operation is integer arithmetic with Euclidean rounding, so results
//! are identical for any thread count.

use rayon::prelude::*;

use crate::noise::value_noise;

use super::FormationError;

/// One square level lattice. Node `(x, y)` sits at `(x * spacing, y * spacing)`
/// micrometres from the fine-terrain origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lattice {
    /// Nodes per row.
    pub width: usize,
    /// Rows.
    pub height: usize,
    /// Node spacing in micrometres.
    pub spacing_um: i64,
    /// Row-major heights in millimetres.
    pub z: Vec<i32>,
}

/// Fallible zeroed allocation charged against the caller's admitted envelope.
pub(crate) fn alloc<T: Clone + Default>(count: usize) -> Result<Vec<T>, FormationError> {
    let mut v = Vec::new();
    v.try_reserve_exact(count)
        .map_err(|_| FormationError::AllocationFailed)?;
    v.resize(count, T::default());
    Ok(v)
}

impl Lattice {
    /// Zeroed lattice.
    ///
    /// # Errors
    /// Allocation failure.
    pub fn new(width: usize, height: usize, spacing_um: i64) -> Result<Self, FormationError> {
        let count = width
            .checked_mul(height)
            .ok_or(FormationError::ArithmeticOverflow)?;
        Ok(Self {
            width,
            height,
            spacing_um,
            z: alloc(count)?,
        })
    }

    #[inline]
    fn clamped(&self, x: i64, y: i64) -> i64 {
        let xc = usize::try_from(x.clamp(0, self.width as i64 - 1)).unwrap_or(0);
        let yc = usize::try_from(y.clamp(0, self.height as i64 - 1)).unwrap_or(0);
        i64::from(self.z[yc * self.width + xc])
    }

    /// Catmull-Rom sample at an absolute position in micrometres; the
    /// lattice edge is clamped.
    #[must_use]
    pub fn sample_um(&self, x_um: i64, y_um: i64) -> i32 {
        let fx = x_um.clamp(0, (self.width as i64 - 1) * self.spacing_um);
        let fy = y_um.clamp(0, (self.height as i64 - 1) * self.spacing_um);
        let x1 = fx.div_euclid(self.spacing_um);
        let y1 = fy.div_euclid(self.spacing_um);
        let tx = (fx.rem_euclid(self.spacing_um) << 16) / self.spacing_um;
        let ty = (fy.rem_euclid(self.spacing_um) << 16) / self.spacing_um;
        let wx = catmull_weights(tx);
        let wy = catmull_weights(ty);
        let mut acc: i64 = 0;
        for (j, wyj) in wy.iter().enumerate() {
            let mut row: i64 = 0;
            for (i, wxi) in wx.iter().enumerate() {
                row += wxi * self.clamped(x1 - 1 + i as i64, y1 - 1 + j as i64);
            }
            acc += wyj * row.div_euclid(1 << 8);
        }
        // Weights are Q16 each: Q32 total, with 8 bits removed per row.
        let v = acc.div_euclid(1 << 24);
        i32::try_from(v.clamp(i64::from(i32::MIN), i64::from(i32::MAX))).unwrap_or(0)
    }

    /// Catmull-Rom sample clamped to the range of the four enclosing nodes.
    /// The cubic overshoots at steep steps: next to a 1.5 km coast cliff it
    /// undershot the lowstand base level, and the cells that sampled the
    /// undershoot became a straight line of fixed sea inside the land
    /// (logic/02 §fine-formation macro warp).
    #[must_use]
    pub fn sample_um_bounded(&self, x_um: i64, y_um: i64) -> i32 {
        let fx = x_um.clamp(0, (self.width as i64 - 1) * self.spacing_um);
        let fy = y_um.clamp(0, (self.height as i64 - 1) * self.spacing_um);
        let (x1, y1) = (
            fx.div_euclid(self.spacing_um),
            fy.div_euclid(self.spacing_um),
        );
        let nodes = [
            self.clamped(x1, y1),
            self.clamped(x1 + 1, y1),
            self.clamped(x1, y1 + 1),
            self.clamped(x1 + 1, y1 + 1),
        ];
        let lo = nodes.iter().copied().min().unwrap_or(0);
        let hi = nodes.iter().copied().max().unwrap_or(0);
        let v = i64::from(self.sample_um(x_um, y_um)).clamp(lo, hi);
        i32::try_from(v).unwrap_or(0)
    }

    /// Resample onto a finer lattice with a smooth warp of amplitude
    /// `warp_um` and wavelength `wavelength_um`; a zero amplitude is a plain
    /// Catmull-Rom upsample. The warp's peak derivative stays below one so
    /// the mapping never folds (logic/02 §fine-formation warp).
    ///
    /// # Errors
    /// Allocation failure.
    pub fn resample_warped(
        &self,
        width: usize,
        height: usize,
        spacing_um: i64,
        warp_um: i64,
        wavelength_um: i64,
        seed: u64,
    ) -> Result<Lattice, FormationError> {
        let mut out = Lattice::new(width, height, spacing_um)?;
        let period_m = i32::try_from((wavelength_um / 1_000_000).max(1))
            .map_err(|_| FormationError::ArithmeticOverflow)?;
        out.z
            .par_chunks_mut(width)
            .enumerate()
            .for_each(|(y, row)| {
                let wy = y as i64 * spacing_um;
                for (x, v) in row.iter_mut().enumerate() {
                    let wx = x as i64 * spacing_um;
                    let (dx, dy) = if warp_um == 0 {
                        (0, 0)
                    } else {
                        let xm = i32::try_from(wx / 1_000_000).unwrap_or(0);
                        let ym = i32::try_from(wy / 1_000_000).unwrap_or(0);
                        (
                            warp_um * i64::from(value_noise(seed, xm, ym, period_m)) / 32_768,
                            warp_um * i64::from(value_noise(seed ^ 0xABCD, xm, ym, period_m))
                                / 32_768,
                        )
                    };
                    *v = self.sample_um(wx + dx, wy + dy);
                }
            });
        Ok(out)
    }
}

/// Q16 Catmull-Rom weights for a Q16 fraction; they sum to 65536.
fn catmull_weights(t: i64) -> [i64; 4] {
    let t2 = (t * t) >> 16;
    let t3 = (t2 * t) >> 16;
    let w0 = (-t3 + 2 * t2 - t) / 2;
    let w1 = (3 * t3 - 5 * t2 + 2 * 65_536) / 2;
    let w2 = (-3 * t3 + 4 * t2 + t) / 2;
    let w3 = 65_536 - w0 - w1 - w2;
    [w0, w1, w2, w3]
}

/// Two-pass separable box blur of radius `r`, into `out`, using `tmp` as the
/// horizontal intermediate. Edges clamp.
pub(crate) fn blur_into(
    src: &[i32],
    width: usize,
    height: usize,
    r: usize,
    tmp: &mut [i32],
    out: &mut [i32],
) {
    out.copy_from_slice(src);
    if r == 0 {
        return;
    }
    for _ in 0..2 {
        tmp.par_chunks_mut(width)
            .zip(out.par_chunks(width))
            .for_each(|(dst, row)| box_line(row, dst, r));
        // Columns in blocks of 64: each block keeps its own running sums.
        let block = 64;
        let cols: Vec<usize> = (0..width).step_by(block).collect();
        let parts: Vec<(usize, Vec<i32>)> = cols
            .par_iter()
            .map(|&x0| {
                let bw = block.min(width - x0);
                let mut res = vec![0_i32; bw * height];
                let mut col = vec![0_i32; height];
                let mut line = vec![0_i32; height];
                for c in 0..bw {
                    for (y, v) in col.iter_mut().enumerate() {
                        *v = tmp[y * width + x0 + c];
                    }
                    box_line(&col, &mut line, r);
                    for y in 0..height {
                        res[y * bw + c] = line[y];
                    }
                }
                (x0, res)
            })
            .collect();
        for (x0, res) in parts {
            let bw = block.min(width - x0);
            for y in 0..height {
                out[y * width + x0..y * width + x0 + bw]
                    .copy_from_slice(&res[y * bw..(y + 1) * bw]);
            }
        }
    }
}

fn box_line(src: &[i32], out: &mut [i32], r: usize) {
    let n = src.len() as i64;
    let ri = r as i64;
    let get = |i: i64| i64::from(src[usize::try_from(i.clamp(0, n - 1)).unwrap_or(0)]);
    let mut s: i64 = (-ri..=ri).map(get).sum();
    let norm = 2 * ri + 1;
    for i in 0..n {
        out[usize::try_from(i).unwrap_or(0)] = i32::try_from(s.div_euclid(norm)).unwrap_or(0);
        s += get(i + ri + 1) - get(i - ri);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catmull_weights_partition_unity_and_interpolate_nodes() {
        for t in [0, 1, 16_384, 32_768, 65_535] {
            assert_eq!(catmull_weights(t).iter().sum::<i64>(), 65_536);
        }
        assert_eq!(catmull_weights(0), [0, 65_536, 0, 0]);
    }

    #[test]
    fn upsample_reproduces_nodes_and_linear_ramps() {
        let mut g = Lattice::new(8, 8, 1_000_000).unwrap();
        for y in 0..8 {
            for x in 0..8 {
                g.z[y * 8 + x] = i32::try_from(1000 * x + 300 * y).unwrap();
            }
        }
        let f = g.resample_warped(15, 15, 500_000, 0, 1, 0).unwrap();
        // Interior only: the clamped edge is not a linear extrapolation.
        for y in 3..12 {
            for x in 3..12 {
                let expect = i32::try_from(500 * x + 150 * y).unwrap();
                assert!((f.z[y * 15 + x] - expect).abs() <= 1, "{x},{y}");
            }
        }
    }

    #[test]
    fn bounded_sample_never_overshoots_a_cliff() {
        // A 1.5 km coast cliff above a -30 m shelf: the plain cubic
        // undershoots the shelf beside the cliff, the bounded one never
        // leaves the range of the enclosing nodes.
        let mut g = Lattice::new(8, 4, 1_000_000).unwrap();
        for y in 0..4 {
            for x in 0..8 {
                g.z[y * 8 + x] = if x < 4 { 1_500_000 } else { -30_000 };
            }
        }
        let y = 1_500_000;
        let x = 4_300_000;
        assert!(g.sample_um(x, y) < -30_000, "cubic undershoot expected");
        for x in (0..7_000_000).step_by(50_000) {
            let v = g.sample_um_bounded(x, y);
            assert!((-30_000..=1_500_000).contains(&v), "{x}: {v}");
        }
        assert_eq!(g.sample_um_bounded(2_000_000, y), 1_500_000);
    }

    #[test]
    fn blur_preserves_constants_and_is_thread_independent() {
        let (w, h) = (97, 53);
        let src: Vec<i32> = (0..w * h)
            .map(|i| i32::try_from((i * 7919) % 1000).unwrap())
            .collect();
        let mut tmp = vec![0; w * h];
        let mut a = vec![0; w * h];
        blur_into(&src, w, h, 5, &mut tmp, &mut a);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        let mut b = vec![0; w * h];
        pool.install(|| blur_into(&src, w, h, 5, &mut tmp, &mut b));
        assert_eq!(a, b);
        let flat = vec![1234; w * h];
        blur_into(&flat, w, h, 9, &mut tmp, &mut b);
        assert!(b.iter().all(|&v| v == 1234));
    }
}
