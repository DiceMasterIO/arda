use super::*;
use arda_core::{Cell, HeightMm};

mod atlas;

fn land() -> AreaCells {
    AreaCells::flat(Cell {
        terrain: TerrainKind::Land,
        height: HeightMm::new(180_000),
        ..Cell::default()
    })
}
fn edge(a: (u32, u32), b: (u32, u32), width: u32) -> ChannelInput {
    ChannelInput {
        from: GlobalCell { x: a.0, y: a.1 },
        to: GlobalCell { x: b.0, y: b.1 },
        from_width_dm: width,
        to_width_dm: width,
        discharge: 1000,
    }
}
fn pixel(rgb: &[u8], side: usize, x: usize, y: usize) -> [u8; 3] {
    rgb[(y * side + x) * 3..(y * side + x) * 3 + 3]
        .try_into()
        .unwrap()
}

fn lake(id: u32, at: CellCoord, depth: i32) -> Lake {
    Lake {
        global_id: arda_core::hydrology::BasinId(u64::from(id)),
        id,
        surface: HeightMm::new(depth),
        depth_mm: u32::try_from(depth).unwrap(),
        outlet: None,
        cells: vec![at],
    }
}

fn direct_pixel(shapes: &[Shape], x: u32, y: u32) -> [u8; 3] {
    let mut work = WorkBudget::new(MAX_COVERAGE_WORK);
    let select = |marker| {
        shapes.iter().filter_map(move |shape| {
            (shape.marker == marker).then_some((&shape.polygon, shape.discharge))
        })
    };
    let physical = coverage(select(false), x, y, &mut work).unwrap();
    let marker = coverage(select(true), x, y, &mut work).unwrap();
    let alpha = visual_alpha(physical.alpha, marker.alpha);
    let water = channel_colour(physical.maximum_discharge.max(marker.maximum_discharge));
    std::array::from_fn(|i| {
        u8::try_from(
            (u32::from(land_colour(180_000)[i]) * (65_535 - alpha)
                + u32::from(water[i]) * alpha
                + 32_767)
                / 65_535,
        )
        .unwrap()
    })
}

#[test]
fn scanline_sweep_matches_direct_geometry_across_rows_and_at_32k_edges() {
    let cells = land();
    let origin = GlobalCell { x: 0, y: 0 };
    let inputs = [
        edge((1, 0), (1, 1), 8),
        edge((1, 1), (2, 2), 100),
        edge((511, 511), (511, 511), 2000),
    ];
    for side in [512, 513, 4096, 8192, 32_768] {
        let scale = AreaImageScale::Custom(ImageQuality::new(side).unwrap());
        let geometry = shapes(inputs, origin, scale).unwrap();
        let mut raster = AreaRaster::new(&cells, inputs.map(Ok), origin, scale, &[]).unwrap();
        let boundary = side / 512;
        let rows = [boundary - 1, boundary, boundary + 1, side - 2, side - 1];
        let xs = [0, side / 512, 3 * side / 1024, 2 * side / 512, side - 1];
        for y in rows {
            let rgb = raster.row(y as usize).unwrap();
            for x in xs {
                let offset = x as usize * 3;
                assert_eq!(
                    &rgb[offset..offset + 3],
                    &direct_pixel(&geometry, x, y),
                    "side={side}, pixel=({x}, {y})"
                );
            }
        }
        assert_eq!(raster.rgb.len(), side as usize * 3);
        assert_eq!(raster.base_row.len(), side as usize * 3);
        assert_eq!(raster.pixels.len(), side as usize);
        assert_eq!(raster.starts.len(), geometry.len());
        assert_eq!(raster.ends.len(), geometry.len());
        assert!(raster.pixel_capacity <= MAX_PIXEL_CAPACITY);
        assert_ne!(
            &raster.rgb[raster.rgb.len() - 3..],
            &land_colour(180_000),
            "the last pixel must retain physical halo coverage"
        );
        assert!(raster.row(side as usize).is_err());
    }
}

#[test]
fn arbitrary_sizes_reach_all_terrain_cells_and_preserve_categorical_water() {
    let mut cells = land();
    for (x, y, terrain) in [(1, 1, TerrainKind::Sea), (511, 511, TerrainKind::Lake)] {
        cells.set(
            CellCoord::new(x, y).unwrap(),
            Cell {
                terrain,
                ..Cell::default()
            },
        );
    }
    let inputs = [edge((511, 511), (511, 511), 2000)];
    let origin = GlobalCell { x: 0, y: 0 };
    for side in [513, 777, 8191, 32_767, 32_768] {
        let scale = AreaImageScale::Custom(ImageQuality::new(side).unwrap());
        let mut raster = AreaRaster::new(&cells, inputs.map(Ok), origin, scale, &[]).unwrap();
        let sea_pixel = (side as usize).div_ceil(512);
        let row = raster.row(sea_pixel).unwrap();
        assert_eq!(&row[sea_pixel * 3..sea_pixel * 3 + 3], &sea_colour(0));
        let last = (side - 1) as usize;
        let row = raster.row(last).unwrap();
        assert_eq!(&row[last * 3..last * 3 + 3], &LAKE_FILL);
    }
}

#[test]
fn custom_detail_geometry_matches_legacy_and_large_coordinates_are_checked() {
    let inputs = [edge((10, 10), (11, 11), 10), edge((11, 11), (11, 11), 20)];
    let origin = GlobalCell { x: 0, y: 0 };
    let detail = shapes(inputs, origin, AreaImageScale::Detail).unwrap();
    let custom = shapes(
        inputs,
        origin,
        AreaImageScale::Custom(ImageQuality::new(4096).unwrap()),
    )
    .unwrap();
    assert_eq!(detail.len(), custom.len());
    for (a, b) in detail.iter().zip(&custom) {
        assert_eq!(a.polygon, b.polygon);
        assert_eq!(a.bounds, b.bounds);
        assert!(!b.marker);
    }
    let max_scale = AreaImageScale::Custom(ImageQuality::new(32_768).unwrap());
    let far = edge((u32::MAX, u32::MAX), (u32::MAX, u32::MAX), u32::MAX);
    assert!(shapes([far], origin, max_scale).is_err());
    let local = edge((u32::MAX, u32::MAX), (u32::MAX, u32::MAX), 8);
    let translated = shapes(
        [local],
        GlobalCell {
            x: u32::MAX - 511,
            y: u32::MAX - 511,
        },
        max_scale,
    )
    .unwrap();
    assert_eq!(translated.len(), 1);
    assert_eq!(coverage_work_limit(4096), MAX_COVERAGE_WORK);
    assert_eq!(coverage_work_limit(32_768), MAX_COVERAGE_WORK * 64);
}

#[test]
fn supplied_depth_changes_only_saved_lake_pixels_at_both_scales() {
    let mut cells = land();
    let positions = [(0, 0, 13), (10, 10, 28), (511, 511, 3000)];
    let lakes: Vec<_> = positions
        .iter()
        .enumerate()
        .map(|(i, &(x, y, depth))| {
            let at = CellCoord::new(x, y).unwrap();
            cells.set(
                at,
                Cell {
                    terrain: TerrainKind::Lake,
                    height: HeightMm::new(0),
                    ..Cell::default()
                },
            );
            lake(u32::try_from(i + 1).unwrap(), at, depth)
        })
        .collect();
    let saved = lakes.clone();
    let inputs = [edge((10, 11), (10, 10), 141), edge((10, 10), (10, 9), 141)];
    let origin = GlobalCell { x: 0, y: 0 };
    for scale in [AreaImageScale::Preview, AreaImageScale::Detail] {
        let old = raster_results(&cells, inputs.into_iter().map(Ok), origin, scale, &[]).unwrap();
        let new =
            raster_results(&cells, inputs.into_iter().map(Ok), origin, scale, &lakes).unwrap();
        let side = scale.side() as usize;
        let ppc = side / 512;
        let mut changed = 0;
        for (i, (a, b)) in old
            .as_chunks::<3>()
            .0
            .iter()
            .zip(new.as_chunks::<3>().0)
            .enumerate()
        {
            let x = i % side / ppc;
            let y = i / side / ppc;
            if let Some(&(_, _, depth)) = positions
                .iter()
                .find(|&&(lx, ly, _)| usize::from(lx) == x && usize::from(ly) == y)
            {
                assert_eq!(*a, LAKE_FILL);
                assert_eq!(*b, lake_colour(u32::try_from(depth).unwrap()));
                assert_ne!(a, b);
                changed += 1;
            } else {
                assert_eq!(a, b, "nonlake pixel changed at {i}");
            }
        }
        assert_eq!(changed, 3 * ppc * ppc);
    }
    assert_eq!(lakes, saved);
    let mut reversed = lakes.clone();
    reversed.reverse();
    assert_eq!(
        lake_colours(&cells, &lakes).unwrap(),
        lake_colours(&cells, &reversed).unwrap()
    );
}

#[test]
fn supplied_lake_surface_and_membership_are_checked_before_raster() {
    let mut cells = land();
    let at = CellCoord::new(1, 1).unwrap();
    let mut record = lake(1, at, 1);
    assert!(lake_colours(&cells, &[record.clone()]).is_err());
    cells.set(
        at,
        Cell {
            terrain: TerrainKind::Lake,
            height: HeightMm::new(0),
            ..Cell::default()
        },
    );
    assert!(lake_colours(&cells, &[record.clone()]).is_ok());
    record.surface = HeightMm::new(0);
    assert!(lake_colours(&cells, &[record.clone()]).is_err());
    record.surface = HeightMm::new(-1);
    assert!(lake_colours(&cells, &[record.clone()]).is_err());
    record.surface = HeightMm::new(1);
    assert!(lake_colours(&cells, &[record.clone(), record.clone()]).is_err());
    record.cells = vec![at; 512 * 512 + 1];
    assert!(lake_colours(&cells, &[record]).is_err());
    assert!(lake_colours(&cells, &[]).unwrap().is_empty());
}

#[test]
fn terminal_point_halo_coverage_preview_and_union_are_canonical() {
    let point = edge((511, 10), (511, 10), 2000);
    let origin = GlobalCell { x: 512, y: 0 };
    let rgb = raster(&land(), [point], origin, AreaImageScale::Preview).unwrap();
    assert_ne!(pixel(&rgb, 512, 0, 10), land_colour(180_000));
    assert_eq!(pixel(&rgb, 512, 1, 10), land_colour(180_000));
    let duplicated = raster(&land(), [point, point], origin, AreaImageScale::Preview).unwrap();
    assert_eq!(rgb, duplicated);
    // Translation to the same local center preserves every physical coverage bit.
    let moved = edge((0, 10), (0, 10), 2000);
    let left = shapes(
        [point],
        GlobalCell { x: 511, y: 0 },
        AreaImageScale::Preview,
    )
    .unwrap();
    let right = shapes([moved], GlobalCell { x: 0, y: 0 }, AreaImageScale::Preview).unwrap();
    assert_eq!(left[0].polygon, right[0].polygon);
    let small = edge((10, 10), (10, 10), 8);
    let preview = shapes([small], GlobalCell { x: 0, y: 0 }, AreaImageScale::Preview).unwrap();
    assert_eq!(preview.len(), 2);
    assert_eq!(preview.iter().filter(|p| p.marker).count(), 1);
    let detail = shapes([small], GlobalCell { x: 0, y: 0 }, AreaImageScale::Detail).unwrap();
    assert_eq!(detail.len(), 1);
    assert!(!detail[0].marker);
    for terrain in [TerrainKind::Sea, TerrainKind::Lake] {
        let cells = AreaCells::flat(Cell {
            terrain,
            ..Cell::default()
        });
        let plain = raster(&cells, [], origin, AreaImageScale::Preview).unwrap();
        assert_eq!(
            raster(&cells, [point], origin, AreaImageScale::Preview).unwrap(),
            plain
        );
    }
    assert!(raster_results(
        &land(),
        [Err(invalid("input failure"))],
        origin,
        AreaImageScale::Preview,
        &[]
    )
    .is_err());
}

#[test]
fn preview_mark_is_faint_separate_and_bounded() {
    assert_eq!(visual_alpha(0, 0), 0);
    assert_eq!(visual_alpha(0, 65_535), 11_822);
    assert_eq!(visual_alpha(65_535, 65_535), 65_535);
    let inputs = [edge((10, 10), (11, 10), 10)];
    let preview = raster(
        &land(),
        inputs,
        GlobalCell { x: 0, y: 0 },
        AreaImageScale::Preview,
    )
    .unwrap();
    let detail_shapes = shapes(inputs, GlobalCell { x: 0, y: 0 }, AreaImageScale::Detail).unwrap();
    assert!(detail_shapes.iter().all(|s| !s.marker));
    let colour = pixel(&preview, 512, 10, 10);
    assert_ne!(colour, land_colour(180_000));
    assert_ne!(colour, channel_colour(1000));
    assert_eq!(pixel(&preview, 512, 10, 11), land_colour(180_000));
}

#[test]
fn saved_water_classification_wins_over_channel_geometry() {
    for terrain in [TerrainKind::Sea, TerrainKind::Lake] {
        let mut cells = land();
        let at = CellCoord::new(10, 10).unwrap();
        cells.set(
            at,
            Cell {
                terrain,
                height: HeightMm::new(-2000),
                ..Cell::default()
            },
        );
        let rgb = raster(
            &cells,
            [edge((10, 10), (11, 10), 230_650)],
            GlobalCell { x: 0, y: 0 },
            AreaImageScale::Preview,
        )
        .unwrap();
        assert_eq!(
            pixel(&rgb, 512, 10, 10),
            if terrain == TerrainKind::Sea {
                sea_colour(-2000)
            } else {
                [132, 176, 205]
            }
        );
    }
}

#[test]
fn halo_edge_outside_the_area_can_cover_its_pixels() {
    let rgb = raster(
        &land(),
        [edge((511, 10), (511, 11), 2000)],
        GlobalCell { x: 512, y: 0 },
        AreaImageScale::Preview,
    )
    .unwrap();
    assert_ne!(pixel(&rgb, 512, 0, 10), land_colour(180_000));
    assert_eq!(pixel(&rgb, 512, 2, 10), land_colour(180_000));
}

#[test]
fn reversed_input_order_and_exact_duplicates_do_not_change_raster() {
    let a = edge((10, 10), (11, 10), 10);
    let b = edge((11, 10), (12, 11), 20);
    let run = |edges| {
        raster(
            &land(),
            edges,
            GlobalCell { x: 0, y: 0 },
            AreaImageScale::Preview,
        )
        .unwrap()
    };
    assert_eq!(run(vec![a, b]), run(vec![b, a, a]));
    let mut conflicting = a;
    conflicting.discharge += 1;
    assert!(raster(
        &land(),
        [a, conflicting],
        GlobalCell { x: 0, y: 0 },
        AreaImageScale::Preview
    )
    .is_err());
}
#[test]
fn wide_halo_candidates_over_water_still_consume_work_budget() {
    for terrain in [TerrainKind::Sea, TerrainKind::Lake] {
        let cells = AreaCells::flat(Cell {
            terrain,
            ..Cell::default()
        });
        let edges = [edge((255, 255), (256, 256), 200_000)];
        let origin = GlobalCell { x: 0, y: 0 };
        assert!(raster_limited(&cells, edges, origin, AreaImageScale::Preview, 1000).is_err());
        assert!(raster_limited(
            &cells,
            edges,
            origin,
            AreaImageScale::Preview,
            MAX_COVERAGE_WORK
        )
        .is_ok());
    }
}
