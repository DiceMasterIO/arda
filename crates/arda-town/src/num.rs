//! Checked numeric conversions in one place, so the workspace's cast lints
//! stay on everywhere else.

// These helpers *are* the reviewed casts: saturating float→int and
// two's-complement reinterpretation for hashing.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

/// `floor(v)` as `i64` (saturating; NaN maps to 0).
#[must_use]
pub fn floor_i(v: f64) -> i64 {
    v.floor() as i64
}

/// `round(v)` as `i64` (saturating; NaN maps to 0).
#[must_use]
pub fn round_i(v: f64) -> i64 {
    v.round() as i64
}

/// `round(v)` clamped into `i32`.
#[must_use]
pub fn round_i32(v: f64) -> i32 {
    v.round().clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

/// `round(v)` clamped into `u8`.
#[must_use]
pub fn round_u8(v: f64) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// `round(v)` clamped into `i16`.
#[must_use]
pub fn round_i16(v: f64) -> i16 {
    v.round().clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
}

/// Reinterprets a signed value as hash input.
#[must_use]
pub const fn bits(v: i64) -> u64 {
    v as u64
}

/// `v % n` as an index (n > 0).
#[must_use]
pub const fn index_of(v: u64, n: usize) -> usize {
    (v % n as u64) as usize
}

/// A float as `f32` for tactical coordinates.
#[must_use]
pub fn f32_of(v: f64) -> f32 {
    v as f32
}

/// An `i64` clamped into `i32`.
#[must_use]
pub fn clamp_i32(v: i64) -> i32 {
    v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// An `i64` clamped into `u32`.
#[must_use]
pub fn clamp_u32(v: i64) -> u32 {
    v.clamp(0, i64::from(u32::MAX)) as u32
}

/// A float clamped into `u32` after rounding.
#[must_use]
pub fn round_u32(v: f64) -> u32 {
    v.round().clamp(0.0, f64::from(u32::MAX)) as u32
}
