#![allow(clippy::unwrap_used)]

use super::*;
use arda_core::{Cell, HeightMm};

const DIRECTIONS: [AtlasNeighbor; 8] = [
    AtlasNeighbor::North,
    AtlasNeighbor::NorthEast,
    AtlasNeighbor::East,
    AtlasNeighbor::SouthEast,
    AtlasNeighbor::South,
    AtlasNeighbor::SouthWest,
    AtlasNeighbor::West,
    AtlasNeighbor::NorthWest,
];

fn cell(height_mm: i32, class: TerrainKind) -> Cell {
    Cell {
        height: HeightMm::new(height_mm),
        terrain: class,
        ..Cell::default()
    }
}

fn filled(height_mm: i32, class: TerrainKind) -> AreaCells {
    AreaCells::flat(cell(height_mm, class))
}

fn edge_halo() -> AtlasHalo {
    let mut halo = AtlasHalo::new();
    for direction in DIRECTIONS {
        halo.mark_world_edge(direction).unwrap();
    }
    halo
}

fn copied_halo(cells: &AreaCells) -> AtlasHalo {
    let mut halo = AtlasHalo::new();
    for direction in DIRECTIONS {
        halo.copy_neighbor(direction, cells).unwrap();
    }
    halo
}

fn planar_area(area_x: i32, area_y: i32) -> AreaCells {
    let mut area = filled(0, TerrainKind::Land);
    for y in 0..512 {
        for x in 0..512 {
            let global_x = area_x * 512 + i32::from(x);
            let global_y = area_y * 512 + i32::from(y);
            let at = CellCoord::new(x, y).unwrap();
            area.set(
                at,
                cell(
                    500_000 + 1_000 * global_x + 2_000 * global_y,
                    TerrainKind::Land,
                ),
            );
        }
    }
    area
}

fn planar_halo() -> AtlasHalo {
    let mut halo = AtlasHalo::new();
    let offsets = [
        (0, -1),
        (1, -1),
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
    ];
    for (direction, (x, y)) in DIRECTIONS.into_iter().zip(offsets) {
        halo.copy_neighbor(direction, &planar_area(x, y)).unwrap();
    }
    halo
}

fn terrain_test_context(cells: &AreaCells, mut halo: AtlasHalo) -> Vec<Option<SourceSample>> {
    for y in 0..512 {
        for x in 0..512 {
            let at = CellCoord::new(x, y).unwrap();
            halo.context
                [context_index(i16::try_from(x).unwrap(), i16::try_from(y).unwrap()).unwrap()] =
                Some(SourceSample {
                    height_mm: cells.get(at).height.raw(),
                    class: cells.get(at).terrain,
                    wetness: cells.get(at).wetness,
                    lake_depth_mm: None,
                });
        }
    }
    halo.context
}

fn record(terrain: &AtlasTerrain, x: i16, y: i16) -> ([u8; 3], u16, TerrainKind) {
    let index = sample_index(x, y).unwrap();
    (
        terrain.palette[index],
        terrain.light[index],
        terrain.classes[index],
    )
}

#[test]
fn exact_axis_kernels_preserve_cell_centres() {
    assert_eq!(
        axis_kernel(0, 512).unwrap(),
        AxisKernel::Linear {
            low: 0,
            high_weight: 0,
            denominator: 1024
        }
    );
    assert_eq!(
        axis_kernel(511, 512).unwrap(),
        AxisKernel::Linear {
            low: 511,
            high_weight: 0,
            denominator: 1024
        }
    );
    for (pixel, low) in [(1, 0), (4, 1), (1534, 511)] {
        assert_eq!(
            axis_kernel(pixel, 1536).unwrap(),
            AxisKernel::Linear {
                low,
                high_weight: 0,
                denominator: 3072
            }
        );
    }
    assert_eq!(
        axis_kernel(0, 513).unwrap(),
        AxisKernel::Linear {
            low: -1,
            high_weight: 1025,
            denominator: 1026
        }
    );
    assert_eq!(
        axis_kernel(512, 513).unwrap(),
        AxisKernel::Linear {
            low: 511,
            high_weight: 1,
            denominator: 1026
        }
    );
    assert_eq!(
        axis_kernel(0, 511).unwrap(),
        AxisKernel::Box { start: 0, end: 1 }
    );
    assert_eq!(
        axis_kernel(510, 511).unwrap(),
        AxisKernel::Box {
            start: 510,
            end: 512
        }
    );
    for result in [
        axis_kernel(0, 0),
        axis_kernel(0, 32769),
        axis_kernel(512, 512),
    ] {
        assert!(matches!(result, Err(RenderError::AtlasContext { .. })));
    }
}

#[test]
fn kernels_include_the_categorical_owner_with_positive_weight() {
    for width in [1, 2, 7, 511, 512, 513, 1024, 1536, 32768] {
        for pixel in 0..width {
            let owner = pixel * 512 / width;
            match axis_kernel(pixel, width).unwrap() {
                AxisKernel::Box { start, end } => {
                    assert!(start < end && end <= 512);
                    assert!(u32::from(start) <= owner && owner < u32::from(end));
                }
                AxisKernel::Linear {
                    low,
                    high_weight,
                    denominator,
                } => {
                    assert!((-1..=511).contains(&low));
                    assert!(high_weight < denominator);
                    let low_weight = denominator - high_weight;
                    let owner_weight = if i32::from(low) == i32::try_from(owner).unwrap() {
                        low_weight
                    } else if i32::from(low) + 1 == i32::try_from(owner).unwrap() {
                        high_weight
                    } else {
                        0
                    };
                    assert!(owner_weight > 0, "width={width}, pixel={pixel}");
                }
            }
        }
    }
}

#[test]
fn halo_copies_two_cells_in_all_eight_directions() {
    let mut halo = AtlasHalo::new();
    let markers = [
        [
            (0, 510, 0, -2),
            (0, 511, 0, -1),
            (511, 510, 511, -2),
            (511, 511, 511, -1),
        ],
        [
            (0, 510, 512, -2),
            (1, 510, 513, -2),
            (0, 511, 512, -1),
            (1, 511, 513, -1),
        ],
        [
            (0, 0, 512, 0),
            (1, 0, 513, 0),
            (0, 511, 512, 511),
            (1, 511, 513, 511),
        ],
        [
            (0, 0, 512, 512),
            (1, 0, 513, 512),
            (0, 1, 512, 513),
            (1, 1, 513, 513),
        ],
        [
            (0, 0, 0, 512),
            (0, 1, 0, 513),
            (511, 0, 511, 512),
            (511, 1, 511, 513),
        ],
        [
            (510, 0, -2, 512),
            (511, 0, -1, 512),
            (510, 1, -2, 513),
            (511, 1, -1, 513),
        ],
        [
            (510, 0, -2, 0),
            (511, 0, -1, 0),
            (510, 511, -2, 511),
            (511, 511, -1, 511),
        ],
        [
            (510, 510, -2, -2),
            (511, 510, -1, -2),
            (510, 511, -2, -1),
            (511, 511, -1, -1),
        ],
    ];
    for (i, direction) in DIRECTIONS.into_iter().enumerate() {
        let base = (i32::try_from(i).unwrap() + 1) * 1000;
        let mut neighbor = filled(base, TerrainKind::Land);
        for (marker, (source_x, source_y, _, _)) in markers[i].iter().enumerate() {
            neighbor.set(
                CellCoord::new(*source_x, *source_y).unwrap(),
                cell(base + i32::try_from(marker).unwrap() + 1, TerrainKind::Land),
            );
        }
        halo.copy_neighbor(direction, &neighbor).unwrap();
        for (marker, (_, _, x, y)) in markers[i].iter().enumerate() {
            let got = halo.context[context_index(*x, *y).unwrap()].unwrap();
            assert_eq!(got.height_mm, base + i32::try_from(marker).unwrap() + 1);
        }
    }
    assert_eq!(halo.context[context_index(0, 0).unwrap()], None);
}

#[test]
fn halo_state_rejects_incomplete_duplicate_and_contradictory_directions() {
    let target = filled(500_000, TerrainKind::Land);
    assert!(matches!(
        AtlasTerrain::new(&target, AtlasHalo::new()),
        Err(RenderError::AtlasContext { .. })
    ));
    let mut halo = AtlasHalo::new();
    halo.copy_neighbor(AtlasNeighbor::North, &target).unwrap();
    assert!(matches!(
        halo.copy_neighbor(AtlasNeighbor::North, &target),
        Err(RenderError::AtlasContext { .. })
    ));
    assert!(matches!(
        halo.mark_world_edge(AtlasNeighbor::North),
        Err(RenderError::AtlasContext { .. })
    ));
    let mut halo = AtlasHalo::new();
    halo.mark_world_edge(AtlasNeighbor::North).unwrap();
    assert!(matches!(
        halo.mark_world_edge(AtlasNeighbor::North),
        Err(RenderError::AtlasContext { .. })
    ));
    assert!(matches!(
        halo.copy_neighbor(AtlasNeighbor::North, &target),
        Err(RenderError::AtlasContext { .. })
    ));
    assert!(AtlasTerrain::new(&target, edge_halo()).is_ok());
    for (diagonal, first, second) in [
        (
            AtlasNeighbor::NorthEast,
            AtlasNeighbor::North,
            AtlasNeighbor::East,
        ),
        (
            AtlasNeighbor::SouthEast,
            AtlasNeighbor::South,
            AtlasNeighbor::East,
        ),
        (
            AtlasNeighbor::SouthWest,
            AtlasNeighbor::South,
            AtlasNeighbor::West,
        ),
        (
            AtlasNeighbor::NorthWest,
            AtlasNeighbor::North,
            AtlasNeighbor::West,
        ),
    ] {
        for outside in [first, second] {
            let mut halo = copied_halo(&target);
            halo.states[outside.index()] = NeighborState::WorldEdge;
            for other_diagonal in [
                AtlasNeighbor::NorthEast,
                AtlasNeighbor::SouthEast,
                AtlasNeighbor::SouthWest,
                AtlasNeighbor::NorthWest,
            ] {
                let cardinals = match other_diagonal {
                    AtlasNeighbor::NorthEast => [AtlasNeighbor::North, AtlasNeighbor::East],
                    AtlasNeighbor::SouthEast => [AtlasNeighbor::South, AtlasNeighbor::East],
                    AtlasNeighbor::SouthWest => [AtlasNeighbor::South, AtlasNeighbor::West],
                    AtlasNeighbor::NorthWest => [AtlasNeighbor::North, AtlasNeighbor::West],
                    _ => unreachable!(),
                };
                if other_diagonal != diagonal && cardinals.contains(&outside) {
                    halo.states[other_diagonal.index()] = NeighborState::WorldEdge;
                }
            }
            assert!(matches!(
                AtlasTerrain::new(&target, halo),
                Err(RenderError::AtlasContext { .. })
            ));
        }
        let mut halo = copied_halo(&target);
        assert_eq!(halo.states[first.index()], NeighborState::Copied);
        assert_eq!(halo.states[second.index()], NeighborState::Copied);
        halo.states[diagonal.index()] = NeighborState::WorldEdge;
        assert!(matches!(
            AtlasTerrain::new(&target, halo),
            Err(RenderError::AtlasContext { .. })
        ));
    }
}

#[test]
fn planar_gradients_continue_at_edges_and_four_area_corner() {
    let cells = planar_area(0, 0);
    let context = terrain_test_context(&cells, planar_halo());
    let terrain = AtlasTerrain::new(&cells, planar_halo()).unwrap();
    for (x, y) in [
        (256, 256),
        (0, 256),
        (511, 256),
        (256, 0),
        (256, 511),
        (0, 0),
        (511, 0),
        (0, 511),
        (511, 511),
        (-1, -1),
        (512, 512),
    ] {
        let (dx, dy) = gradient_numerators(&context, x, y, [false; 4]).unwrap();
        assert_eq!((dx, dy), (2000, 4000), "at ({x},{y})");
        assert_eq!(record(&terrain, x, y).1, relief_light(2000, 4000));
    }
}

#[test]
fn outer_edges_clamp_each_axis_independently() {
    let shifted_plane = |area_x, area_y| {
        let mut area = planar_area(area_x, area_y);
        for y in 0..512 {
            for x in 0..512 {
                let at = CellCoord::new(x, y).unwrap();
                let height = area.get(at).height.raw();
                area.set(at, cell(height - 10_000, TerrainKind::Land));
            }
        }
        area
    };
    let target = shifted_plane(0, 0);
    let mut halo = AtlasHalo::new();
    for direction in DIRECTIONS {
        match direction {
            AtlasNeighbor::South | AtlasNeighbor::SouthEast | AtlasNeighbor::East => {
                let (x, y) = match direction {
                    AtlasNeighbor::South => (0, 1),
                    AtlasNeighbor::SouthEast => (1, 1),
                    _ => (1, 0),
                };
                halo.copy_neighbor(direction, &shifted_plane(x, y)).unwrap();
            }
            _ => halo.mark_world_edge(direction).unwrap(),
        }
    }
    let terrain = AtlasTerrain::new(&target, halo).unwrap();
    assert_eq!(record(&terrain, -1, 512), record(&terrain, 0, 512));
    // Palette quantization may make adjacent elevations identical, but the
    // south halo must still supply its distinct saved height at this corner.
    assert_eq!(terrain.heights[sample_index(-1, 512).unwrap()], 1_514_000);
    assert_eq!(terrain.heights[sample_index(0, 511).unwrap()], 1_512_000);
    assert_eq!(record(&terrain, 0, 512).1, relief_light(2000, 4000));
    assert_eq!(record(&terrain, 0, 0).1, relief_light(2000, 4000));
}

#[test]
fn diagonal_two_by_two_changes_only_its_corner_sampling() {
    let target = filled(500_000, TerrainKind::Land);
    let build = |changed: bool| {
        let mut halo = AtlasHalo::new();
        for direction in DIRECTIONS {
            let mut adjacent = target.clone();
            if changed && direction == AtlasNeighbor::NorthWest {
                adjacent.set(
                    CellCoord::new(511, 511).unwrap(),
                    cell(900_000, TerrainKind::Land),
                );
                adjacent.set(
                    CellCoord::new(510, 510).unwrap(),
                    cell(900_000, TerrainKind::Land),
                );
            }
            halo.copy_neighbor(direction, &adjacent).unwrap();
        }
        AtlasTerrain::new(&target, halo).unwrap()
    };
    let before = build(false);
    let after = build(true);
    let sample = |terrain: &AtlasTerrain, x, y| {
        terrain
            .sample(
                axis_kernel(x, 1024).unwrap(),
                axis_kernel(y, 1024).unwrap(),
                TerrainKind::Land,
            )
            .unwrap()
    };
    assert_ne!(sample(&before, 0, 0), sample(&after, 0, 0));
    for (x, y) in [(1023, 0), (0, 1023), (1023, 1023)] {
        assert_eq!(sample(&before, x, y), sample(&after, x, y));
    }
}

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

fn fixture() -> AtlasTerrain {
    AtlasTerrain {
        palette: vec![[0; 3]; SAMPLE_SIDE * SAMPLE_SIDE],
        light: vec![LIGHT_ONE; SAMPLE_SIDE * SAMPLE_SIDE],
        classes: vec![TerrainKind::Sea; SAMPLE_SIDE * SAMPLE_SIDE],
        heights: vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
        has_lake_depths: false,
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
    assert_eq!(std::mem::size_of::<Option<SourceSample>>(), 16);
    assert_eq!(AtlasHalo::new().context.len(), 516 * 516);
    let terrain = AtlasTerrain::new(&filled(0, TerrainKind::Land), edge_halo()).unwrap();
    assert_eq!(terrain.palette.len(), 514 * 514);
    assert_eq!(terrain.light.len(), 514 * 514);
    assert_eq!(terrain.classes.len(), 514 * 514);
    assert_eq!(terrain.heights.len(), 514 * 514);
}

fn saved_lake(cells: Vec<CellCoord>, surface: i32) -> Lake {
    Lake {
        global_id: arda_core::hydrology::BasinId(1),
        id: 1,
        surface: HeightMm::new(surface),
        depth_mm: 250_000,
        outlet: None,
        cells,
    }
}

#[test]
fn atlas_lake_depth_varies_and_interpolates_within_membership() {
    let mut cells = filled(100_000, TerrainKind::Land);
    let shallow = CellCoord::new(100, 100).unwrap();
    let deep = CellCoord::new(101, 100).unwrap();
    cells.set(shallow, cell(190_000, TerrainKind::Lake));
    cells.set(deep, cell(0, TerrainKind::Lake));
    let terrain = AtlasTerrain::new_with_lakes(
        &cells,
        &[saved_lake(vec![shallow, deep], 200_000)],
        edge_halo(),
    )
    .unwrap();
    assert_eq!(terrain.colour(shallow), water_depth_palette(10_000));
    assert_eq!(terrain.colour(deep), water_depth_palette(200_000));
    let mid = terrain
        .sample(
            AxisKernel::Linear {
                low: 100,
                high_weight: 1,
                denominator: 2,
            },
            AxisKernel::Linear {
                low: 100,
                high_weight: 0,
                denominator: 2,
            },
            TerrainKind::Lake,
        )
        .unwrap();
    assert!(mid[0] < terrain.colour(shallow)[0]);
    assert!(mid[0] > terrain.colour(deep)[0]);
}

#[test]
fn atlas_lake_uses_validated_neighbor_depth_at_seam() {
    let mut target = filled(100_000, TerrainKind::Land);
    let mut east = filled(100_000, TerrainKind::Land);
    let edge = CellCoord::new(511, 100).unwrap();
    let adjacent = CellCoord::new(0, 100).unwrap();
    target.set(edge, cell(190_000, TerrainKind::Lake));
    east.set(adjacent, cell(0, TerrainKind::Lake));
    let mut halo = copied_halo(&target);
    halo.states[AtlasNeighbor::East.index()] = NeighborState::Unset;
    halo.copy_neighbor_with_lakes(
        AtlasNeighbor::East,
        &east,
        &[saved_lake(vec![adjacent], 200_000)],
    )
    .unwrap();
    let terrain =
        AtlasTerrain::new_with_lakes(&target, &[saved_lake(vec![edge], 200_000)], halo).unwrap();
    let sample = terrain
        .sample(
            AxisKernel::Linear {
                low: 511,
                high_weight: 1,
                denominator: 2,
            },
            AxisKernel::Linear {
                low: 100,
                high_weight: 0,
                denominator: 2,
            },
            TerrainKind::Lake,
        )
        .unwrap();
    assert!(sample[0] < water_depth_palette(10_000)[0]);
    assert!(sample[0] > water_depth_palette(200_000)[0]);
}

#[test]
fn atlas_lake_rejects_invalid_saved_surface() {
    let mut cells = filled(0, TerrainKind::Land);
    let at = CellCoord::new(1, 1).unwrap();
    cells.set(at, cell(200_000, TerrainKind::Lake));
    assert!(matches!(
        AtlasTerrain::new_with_lakes(&cells, &[saved_lake(vec![at], 200_000)], edge_halo()),
        Err(RenderError::LakeGeometry { .. })
    ));
}

#[test]
fn atlas_lake_uses_diagonal_neighbor_depth_at_corner() {
    let mut target = filled(100_000, TerrainKind::Land);
    let mut northeast = filled(100_000, TerrainKind::Land);
    let corner = CellCoord::new(511, 0).unwrap();
    let diagonal = CellCoord::new(0, 511).unwrap();
    target.set(corner, cell(190_000, TerrainKind::Lake));
    northeast.set(diagonal, cell(0, TerrainKind::Lake));
    let mut halo = copied_halo(&target);
    halo.states[AtlasNeighbor::NorthEast.index()] = NeighborState::Unset;
    halo.copy_neighbor_with_lakes(
        AtlasNeighbor::NorthEast,
        &northeast,
        &[saved_lake(vec![diagonal], 200_000)],
    )
    .unwrap();
    let terrain =
        AtlasTerrain::new_with_lakes(&target, &[saved_lake(vec![corner], 200_000)], halo).unwrap();
    let corner_mix = terrain
        .sample(
            AxisKernel::Linear {
                low: 511,
                high_weight: 1,
                denominator: 2,
            },
            AxisKernel::Linear {
                low: -1,
                high_weight: 1,
                denominator: 2,
            },
            TerrainKind::Lake,
        )
        .unwrap();
    assert_eq!(corner_mix, [59, 117, 141]);
}
