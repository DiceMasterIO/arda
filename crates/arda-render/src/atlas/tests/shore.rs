//! Shorelines and ownership: the optional fine window, contour classes,
//! thin land and water, true-neighbour seams and class-filtered sampling.

use super::*;

fn fixture() -> AtlasTerrain {
    AtlasTerrain {
        palette: vec![[0; 3]; SAMPLE_SIDE * SAMPLE_SIDE],
        light: vec![LIGHT_ONE; SAMPLE_SIDE * SAMPLE_SIDE],
        classes: vec![TerrainKind::Sea; SAMPLE_SIDE * SAMPLE_SIDE],
        heights: vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
        wetness: vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
        moisture: vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
        forest_density: vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
        temperature: vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
        lake_surface: vec![i32::MIN; SAMPLE_SIDE * SAMPLE_SIDE],
        channel_width: vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
        shore: vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
        salt: Vec::new(),
        has_lake_depths: false,
        data_meanders: false,
        fine: None,
    }
}

#[test]
fn optional_fine_window_preserves_saved_sea_and_lake_colours() {
    use arda_core::{AreaCoord, HeightMm, TerrainField, TerrainPoint};

    let origin = TerrainPoint {
        x_um: -200_000_000,
        y_um: -200_000_000,
    };
    let last = TerrainPoint {
        x_um: 51_400_000_000,
        y_um: 51_400_000_000,
    };
    let field = TerrainField::new(
        origin,
        100_000_000,
        517,
        517,
        vec![HeightMm::new(1_000_000); 517 * 517],
    )
    .unwrap();
    let bounds = AtlasFineWorldBounds {
        min: origin,
        max: last,
    };
    let x = AxisKernel::Linear {
        low: 50,
        high_weight: 1,
        denominator: 2,
    };
    let y = AxisKernel::Linear {
        low: 60,
        high_weight: 1,
        denominator: 2,
    };
    for (height, class) in [(-3_000_000, TerrainKind::Sea), (500_000, TerrainKind::Lake)] {
        let cells = filled(height, class);
        let legacy = AtlasTerrain::new(&cells, edge_halo()).unwrap();
        let fine = AtlasTerrain::new_with_fine(
            &cells,
            &[],
            edge_halo(),
            AreaCoord::new(0, 0),
            bounds,
            field.clone(),
        )
        .unwrap();
        assert_eq!(
            fine.sample(x, y, class).unwrap(),
            legacy.sample(x, y, class).unwrap()
        );
    }
}

#[test]
fn contour_uses_class_directed_zero_heights_and_saved_ties() {
    let mut terrain = fixture();
    for (x, y, class, height) in [
        (99, 100, TerrainKind::Land, 0),
        (100, 99, TerrainKind::Land, 0),
        (100, 100, TerrainKind::Land, 0),
        (101, 100, TerrainKind::Sea, 0),
        (100, 101, TerrainKind::Land, 0),
        (101, 101, TerrainKind::Sea, 0),
    ] {
        let i = sample_index(x, y).unwrap();
        terrain.classes[i] = class;
        terrain.heights[i] = height;
    }
    let owner = CellCoord::new(100, 100).unwrap();
    let quarter = AxisKernel::Linear {
        low: 100,
        high_weight: 1,
        denominator: 4,
    };
    let middle = AxisKernel::Linear {
        low: 100,
        high_weight: 1,
        denominator: 2,
    };
    let centre = AxisKernel::Linear {
        low: 100,
        high_weight: 0,
        denominator: 4,
    };
    assert_eq!(
        terrain
            .contour_class(quarter, middle, owner, TerrainKind::Land)
            .unwrap(),
        TerrainKind::Land
    );
    assert_eq!(
        terrain
            .contour_class(middle, middle, owner, TerrainKind::Land)
            .unwrap(),
        TerrainKind::Land
    );
    assert_eq!(
        terrain
            .contour_class(
                middle,
                middle,
                CellCoord::new(101, 100).unwrap(),
                TerrainKind::Sea
            )
            .unwrap(),
        TerrainKind::Sea
    );
    assert_eq!(
        terrain
            .contour_class(centre, centre, owner, TerrainKind::Land)
            .unwrap(),
        TerrainKind::Land
    );
    assert_eq!(
        terrain
            .contour_class(
                quarter,
                AxisKernel::Box {
                    start: 100,
                    end: 101
                },
                owner,
                TerrainKind::Land
            )
            .unwrap(),
        TerrainKind::Land
    );
    terrain.classes[sample_index(100, 101).unwrap()] = TerrainKind::Lake;
    let near_middle = AxisKernel::Linear {
        low: 100,
        high_weight: 49,
        denominator: 100,
    };
    assert_eq!(
        terrain
            .contour_class(near_middle, middle, owner, TerrainKind::Land)
            .unwrap(),
        TerrainKind::Land
    );
    terrain.classes[sample_index(100, 101).unwrap()] = TerrainKind::Sea;
    terrain.classes[sample_index(101, 101).unwrap()] = TerrainKind::Land;
    terrain.heights[sample_index(100, 100).unwrap()] = 100_000;
    terrain.heights[sample_index(101, 101).unwrap()] = 100_000;
    let three_quarters = AxisKernel::Linear {
        low: 100,
        high_weight: 3,
        denominator: 4,
    };
    assert_eq!(
        terrain
            .contour_class(
                three_quarters,
                quarter,
                CellCoord::new(101, 100).unwrap(),
                TerrainKind::Sea,
            )
            .unwrap(),
        TerrainKind::Sea
    );
}

#[test]
fn thin_island_and_strait_keep_connected_rendered_footprints_at_even_scales() {
    use crate::channels::{AreaImageScale, AreaRaster, ChannelInput};
    use crate::GlobalCell;
    for (background, feature, background_height) in [
        (TerrainKind::Sea, TerrainKind::Land, i32::MIN),
        (TerrainKind::Land, TerrainKind::Sea, i32::MAX),
    ] {
        let mut cells = filled(background_height, background);
        let y_range = if feature == TerrainKind::Land {
            100..101
        } else {
            0..512
        };
        for y in y_range {
            cells.set(CellCoord::new(100, y).unwrap(), cell(0, feature));
        }
        let terrain = AtlasTerrain::new(&cells, edge_halo()).unwrap();
        for side in [2048_u32, 8192] {
            let quality = crate::ImageQuality::new(side).unwrap();
            let mut raster = AreaRaster::new_with_terrain(
                &cells,
                Some(&terrain),
                Vec::<Result<ChannelInput, RenderError>>::new(),
                GlobalCell { x: 0, y: 0 },
                AreaImageScale::Custom(quality),
                &[],
            )
            .unwrap();
            let scale = usize::try_from(side / 512).unwrap();
            let first_y = 100 * scale;
            let mut visible = 0;
            for py in first_y..first_y + scale {
                let row = raster.row(py).unwrap();
                for px in 100 * scale..101 * scale {
                    let x = axis_kernel(u32::try_from(px).unwrap(), side).unwrap();
                    let y = axis_kernel(u32::try_from(py).unwrap(), side).unwrap();
                    assert_eq!(
                        &row[px * 3..px * 3 + 3],
                        &terrain.sample(x, y, feature).unwrap()
                    );
                    visible += 1;
                }
            }
            assert_eq!(visible, scale * scale);
        }
    }
}

#[test]
fn true_neighbor_halo_matches_same_shore_inside_an_area() {
    let sea = filled(-100_000, TerrainKind::Sea);
    let land = filled(100_000, TerrainKind::Land);
    let mut left_halo = edge_halo();
    left_halo.states[AtlasNeighbor::East.index()] = NeighborState::Unset;
    left_halo.copy_neighbor(AtlasNeighbor::East, &land).unwrap();
    let mut right_halo = edge_halo();
    right_halo.states[AtlasNeighbor::West.index()] = NeighborState::Unset;
    right_halo.copy_neighbor(AtlasNeighbor::West, &sea).unwrap();
    let left = AtlasTerrain::new(&sea, left_halo).unwrap();
    let right = AtlasTerrain::new(&land, right_halo).unwrap();
    let mut interior = sea.clone();
    for y in 0..512 {
        for x in 256..512 {
            interior.set(
                CellCoord::new(x, y).unwrap(),
                cell(100_000, TerrainKind::Land),
            );
        }
    }
    let reference = AtlasTerrain::new(&interior, edge_halo()).unwrap();
    for side in [2048, 8192] {
        let y = axis_kernel(side / 2, side).unwrap();
        for (boundary, pixel, owner, saved, reference_pixel, reference_owner) in [
            (
                &left,
                side - 1,
                CellCoord::new(511, 256).unwrap(),
                TerrainKind::Sea,
                side / 2 - 1,
                CellCoord::new(255, 256).unwrap(),
            ),
            (
                &right,
                0,
                CellCoord::new(0, 256).unwrap(),
                TerrainKind::Land,
                side / 2,
                CellCoord::new(256, 256).unwrap(),
            ),
        ] {
            let x = axis_kernel(pixel, side).unwrap();
            let rx = axis_kernel(reference_pixel, side).unwrap();
            let got = boundary.contour_class(x, y, owner, saved).unwrap();
            let want = reference
                .contour_class(rx, y, reference_owner, saved)
                .unwrap();
            assert_eq!(got, want);
            assert_eq!(
                boundary.sample(x, y, got).unwrap(),
                reference.sample(rx, y, want).unwrap()
            );
        }
    }
}

#[test]
fn palette_and_light_are_interpolated_separately() {
    let mut terrain = fixture();
    for (x, y, colour, light) in [
        (0, 0, [100, 80, 60], 2048),
        (1, 0, [200, 160, 120], 5120),
        (0, 1, [100, 80, 60], 2048),
        (1, 1, [200, 160, 120], 5120),
    ] {
        let index = sample_index(x, y).unwrap();
        terrain.palette[index] = colour;
        terrain.light[index] = light;
        terrain.classes[index] = TerrainKind::Land;
    }
    let midpoint = AxisKernel::Linear {
        low: 0,
        high_weight: 1,
        denominator: 2,
    };
    let got = terrain
        .sample(midpoint, midpoint, TerrainKind::Land)
        .unwrap();
    assert_eq!(got, modulate([150, 120, 90], 3584));
    assert_ne!(got, [150, 120, 90]);
    let centre = AxisKernel::Linear {
        low: 0,
        high_weight: 0,
        denominator: 1024,
    };
    assert_eq!(
        terrain.sample(centre, centre, TerrainKind::Land).unwrap(),
        terrain.colour(CellCoord::new(0, 0).unwrap())
    );
}

#[test]
fn class_filter_prevents_land_sea_and_lake_bleed() {
    let mut terrain = fixture();
    for (x, y, class, colour) in [
        (0, 0, TerrainKind::Land, [200, 100, 50]),
        (1, 0, TerrainKind::Sea, [10, 20, 30]),
        (0, 1, TerrainKind::Sea, [10, 20, 30]),
        (1, 1, TerrainKind::Lake, [50, 80, 120]),
    ] {
        let index = sample_index(x, y).unwrap();
        terrain.palette[index] = colour;
        terrain.classes[index] = class;
    }
    let kernel = AxisKernel::Linear {
        low: 0,
        high_weight: 1,
        denominator: 2,
    };
    assert_eq!(
        terrain.sample(kernel, kernel, TerrainKind::Land).unwrap(),
        [200, 100, 50]
    );
    assert_eq!(
        terrain.sample(kernel, kernel, TerrainKind::Sea).unwrap(),
        [10, 20, 30]
    );
    assert_eq!(
        terrain.sample(kernel, kernel, TerrainKind::Lake).unwrap(),
        [50, 80, 120]
    );
    let absent = AxisKernel::Linear {
        low: 2,
        high_weight: 1,
        denominator: 2,
    };
    assert!(matches!(
        terrain.sample(absent, absent, TerrainKind::Land),
        Err(RenderError::AtlasContext { .. })
    ));
    let oversized = AxisKernel::Linear {
        low: 0,
        high_weight: u32::MAX - 1,
        denominator: u32::MAX,
    };
    assert!(matches!(
        terrain.sample(oversized, oversized, TerrainKind::Land),
        Err(RenderError::AtlasContext { .. })
    ));
}

#[test]
fn mixed_box_linear_kernel_uses_full_cartesian_weights() {
    let mut terrain = fixture();
    for (x, y, colour, light) in [
        (510, 0, [10, 20, 30], 2048),
        (511, 0, [30, 40, 50], 3072),
        (510, 1, [50, 60, 70], 4096),
        (511, 1, [70, 80, 90], 5120),
    ] {
        let index = sample_index(x, y).unwrap();
        terrain.palette[index] = colour;
        terrain.light[index] = light;
        terrain.classes[index] = TerrainKind::Land;
    }
    let x = axis_kernel(510, 511).unwrap();
    let y = axis_kernel(1, 513).unwrap();
    assert_eq!(
        x,
        AxisKernel::Box {
            start: 510,
            end: 512
        }
    );
    assert_eq!(
        y,
        AxisKernel::Linear {
            low: 0,
            high_weight: 1023,
            denominator: 1026
        }
    );
    assert_eq!(
        terrain.sample(x, y, TerrainKind::Land).unwrap(),
        modulate([60, 70, 80], 4602)
    );
}

#[test]
fn buffers_have_bounded_exact_lengths() {
    // 24 bytes since recipe-5 snow and meanders read saved temperature and
    // channel width (logic/04 §atlas-formed): a fixed 6.4 MB halo per area.
    assert_eq!(std::mem::size_of::<Option<SourceSample>>(), 24);
    assert_eq!(AtlasHalo::new().context.len(), 516 * 516);
    let terrain = AtlasTerrain::new(&filled(0, TerrainKind::Land), edge_halo()).unwrap();
    assert_eq!(terrain.palette.len(), 514 * 514);
    assert_eq!(terrain.light.len(), 514 * 514);
    assert_eq!(terrain.classes.len(), 514 * 514);
    assert_eq!(terrain.heights.len(), 514 * 514);
}
