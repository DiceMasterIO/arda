use super::rivers::{atlas_river_edge_colour, atlas_river_radius, atlas_river_water_colour};
use super::*;
use crate::atlas::axis_kernel;
use crate::carto::{land_colour, river_band_colour, LAKE_FILL};
use crate::{AtlasHalo, AtlasNeighbor, AtlasTerrain};
use arda_core::{Cell, DischargeMilli, HeightMm};
use arda_core::{CellCoord, TerrainKind, AREA_CELLS};

fn standalone_atlas(cells: &AreaCells) -> AtlasTerrain {
    let mut halo = AtlasHalo::new();
    for direction in [
        AtlasNeighbor::North,
        AtlasNeighbor::NorthEast,
        AtlasNeighbor::East,
        AtlasNeighbor::SouthEast,
        AtlasNeighbor::South,
        AtlasNeighbor::SouthWest,
        AtlasNeighbor::West,
        AtlasNeighbor::NorthWest,
    ] {
        halo.mark_world_edge(direction).unwrap();
    }
    AtlasTerrain::new(cells, halo).unwrap()
}

#[test]
fn atlas_overview_mixed_axes_and_exact_centers() {
    let mut cells = AreaCells::flat(Cell {
        terrain: TerrainKind::Land,
        ..Cell::default()
    });
    for y in 0..AREA_CELLS {
        for x in 0..AREA_CELLS {
            cells.set(
                CellCoord::new(x, y).unwrap(),
                Cell {
                    height: HeightMm::new(i32::from(x) * 2_000 + i32::from(y) * 3_000),
                    terrain: TerrainKind::Land,
                    ..Cell::default()
                },
            );
        }
    }
    let terrain = standalone_atlas(&cells);
    let mut mixed = OverviewRaster::new_exact(1, 1, 511, 513).unwrap();
    mixed
        .push_atlas(AreaCoord::new(0, 0), &cells, &terrain)
        .unwrap();
    for y in [0, 1, 255, 512] {
        for x in [0, 255, 510] {
            let pixel = usize::try_from((y * 511 + x) * 3).unwrap();
            assert_eq!(
                &mixed.rgb[pixel..pixel + 3],
                &terrain
                    .sample(
                        crate::atlas::axis_kernel(x, 511).unwrap(),
                        crate::atlas::axis_kernel(y, 513).unwrap(),
                        TerrainKind::Land
                    )
                    .unwrap()
            );
        }
    }
    let mut enlarged = OverviewRaster::new_exact(1, 1, 1536, 1536).unwrap();
    enlarged
        .push_atlas(AreaCoord::new(0, 0), &cells, &terrain)
        .unwrap();
    for (x, y) in [(0, 0), (255, 255), (511, 511)] {
        let pixel = ((y * 3 + 1) * 1536 + (x * 3 + 1)) * 3;
        assert_eq!(
            &enlarged.rgb[pixel..pixel + 3],
            &terrain.colour(
                CellCoord::new(u16::try_from(x).unwrap(), u16::try_from(y).unwrap()).unwrap()
            )
        );
    }
}

#[test]
fn atlas_overview_resolves_linear_shore_and_propagates_context_errors() {
    let mut cells = AreaCells::flat(Cell {
        height: HeightMm::new(-2000),
        terrain: TerrainKind::Sea,
        ..Cell::default()
    });
    for y in 0..AREA_CELLS {
        for x in 256..AREA_CELLS {
            cells.set(
                CellCoord::new(x, y).unwrap(),
                Cell {
                    height: HeightMm::new(200_000),
                    terrain: TerrainKind::Land,
                    ..Cell::default()
                },
            );
        }
    }
    let terrain = standalone_atlas(&cells);
    let mut classic = OverviewRaster::new_exact(1, 1, 513, 513).unwrap();
    classic.push(AreaCoord::new(0, 0), &cells).unwrap();
    let mut atlas = OverviewRaster::new_exact(1, 1, 513, 513).unwrap();
    atlas
        .push_atlas(AreaCoord::new(0, 0), &cells, &terrain)
        .unwrap();
    assert_eq!(classic.features[256 * 513 + 255], Feature::Sea);
    for x in [255, 256, 257] {
        let owner = CellCoord::new(u16::try_from(x * 512 / 513).unwrap(), 255).unwrap();
        let x_kernel = crate::atlas::axis_kernel(u32::try_from(x).unwrap(), 513).unwrap();
        let y_kernel = crate::atlas::axis_kernel(256, 513).unwrap();
        let class = terrain
            .contour_class(x_kernel, y_kernel, owner, cells.get(owner).terrain)
            .unwrap();
        let i = (256 * 513 + x) * 3;
        assert_eq!(
            atlas.features[256 * 513 + x],
            if class == TerrainKind::Sea {
                Feature::AtlasSea
            } else {
                Feature::AtlasLand
            }
        );
        assert_eq!(
            &atlas.rgb[i..i + 3],
            &terrain.sample(x_kernel, y_kernel, class).unwrap()
        );
    }
    let wrong = AreaCells::flat(Cell {
        terrain: TerrainKind::Land,
        ..Cell::default()
    });
    let mut rejected = OverviewRaster::new_exact(1, 1, 512, 512).unwrap();
    assert!(matches!(
        rejected.push_atlas(AreaCoord::new(0, 0), &wrong, &terrain),
        Err(RenderError::AtlasContext { .. })
    ));
}

#[test]
fn linear_overview_and_area_use_the_same_shoreline_decision() {
    use crate::channels::{AreaImageScale, AreaRaster};
    use crate::{GlobalCell, ImageQuality};
    let mut cells = AreaCells::flat(Cell {
        terrain: TerrainKind::Sea,
        height: HeightMm::new(0),
        ..Cell::default()
    });
    for y in 0..512 {
        for x in 101..512 {
            cells.set(
                CellCoord::new(x, y).unwrap(),
                Cell {
                    terrain: TerrainKind::Land,
                    height: HeightMm::new(100_000),
                    ..Cell::default()
                },
            );
        }
    }
    let terrain = standalone_atlas(&cells);
    for side in [2048, 8192] {
        let py = 100 * (side / 512) + 1;
        let px = 101 * (side / 512) - 1;
        let (overview_colour, feature) =
            sample_pixel(&cells, Some(&terrain), side, side, px, py).unwrap();
        assert_eq!(feature, Feature::AtlasLand);
        let mut area = AreaRaster::new_with_terrain(
            &cells,
            Some(&terrain),
            std::iter::empty(),
            GlobalCell { x: 0, y: 0 },
            AreaImageScale::Custom(ImageQuality::new(side).unwrap()),
            &[],
        )
        .unwrap();
        let row = area.row(usize::try_from(py).unwrap()).unwrap();
        let start = usize::try_from(px).unwrap() * 3;
        assert_eq!(&row[start..start + 3], &overview_colour);
    }
}

#[test]
fn incremental_order_and_batch_wrapper_have_identical_bytes() {
    let dry = AreaCells::flat(Cell {
        height: HeightMm::new(321_000),
        terrain: TerrainKind::Land,
        ..Cell::default()
    });
    let river = AreaCells::flat(Cell {
        discharge: DischargeMilli::new(80_000),
        terrain: TerrainKind::Land,
        ..Cell::default()
    });
    let mut forward = OverviewRaster::new(2, 1, 48).unwrap();
    forward.push(AreaCoord::new(0, 0), &dry).unwrap();
    forward.push(AreaCoord::new(1, 0), &river).unwrap();
    let mut reverse = OverviewRaster::new(2, 1, 48).unwrap();
    reverse.push(AreaCoord::new(1, 0), &river).unwrap();
    reverse.push(AreaCoord::new(0, 0), &dry).unwrap();
    let bytes = forward.finish().unwrap();
    assert_eq!(bytes, reverse.finish().unwrap());
    assert_eq!(
        bytes,
        crate::carto::render_overview_png(&[(0, 0, &dry), (1, 0, &river)], 2, 1, 48).unwrap()
    );
}

#[test]
fn invalid_scale_budget_and_duplicate_area_are_typed_errors() {
    for (w, h, px) in [
        (0, 1, 48),
        (1, 1, 0),
        (79, 1, 48),
        (78, 78, 512),
        (1, 1, 513),
    ] {
        assert!(matches!(
            OverviewRaster::new(w, h, px),
            Err(RenderError::OverviewDimensions)
        ));
    }
    let mut canvas = OverviewRaster::new(1, 1, 48).unwrap();
    let cells = AreaCells::flat(Cell::default());
    canvas.push(AreaCoord::new(0, 0), &cells).unwrap();
    assert!(matches!(
        canvas.push(AreaCoord::new(0, 0), &cells),
        Err(RenderError::DuplicateOverviewArea { .. })
    ));
    assert!(matches!(
        canvas.push(AreaCoord::new(1, 0), &cells),
        Err(RenderError::OverviewDimensions)
    ));
}

#[test]
fn exact_dimensions_refuse_invalid_axes_and_pixel_budgets() {
    for (aw, ah, width, height) in [
        (0, 1, 1, 1),
        (1, -1, 1, 1),
        (79, 1, 79, 1),
        (1, 1, 0, 1),
        (1, 1, 1, 32_769),
        (1, 1, u32::MAX, 1),
        (3, 1, 2, 1),
        (1, 3, 1, 2),
        (1, 1, 16_384, 16_384),
    ] {
        assert!(matches!(
            OverviewRaster::new_exact(aw, ah, width, height),
            Err(RenderError::ExactOverviewDimensions)
        ));
    }
}

#[test]
fn exact_long_axis_supports_32k_within_the_buffer_budget() {
    let raster = OverviewRaster::new_exact(1, 1, 32_768, 1).unwrap();
    assert_eq!((raster.width, raster.height), (32_768, 1));
}

#[test]
fn exact_uneven_aspect_covers_every_pixel_without_area_gaps() {
    let mut canvas = OverviewRaster::new_exact(3, 2, 8, 5).unwrap();
    let heights = [0, 200_000, 500_000, 900_000, 1_400_000, 2_800_000];
    for (index, height) in heights.into_iter().enumerate().rev() {
        let cells = AreaCells::flat(Cell {
            height: HeightMm::new(height),
            terrain: TerrainKind::Land,
            ..Cell::default()
        });
        canvas
            .push(
                AreaCoord::new(
                    i32::try_from(index % 3).unwrap(),
                    i32::try_from(index / 3).unwrap(),
                ),
                &cells,
            )
            .unwrap();
    }
    let png = canvas.finish().unwrap();
    let mut reader = png::Decoder::new(png.as_slice()).read_info().unwrap();
    let mut rgb = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut rgb).unwrap();
    assert_eq!((info.width, info.height), (8, 5));
    // Three areas occupy widths 2/3/3; two rows occupy heights 2/3.
    let columns = [0, 0, 1, 1, 1, 2, 2, 2];
    let rows = [0, 0, 1, 1, 1];
    for (y, row) in rows.into_iter().enumerate() {
        for (x, column) in columns.into_iter().enumerate() {
            let pixel = (y * 8 + x) * 3;
            assert_eq!(
                &rgb[pixel..pixel + 3],
                &land_colour(heights[row * 3 + column])
            );
        }
    }
}

#[test]
fn atlas_lake_overview_matches_area_palette_with_mixed_axis_sampling() {
    let mut cells = AreaCells::flat(Cell {
        height: HeightMm::new(100_000),
        terrain: TerrainKind::Land,
        ..Cell::default()
    });
    let locations = [
        (CellCoord::new(100, 100).unwrap(), 190_000),
        (CellCoord::new(100, 101).unwrap(), 0),
    ];
    for &(at, height) in &locations {
        cells.set(
            at,
            Cell {
                height: HeightMm::new(height),
                terrain: TerrainKind::Lake,
                ..Cell::default()
            },
        );
    }
    let lake = arda_core::Lake {
        global_id: arda_core::hydrology::BasinId(1),
        id: 1,
        surface: HeightMm::new(200_000),
        depth_mm: 200_000,
        outlet: None,
        cells: locations.into_iter().map(|(at, _)| at).collect(),
    };
    let mut halo = AtlasHalo::new();
    for direction in [
        AtlasNeighbor::North,
        AtlasNeighbor::NorthEast,
        AtlasNeighbor::East,
        AtlasNeighbor::SouthEast,
        AtlasNeighbor::South,
        AtlasNeighbor::SouthWest,
        AtlasNeighbor::West,
        AtlasNeighbor::NorthWest,
    ] {
        halo.mark_world_edge(direction).unwrap();
    }
    let terrain = AtlasTerrain::new_with_lakes(&cells, &[lake], halo).unwrap();
    let (overview, feature) = sample_pixel(&cells, Some(&terrain), 512, 256, 100, 50).unwrap();
    assert_eq!(feature, Feature::Lake);
    assert_eq!(
        overview,
        terrain
            .sample(
                axis_kernel(100, 512).unwrap(),
                axis_kernel(50, 256).unwrap(),
                TerrainKind::Lake
            )
            .unwrap()
    );
    assert_eq!(overview, [59, 117, 141]);

    let mut objects = arda_core::AreaObjects::default();
    objects.lakes.push(arda_core::Lake {
        global_id: arda_core::hydrology::BasinId(1),
        id: 1,
        surface: HeightMm::new(200_000),
        depth_mm: 200_000,
        outlet: None,
        cells: locations.into_iter().map(|(at, _)| at).collect(),
    });
    let mut area_png = Vec::new();
    crate::render_area_png_to_atlas(
        &cells,
        &objects,
        arda_core::GlobalCell { x: 0, y: 0 },
        crate::AreaImageScale::Preview,
        &terrain,
        &mut area_png,
    )
    .unwrap();
    let decode = |png: &[u8]| {
        let mut reader = png::Decoder::new(png).read_info().unwrap();
        let mut rgb = vec![0; reader.output_buffer_size()];
        reader.next_frame(&mut rgb).unwrap();
        rgb
    };
    let area_rgb = decode(&area_png);
    assert_eq!(
        &area_rgb[(100 * 512 + 100) * 3..(100 * 512 + 100) * 3 + 3],
        &terrain.colour(locations[0].0)
    );

    let mut buffered = OverviewRaster::new_exact(1, 1, 512, 256).unwrap();
    buffered
        .push_atlas(AreaCoord::new(0, 0), &cells, &terrain)
        .unwrap();
    let buffered_rgb = decode(&buffered.finish().unwrap());
    let mut streamed_png = Vec::new();
    let mut payload = Some((cells, terrain));
    write_atlas_overview_png(1, 1, 512, 256, &mut streamed_png, |_| {
        Ok::<_, RenderError>(payload.take().unwrap())
    })
    .unwrap();
    let streamed_rgb = decode(&streamed_png);
    assert_eq!(streamed_rgb, buffered_rgb);
    assert_eq!(
        &streamed_rgb[(50 * 512 + 100) * 3..(50 * 512 + 100) * 3 + 3],
        &overview
    );
}

#[test]
fn exact_upscale_retains_last_source_row_and_column() {
    let mut cells = AreaCells::flat(Cell {
        height: HeightMm::new(200_000),
        terrain: TerrainKind::Land,
        ..Cell::default()
    });
    for offset in 0..AREA_CELLS {
        cells.set(
            CellCoord::new(AREA_CELLS - 1, offset).unwrap(),
            Cell {
                terrain: TerrainKind::Lake,
                ..Cell::default()
            },
        );
        cells.set(
            CellCoord::new(offset, AREA_CELLS - 1).unwrap(),
            Cell {
                terrain: TerrainKind::Sea,
                ..Cell::default()
            },
        );
    }
    let mut canvas = OverviewRaster::new_exact(1, 1, 513, 515).unwrap();
    canvas.push(AreaCoord::new(0, 0), &cells).unwrap();
    for y in 0..515 {
        for x in 0..513 {
            let want = if y == 514 {
                OVERVIEW_SEA
            } else if x == 512 {
                LAKE_FILL
            } else {
                land_colour(200_000)
            };
            let pixel = (y * 513 + x) * 3;
            assert_eq!(&canvas.rgb[pixel..pixel + 3], &want);
        }
    }
}

#[test]
fn exact_uniform_scale_preserves_existing_png_bytes() {
    let cells = AreaCells::flat(Cell {
        height: HeightMm::new(321_000),
        terrain: TerrainKind::Land,
        discharge: DischargeMilli::new(80_000),
        ..Cell::default()
    });
    let mut normal = OverviewRaster::new(2, 1, 48).unwrap();
    let mut exact = OverviewRaster::new_exact(2, 1, 96, 48).unwrap();
    for x in 0..2 {
        normal.push(AreaCoord::new(x, 0), &cells).unwrap();
        exact.push(AreaCoord::new(x, 0), &cells).unwrap();
    }
    assert_eq!(normal.finish().unwrap(), exact.finish().unwrap());
}

mod rivers;
