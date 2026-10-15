//! The climate-to-biome rules every layer shares (review 2026-10-14 §4 item
//! 6): world ecology (`arda-gen` `fine_materials`, area fields), tactical
//! refinement (`arda-refine` biome, prior and ecology), the Atlas and relief
//! shader (`arda-render` `atlas::formed`) and the server's snow proxy read
//! their evaporation demand, tree line, snow line, rock exposure, marsh and
//! cover thresholds and slope aspect from here, so forest on the world map is
//! forest on the battle map and snow starts at the same temperature at every
//! zoom (logic/02 §ground material, logic/09 §biome signals).
//!
//! The rules live in `arda-core` because every consumer already depends on
//! it and the aspect encoding (flat sentinel, north as 360) is part of the
//! stored cell format. The core is integer (centi-degrees Celsius,
//! milli-degrees of slope, Q12 fractions) for world generation and the
//! Atlas, which are golden-pinned across platforms; `f64` mirrors serve the
//! refiner. No function here calls a platform maths routine.

pub mod aspect;
pub mod climate;
pub mod ground;
pub mod latitude;

pub use climate::Lines;
pub use latitude::Frame;

/// One in Q12 fixed point.
pub const ONE: i64 = 4_096;

/// Cubic smoothstep in Q12: `0` at or before `e0`, [`ONE`] at or beyond
/// `e1` (either order), the Atlas's `smooth`.
#[must_use]
pub fn smooth_q12(e0: i64, e1: i64, x: i64) -> i64 {
    if e0 == e1 {
        return if x >= e1 { ONE } else { 0 };
    }
    let t = ((x - e0) * ONE / (e1 - e0)).clamp(0, ONE);
    t * t / ONE * (3 * ONE - 2 * t) / ONE
}

/// Cubic smoothstep: `0` at or before `e0`, `1` at or beyond `e1` (either
/// order).
#[must_use]
pub fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    if (e1 - e0).abs() < f64::EPSILON {
        return if x >= e1 { 1.0 } else { 0.0 };
    }
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// `tan` of an angle in degrees, `0..=80`, by the ratio of the sine and
/// cosine Taylor series (relative error under 1e-9 there), identical on
/// every platform.
#[must_use]
pub fn tan_deg(deg: f64) -> f64 {
    let x = deg.clamp(0.0, 80.0) * std::f64::consts::PI / 180.0;
    let x2 = x * x;
    let (mut sin, mut cos) = (x, 1.0);
    let (mut ts, mut tc) = (x, 1.0);
    for k in 1..12 {
        let k = f64::from(k);
        ts *= -x2 / ((2.0 * k) * (2.0 * k + 1.0));
        tc *= -x2 / ((2.0 * k - 1.0) * (2.0 * k));
        sin += ts;
        cos += tc;
    }
    sin / cos.max(1e-12)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothsteps_agree_and_run_either_way() {
        assert_eq!(smooth_q12(0, 100, -5), 0);
        assert_eq!(smooth_q12(0, 100, 100), ONE);
        assert_eq!(smooth_q12(100, 0, 50), ONE / 2);
        assert!((smoothstep(0.0, 1.0, 0.5) - 0.5).abs() < 1e-12);
        assert!((smoothstep(1.0, 0.0, 0.25) - 0.843_75).abs() < 1e-12);
        for x in [-10, 13, 47, 71, 120] {
            let q = smooth_q12(0, 100, x) as f64 / ONE as f64;
            assert!((q - smoothstep(0.0, 100.0, f64::from(x as i32))).abs() < 2e-3);
        }
    }

    #[test]
    fn tangent_series_matches_known_values() {
        assert!(tan_deg(0.0).abs() < 1e-15);
        assert!((tan_deg(45.0) - 1.0).abs() < 1e-9);
        assert!((tan_deg(60.0) - 3.0_f64.sqrt()).abs() < 1e-9);
        assert!((tan_deg(30.0) - 1.0 / 3.0_f64.sqrt()).abs() < 1e-9);
    }
}
