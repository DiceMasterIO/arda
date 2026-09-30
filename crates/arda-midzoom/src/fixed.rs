//! Deterministic integer helpers: Q14 trigonometry, Q12 smoothstep and a
//! SplitMix hash. No floating point reaches a stored or rendered value
//! (goal-prompt §8 determinism).

/// Q12 one.
pub const ONE: i64 = 4_096;
/// Q14 one (trigonometry).
pub const TRIG_ONE: i64 = 16_384;
/// Turn resolution of [`cos_sin_q14`]: phases are Q16 turns.
pub const TURN: i64 = 65_536;

/// Quarter-wave cosine table, Q14, 257 entries over a quarter turn, built at
/// compile time from an integer Taylor series (Q40), so every platform
/// agrees bit for bit.
const QUARTER: [i64; 257] = quarter_table();

const fn quarter_table() -> [i64; 257] {
    // pi/2 in Q40.
    const HALF_PI_Q40: i128 = 1_727_133_214_476;
    const Q40: i128 = 1 << 40;
    let mut out = [0_i64; 257];
    let mut i = 0;
    while i < 257 {
        let x = HALF_PI_Q40 * (i as i128) / 256;
        let x2 = x * x / Q40;
        // cos x = 1 - x²/2! + x⁴/4! - ... (terms to x^14: error < 1e-9).
        let mut term = Q40;
        let mut sum = Q40;
        let mut k = 1;
        while k <= 7 {
            term = -term * x2 / Q40 / ((2 * k - 1) * (2 * k)) as i128;
            sum += term;
            k += 1;
        }
        #[allow(clippy::cast_possible_truncation)]
        {
            out[i] = ((sum * 16_384 + Q40 / 2) / Q40) as i64;
        }
        i += 1;
    }
    out
}

fn quarter(i: i64) -> i64 {
    usize::try_from(i.clamp(0, 256)).map_or(0, |i| QUARTER[i])
}

/// Cosine of a Q16 turn phase, Q14, with linear table interpolation.
fn cos_q14(phase: i64) -> i64 {
    let p = phase.rem_euclid(TURN);
    // Quarter index in 1/256 steps with a 6-bit fraction (65536 / 1024).
    let (quadrant, within) = (p / (TURN / 4), p % (TURN / 4));
    let pos = within * 256;
    let (i, f) = (pos / (TURN / 4), pos % (TURN / 4));
    let lerp = |a: i64, b: i64| a + (b - a) * f / (TURN / 4);
    match quadrant {
        0 => lerp(quarter(i), quarter(i + 1)),
        1 => -lerp(quarter(256 - i), quarter(255 - i)),
        2 => -lerp(quarter(i), quarter(i + 1)),
        _ => lerp(quarter(256 - i), quarter(255 - i)),
    }
}

/// Cosine and sine of a Q16 turn phase, both Q14.
#[must_use]
pub fn cos_sin_q14(phase: i64) -> (i64, i64) {
    (cos_q14(phase), cos_q14(phase - TURN / 4))
}

/// Cubic smoothstep from `e0` to `e1`, Q12 in `[0, ONE]`.
#[must_use]
pub fn smooth(e0: i64, e1: i64, x: i64) -> i64 {
    if e0 == e1 {
        return if x < e0 { 0 } else { ONE };
    }
    let t = ((x - e0) * ONE / (e1 - e0)).clamp(0, ONE);
    t * t / ONE * (3 * ONE - 2 * t) / ONE
}

/// Integer square root of a non-negative value (0 for negatives).
#[must_use]
pub fn isqrt(v: i64) -> i64 {
    i64::try_from(v.max(0).unsigned_abs().isqrt()).unwrap_or(0)
}

/// Integer square root on 128 bits.
#[must_use]
pub fn isqrt128(v: i128) -> i64 {
    i64::try_from(v.max(0).unsigned_abs().isqrt()).unwrap_or(i64::MAX)
}

/// SplitMix64 finaliser.
#[must_use]
pub const fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Hash of a seed, a stream tag and two signed coordinates.
#[must_use]
pub const fn hash2(seed: u64, tag: u64, x: i64, y: i64) -> u64 {
    let a = mix(seed ^ mix(tag));
    let b = mix(a ^ x.cast_unsigned());
    mix(b ^ y.cast_unsigned().rotate_left(29))
}

/// Rounded integer division (half away from zero) on 128 bits.
#[must_use]
pub fn round_div(n: i128, d: i128) -> i128 {
    if d == 0 {
        return 0;
    }
    let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
    if n < 0 {
        -((-n + d / 2) / d)
    } else {
        (n + d / 2) / d
    }
}

/// Smoothstep-bilinear value noise in `[0, ONE]` on a `period_um` lattice.
fn value_noise(seed: u64, tag: u64, x_um: i64, y_um: i64, period_um: i64) -> i64 {
    let (ix, iy) = (x_um.div_euclid(period_um), y_um.div_euclid(period_um));
    let tx = smooth(0, ONE, x_um.rem_euclid(period_um) * ONE / period_um);
    let ty = smooth(0, ONE, y_um.rem_euclid(period_um) * ONE / period_um);
    let h = |x: i64, y: i64| i64::try_from(hash2(seed, tag, x, y) >> 52).unwrap_or(0);
    let top = h(ix, iy) + (h(ix + 1, iy) - h(ix, iy)) * tx / ONE;
    let bottom = h(ix, iy + 1) + (h(ix + 1, iy + 1) - h(ix, iy + 1)) * tx / ONE;
    top + (bottom - top) * ty / ONE
}

/// Value noise with its lattice axes broken up: the mean of two copies in
/// domains rotated by 37° and −23°, so patches never tile into squares
/// (goal-prompt §7: no raw axis-aligned noise). `[0, ONE]`.
#[must_use]
pub fn organic_q12(seed: u64, tag: u64, x_um: i64, y_um: i64, period_um: i64) -> i64 {
    let rot = |c: i64, s: i64| {
        (
            i64::try_from(
                (i128::from(x_um) * i128::from(c) - i128::from(y_um) * i128::from(s)) / 16_384,
            )
            .unwrap_or(0),
            i64::try_from(
                (i128::from(x_um) * i128::from(s) + i128::from(y_um) * i128::from(c)) / 16_384,
            )
            .unwrap_or(0),
        )
    };
    let (ax, ay) = rot(13_085, 9_860);
    let (bx, by) = rot(15_082, -6_402);
    (value_noise(seed, tag, ax, ay, period_um) + value_noise(seed, tag ^ 0x5bd1, bx, by, period_um))
        / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trig_matches_float_reference_closely() {
        for phase in (0..TURN).step_by(97) {
            let (c, s) = cos_sin_q14(phase);
            #[allow(clippy::cast_precision_loss)]
            let a = phase as f64 / TURN as f64 * std::f64::consts::TAU;
            #[allow(clippy::cast_possible_truncation)]
            let (fc, fs) = (
                (a.cos() * 16_384.0).round() as i64,
                (a.sin() * 16_384.0).round() as i64,
            );
            assert!((c - fc).abs() <= 2, "cos {phase}: {c} vs {fc}");
            assert!((s - fs).abs() <= 2, "sin {phase}: {s} vs {fs}");
        }
        assert_eq!(cos_sin_q14(0), (16_384, 0));
        assert_eq!(cos_sin_q14(-TURN / 4).0, cos_sin_q14(TURN / 4).0);
    }

    #[test]
    fn smoothstep_is_clamped_and_monotone() {
        assert_eq!(smooth(0, 100, -5), 0);
        assert_eq!(smooth(0, 100, 500), ONE);
        let mut last = 0;
        for x in 0..=100 {
            let v = smooth(0, 100, x);
            assert!(v >= last);
            last = v;
        }
        assert!(smooth(100, 0, 0) == ONE && smooth(100, 0, 100) == 0);
    }
}
