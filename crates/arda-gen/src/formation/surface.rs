//! Surface passes on formed lattices (logic/02 §fine-formation roughness
//! and seams) and the fixed flags of the final drainage fills.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use rayon::prelude::*;

use super::drainage::{self, FIXED};
use super::lattice::{alloc, Lattice};
use super::FormationError;
use crate::noise::value_noise;

/// logic/02 §fine-formation roughness: multi-scale (600-75 m) rugged
/// texture added on top of the formed surface, so slopes are not glassy.
/// Amplitude grows with local slope (rock faces) and macro relief, and
/// fades to zero near channels (area ≥ 0.05 km²) so drainage stays clean.
/// Octaves are sampled in two rotated domains, never axis-aligned.
pub(super) fn roughen(
    g: &mut Lattice,
    flags: &[u8],
    area: Option<&[u64]>,
    relief: &[u8],
    seed: u64,
) -> Result<(), FormationError> {
    let (w, h) = (g.width, g.height);
    let d_mm = g.spacing_um / 1000;
    let mut out: Vec<i32> = alloc(w * h)?;
    let z = &g.z;
    // 0.05 km^2 in Q8 finest-cell units.
    let channel_q8: u64 = 8_389;
    out.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (x, o) in row.iter_mut().enumerate() {
            let i = y * w + x;
            *o = z[i];
            if flags[i] & FIXED != 0 || z[i] <= 0 || x == 0 || y == 0 || x + 1 == w || y + 1 == h {
                continue;
            }
            let gx = i64::from(z[i + 1]) - i64::from(z[i - 1]);
            let gy = i64::from(z[i + w]) - i64::from(z[i - w]);
            let slope_q12 = i64::try_from(
                (i128::from(gx) * i128::from(gx) + i128::from(gy) * i128::from(gy))
                    .unsigned_abs()
                    .isqrt(),
            )
            .unwrap_or(0)
                * 4_096
                / (2 * d_mm).max(1);
            // 0.25 at flat ground to 1 at slope >= 0.6.
            let steep = 1_024 + 3_072 * slope_q12.min(2_458) / 2_458;
            let rel = 64 + i64::from(relief[i]) * 192 / 255;
            let a = area.map_or(0, |a| a[i]);
            let away = if a >= channel_q8 {
                0
            } else {
                4_096 - i64::try_from(a * 4_096 / channel_q8).unwrap_or(4_096)
            };
            let amp_mm = 70_000 * steep / 4_096 * rel / 256 * away / 4_096;
            if amp_mm == 0 {
                continue;
            }
            let xm = x as i64 * g.spacing_um / 1_000_000;
            let ym = y as i64 * g.spacing_um / 1_000_000;
            let (ax, ay) = (
                (xm * 3_271 - ym * 2_465) / 4_096,
                (xm * 2_465 + ym * 3_271) / 4_096,
            );
            let (bx, by) = (
                (xm * 3_770 + ym * 1_600) / 4_096,
                (-xm * 1_600 + ym * 3_770) / 4_096,
            );
            // Octaves no finer than ~5 cells (finer ones alias against the
            // 39 m lattice); each is the mean of two rotated domains so no
            // lattice direction survives.
            let mut sum = 0_i64;
            for (k, (period, weight)) in [(800, 4), (400, 2), (200, 1)].into_iter().enumerate() {
                let salt = seed ^ ((k as u64 + 1) << 48);
                let va = i64::from(value_noise(salt, ax as i32, ay as i32, period));
                let vb = i64::from(value_noise(salt ^ 0x5A5A, bx as i32, by as i32, period));
                sum += weight * (va + vb);
            }
            // sum within about ±14 * 32768; two-domain mean halves variance,
            // so scale back up by ~sqrt(2).
            let dz = amp_mm * sum * 181 / (128 * 7 * 32_768);
            *o = i32::try_from(i64::from(z[i]) + dz).unwrap_or(z[i]).max(1);
        }
    });
    g.z = out;
    Ok(())
}

/// `passes` of a 3x3 binomial (1-2-1) filter over non-fixed land cells
/// (two for recipe 5, one for recipe 6).
pub(super) fn smooth_seams(
    g: &mut Lattice,
    flags: &[u8],
    passes: usize,
) -> Result<(), FormationError> {
    let (w, h) = (g.width, g.height);
    let mut tmp: Vec<i32> = alloc(w * h)?;
    for _ in 0..passes {
        let z = &g.z;
        tmp.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
            for (x, out) in row.iter_mut().enumerate() {
                let i = y * w + x;
                if flags[i] & FIXED != 0
                    || z[i] <= 0
                    || x == 0
                    || y == 0
                    || x + 1 == w
                    || y + 1 == h
                {
                    *out = z[i];
                    continue;
                }
                let at = |dx: i64, dy: i64| {
                    i64::from(z[((y as i64 + dy) as usize) * w + (x as i64 + dx) as usize])
                };
                let sum = 4 * at(0, 0)
                    + 2 * (at(1, 0) + at(-1, 0) + at(0, 1) + at(0, -1))
                    + at(1, 1)
                    + at(-1, 1)
                    + at(1, -1)
                    + at(-1, -1);
                *out = i32::try_from(sum.div_euclid(16)).unwrap_or(z[i]).max(1);
            }
        });
        g.z.copy_from_slice(&tmp);
    }
    Ok(())
}

/// Fixed flags for the final fills: the open sea and protected sinks.
pub(super) fn sea_and_sinks(
    g: &Lattice,
    is_sink: &(dyn Fn(i64, i64) -> bool + Sync),
) -> Result<Vec<u8>, FormationError> {
    let (w, h) = (g.width, g.height);
    let mut flags: Vec<u8> = alloc(w * h)?;
    drainage::open_sea_flags(&g.z, w, h, &mut flags);
    let d = g.spacing_um;
    flags.par_iter_mut().enumerate().for_each(|(i, f)| {
        if is_sink((i % w) as i64 * d, (i / w) as i64 * d) {
            *f = FIXED;
        }
    });
    Ok(flags)
}
