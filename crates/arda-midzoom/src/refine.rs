//! On-demand refinement of the stored 39 m field (logic/17 §means, §pits).
//!
//! Per refined node: the sharpened spline base, plus drainage-aligned
//! detail whose amplitude follows local roughness (the stored field's own
//! short-wavelength energy, a maturity proxy), slope, relief and cover, and
//! fades to nothing on watercourses and water. Two correction rounds then
//! restore every stored node's 39 m cell mean, and two passes lift any
//! single-node hollow. Each step is a fixed stencil of global nodes, so a
//! node's height never depends on the window that asked for it: tiles join
//! pixel-exactly.

use crate::base::{bilinear_at, sharpen, spline_at, Coarse, COARSE_HALO};
use crate::detail::{detail, mantle, Style};
use crate::fixed::{smooth, ONE};
use crate::height::HeightTile;
use crate::masks::CellMasks;
use crate::source::{Terrain, FINE_UM};
use crate::MidzoomError;
use rayon::prelude::*;

/// Largest refined window side, in nodes.
pub const MAX_NODES: usize = 2_048;
const CORRECTION_ROUNDS: i64 = 2;
const PIT_PASSES: i64 = 2;
/// Soil-mantle hummock amplitude on gentle ground, mm.
const MANTLE_MM: i64 = 800;
/// Detail gain over the stored roughness, Q12.
const AMP_GAIN_Q12: i64 = 8_192;

/// Refined node halo needed around a window for `n` subdivisions.
const fn halo(n: i64) -> i64 {
    CORRECTION_ROUNDS * (3 * n + n / 2 + 1) + PIT_PASSES + 2
}

/// Refines nodes `[i0, i0 + w) × [j0, j0 + h)` of the lattice `n` times
/// finer than the stored one (`n` ∈ {1, 2, 4, 8}; 4 gives 9.765625 m).
///
/// # Errors
/// Unsupported `n`, an empty or oversized window, or a read failure.
pub fn refine_nodes(
    src: &dyn Terrain,
    i0: i64,
    j0: i64,
    w: usize,
    h: usize,
    n: i64,
) -> Result<HeightTile, MidzoomError> {
    refine_nodes_for(src, (i0, j0, w, h), n, 0)
}

/// As [`refine_nodes`], fading detail wavelengths below `min_wave_um`
/// (screen pixels would alias them). The result depends only on the
/// global node, `n` and `min_wave_um`, so one pyramid level is seamless.
///
/// # Errors
/// Unsupported `n`, an empty or oversized window, or a read failure.
pub fn refine_nodes_for(
    src: &dyn Terrain,
    (i0, j0, w, h): (i64, i64, usize, usize),
    n: i64,
    min_wave_um: i64,
) -> Result<HeightTile, MidzoomError> {
    if ![1, 2, 4, 8].contains(&n) {
        return Err(MidzoomError::Window(format!("{n} subdivisions")));
    }
    if w == 0 || h == 0 || w > MAX_NODES || h > MAX_NODES {
        return Err(MidzoomError::Window(format!("{w}x{h} nodes")));
    }
    let hr = halo(n);
    let (wi0, wj0) = (i0 - hr, j0 - hr);
    let ww = w + 2 * usize::try_from(hr).unwrap_or(0);
    let wh = h + 2 * usize::try_from(hr).unwrap_or(0);
    let (ww64, wh64) = (
        i64::try_from(ww).unwrap_or(0),
        i64::try_from(wh).unwrap_or(0),
    );
    let kx0 = wi0.div_euclid(n) - COARSE_HALO;
    let ky0 = wj0.div_euclid(n) - COARSE_HALO;
    let kx1 = (wi0 + ww64 - 1).div_euclid(n) + COARSE_HALO + 1;
    let ky1 = (wj0 + wh64 - 1).div_euclid(n) + COARSE_HALO + 1;
    let cw = usize::try_from(kx1 - kx0 + 1).unwrap_or(0);
    let ch = usize::try_from(ky1 - ky0 + 1).unwrap_or(0);
    let coarse = Coarse::read(src, kx0, ky0, cw, ch)?;
    let s = FINE_UM / n;
    let masks = CellMasks::read(src, wi0 * s, wj0 * s, (wi0 + ww64) * s, (wj0 + wh64) * s)?;
    let seed = src.seed();
    let mut r: Vec<i64> = (0..wh64)
        .into_par_iter()
        .flat_map_iter(|y| {
            let (coarse, masks) = (&coarse, &masks);
            (0..ww64).map(move |x| {
                let (i, j) = (wi0 + x, wj0 + y);
                let (b, gx, gy) = spline_at(coarse, &coarse.coef, i, j, n);
                let (amp, rib_q12, turf) = amplitude(coarse, masks, (i, j), n, (gx, gy), b, seed);
                let style = Style {
                    min_wave_um,
                    rib_q12,
                };
                let slope = crate::fixed::isqrt(gx * gx + gy * gy);
                b + detail(seed, i * s, j * s, s, (gx, gy), amp, style).height_mm
                    + mantle(seed, i * s, j * s, s, slope, turf, min_wave_um)
            })
        })
        .collect();
    for _ in 0..CORRECTION_ROUNDS {
        correct_means(&coarse, &mut r, (wi0, wj0, ww, wh), n);
    }
    for _ in 0..PIT_PASSES {
        lift_pits(&masks, &mut r, (wi0, wj0, ww, wh), s);
    }
    let off = usize::try_from(hr).unwrap_or(0);
    let mut heights_mm = Vec::new();
    heights_mm
        .try_reserve_exact(w * h)
        .map_err(|_| MidzoomError::ResourceLimit("refined tile"))?;
    for y in 0..h {
        let row = &r[(y + off) * ww + off..(y + off) * ww + off + w];
        heights_mm.extend(row.iter().map(|&v| i32::try_from(v).unwrap_or(0)));
    }
    Ok(HeightTile {
        i0,
        j0,
        width: w,
        height: h,
        n,
        heights_mm,
    })
}

/// Coarsest-octave detail amplitude (mm) at node `(i, j)`, its rib blend
/// (Q12) and the soil-mantle amplitude (mm).
fn amplitude(
    coarse: &Coarse,
    masks: &CellMasks,
    (i, j): (i64, i64),
    n: i64,
    grad: (i64, i64),
    base_mm: i64,
    seed: u64,
) -> (i64, i64, i64) {
    if base_mm <= 0 {
        return (0, 0, 0);
    }
    let s = FINE_UM / n;
    let (x, y) = (i * s, j * s);
    let slope = crate::fixed::isqrt(grad.0 * grad.0 + grad.1 * grad.1);
    let rough = bilinear_at(coarse, &coarse.rough, i, j, n);
    let relief = bilinear_at(coarse, &coarse.relief, i, j, n);
    // Low, gentle ground keeps its soil mantle; steep, high-relief ground
    // gets its ribs and gullies.
    let steep = smooth(492, 2_048, slope);
    let high = smooth(40_000, 200_000, relief);
    let river = ONE - masks.river(x, y);
    let dry = ONE - smooth(0, ONE / 2, masks.water(x, y));
    // Slope position: gullies head just below the divide, deepen downslope
    // and die out on the valley floor, where the stored channel takes over.
    let (lo, hi) = (
        bilinear_at(coarse, &coarse.low, i, j, n),
        bilinear_at(coarse, &coarse.high, i, j, n),
    );
    let rel = if hi > lo {
        (base_mm - lo) * ONE / (hi - lo)
    } else {
        0
    };
    let position =
        1_024 + 3_072 * smooth(328, 1_434, rel) / ONE * (ONE - smooth(3_277, ONE, rel)) / ONE;
    // Patchy erosion: whole hillsides are more or less dissected.
    let patch =
        1_229 + 5_734 * crate::fixed::organic_q12(seed, 0x7061_7463, x, y, 260_000_000) / ONE;
    let cover = masks.cover(x, y);
    let mut a = rough * AMP_GAIN_Q12 / ONE;
    for f in [steep, high, cover, river, dry, position, patch] {
        a = a * f / ONE;
    }
    // Turf hummocks where the ground is gentle but not a flat floor.
    let mut turf = MANTLE_MM;
    for f in [
        ONE - steep * high / ONE,
        smooth(123, 328, slope),
        river,
        dry,
    ] {
        turf = turf * f / ONE;
    }
    (a, smooth(2_458, 4_506, slope), turf)
}

/// One mean-restoring round: every stored node's 39 m cell mean
/// (trapezoid over its refined nodes) moves back to the stored height, via
/// the spline of the sharpened residuals.
fn correct_means(coarse: &Coarse, r: &mut [i64], win: (i64, i64, usize, usize), n: i64) {
    let (wi0, wj0, ww, wh) = win;
    let (ww64, wh64) = (
        i64::try_from(ww).unwrap_or(0),
        i64::try_from(wh).unwrap_or(0),
    );
    let half = n / 2;
    let weight = |d: i64| -> i64 {
        if n == 1 || d.abs() == half {
            1
        } else {
            2
        }
    };
    let total = if n == 1 { 1 } else { (2 * n) * (2 * n) };
    let at = |i: i64, j: i64| -> Option<i64> {
        let (x, y) = (i - wi0, j - wj0);
        if x < 0 || y < 0 || x >= ww64 || y >= wh64 {
            return None;
        }
        usize::try_from(y * ww64 + x)
            .ok()
            .and_then(|k| r.get(k))
            .copied()
    };
    let mut residual = vec![0_i64; coarse.w * coarse.h];
    for cy in 0..coarse.h {
        for cx in 0..coarse.w {
            let (kx, ky) = (
                coarse.kx0 + i64::try_from(cx).unwrap_or(0),
                coarse.ky0 + i64::try_from(cy).unwrap_or(0),
            );
            let mut sum = 0_i64;
            let mut inside = true;
            'cell: for dy in -half..=half {
                for dx in -half..=half {
                    if let Some(v) = at(kx * n + dx, ky * n + dy) {
                        sum += v * weight(dx) * weight(dy);
                    } else {
                        inside = false;
                        break 'cell;
                    }
                }
            }
            if inside {
                let mean = crate::fixed::round_div(i128::from(sum), i128::from(total));
                residual[cy * coarse.w + cx] =
                    coarse.stored[cy * coarse.w + cx] - i64::try_from(mean).unwrap_or(0);
            }
        }
    }
    let field = Coarse {
        coef: sharpen(&residual, coarse.w, coarse.h),
        ..coarse.clone()
    };
    r.par_chunks_mut(ww).enumerate().for_each(|(y, row)| {
        let j = wj0 + i64::try_from(y).unwrap_or(0);
        for (x, v) in row.iter_mut().enumerate() {
            let i = wi0 + i64::try_from(x).unwrap_or(0);
            *v += spline_at(&field, &field.coef, i, j, n).0;
        }
    });
}

/// Lifts every land node lower than all eight neighbours to 1 mm above the
/// lowest of them (window edges are left alone; the halo absorbs them).
fn lift_pits(masks: &CellMasks, r: &mut [i64], win: (i64, i64, usize, usize), s: i64) {
    let (wi0, wj0, ww, wh) = win;
    let src = r.to_vec();
    r.par_chunks_mut(ww).enumerate().for_each(|(y, row)| {
        if y == 0 || y + 1 >= wh {
            return;
        }
        for x in 1..ww.saturating_sub(1) {
            let c = src[y * ww + x];
            let mut low = i64::MAX;
            for yy in y - 1..=y + 1 {
                for xx in x - 1..=x + 1 {
                    if xx != x || yy != y {
                        low = low.min(src[yy * ww + xx]);
                    }
                }
            }
            if c < low && c > 0 {
                let (i, j) = (
                    wi0 + i64::try_from(x).unwrap_or(0),
                    wj0 + i64::try_from(y).unwrap_or(0),
                );
                if masks.water(i * s, j * s) < ONE / 2 {
                    row[x] = low + 1;
                }
            }
        }
    });
}
