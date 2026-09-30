//! Globally addressable integer structural relief.
//!
//! The logic/02 structural-relief rule adds kilometre-scale relief to the
//! initial 100 m bed using absolute coordinates and nearby coarse relief.
//! The two integer-noise lattices have 8 km and 4 km spacing; the maximum
//! ungated height change is 1,081,344 mm.
use crate::noise::value_noise;

const OFFSETS: [(i32, i32); 8] = [
    (0, 50),
    (0, -50),
    (50, 0),
    (-50, 0),
    (35, 35),
    (35, -35),
    (-35, 35),
    (-35, -35),
];
const SEED_BROAD: u64 = 0x5354_5255_4354_3234;
const SEED_DETAIL: u64 = 0x5354_5255_4354_3132;

/// The ungated structural elevation delta at an absolute 100 m cell, in mm.
#[must_use]
pub(crate) fn raw_delta_mm(seed: u64, abs_x: i32, abs_y: i32) -> i64 {
    let broad = i64::from(value_noise(seed ^ SEED_BROAD, abs_x, abs_y, 80));
    let detail = i64::from(value_noise(seed ^ SEED_DETAIL, abs_x, abs_y, 40));
    23 * broad + 10 * detail
}

/// Window-independent gate from the absolute-coordinate nearby coarse range, Q16.
#[must_use]
pub(crate) fn relief_gate_q16(
    abs_x: i32,
    abs_y: i32,
    coarse_here: i32,
    mut coarse_at: impl FnMut(i32, i32) -> i32,
) -> i64 {
    let mut lo = i64::from(coarse_here);
    let mut hi = lo;
    for (dx, dy) in OFFSETS {
        let v = i64::from(coarse_at(
            abs_x.saturating_add(dx),
            abs_y.saturating_add(dy),
        ));
        lo = lo.min(v);
        hi = hi.max(v);
    }
    // Exactly the saved local-relief gate: (range_m - 400) / 1600.
    ((hi - lo - 400_000) * 65_536 / 1_600_000).clamp(0, 65_536)
}

/// Structural height at an absolute cell, based only on global inputs.
///
/// Ocean and nonpositive regional cells retain their original height. Land may
/// cross below sea level without a shoreline clamp; the shared physical water
/// pass classifies the resulting bed.
#[must_use]
pub(crate) fn sample_height(
    seed: u64,
    abs_x: i32,
    abs_y: i32,
    initial_mm: i32,
    coarse_mm: i32,
    coarse_at: impl FnMut(i32, i32) -> i32,
) -> i32 {
    if initial_mm <= 0 || coarse_mm <= 0 {
        return initial_mm;
    }
    let gate = relief_gate_q16(abs_x, abs_y, coarse_mm, coarse_at);
    let delta = raw_delta_mm(seed, abs_x, abs_y) * gate / 65_536;
    #[allow(clippy::cast_possible_truncation)] // The sum is explicitly clamped to i32.
    let bounded =
        (i64::from(initial_mm) + delta).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    bounded
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gate_thresholds_and_unclamped_coast_crossing_are_physical() {
        let at = (2432, 1920);
        let one_neighbour = |height| {
            move |x, y| {
                if (x, y) == (at.0 + 50, at.1) {
                    height
                } else {
                    1_000_000
                }
            }
        };
        assert_eq!(
            relief_gate_q16(at.0, at.1, 1_000_000, one_neighbour(1_400_000)),
            0
        );
        assert_eq!(
            relief_gate_q16(at.0, at.1, 1_000_000, one_neighbour(1_800_000)),
            16_384
        );
        assert_eq!(
            relief_gate_q16(at.0, at.1, 1_000_000, one_neighbour(3_000_000)),
            65_536
        );
        assert_eq!(
            sample_height(42, at.0, at.1, 100_000, 1_000_000, one_neighbour(1_400_000)),
            100_000
        );
        let raw = raw_delta_mm(42, at.0, at.1);
        assert!(raw < -100_000);
        assert_eq!(
            i64::from(sample_height(
                42,
                at.0,
                at.1,
                100_000,
                1_000_000,
                one_neighbour(1_800_000)
            )),
            100_000 + raw / 4
        );
        let crossed = sample_height(42, at.0, at.1, 100_000, 1_000_000, one_neighbour(3_000_000));
        assert_eq!(i64::from(crossed), 100_000 + raw);
        assert!(
            crossed <= 0,
            "shallow land was incorrectly clamped to shore"
        );
    }

    #[test]
    fn both_nonpositive_inputs_preserve_original_height() {
        let at = (2432, 1920);
        let high_range = |x, y| {
            if (x, y) == (at.0 + 50, at.1) {
                3_000_000
            } else {
                1_000_000
            }
        };
        for initial in [-100_000, 0] {
            assert_eq!(
                sample_height(42, at.0, at.1, initial, 1_000_000, high_range),
                initial
            );
        }
        for coarse in [-1_000_000, 0] {
            assert_eq!(
                sample_height(42, at.0, at.1, 100_000, coarse, high_range),
                100_000
            );
        }
    }

    #[test]
    fn integer_extremes_and_analytical_delta_bound() {
        for (x, y) in [
            (0, 0),
            (-1, -1),
            (i32::MAX, i32::MIN),
            (i32::MIN, i32::MAX),
            (2432, 1920),
            (3199, 2687),
        ] {
            assert!(raw_delta_mm(42, x, y).abs() <= 1_081_344);
            let max = sample_height(42, x, y, i32::MAX, i32::MAX, |_, _| i32::MIN);
            assert!((i32::MAX - 1_081_344..=i32::MAX).contains(&max));
            let min = sample_height(42, x, y, i32::MIN, i32::MIN, |_, _| i32::MAX);
            assert_eq!(min, i32::MIN);
        }
    }
}
