//! Drainage-aligned sub-39 m relief (logic/17 §detail).
//!
//! Each octave is a sum of short stripe kernels around jittered feature
//! points. A stripe's phase runs *across* the local fall line, so its
//! crests and troughs run downslope: spurs and gullies aligned with
//! drainage. Each octave reads the gradient left by the coarser ones, so
//! finer gullies turn down the walls of coarser ones and join them —
//! branching, converging drainage rather than parallel rills. Octave
//! domains are rotated by unrelated angles so no lattice axis shows, and
//! every value is a pure function of the seed and the global position.

use crate::fixed::{cos_sin_q14, hash2, isqrt128, round_div, TRIG_ONE};

/// One octave: wavelength, domain rotation (Q14 cos/sin), relative weight
/// (Q12) and hash stream.
#[derive(Debug, Clone, Copy)]
struct Octave {
    lambda_um: i64,
    cos: i64,
    sin: i64,
    weight_q12: i64,
    tag: u64,
}

/// Wavelengths 72, 44 and 30 m; rotations 37°, −23° and 61°.
const OCTAVES: [Octave; 3] = [
    Octave {
        lambda_um: 72_000_000,
        cos: 13_085,
        sin: 9_860,
        weight_q12: 4_096,
        tag: 0x6d69_647a_6f6f_6d31,
    },
    Octave {
        lambda_um: 44_000_000,
        cos: 15_082,
        sin: -6_402,
        weight_q12: 2_458,
        tag: 0x6d69_647a_6f6f_6d32,
    },
    Octave {
        lambda_um: 30_000_000,
        cos: 7_943,
        sin: 14_330,
        weight_q12: 1_434,
        tag: 0x6d69_647a_6f6f_6d33,
    },
];

/// Octaves need at least this many lattice steps per wavelength.
const MIN_STEPS_PER_WAVE: i64 = 3;
/// Kernel radius in cells, Q16 (1.0): with features anywhere in their
/// cells, the 3×3 cells around a point hold every feature within reach.
const RADIUS2_Q16: i64 = 65_536;
/// Weight floor of the normalisation, Q16: where no feature is near, the
/// stripes fade to zero instead of amplifying one distant kernel.
const WEIGHT_FLOOR_Q16: i128 = 6_554;
/// 2π, Q12.
const TAU_Q12: i64 = 25_736;
/// Cross-slope gully walls may reach this fraction (Q12) of the fall-line
/// slope, so detail never reverses the drainage direction.
const WALL_SLOPE_Q12: i64 = 3_277;

/// How detail is drawn at one point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    /// Wavelengths below this fade out (they would alias on screen), µm.
    pub min_wave_um: i64,
    /// 0 = incised gullies between broad spurs (soil), ONE = sharp rock
    /// ribs between broad chutes, Q12.
    pub rib_q12: i64,
}

/// Detail at one point: height (mm) and gradient (Q12 slopes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Detail {
    /// Height offset, mm.
    pub height_mm: i64,
    /// Gradient offset, Q12.
    pub gradient_q12: (i64, i64),
}

/// Drainage-aligned detail at lattice position `(x_um, y_um)` on a lattice
/// of `spacing_um`, over a base with gradient `grad_q12`, with at most
/// `amp_mm` in the coarsest octave.
#[must_use]
pub fn detail(
    seed: u64,
    x_um: i64,
    y_um: i64,
    spacing_um: i64,
    grad_q12: (i64, i64),
    amp_mm: i64,
    style: Style,
) -> Detail {
    let mut out = Detail::default();
    if amp_mm <= 0 {
        return out;
    }
    let (mut gx, mut gy) = grad_q12;
    for o in OCTAVES {
        let fade = crate::fixed::smooth(
            style.min_wave_um * 3 / 4,
            style.min_wave_um * 5 / 4,
            o.lambda_um,
        );
        if o.lambda_um < MIN_STEPS_PER_WAVE * spacing_um || fade == 0 {
            continue;
        }
        let base_slope =
            isqrt128(i128::from(gx) * i128::from(gx) + i128::from(gy) * i128::from(gy));
        // Wall slope 2π·A/λ ≤ k·S: amplitude cap from the local slope.
        let cap = base_slope * WALL_SLOPE_Q12 / 4_096 * (o.lambda_um / 1_000) / TAU_Q12;
        let amp = (amp_mm * o.weight_q12 / 4_096).min(cap) * fade / 4_096;
        if amp <= 0 || base_slope == 0 {
            continue;
        }
        let (v, dv) = stripes(seed, o, x_um, y_um, (gx, gy), base_slope);
        let d_mm = amp * profile(v, style.rib_q12) / TRIG_ONE;
        // d/ds of A·cos(2π·phase) along the unit cross direction, Q12.
        let slope = i64::try_from(round_div(
            i128::from(amp) * i128::from(dv) * i128::from(TAU_Q12),
            i128::from(o.lambda_um / 1_000) * i128::from(TRIG_ONE),
        ))
        .unwrap_or(0);
        // Cross-slope unit vector in world axes: (−gy, gx) / |g|.
        let dgx = -i64::try_from(i128::from(gy) * i128::from(slope) / i128::from(base_slope))
            .unwrap_or(0);
        let dgy =
            i64::try_from(i128::from(gx) * i128::from(slope) / i128::from(base_slope)).unwrap_or(0);
        out.height_mm += d_mm;
        out.gradient_q12.0 += dgx;
        out.gradient_q12.1 += dgy;
        gx += dgx;
        gy += dgy;
    }
    out
}

/// Stripe cross-section, Q14: `2·√((1+v)/2) − 1` cuts narrow V gullies
/// between broad rounded spurs; its mirror `1 − 2·√((1−v)/2)` raises sharp
/// ribs between broad chutes. `rib` blends them (Q12).
fn profile(v: i64, rib_q12: i64) -> i64 {
    let one = TRIG_ONE;
    let v = v.clamp(-one, one);
    // √(u) for u in [0, 1] Q14 → Q14.
    let root = |u: i64| crate::fixed::isqrt(u.clamp(0, one) * one);
    let gully = 2 * root((one + v) / 2) - one;
    let ribs = one - 2 * root((one - v) / 2);
    gully + (ribs - gully) * rib_q12 / 4_096
}

/// Normalised stripe sum of one octave: value and its derivative along the
/// cross-slope unit direction, both Q14 (derivative per cycle / 2π).
fn stripes(seed: u64, o: Octave, x_um: i64, y_um: i64, grad: (i64, i64), slope: i64) -> (i64, i64) {
    let (x, y) = (i128::from(x_um), i128::from(y_um));
    let (c, s) = (i128::from(o.cos), i128::from(o.sin));
    let one = i128::from(TRIG_ONE);
    // Rotated domain in Q16 cells of one wavelength.
    let lam = i128::from(o.lambda_um);
    let qx = (x * c - y * s) * 65_536 / (one * lam);
    let qy = (x * s + y * c) * 65_536 / (one * lam);
    // Cross-slope direction, rotated into the same domain, Q14 unit.
    let (gx, gy) = (i128::from(grad.0), i128::from(grad.1));
    let (rx, ry) = ((gx * c - gy * s) / one, (gx * s + gy * c) / one);
    let slope = i128::from(slope.max(1));
    let (dx, dy) = (-ry * one / slope, rx * one / slope);
    let (ix, iy) = (qx.div_euclid(65_536), qy.div_euclid(65_536));
    let (mut sum_v, mut sum_d, mut sum_w) = (0_i128, 0_i128, 0_i128);
    for cy in iy - 1..=iy + 1 {
        for cx in ix - 1..=ix + 1 {
            let h = hash2(
                seed,
                o.tag,
                i64::try_from(cx).unwrap_or(0),
                i64::try_from(cy).unwrap_or(0),
            );
            // Feature point anywhere in its cell: a regular lattice would
            // favour its own axes (goal-prompt §7).
            let fx = cx * 65_536 + i128::from(h & 0xFFFF);
            let fy = cy * 65_536 + i128::from((h >> 16) & 0xFFFF);
            let (px, py) = (qx - fx, qy - fy);
            let d2 = (px * px + py * py) >> 16;
            let r2 = i128::from(RADIUS2_Q16);
            if d2 >= r2 {
                continue;
            }
            let t = (r2 - d2) * 65_536 / r2;
            let w = (t * t) >> 16;
            let phase = (px * dx + py * dy) / one;
            let (cv, sv) = cos_sin_q14(i64::try_from(phase).unwrap_or(0));
            // Each gully has its own depth (0.4–1.6×).
            let a = 26_214 + i128::from((h >> 32) & 0xFFFF) * 78_643 / 65_536;
            sum_v += w * a * i128::from(cv) / 65_536;
            sum_d -= w * a * i128::from(sv) / 65_536;
            sum_w += w;
        }
    }
    let norm = sum_w + WEIGHT_FLOOR_Q16;
    (
        i64::try_from(sum_v / norm).unwrap_or(0),
        i64::try_from(sum_d / norm).unwrap_or(0),
    )
}

/// Soil-mantle hummocks on moderate, gentle slopes (logic/17
/// §mantle): isotropic, rotated value noise at 56 m and 32 m, at most
/// `amp_mm`, and never steeper than half the fall-line slope so it cannot
/// make hollows. Gentle ground reads as turf, not as a moulded sheet.
#[must_use]
pub fn mantle(
    seed: u64,
    x_um: i64,
    y_um: i64,
    spacing_um: i64,
    slope_q12: i64,
    amp_mm: i64,
    min_wave_um: i64,
) -> i64 {
    if amp_mm <= 0 {
        return 0;
    }
    let mut out = 0;
    for (lambda_um, weight_q12, tag) in [
        (56_000_000_i64, 4_096_i64, 0x006d_616e_746c_6531_u64),
        (32_000_000, 2_048, 0x006d_616e_746c_6532),
    ] {
        let fade = crate::fixed::smooth(min_wave_um * 3 / 4, min_wave_um * 5 / 4, lambda_um);
        if lambda_um < MIN_STEPS_PER_WAVE * spacing_um || fade == 0 {
            continue;
        }
        let cap = slope_q12 * 2_048 / 4_096 * (lambda_um / 1_000) / TAU_Q12;
        let amp = (amp_mm * weight_q12 / 4_096).min(cap) * fade / 4_096;
        // Mean of two rotated copies spans about ±0.5; scale to ±1.
        let v = (crate::fixed::organic_q12(seed, tag, x_um, y_um, lambda_um) - 2_048) * 2;
        out += amp * v / 4_096;
    }
    out
}
