use super::*;
use crate::carto::{river_band_colour, RiverBand};
use crate::overview::OverviewRaster;
use crate::{AtlasHalo, AtlasNeighbor, AtlasTerrain};
use arda_core::{Cell, CellCoord, DischargeMilli, HeightMm, TerrainKind, AREA_CELLS};

fn area(at: AreaCoord) -> AreaCells {
    let mut cells = AreaCells::flat(Cell {
        height: HeightMm::new(200_000 + at.x * 300_000 + at.y * 700_000),
        terrain: TerrainKind::Land,
        ..Cell::default()
    });
    // Edge trunks exercise both area seams and the streaming band seam.
    for y in 0..AREA_CELLS {
        for x in 0..AREA_CELLS {
            if x == AREA_CELLS - 1 || y == 255 || y == AREA_CELLS - 1 {
                cells.set(
                    CellCoord::new(x, y).unwrap(),
                    Cell {
                        terrain: TerrainKind::Land,
                        discharge: DischargeMilli::new(800_000),
                        ..Cell::default()
                    },
                );
            } else if (x / 19 + y / 23) % 11 == 0 {
                cells.set(
                    CellCoord::new(x, y).unwrap(),
                    Cell {
                        terrain: TerrainKind::Lake,
                        ..Cell::default()
                    },
                );
            }
        }
    }
    cells
}

fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut reader = png::Decoder::new(bytes).read_info().unwrap();
    let mut rgb = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut rgb).unwrap();
    (frame.width, frame.height, rgb)
}

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
fn atlas_streaming_matches_unequal_buffered_partitions_and_push_order() {
    for (width, height) in [(1541, 1031), (1537, 513)] {
        let mut expected = None;
        for reverse in [false, true] {
            let mut raster = OverviewRaster::new_exact(3, 2, width, height).unwrap();
            let mut coords = (0..2)
                .flat_map(|y| (0..3).map(move |x| AreaCoord::new(x, y)))
                .collect::<Vec<_>>();
            if reverse {
                coords.reverse();
            }
            for at in coords {
                let cells = area(at);
                let terrain = standalone_atlas(&cells);
                raster.push_atlas(at, &cells, &terrain).unwrap();
            }
            let pixels = decode(&raster.finish().unwrap());
            if let Some(ref previous) = expected {
                assert_eq!(&pixels, previous);
            }
            expected = Some(pixels);
        }
        let mut actual = Vec::new();
        write_atlas_overview_png(3, 2, width, height, &mut actual, |at| {
            let cells = area(at);
            let terrain = standalone_atlas(&cells);
            Ok::<_, RenderError>((cells, terrain))
        })
        .unwrap();
        assert_eq!(decode(&actual), expected.unwrap());
    }
}

#[test]
fn streaming_matches_buffered_pixels_across_area_and_band_seams() {
    for (areas_wide, areas_high, width, height) in [
        (3, 2, 19, 13),
        (1, 1, 512, 512),
        (1, 1, 513, 515),
        (3, 2, 1541, 1031),
    ] {
        let mut raster = OverviewRaster::new_exact(areas_wide, areas_high, width, height).unwrap();
        for y in 0..areas_high {
            for x in 0..areas_wide {
                let at = AreaCoord::new(x, y);
                raster.push(at, &area(at)).unwrap();
            }
        }
        let expected = decode(&raster.finish().unwrap());
        let mut actual = Vec::new();
        write_overview_png(areas_wide, areas_high, width, height, &mut actual, |at| {
            Ok::<_, RenderError>(area(at))
        })
        .unwrap();
        assert_eq!(decode(&actual), expected);
    }
}

#[test]
fn mixed_style_trunk_widening_protects_atlas_sea_across_area_and_band_seams() {
    let mut trunk = AreaCells::flat(Cell {
        terrain: TerrainKind::Land,
        height: HeightMm::new(100_000),
        ..Cell::default()
    });
    for y in 0..512 {
        trunk.set(
            CellCoord::new(511, y).unwrap(),
            Cell {
                terrain: TerrainKind::Land,
                height: HeightMm::new(100_000),
                discharge: DischargeMilli::new(800_000),
                ..Cell::default()
            },
        );
    }
    let sea = AreaCells::flat(Cell {
        terrain: TerrainKind::Sea,
        height: HeightMm::new(-100_000),
        ..Cell::default()
    });
    for atlas_sea in [false, true] {
        let mut buffered = OverviewRaster::new_exact(2, 1, 1024, 512).unwrap();
        buffered.push(AreaCoord::new(0, 0), &trunk).unwrap();
        if atlas_sea {
            buffered
                .push_atlas(AreaCoord::new(1, 0), &sea, &standalone_atlas(&sea))
                .unwrap();
        } else {
            buffered.push(AreaCoord::new(1, 0), &sea).unwrap();
        }
        let expected = decode(&buffered.finish().unwrap());
        let mut encoded = Vec::new();
        write_overview_png_inner(
            2,
            1,
            1024,
            512,
            &mut encoded,
            (ChannelStyle::Legacy, None),
            |at| {
                if at.x == 0 {
                    Ok::<_, RenderError>((trunk.clone(), None, None))
                } else {
                    Ok::<_, RenderError>((
                        sea.clone(),
                        atlas_sea.then(|| standalone_atlas(&sea)),
                        None,
                    ))
                }
            },
        )
        .unwrap();
        let actual = decode(&encoded);
        assert_eq!(actual, expected);
        for y in [255, 256] {
            let p = (y * 1024 + 512) * 3;
            let want = if atlas_sea {
                standalone_atlas(&sea)
                    .sample(
                        crate::atlas::axis_kernel(0, 512).unwrap(),
                        crate::atlas::axis_kernel(u32::try_from(y).unwrap(), 512).unwrap(),
                        TerrainKind::Sea,
                    )
                    .unwrap()
            } else {
                river_band_colour(RiverBand::Dark)
            };
            assert_eq!(&actual.2[p..p + 3], &want);
        }
    }
}

#[test]
fn streaming_accepts_32k_square_without_a_full_image_allocation() {
    assert!(validate_exact_dimensions(78, 78, 32_768, 32_768).is_ok());
    assert!(validate_exact_dimensions(1, 1, 32_768, 32_768).is_ok());
    assert_eq!(band_height(32_768), 256);
    assert_eq!(band_height(7), 7);
    let band = RasterBand::new(32_768, 32_768).unwrap();
    assert_eq!(band.rgb.len(), (256 + 28) * 32_768 * 3);
    assert_eq!(band.features.len(), (256 + 28) * 32_768);
}

#[test]
fn streaming_validates_before_loading_and_propagates_load_errors() {
    let mut loaded = false;
    let error = write_overview_png(1, 1, 32_769, 1, Vec::new(), |_| {
        loaded = true;
        Ok::<_, RenderError>(AreaCells::flat(Cell::default()))
    });
    assert!(matches!(error, Err(RenderError::ExactOverviewDimensions)));
    assert!(!loaded);
    let error = write_overview_png(1, 1, 1, 1, Vec::new(), |_| {
        Err::<AreaCells, _>(RenderError::PartialWorld)
    });
    assert!(matches!(error, Err(RenderError::PartialWorld)));

    let mut loaded = false;
    let invalid = write_atlas_overview_png(1, 1, 32_769, 1, Vec::new(), |_| {
        loaded = true;
        Err::<(AreaCells, AtlasTerrain), _>(RenderError::PartialWorld)
    });
    assert!(matches!(invalid, Err(RenderError::ExactOverviewDimensions)));
    assert!(!loaded);
    let missing = write_atlas_overview_png(1, 1, 1, 1, Vec::new(), |_| {
        Err::<(AreaCells, AtlasTerrain), _>(RenderError::PartialWorld)
    });
    assert!(matches!(missing, Err(RenderError::PartialWorld)));
    let sea = AreaCells::flat(Cell {
        terrain: TerrainKind::Sea,
        ..Cell::default()
    });
    let wrong = AreaCells::flat(Cell {
        terrain: TerrainKind::Land,
        ..Cell::default()
    });
    let mismatch = write_atlas_overview_png(1, 1, 512, 512, Vec::new(), |_| {
        Ok::<_, RenderError>((wrong.clone(), standalone_atlas(&sea)))
    });
    assert!(matches!(mismatch, Err(RenderError::AtlasContext { .. })));
}
