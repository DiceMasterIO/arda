//! Integer area relief keyed only by absolute cell coordinates.
//!
//! Four octave lattice spacings (40, 20, 10 and 5 cells) keep the finest
//! lattice at 500 m. The former fifth octave placed independent lattice
//! values only 200 m apart and dominated the number of tiny physical
//! depressions in the measured controls. This changes raw terrain; no water
//! body is removed or hidden.

pub(super) fn sample(seed: u64, x: i32, y: i32) -> i32 {
    crate::noise::fbm(seed, x, y, 40, 4)
}

#[cfg(test)]
mod tests {
    use super::sample;
    use crate::continent::bundles::{abs_cell, refine_height};
    use arda_core::AreaCoord;

    #[test]
    fn opposite_area_edges_name_identical_samples_including_negative_areas() {
        for ay in -2..=2 {
            for ax in -2..=2 {
                for offset in 0..512 {
                    let (x, y) = abs_cell(AreaCoord::new(ax, ay), 512, offset);
                    let (nx, ny) = abs_cell(AreaCoord::new(ax + 1, ay), 0, offset);
                    assert_eq!(sample(42, x, y), sample(42, nx, ny));
                    let (x, y) = abs_cell(AreaCoord::new(ax, ay), offset, 512);
                    let (nx, ny) = abs_cell(AreaCoord::new(ax, ay + 1), offset, 0);
                    assert_eq!(sample(42, x, y), sample(42, nx, ny));
                }
            }
        }
    }

    #[test]
    fn full_coordinate_range_preserves_amplitude_and_regional_sign() {
        for x in [i32::MIN, -513, -512, -1, 0, 1, 511, 512, i32::MAX] {
            for y in [i32::MIN, -513, -1, 0, 1, 512, i32::MAX] {
                for seed in [0, 42, 436_342, u64::MAX] {
                    assert!((-32_768..=32_767).contains(&sample(seed, x, y)));
                    for coarse in [i32::MIN, -1, 0, 1, 10_000, 1_500_000, i32::MAX] {
                        let height = refine_height(seed, coarse, x, y);
                        assert_eq!(height > 0, coarse > 0);
                        let amplitude = if coarse > 0 {
                            i64::from((coarse / 12).clamp(2_000, 90_000))
                        } else {
                            1_500
                        };
                        assert!((i64::from(height) - i64::from(coarse)).abs() <= amplitude);
                    }
                }
            }
        }
    }

    #[test]
    fn flat_regional_control_does_not_create_sampling_scale_pit_carpet() {
        // This is a relief-sampling regression, not a limit on lake count.
        // The old two-cell octave produced hundreds of isolated minima in
        // this 12.8 km square even on a constant regional surface.
        let height: Vec<_> = (0..128 * 128)
            .map(|i| refine_height(436_342, 750_000, i % 128, i / 128))
            .collect();
        let mut minima = 0;
        for y in 1..127 {
            for x in 1..127 {
                let center = height[y * 128 + x];
                let is_minimum = (-1..=1).all(|dy| {
                    (-1..=1).all(|dx| {
                        (dx == 0 && dy == 0)
                            || center
                                < height[(y.checked_add_signed(dy).unwrap() * 128)
                                    + x.checked_add_signed(dx).unwrap()]
                    })
                });
                minima += usize::from(is_minimum);
            }
        }
        assert!(minima < 128, "sampling-scale pit carpet: {minima} minima");
    }

    #[test]
    fn broad_regional_depression_retains_its_surrounding_rim() {
        // Regional relief exceeds the allowed detail envelope. Removing the
        // shortest octave must not imply filling or breaching such bowls.
        let center = refine_height(42, 750_000, 0, 0);
        for i in -64..=64 {
            for (x, y) in [(i, -64), (i, 64), (-64, i), (64, i)] {
                let coarse = 750_000 + 64 * (x * x + y * y);
                assert!(refine_height(42, coarse, x, y) > center);
            }
        }
    }
}
