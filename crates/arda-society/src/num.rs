//! Saturating integer conversions, so the crate never needs a lossy `as`.

/// `i64` → `u32`, clamped to `0..=u32::MAX`.
#[must_use]
pub fn u32_of(v: i64) -> u32 {
    u32::try_from(v.max(0)).unwrap_or(u32::MAX)
}

/// `i64` → `u8`, clamped to `0..=255`.
#[must_use]
pub fn u8_of(v: i64) -> u8 {
    u8::try_from(v.clamp(0, 255)).unwrap_or(u8::MAX)
}

/// `u64` → `i64`, clamped.
#[must_use]
pub fn i64_of(v: u64) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

/// `i64` → `u64`, negative becomes 0.
#[must_use]
pub fn u64_of(v: i64) -> u64 {
    u64::try_from(v.max(0)).unwrap_or(0)
}

/// `i64` → `i32`, clamped.
#[must_use]
pub fn i32_of(v: i64) -> i32 {
    i32::try_from(v.clamp(i64::from(i32::MIN), i64::from(i32::MAX))).unwrap_or(0)
}

/// `usize` → `u64` (lossless on every supported target).
#[must_use]
pub fn u64_of_usize(v: usize) -> u64 {
    u64::try_from(v).unwrap_or(u64::MAX)
}

/// `u64` → `usize`, clamped.
#[must_use]
pub fn usize_of(v: u64) -> usize {
    usize::try_from(v).unwrap_or(usize::MAX)
}

/// `u128` → `u64`, clamped.
#[must_use]
pub fn u64_of_u128(v: u128) -> u64 {
    u64::try_from(v).unwrap_or(u64::MAX)
}

/// Integer square root (floor) of a `u64`.
#[must_use]
pub fn isqrt(v: u64) -> u64 {
    if v < 2 {
        return v;
    }
    let mut x = v;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + v / x) / 2;
    }
    x
}

/// Euclidean distance between two points in metres.
#[must_use]
pub fn dist_m(ax: i64, ay: i64, bx: i64, by: i64) -> u64 {
    let dx = ax.abs_diff(bx);
    let dy = ay.abs_diff(by);
    isqrt(dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isqrt_is_floor() {
        for v in [0_u64, 1, 2, 3, 4, 15, 16, 17, 1_000_000, 999_999] {
            let r = isqrt(v);
            assert!(r * r <= v && (r + 1) * (r + 1) > v, "{v}");
        }
        assert_eq!(dist_m(0, 0, 3000, 4000), 5000);
    }
}
