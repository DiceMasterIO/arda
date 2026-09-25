//! Continuous integer composition of continental macro and signed spectral relief.
//!
//! The gate reproduces the saved regional structural-relief rule. Callers own
//! physical sampling of the eight surrounding macro heights; this module
//! chooses neither a lattice nor a coast mask.

use arda_core::{HeightMm, TerrainPoint};

use crate::spectral::SpectralError;

/// Full-scale gain for source-composition recipe version 2.
pub const FULL_GAIN_Q32: u64 = 1_u64 << 32;
/// The inherited range where the previous gate first reached full relief.
pub const FULL_RANGE_MM: u32 = 2_000_000;
const KM_UM: i64 = 1_000_000_000;

/// Monotone C1 relief gain with no 400 m activation threshold.
///
/// The quartic preserves the old 800 m quarter-gain anchor and full gain at
/// 2 km. Q32 representation only quantizes physically negligible sub-metre
/// ranges; the final composed height is rounded once to millimetres.
#[must_use]
pub fn smooth_relief_gain_q32(range_mm: u32) -> u64 {
    let r = u128::from(range_mm.min(FULL_RANGE_MM));
    let s = u128::from(FULL_RANGE_MM);
    let numerator = r * r * (59 * s * s + 74 * r * s - 85 * r * r);
    let denominator = 48 * s * s * s * s;
    u64::try_from((numerator * u128::from(FULL_GAIN_Q32) + denominator / 2) / denominator)
        .unwrap_or(FULL_GAIN_Q32)
}

fn smoothstep_q32(distance_um: i64) -> u64 {
    let d = u128::try_from(distance_um.clamp(0, KM_UM)).unwrap_or(0);
    let s = u128::try_from(KM_UM).unwrap_or(1);
    let numerator = d * d * (3 * s - 2 * d);
    let denominator = s * s * s;
    u64::try_from((numerator * u128::from(FULL_GAIN_Q32) + denominator / 2) / denominator)
        .unwrap_or(FULL_GAIN_Q32)
}

fn multiply_q32(a: u64, b: u64) -> u64 {
    u64::try_from(
        (u128::from(a) * u128::from(b) + u128::from(FULL_GAIN_Q32 / 2)) / u128::from(FULL_GAIN_Q32),
    )
    .unwrap_or(FULL_GAIN_Q32)
}

/// C1 spectral taper based solely on the existing two-node forced-ocean rim.
///
/// The macro source is already negative between its forced nodes: x<=1 km
/// at the west and x>=width-2 km at the east, likewise north/south. Relief is
/// zero there, rises over the adjacent 1 km cell, and is unchanged inland.
/// Beyond the nominal right/bottom extent it remains zero.
#[must_use]
pub fn forced_rim_taper_q32(point: TerrainPoint, width_km: u32, height_km: u32) -> u64 {
    let right = (i64::from(width_km) - 2) * KM_UM;
    let bottom = (i64::from(height_km) - 2) * KM_UM;
    [
        smoothstep_q32(point.x_um.saturating_sub(KM_UM)),
        smoothstep_q32(right.saturating_sub(point.x_um)),
        smoothstep_q32(point.y_um.saturating_sub(KM_UM)),
        smoothstep_q32(bottom.saturating_sub(point.y_um)),
    ]
    .into_iter()
    .fold(FULL_GAIN_Q32, multiply_q32)
}

/// Source-composition recipe version 2: local-range gain and forced-rim taper.
///
/// Both factors are applied to signed relief before one final height rounding.
/// The macro surface remains unchanged, including its forced-ocean values.
pub fn compose_smooth_relief(
    macro_height: HeightMm,
    relief: HeightMm,
    range_mm: u32,
    taper_q32: u64,
) -> Result<HeightMm, SpectralError> {
    if taper_q32 > FULL_GAIN_Q32 {
        return Err(SpectralError::InvalidReliefGate);
    }
    let gain = i128::from(smooth_relief_gain_q32(range_mm));
    let product = i128::from(relief.raw()) * gain * i128::from(taper_q32);
    let denominator = i128::from(FULL_GAIN_Q32) * i128::from(FULL_GAIN_Q32);
    let delta = if product < 0 {
        -((-product + denominator / 2) / denominator)
    } else {
        (product + denominator / 2) / denominator
    };
    let composed = i128::from(macro_height.raw()) + delta;
    i32::try_from(composed)
        .map(HeightMm::new)
        .map_err(|_| SpectralError::ArithmeticOverflow)
}

/// Full-scale Q16 gate value.
pub const FULL_RELIEF_GATE_Q16: u32 = 65_536;

/// Existing 5 km macro-range gate for eight caller-supplied surrounding heights.
///
/// To reproduce `structural_relief::relief_gate_q16` on the 100 m lattice,
/// samples are the four cardinal offsets `(0,±50)`, `(±50,0)` and four
/// diagonals `(±35,±35)` in any order. The center is included in the range.
/// No land/sea eligibility check is applied.
#[must_use]
pub fn relief_gate_q16(macro_height: HeightMm, surrounding_heights: &[HeightMm; 8]) -> u32 {
    let mut lo = i64::from(macro_height.raw());
    let mut hi = lo;
    for &height in surrounding_heights {
        let value = i64::from(height.raw());
        lo = lo.min(value);
        hi = hi.max(value);
    }
    let numerator = (hi - lo - 400_000) * i64::from(FULL_RELIEF_GATE_Q16);
    let gate = (numerator / 1_600_000).clamp(0, i64::from(FULL_RELIEF_GATE_Q16));
    u32::try_from(gate).unwrap_or(FULL_RELIEF_GATE_Q16)
}

/// Applies a precomputed Q16 gate to signed relief, rounding once to the
/// nearest millimetre with half ties away from zero.
///
/// A gate outside `0..=65536` is invalid. Overflow of the final `HeightMm`
/// returns an error; no visually attractive clamping is performed.
pub fn apply_relief_gate(
    macro_height: HeightMm,
    relief: HeightMm,
    gate_q16: u32,
) -> Result<HeightMm, SpectralError> {
    if gate_q16 > FULL_RELIEF_GATE_Q16 {
        return Err(SpectralError::InvalidReliefGate);
    }
    let numerator = i64::from(relief.raw()) * i64::from(gate_q16);
    let half = i64::from(FULL_RELIEF_GATE_Q16 / 2);
    let denominator = i64::from(FULL_RELIEF_GATE_Q16);
    let delta = if numerator < 0 {
        -((-numerator + half) / denominator)
    } else {
        (numerator + half) / denominator
    };
    let composed = i64::from(macro_height.raw()) + delta;
    i32::try_from(composed)
        .map(HeightMm::new)
        .map_err(|_| SpectralError::ArithmeticOverflow)
}

/// Computes the existing macro-range gate and applies it without a land mask.
pub fn compose_relief(
    macro_height: HeightMm,
    relief: HeightMm,
    surrounding_heights: &[HeightMm; 8],
) -> Result<HeightMm, SpectralError> {
    apply_relief_gate(
        macro_height,
        relief,
        relief_gate_q16(macro_height, surrounding_heights),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn neighbors(other: i32) -> [HeightMm; 8] {
        [HeightMm::new(other); 8]
    }

    #[test]
    fn smooth_gain_has_no_lowland_threshold_and_preserves_mountain_full_gain() {
        assert_eq!(smooth_relief_gain_q32(0), 0);
        assert!(smooth_relief_gain_q32(100_000) > 0);
        assert!(smooth_relief_gain_q32(400_000) > 0);
        assert_eq!(smooth_relief_gain_q32(800_000), FULL_GAIN_Q32 / 4);
        assert_eq!(smooth_relief_gain_q32(FULL_RANGE_MM), FULL_GAIN_Q32);
        assert_eq!(smooth_relief_gain_q32(u32::MAX), FULL_GAIN_Q32);
        let mut previous = 0;
        for range in (0..=FULL_RANGE_MM).step_by(1_000) {
            let value = smooth_relief_gain_q32(range);
            assert!(previous <= value && value <= FULL_GAIN_Q32);
            previous = value;
        }
        for base in [-1_000_000, 0, 1_000_000] {
            for relief in [-700_000, 0, 700_000] {
                assert_eq!(
                    compose_smooth_relief(
                        HeightMm::new(base),
                        HeightMm::new(relief),
                        FULL_RANGE_MM,
                        FULL_GAIN_Q32
                    ),
                    apply_relief_gate(
                        HeightMm::new(base),
                        HeightMm::new(relief),
                        FULL_RELIEF_GATE_Q16
                    ),
                );
            }
        }
    }

    #[test]
    fn smooth_signed_relief_and_overflow_are_symmetric() {
        let positive = compose_smooth_relief(
            HeightMm::SEA_LEVEL,
            HeightMm::new(900_000),
            400_000,
            FULL_GAIN_Q32,
        )
        .unwrap()
        .raw();
        let negative = compose_smooth_relief(
            HeightMm::SEA_LEVEL,
            HeightMm::new(-900_000),
            400_000,
            FULL_GAIN_Q32,
        )
        .unwrap()
        .raw();
        assert!(positive > 0);
        assert_eq!(positive, -negative);
        assert_eq!(
            compose_smooth_relief(
                HeightMm::new(i32::MAX),
                HeightMm::new(1),
                FULL_RANGE_MM,
                FULL_GAIN_Q32
            ),
            Err(SpectralError::ArithmeticOverflow)
        );
        assert_eq!(
            compose_smooth_relief(
                HeightMm::new(i32::MIN),
                HeightMm::new(-1),
                FULL_RANGE_MM,
                FULL_GAIN_Q32
            ),
            Err(SpectralError::ArithmeticOverflow)
        );
        assert_eq!(
            compose_smooth_relief(
                HeightMm::SEA_LEVEL,
                HeightMm::new(1),
                FULL_RANGE_MM,
                FULL_GAIN_Q32 + 1
            ),
            Err(SpectralError::InvalidReliefGate)
        );
    }

    #[test]
    fn forced_rim_taper_is_zero_on_all_four_sides_and_padded_fringe() {
        let point = |x_km: i64, y_km: i64| TerrainPoint {
            x_um: x_km * KM_UM,
            y_um: y_km * KM_UM,
        };
        let (width, height) = (102, 204);
        for p in [
            point(0, 80),
            point(1, 80),
            point(100, 80),
            point(101, 80),
            point(103, 80),
            point(40, 0),
            point(40, 1),
            point(40, 202),
            point(40, 203),
            point(40, 205),
            point(0, 0),
        ] {
            assert_eq!(forced_rim_taper_q32(p, width, height), 0);
        }
        assert_eq!(
            forced_rim_taper_q32(point(2, 80), width, height),
            FULL_GAIN_Q32
        );
        assert_eq!(
            forced_rim_taper_q32(point(99, 80), width, height),
            FULL_GAIN_Q32
        );
        assert_eq!(
            forced_rim_taper_q32(point(40, 2), width, height),
            FULL_GAIN_Q32
        );
        assert_eq!(
            forced_rim_taper_q32(point(40, 201), width, height),
            FULL_GAIN_Q32
        );
        assert_eq!(
            forced_rim_taper_q32(
                TerrainPoint {
                    x_um: 1_500_000_000,
                    y_um: 80 * KM_UM
                },
                width,
                height
            ),
            FULL_GAIN_Q32 / 2
        );
        assert_eq!(
            forced_rim_taper_q32(
                TerrainPoint {
                    x_um: 99_500_000_000,
                    y_um: 80 * KM_UM
                },
                width,
                height
            ),
            FULL_GAIN_Q32 / 2
        );
    }

    #[test]
    fn flat_macro_has_zero_gate_and_remains_exact() {
        let base = HeightMm::new(1_000_000);
        let surrounding = neighbors(1_000_000);
        assert_eq!(relief_gate_q16(base, &surrounding), 0);
        assert_eq!(
            compose_relief(base, HeightMm::new(-900_000), &surrounding),
            Ok(base)
        );
    }

    #[test]
    fn saved_thresholds_give_zero_quarter_and_full_gate() {
        let base = HeightMm::new(1_000_000);
        assert_eq!(relief_gate_q16(base, &neighbors(1_400_000)), 0);
        assert_eq!(relief_gate_q16(base, &neighbors(1_800_000)), 16_384);
        assert_eq!(relief_gate_q16(base, &neighbors(3_000_000)), 65_536);
    }

    #[test]
    fn sea_level_crossing_has_no_hard_eligibility_jump() {
        let mut surrounding = neighbors(0);
        surrounding[0] = HeightMm::new(2_100_000);
        let relief = HeightMm::new(1_000_000);
        let values = [-1, 0, 1].map(|mm| {
            compose_relief(HeightMm::new(mm), relief, &surrounding)
                .unwrap()
                .raw()
        });
        assert_eq!(values, [999_999, 1_000_000, 1_000_001]);
    }

    #[test]
    fn signed_half_millimetre_rounds_away_from_zero() {
        let base = HeightMm::new(100);
        assert_eq!(
            apply_relief_gate(base, HeightMm::new(1), 32_768),
            Ok(HeightMm::new(101))
        );
        assert_eq!(
            apply_relief_gate(base, HeightMm::new(-1), 32_768),
            Ok(HeightMm::new(99))
        );
    }

    #[test]
    fn invalid_gate_and_signed_height_overflow_are_errors() {
        assert_eq!(
            apply_relief_gate(HeightMm::SEA_LEVEL, HeightMm::new(1), 65_537),
            Err(SpectralError::InvalidReliefGate)
        );
        assert_eq!(
            apply_relief_gate(HeightMm::new(i32::MAX), HeightMm::new(1), 65_536),
            Err(SpectralError::ArithmeticOverflow)
        );
        assert_eq!(
            apply_relief_gate(HeightMm::new(i32::MIN), HeightMm::new(-1), 65_536),
            Err(SpectralError::ArithmeticOverflow)
        );
    }
}
