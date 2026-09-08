//! Integer area relief keyed only by absolute cell coordinates.
//!
//! Four octave lattice spacings (40, 20, 10 and 5 cells) keep the finest
//! lattice at 500 m. The former fifth octave placed independent lattice
//! values only 200 m apart and dominated the number of tiny physical
//! depressions in the measured controls. This changes raw terrain; no water
//! body is removed or hidden. Its amplitude follows the surrounding 1 km
//! regional relief, independently of absolute land elevation (`logic/02`,
//! Regional detail correction).

pub(super) fn sample(seed: u64, x: i32, y: i32) -> i32 {
    crate::noise::fbm(seed, x, y, 40, 4)
}

#[cfg(test)]
mod tests {
    use super::sample;
    use crate::continent::bundles::{abs_cell, refine_height, refinement_relief};
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
                        for relief in [i64::MIN, 0, 1, 2_000, 90_000, i64::MAX] {
                            let height = refine_height(seed, coarse, relief, x, y);
                            assert_eq!(height > 0, coarse > 0);
                            let amplitude = if coarse > 0 {
                                relief.clamp(0, 90_000)
                            } else {
                                1_500
                            };
                            assert!((i64::from(height) - i64::from(coarse)).abs() <= amplitude);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn flat_regional_control_does_not_create_sampling_scale_pit_carpet() {
        // A flat regional surface supplies no local relief. Its height alone
        // must not create hollows; this asserts physical geometry, not a
        // target lake count or water-body inclusion threshold.
        for coarse in [1, 750_000, 2_500_000] {
            let relief = refinement_relief(coarse, |_, _| coarse);
            for y in 0..128 {
                for x in 0..128 {
                    assert_eq!(refine_height(436_342, coarse, relief, x, y), coarse);
                }
            }
        }
    }

    #[test]
    fn broad_regional_depression_retains_its_surrounding_rim() {
        // Regional relief exceeds the allowed detail envelope. Removing the
        // shortest octave must not imply filling or breaching such bowls.
        let at = |x, y| 750_000 + 64 * (x * x + y * y);
        let refined = |x, y| {
            let coarse = at(x, y);
            let relief = refinement_relief(coarse, |dx, dy| at(x + dx, y + dy));
            refine_height(42, coarse, relief, x, y)
        };
        let center = refined(0, 0);
        for i in -64..=64 {
            for (x, y) in [(i, -64), (i, 64), (-64, i), (64, i)] {
                assert!(refined(x, y) > center);
            }
        }
    }

    fn sloping_regional_grid(base: i32) -> crate::continent::ContinentGrid {
        crate::continent::ContinentGrid {
            width: 64,
            height: 64,
            height_mm: (0..64 * 64)
                .map(|i| base + (i % 64) * 1_000 - (i / 64) * 200)
                .collect(),
        }
    }

    #[test]
    fn translating_all_land_regional_terrain_does_not_amplify_detail() {
        use crate::continent::bundles::{boundary_height, coarse_height};
        let low = sloping_regional_grid(250_000);
        let high = sloping_regional_grid(1_750_000);
        for seed in [42, 436_342] {
            for y in 125..155 {
                for x in 125..155 {
                    let low_detail = boundary_height(seed, &low, x, y) - coarse_height(&low, x, y);
                    let high_detail =
                        boundary_height(seed, &high, x, y) - coarse_height(&high, x, y);
                    assert_eq!(
                        low_detail, high_detail,
                        "height alone must not manufacture roughness"
                    );
                }
            }
        }
    }

    #[test]
    fn gentle_regional_slope_retains_a_physical_downhill_neighbor() {
        use crate::continent::bundles::boundary_height;
        let grid = sloping_regional_grid(750_000);
        for seed in [42, 436_342] {
            for y in 125..155 {
                for x in 125..155 {
                    let center = boundary_height(seed, &grid, x, y);
                    assert!((-1..=1).any(|dy| {
                        (-1..=1).any(|dx| {
                            (dx != 0 || dy != 0)
                                && boundary_height(seed, &grid, x + dx, y + dy) < center
                        })
                    }));
                }
            }
        }
    }

    #[test]
    fn regional_relief_varies_smoothly_across_one_kilometre_grid_lines() {
        use crate::continent::bundles::coarse_height;
        let grid = crate::continent::ContinentGrid {
            width: 64,
            height: 64,
            height_mm: (0..64 * 64)
                .map(|i| 750_000 + (i % 64) * (i % 64) * 1_000)
                .collect(),
        };
        let relief = |x| {
            let coarse = coarse_height(&grid, x, 135);
            refinement_relief(coarse, |dx, dy| coarse_height(&grid, x + dx, 135 + dy))
        };
        // This quadratic increases its slope by 2 m per kilometre, so its
        // 1 km relief increases smoothly by about 200 mm per 100 m sample.
        // A range held constant inside each enclosing 1 km square would
        // instead jump by about 2 m at the next grid line.
        for x in 125..145 {
            let step = relief(x + 1) - relief(x);
            assert!((1..=300).contains(&step), "regional relief ledge: {step}");
        }
    }

    #[test]
    fn regional_relief_handles_opposite_height_extremes() {
        assert_eq!(
            refinement_relief(i32::MIN, |_, _| i32::MAX),
            i64::from(u32::MAX)
        );
        assert_eq!(
            refinement_relief(i32::MAX, |_, _| i32::MIN),
            i64::from(u32::MAX)
        );
    }
}
