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
    assert_ne!(record(&terrain, -1, 512).0, record(&terrain, 0, 511).0);
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
fn flat_opposed_and_extreme_relief_have_exact_records() {
    assert_eq!(
        modulate(land_palette(500_000), relief_light(0, 0)),
        [165, 161, 97]
    );
    assert_eq!(
        modulate(land_palette(500_000), relief_light(200_000, 0)),
        [169, 165, 99]
    );
    assert_eq!(
        modulate(land_palette(500_000), relief_light(-200_000, 0)),
        [121, 119, 71]
    );
    assert_eq!(sea_palette(-3_000_000), [10, 37, 68]);
    assert_eq!(sea_palette(-6_000_000), [7, 26, 51]);
    assert_eq!(land_palette(2_800_000), [232, 232, 226]);
    assert_eq!(
        modulate(land_palette(2_800_000), relief_light(-4_294_967_295, 0)),
        [116, 116, 113]
    );
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
    let sea = AtlasTerrain::new(&filled(-3_000_000, TerrainKind::Sea), edge_halo()).unwrap();
    let lake = AtlasTerrain::new(&filled(500_000, TerrainKind::Lake), edge_halo()).unwrap();
    assert_eq!(land.colour(CellCoord::new(0, 0).unwrap()), [165, 161, 97]);
    assert_eq!(
        land.colour(CellCoord::new(200, 200).unwrap()),
        [169, 165, 99]
    );
    assert_eq!(
        land.colour(CellCoord::new(300, 300).unwrap()),
        [121, 119, 71]
    );
    assert_eq!(record(&sea, 0, 0).1, 4096);
    assert_eq!(record(&lake, 0, 0).1, 4096);
    assert_eq!(sea.colour(CellCoord::new(0, 0).unwrap()), [10, 37, 68]);
    assert_eq!(
        lake.colour(CellCoord::new(0, 0).unwrap()),
        crate::carto::LAKE_FILL
    );
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
        [232, 232, 226]
    );
    assert_eq!(
        extreme.colour(CellCoord::new(511, 511).unwrap()),
        [116, 116, 113]
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
    assert_eq!(std::mem::size_of::<Option<SourceSample>>(), 8);
    assert_eq!(AtlasHalo::new().context.len(), 516 * 516);
    let terrain = AtlasTerrain::new(&filled(0, TerrainKind::Land), edge_halo()).unwrap();
    assert_eq!(terrain.palette.len(), 514 * 514);
    assert_eq!(terrain.light.len(), 514 * 514);
    assert_eq!(terrain.classes.len(), 514 * 514);
}
