//! Integer helpers: saturating conversions, a deterministic integer sqrt and
//! a platform-independent natural logarithm.
//!
//! Generation stays in integers (goal-prompt §8), so these replace `as` casts.

/// Saturating `i64` → `u8`.
#[must_use]
pub fn sat_u8(v: i64) -> u8 {
    u8::try_from(v.clamp(0, 255)).unwrap_or(u8::MAX)
}

/// Saturating `i64` → `u16`.
#[must_use]
pub fn sat_u16(v: i64) -> u16 {
    u16::try_from(v.clamp(0, i64::from(u16::MAX))).unwrap_or(u16::MAX)
}

/// Saturating `i64` → `u32`.
#[must_use]
pub fn sat_u32(v: i64) -> u32 {
    u32::try_from(v.clamp(0, i64::from(u32::MAX))).unwrap_or(u32::MAX)
}

/// Saturating `i64` → `i32`.
#[must_use]
pub fn sat_i32(v: i64) -> i32 {
    i32::try_from(v.clamp(i64::from(i32::MIN), i64::from(i32::MAX))).unwrap_or(i32::MAX)
}

/// Saturating `usize` → `i64`.
#[must_use]
pub fn ui(v: usize) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

/// Saturating non-negative `i64` → `usize` (negatives become 0).
#[must_use]
pub fn iu(v: i64) -> usize {
    usize::try_from(v.max(0)).unwrap_or(usize::MAX)
}

/// Saturating `usize` → `u32`.
#[must_use]
pub fn u32_of(v: usize) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}

/// Saturating `u64` → `u32`.
#[must_use]
pub fn u32_of_u64(v: u64) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}

/// Floor integer square root.
#[must_use]
pub fn isqrt(v: u64) -> u64 {
    if v < 2 {
        return v;
    }
    // Newton iteration from an upper bound; exact for every u64.
    let mut x = 1_u64 << (v.ilog2() / 2 + 1);
    loop {
        let y = (x + v / x) / 2;
        if y >= x {
            return x;
        }
        x = y;
    }
}

/// Euclidean distance in metres between two cells `dx`, `dy` apart (100 m cells).
#[must_use]
pub fn dist_m(dx: i64, dy: i64) -> u32 {
    let d2 = u64::try_from(dx * dx + dy * dy).unwrap_or(u64::MAX);
    u32_of_u64(isqrt(d2.saturating_mul(10_000)))
}

/// Rounds `v * num / den` to the nearest integer (den > 0).
#[must_use]
pub fn mul_div(v: i64, num: i64, den: i64) -> i64 {
    if den == 0 {
        return 0;
    }
    let p = i128::from(v) * i128::from(num);
    let d = i128::from(den);
    let q = if p >= 0 {
        (p + d / 2) / d
    } else {
        (p - d / 2) / d
    };
    i64::try_from(q).unwrap_or(if q > 0 { i64::MAX } else { i64::MIN })
}

/// Natural logarithm of a finite positive `x`, from IEEE basic operations
/// only (`+`, `−`, `×`, `÷` and bit manipulation), so it rounds the same on
/// every platform. `f64::ln` is libm's and is not correctly rounded, so
/// `stats.json`, which is in the byte-identical set, could differ between
/// libm implementations (review round 2 #43). Accurate to a few ulp; 0 for
/// non-positive or non-finite input.
#[must_use]
#[allow(clippy::cast_possible_wrap, clippy::cast_precision_loss)] // 11-bit exponent
pub fn ln(x: f64) -> f64 {
    if !(x.is_finite() && x > 0.0) {
        return 0.0;
    }
    // Normalise subnormals into the normal range first.
    let (x, bias) = if x < f64::MIN_POSITIVE {
        (x * 2f64.powi(64), -64)
    } else {
        (x, 0)
    };
    let bits = x.to_bits();
    let mut e = ((bits >> 52) & 0x7ff) as i64 - 1023 + bias;
    let mut m = f64::from_bits((bits & ((1 << 52) - 1)) | (1023 << 52));
    if m > std::f64::consts::SQRT_2 {
        m /= 2.0;
        e += 1;
    }
    // ln m = 2 atanh t with t = (m - 1) / (m + 1), |t| <= 0.172.
    let t = (m - 1.0) / (m + 1.0);
    let t2 = t * t;
    let mut series = 0.0;
    for k in (0..24_u32).rev() {
        series = series * t2 + 1.0 / f64::from(2 * k + 1);
    }
    e as f64 * std::f64::consts::LN_2 + 2.0 * t * series
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isqrt_is_exact() {
        for v in [0_u64, 1, 2, 3, 4, 15, 16, 17, 99, 100, 1 << 40, u64::MAX] {
            let r = isqrt(v);
            assert!(u128::from(r) * u128::from(r) <= u128::from(v));
            assert!(u128::from(r + 1) * u128::from(r + 1) > u128::from(v));
        }
    }

    #[test]
    fn distances_and_saturation() {
        assert_eq!(dist_m(3, 4), 500);
        assert_eq!(sat_u8(-4), 0);
        assert_eq!(sat_u8(900), 255);
        assert_eq!(mul_div(10, 1, 3), 3);
        assert_eq!(mul_div(-10, 1, 4), -3);
    }

    #[test]
    fn ln_matches_the_platform_logarithm_to_a_few_ulp() {
        for x in [
            1.0, 2.0, 2.5, 10.0, 7_999.0, 1e-300, 4.9e-324, 1e300, 0.5, 1.5,
        ] {
            let (got, want) = (ln(x), x.ln());
            assert!(
                (got - want).abs() <= 4.0 * f64::EPSILON * want.abs().max(1.0),
                "{x}: {got} vs {want}"
            );
        }
        for k in 1..2_000_u32 {
            let x = f64::from(k) * 1.37;
            assert!((ln(x) - x.ln()).abs() <= 4.0 * f64::EPSILON * x.ln().abs().max(1.0));
        }
        assert!(ln(1.0).abs() < f64::EPSILON);
        assert!(ln(0.0).abs() < f64::EPSILON && ln(-3.0).abs() < f64::EPSILON);
    }
}
