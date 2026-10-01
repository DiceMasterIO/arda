//! Saved lake depths: interpolation within membership and validated
//! neighbour depths at seams and corners.

use super::*;

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
