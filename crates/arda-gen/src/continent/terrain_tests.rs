use super::{
    bundles::{boundary_height, coarse_height},
    ContinentGrid,
};

#[test]
fn coarse_sampling_preserves_an_affine_plane_between_nodes() {
    let grid = ContinentGrid {
        width: 10,
        height: 10,
        height_mm: (0..100)
            .map(|i| 300_000 + 3_000 * (i % 10) * 10 + 1_000 * (i / 10) * 10)
            .collect(),
    };
    for y in 20..70 {
        for x in 20..70 {
            assert_eq!(
                coarse_height(&grid, x, y),
                300_000 + 3_000 * x + 1_000 * y,
                "phase ({x},{y})"
            );
        }
    }
}

#[test]
fn detail_preserves_regional_land_zero_and_ocean_signs() {
    for coarse in [i32::MIN, -1, 0, 1, 1_000, i32::MAX] {
        let grid = ContinentGrid {
            width: 3,
            height: 3,
            height_mm: vec![coarse; 9],
        };
        for y in 0..32 {
            for x in 0..32 {
                let h = boundary_height(42, &grid, x, y);
                assert_eq!(
                    h > 0,
                    coarse > 0,
                    "coarse {coarse}, cell {x},{y}, height {h}"
                );
            }
        }
    }
}
