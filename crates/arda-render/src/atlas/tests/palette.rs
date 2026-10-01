//! Relief light and palette: saved slope, wetness and ecology tints, rock
//! and snow, and the sea depth bands.

use super::*;

#[test]
fn saved_slope_controls_rock_and_light_without_changing_flat_land() {
    assert_eq!(land_material(500_000, 0, 0, 0), [165, 161, 97]);
    assert_eq!(relief_light(0, 0), LIGHT_ONE);
    assert_eq!(
        land_material(500_000, 200_000, 0, 0),
        land_material(500_000, -200_000, 0, 0)
    );
    assert_ne!(
        land_material(500_000, 200_000, 0, 0),
        land_material(500_000, 0, 0, 0)
    );
    assert!(relief_light(200_000, 0) > LIGHT_ONE);
    assert!(relief_light(-200_000, 0) < LIGHT_ONE);

    let mut land_cells = filled(500_000, TerrainKind::Land);
    for (x, y, height) in [
        (199, 200, 400_000),
        (201, 200, 600_000),
        (299, 300, 600_000),
        (301, 300, 400_000),
    ] {
        land_cells.set(
            CellCoord::new(x, y).unwrap(),
            cell(height, TerrainKind::Land),
        );
    }
    let land = AtlasTerrain::new(&land_cells, edge_halo()).unwrap();
    assert_eq!(land.colour(CellCoord::new(0, 0).unwrap()), [165, 161, 97]);
    assert_eq!(
        land.colour(CellCoord::new(200, 200).unwrap()),
        modulate(
            land_material(500_000, 200_000, 0, 0),
            relief_light(200_000, 0)
        )
    );
    assert_eq!(
        land.colour(CellCoord::new(300, 300).unwrap()),
        modulate(
            land_material(500_000, -200_000, 0, 0),
            relief_light(-200_000, 0)
        )
    );
    let sea = AtlasTerrain::new(&filled(-3_000_000, TerrainKind::Sea), edge_halo()).unwrap();
    let lake = AtlasTerrain::new(&filled(500_000, TerrainKind::Lake), edge_halo()).unwrap();
    assert_eq!(record(&sea, 0, 0).1, LIGHT_ONE);
    assert_eq!(record(&lake, 0, 0).1, LIGHT_ONE);
    assert_eq!(sea.colour(CellCoord::new(0, 0).unwrap()), [10, 37, 68]);
    assert_eq!(
        lake.colour(CellCoord::new(0, 0).unwrap()),
        crate::carto::LAKE_FILL
    );
}

#[test]
fn wetness_tint_uses_saved_index_without_changing_zero_wetness() {
    assert_eq!(land_material(500_000, 0, 0, 0), [165, 161, 97]);
    assert_eq!(land_material(500_000, 0, 0, 12), [144, 152, 91]);
    assert_ne!(land_material(500_000, 0, 0, 255), [165, 161, 97]);
}

#[test]
fn saved_ecology_changes_plain_colour_before_rock_or_snow() {
    let old = land_material(300_000, 0, 0, 100);
    assert_eq!(old, land_material_ecology(300_000, 0, 0, 100, 0, 0));
    let meadow = land_material_ecology(300_000, 0, 0, 100, 130, 0);
    let forest = land_material_ecology(300_000, 0, 0, 100, 130, 220);
    let dry = land_material_ecology(300_000, 0, 0, 100, 40, 0);
    assert!(forest[1] < meadow[1]);
    assert!(dry[0] > meadow[0]);
    assert_ne!(meadow, old);
}

#[test]
fn saved_ecology_colour_is_independent_of_routing_wetness() {
    let dry_route = land_material_ecology(300_000, 0, 0, 10, 130, 220);
    let wet_route = land_material_ecology(300_000, 0, 0, 250, 130, 220);
    assert_eq!(dry_route, wet_route);
}

#[test]
fn wetness_halo_matches_internal_cardinal_and_diagonal_samples() {
    let wet_area = |wetness| {
        let mut saved = cell(500_000, TerrainKind::Land);
        saved.wetness = wetness;
        AreaCells::flat(saved)
    };
    let target = wet_area(12);
    let mut halo = edge_halo();
    for (direction, wetness) in [
        (AtlasNeighbor::North, 80),
        (AtlasNeighbor::West, 120),
        (AtlasNeighbor::NorthWest, 255),
    ] {
        halo.states[direction.index()] = NeighborState::Unset;
        halo.copy_neighbor(direction, &wet_area(wetness)).unwrap();
    }
    let boundary = AtlasTerrain::new(&target, halo).unwrap();
    assert_eq!(
        record(&boundary, -1, -1).0,
        land_material(500_000, 0, 0, 255)
    );
    assert_eq!(record(&boundary, 0, -1).0, land_material(500_000, 0, 0, 80));
    assert_eq!(
        record(&boundary, -1, 0).0,
        land_material(500_000, 0, 0, 120)
    );

    let mut internal = target.clone();
    for y in 0..512 {
        let mut saved = cell(500_000, TerrainKind::Land);
        saved.wetness = 120;
        internal.set(CellCoord::new(255, y).unwrap(), saved);
    }
    for x in 0..512 {
        let mut saved = cell(500_000, TerrainKind::Land);
        saved.wetness = 80;
        internal.set(CellCoord::new(x, 255).unwrap(), saved);
    }
    let mut corner = cell(500_000, TerrainKind::Land);
    corner.wetness = 255;
    internal.set(CellCoord::new(255, 255).unwrap(), corner);
    let reference = AtlasTerrain::new(&internal, edge_halo()).unwrap();
    let shifted = |kernel| match kernel {
        AxisKernel::Linear {
            low,
            high_weight,
            denominator,
        } => AxisKernel::Linear {
            low: low + 256,
            high_weight,
            denominator,
        },
        AxisKernel::Box { .. } => unreachable!(),
    };
    for (px, py) in [(0, 0), (0, 256), (256, 0)] {
        let x = axis_kernel(px, 1024).unwrap();
        let y = axis_kernel(py, 1024).unwrap();
        assert_eq!(
            boundary.sample(x, y, TerrainKind::Land).unwrap(),
            reference
                .sample(shifted(x), shifted(y), TerrainKind::Land)
                .unwrap(),
            "halo mismatch at ({px}, {py})"
        );
    }
}

#[test]
fn wetness_changes_land_without_recoloring_water_or_ownership() {
    let mut dry = filled(-100_000, TerrainKind::Sea);
    dry.set(
        CellCoord::new(200, 200).unwrap(),
        cell(500_000, TerrainKind::Land),
    );
    dry.set(
        CellCoord::new(201, 200).unwrap(),
        cell(500_000, TerrainKind::Lake),
    );
    let mut wet = dry.clone();
    for (x, y) in [(199, 200), (200, 200), (201, 200)] {
        let at = CellCoord::new(x, y).unwrap();
        let mut saved = *wet.get(at);
        saved.wetness = 255;
        wet.set(at, saved);
    }
    let control = AtlasTerrain::new(&dry, edge_halo()).unwrap();
    let variant = AtlasTerrain::new(&wet, edge_halo()).unwrap();
    assert_ne!(record(&control, 200, 200).0, record(&variant, 200, 200).0);
    for (x, class, pixel) in [(199, TerrainKind::Sea, 399), (201, TerrainKind::Lake, 403)] {
        assert_eq!(record(&control, x, 200), record(&variant, x, 200));
        assert_eq!(
            control
                .sample(
                    axis_kernel(pixel, 1024).unwrap(),
                    axis_kernel(400, 1024).unwrap(),
                    class
                )
                .unwrap(),
            variant
                .sample(
                    axis_kernel(pixel, 1024).unwrap(),
                    axis_kernel(400, 1024).unwrap(),
                    class
                )
                .unwrap()
        );
    }
    assert_eq!(control.classes, variant.classes);
}

#[test]
fn nonzero_wetness_matches_buffered_and_streamed_mixed_axis_overviews() {
    let mut low = cell(500_000, TerrainKind::Land);
    low.wetness = 12;
    let mut high = cell(500_000, TerrainKind::Land);
    high.wetness = 120;
    let cells = [AreaCells::flat(low), AreaCells::flat(high)];
    let atlas_for = |index| {
        let mut halo = edge_halo();
        let direction = if index == 0 {
            AtlasNeighbor::East
        } else {
            AtlasNeighbor::West
        };
        halo.states[direction.index()] = NeighborState::Unset;
        halo.copy_neighbor(direction, &cells[1 - index]).unwrap();
        AtlasTerrain::new(&cells[index], halo).unwrap()
    };
    assert_ne!(
        atlas_for(0).colour(CellCoord::new(256, 256).unwrap()),
        atlas_for(1).colour(CellCoord::new(256, 256).unwrap())
    );
    let decode = |bytes: &[u8]| {
        let mut reader = png::Decoder::new(bytes).read_info().unwrap();
        let mut rgb = vec![0; reader.output_buffer_size()];
        let frame = reader.next_frame(&mut rgb).unwrap();
        (frame.width, frame.height, rgb)
    };
    for (width, height) in [(1025, 257), (1001, 513)] {
        let mut buffered = crate::OverviewRaster::new_exact(2, 1, width, height).unwrap();
        for (index, saved) in cells.iter().enumerate() {
            buffered
                .push_atlas(
                    arda_core::AreaCoord::new(i32::try_from(index).unwrap(), 0),
                    saved,
                    &atlas_for(index),
                )
                .unwrap();
        }
        let buffered = buffered.finish().unwrap();
        let mut streamed = Vec::new();
        crate::write_atlas_overview_png(2, 1, width, height, &mut streamed, |at| {
            let index = usize::try_from(at.x).unwrap();
            Ok::<_, RenderError>((cells[index].clone(), atlas_for(index)))
        })
        .unwrap();
        assert_eq!(decode(&buffered), decode(&streamed));
    }
}

#[test]
fn rock_and_snow_vary_continuously_with_saved_height() {
    let low_snow = land_material(2_850_000, 0, 0, 0);
    let high_snow = land_material(4_300_000, 0, 0, 0);
    assert!(high_snow
        .iter()
        .zip(low_snow)
        .all(|(high, low)| *high > low));
    let exposed_peak = land_material(4_300_000, 200_000, 0, 0);
    assert!(exposed_peak
        .iter()
        .zip(high_snow)
        .all(|(rock, snow)| *rock < snow));
    for edge in [1_000_000, 1_500_000, 2_000_000, 2_850_000, 4_300_000] {
        let below = land_material(edge - 1, 200_000, 0, 0);
        let above = land_material(edge, 200_000, 0, 0);
        assert!(
            below.iter().zip(above).all(|(a, b)| a.abs_diff(b) <= 1),
            "material discontinuity at {edge} mm"
        );
    }
}

#[test]
fn extreme_saved_gradients_remain_bounded_and_deterministic() {
    let maximum = i64::from(i32::MAX) - i64::from(i32::MIN);
    for (dx, dy) in [(maximum, maximum), (-maximum, maximum), (maximum, -maximum)] {
        assert_eq!(
            land_material(i32::MAX, dx, dy, 0),
            land_material(i32::MAX, -dx, -dy, 0),
            "rock and snow depend on slope magnitude"
        );
        assert!((MIN_LIGHT..=MAX_LIGHT).contains(&i128::from(relief_light(dx, dy))));
        if dx == dy {
            assert_ne!(relief_light(dx, dy), relief_light(-dx, -dy));
        }
    }
    let high = filled(i32::MAX, TerrainKind::Land);
    let low = filled(i32::MIN, TerrainKind::Land);
    let mut extreme_halo = AtlasHalo::new();
    for direction in DIRECTIONS {
        extreme_halo
            .copy_neighbor(
                direction,
                if direction == AtlasNeighbor::East {
                    &low
                } else {
                    &high
                },
            )
            .unwrap();
    }
    let extreme = AtlasTerrain::new(&high, extreme_halo).unwrap();
    assert_eq!(
        extreme.colour(CellCoord::new(256, 256).unwrap()),
        modulate(land_material(i32::MAX, 0, 0, 0), LIGHT_ONE)
    );
    assert_eq!(
        extreme.colour(CellCoord::new(511, 511).unwrap()),
        modulate(
            land_material(i32::MAX, -maximum, 0, 0),
            relief_light(-maximum, 0)
        )
    );
}

#[test]
fn sea_palette_interpolates_depth_bands_without_seams_or_overflow() {
    assert_eq!(sea_palette(1), [103, 163, 168]);
    assert_eq!(sea_palette(0), [103, 163, 168]);
    assert_eq!(sea_palette(-25_000), [73, 137, 155]);
    assert_eq!(sea_palette(-50_000), [43, 110, 141]);
    assert_eq!(sea_palette(-150_000), [32, 91, 126]);
    assert_eq!(sea_palette(-249_999), [20, 72, 110]);
    assert_eq!(sea_palette(-250_000), [20, 72, 110]);
    assert_eq!(sea_palette(-250_001), [20, 72, 110]);
    assert_eq!(sea_palette(-1_000_000), [12, 45, 80]);
    assert_eq!(sea_palette(-6_000_000), [7, 26, 51]);
    assert_eq!(sea_palette(i32::MIN), [7, 26, 51]);

    let depths = [0, 25_000, 50_000, 150_000, 250_000, 1_000_000, 6_000_000];
    for pair in depths.windows(2) {
        let shallow = sea_palette(-pair[0]);
        let deep = sea_palette(-pair[1]);
        assert!(shallow.into_iter().zip(deep).all(|(a, b)| a >= b));
    }
}
