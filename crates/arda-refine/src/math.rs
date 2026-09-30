//! Portable `exp` and `ln` built from `+ - * /` and bit manipulation only,
//! so weights are bit-identical on every platform (§8: determinism does not
//! trust the platform's libm).

const LN2: f64 = std::f64::consts::LN_2;

/// `e^x`, relative error below 1e-13 for `|x| < 700`.
#[must_use]
pub fn exp(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    let x = x.clamp(-700.0, 700.0);
    let kf = (x / LN2 + 0.5).floor();
    let r = x - kf * LN2;
    // Taylor series on |r| <= ln2 / 2.
    let mut term = 1.0;
    let mut sum = 1.0;
    for n in 1..=14 {
        term *= r / f64::from(n);
        sum += term;
    }
    #[allow(clippy::cast_possible_truncation)]
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
        // Subnormal: scale up first.
        return ln(x * 4_503_599_627_370_496.0) - 52.0 * LN2;
    }
    let e = exp_bits - 1023;
    let m = f64::from_bits((bits & 0x000F_FFFF_FFFF_FFFF) | 0x3FF0_0000_0000_0000);
    // m in [1, 2); centre it on sqrt(2) for fast convergence.
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
    #[allow(clippy::cast_precision_loss)]
    let ef = e as f64;
    2.0 * sum + ef * LN2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exp_and_ln_match_std_closely() {
        for i in -200..200 {
            let x = f64::from(i) * 0.173;
            assert!((exp(x) / x.exp() - 1.0).abs() < 1e-12, "exp {x}");
        }
        for i in 1..400 {
            let x = f64::from(i) * 0.0371;
            assert!((ln(x) - x.ln()).abs() < 1e-12, "ln {x}");
        }
        assert!((ln(1e-300) - 1e-300f64.ln()).abs() < 1e-9);
    }
}
