//! Integer helpers: saturating conversions and a deterministic integer sqrt.
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
}
