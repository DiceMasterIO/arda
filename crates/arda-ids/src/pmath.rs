//! Portable math built from `+ - * /`, `sqrt` and bit manipulation only,
//! so every platform computes the same bits (goal-prompt §8: determinism
//! does not trust the platform's libm; review 3.5-14: the fields and ways
//! layers called libm at 35 sites). `sqrt` is correctly rounded by IEEE
//! 754, so it may be used directly.

use std::f64::consts::{FRAC_PI_2, LN_2, PI};

/// `e^x`, relative error below 1e-13 for `|x| < 700`.
#[must_use]
pub fn exp(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    let x = x.clamp(-700.0, 700.0);
    let kf = (x / LN_2 + 0.5).floor();
    let r = x - kf * LN_2;
    let mut term = 1.0;
    let mut sum = 1.0;
    for n in 1..=14 {
        term *= r / f64::from(n);
        sum += term;
    }
    #[allow(clippy::cast_possible_truncation)] // |kf| <= 1010
    let k = kf as i64;
    let bits = u64::try_from(k + 1023).unwrap_or(0) << 52;
    sum * f64::from_bits(bits)
}

/// Natural logarithm for `x > 0`; `-745` for non-positive input.
#[must_use]
pub fn ln(x: f64) -> f64 {
    if x <= 0.0 || x.is_nan() {
        return -745.0;
    }
    let bits = x.to_bits();
    let exp_bits = i64::try_from((bits >> 52) & 0x7FF).unwrap_or(1023);
    if exp_bits == 0 {
        return ln(x * 4_503_599_627_370_496.0) - 52.0 * LN_2;
    }
    let e = exp_bits - 1023;
    let m = f64::from_bits((bits & 0x000F_FFFF_FFFF_FFFF) | 0x3FF0_0000_0000_0000);
    let (m, e) = if m > std::f64::consts::SQRT_2 {
        (m * 0.5, e + 1)
    } else {
        (m, e)
    };
    let s = (m - 1.0) / (m + 1.0);
    let s2 = s * s;
    let mut term = s;
    let mut sum = 0.0;
    for k in 0..16 {
        sum += term / f64::from(2 * k + 1);
        term *= s2;
    }
    #[allow(clippy::cast_precision_loss)] // |e| <= 1075
    let ef = e as f64;
    2.0 * sum + ef * LN_2
}

/// `sqrt(x² + y²)`.
#[must_use]
pub fn hypot(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

/// Sine and cosine of `r` with `|r| <= π/4`, by Taylor series.
fn sin_cos_small(r: f64) -> (f64, f64) {
    let r2 = r * r;
    let (mut s, mut c) = (r, 1.0);
    let (mut ts, mut tc) = (r, 1.0);
    for n in 1..=11 {
        let k = f64::from(2 * n);
        ts *= -r2 / (k * (k + 1.0));
        tc *= -r2 / ((k - 1.0) * k);
        s += ts;
        c += tc;
    }
    (s, c)
}

/// `(sin x, cos x)`, absolute error below 1e-14 for `|x| < 1e6`.
#[must_use]
pub fn sin_cos(x: f64) -> (f64, f64) {
    if !x.is_finite() {
        return (f64::NAN, f64::NAN);
    }
    let q = (x / FRAC_PI_2).round();
    // Two-part π/2 (the f64 nearest it, and the rest) so the reduction
    // keeps its precision.
    const A: f64 = FRAC_PI_2;
    const B: f64 = 6.123_233_995_736_766e-17;
    let r = (x - q * A) - q * B;
    let (s, c) = sin_cos_small(r);
    #[allow(clippy::cast_possible_truncation)] // |q| < 2^53
    match (q as i64).rem_euclid(4) {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    }
}

/// `sin x`.
#[must_use]
pub fn sin(x: f64) -> f64 {
    sin_cos(x).0
}

/// `cos x`.
#[must_use]
pub fn cos(x: f64) -> f64 {
    sin_cos(x).1
}

/// `atan t` for `|t| <= 1`.
fn atan_unit(t: f64) -> f64 {
    // Reduce twice by the half-angle identity, then a series.
    let t1 = t / (1.0 + (1.0 + t * t).sqrt());
    let t2 = t1 / (1.0 + (1.0 + t1 * t1).sqrt());
    let z = t2 * t2;
    let mut term = t2;
    let mut sum = 0.0;
    for k in 0..14 {
        sum += term / f64::from(2 * k + 1);
        term *= -z;
    }
    4.0 * sum
}

/// `atan2(y, x)` in `(-π, π]`.
#[must_use]
pub fn atan2(y: f64, x: f64) -> f64 {
    if y.is_nan() || x.is_nan() {
        return f64::NAN;
    }
    if x == 0.0 && y == 0.0 {
        return if x.is_sign_negative() {
            if y.is_sign_negative() {
                -PI
            } else {
                PI
            }
        } else {
            y
        };
    }
    let (ax, ay) = (x.abs(), y.abs());
    let a = if ay <= ax {
        atan_unit(ay / ax)
    } else {
        FRAC_PI_2 - atan_unit(ax / ay)
    };
    let a = if x < 0.0 { PI - a } else { a };
    if y < 0.0 {
        -a
    } else {
        a
    }
}

/// `asin x` for `|x| <= 1` (clamped).
#[must_use]
pub fn asin(x: f64) -> f64 {
    let x = x.clamp(-1.0, 1.0);
    atan2(x, (1.0 - x * x).sqrt())
}

/// `acos x` for `|x| <= 1` (clamped).
#[must_use]
pub fn acos(x: f64) -> f64 {
    let x = x.clamp(-1.0, 1.0);
    atan2((1.0 - x * x).sqrt(), x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn functions_match_std_closely() {
        for i in -2000..2000 {
            let x = f64::from(i) * 0.013_7;
            let (s, c) = sin_cos(x);
            assert!((s - x.sin()).abs() < 1e-14, "sin {x}");
            assert!((c - x.cos()).abs() < 1e-14, "cos {x}");
            assert!(
                (exp(x / 10.0) / (x / 10.0).exp() - 1.0).abs() < 1e-12,
                "exp {x}"
            );
        }
        for i in -100..=100 {
            for j in -100..=100 {
                let (y, x) = (f64::from(i) * 0.37, f64::from(j) * 0.29);
                assert!((atan2(y, x) - y.atan2(x)).abs() < 1e-14, "atan2 {y} {x}");
            }
        }
        for i in -100..=100 {
            let x = f64::from(i) / 100.0;
            assert!((asin(x) - x.asin()).abs() < 1e-13, "asin {x}");
            assert!((acos(x) - x.acos()).abs() < 1e-13, "acos {x}");
        }
        for i in 1..400 {
            let x = f64::from(i) * 0.0371;
            assert!((ln(x) - x.ln()).abs() < 1e-12, "ln {x}");
        }
        assert_eq!(hypot(3.0, 4.0), 5.0);
    }
}
