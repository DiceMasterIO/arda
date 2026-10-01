//! Channel coverage carried into the Atlas terrain, at area and 32k-row scale.
use super::*;
use crate::{AtlasHalo, AtlasNeighbor, AtlasTerrain};

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
fn atlas_preserves_lake_depth_and_channel_coverage_mask() {
    let mut cells = land();
    let lake_at = CellCoord::new(10, 10).unwrap();
    cells.set(
        lake_at,
        Cell {
            terrain: TerrainKind::Lake,
            height: HeightMm::new(0),
            ..Cell::default()
        },
    );
    let sea_at = CellCoord::new(12, 10).unwrap();
    cells.set(
        sea_at,
        Cell {
            terrain: TerrainKind::Sea,
            height: HeightMm::new(-2000),
            ..Cell::default()
        },
    );
    let lakes = [lake(1, lake_at, 250)];
    let terrain = standalone_atlas(&cells);
    let edge = edge((9, 11), (10, 11), 100);
    let origin = GlobalCell { x: 0, y: 0 };
    let scale = AreaImageScale::Preview;
    let render = |atlas: bool, channels: bool| {
        let inputs = channels.then_some(edge).into_iter().map(Ok);
        let mut raster = AreaRaster::new_with_terrain(
            &cells,
            atlas.then_some(&terrain),
            inputs,
            origin,
            scale,
            &lakes,
        )
        .unwrap();
        let mut rgb = Vec::new();
        for y in 0..512 {
            rgb.extend_from_slice(raster.row(y).unwrap());
        }
        rgb
    };
    let classic_base = render(false, false);
    let classic_channel = render(false, true);
    let atlas_base = render(true, false);
    let atlas_channel = render(true, true);
    for i in 0..512 * 512 {
        let p = i * 3;
        assert_eq!(
            classic_base[p..p + 3] != classic_channel[p..p + 3],
            atlas_base[p..p + 3] != atlas_channel[p..p + 3]
        );
    }
    assert_eq!(pixel(&classic_channel, 512, 10, 10), lake_colour(250));
    assert_eq!(pixel(&atlas_channel, 512, 10, 10), lake_colour(250));
    assert_eq!(
        pixel(&atlas_channel, 512, 12, 10),
        terrain
            .sample(
                axis_kernel(12, 512).unwrap(),
                axis_kernel(10, 512).unwrap(),
                TerrainKind::Sea
            )
            .unwrap()
    );
}

#[test]
fn atlas_32k_rows_keep_bounded_storage() {
    let cells = land();
    let terrain = standalone_atlas(&cells);
    let side = 32_768;
    let scale = AreaImageScale::Custom(ImageQuality::new(side).unwrap());
    let mut raster = AreaRaster::new_with_terrain(
        &cells,
        Some(&terrain),
        std::iter::empty(),
        GlobalCell { x: 0, y: 0 },
        scale,
        &[],
    )
    .unwrap();
    for y in [0, 1, 255, 256, 32_767] {
        assert_eq!(raster.row(y).unwrap().len(), side as usize * 3);
    }
    assert_eq!(raster.base_row.len(), side as usize * 3);
    assert_eq!(raster.rgb.len(), side as usize * 3);
}

#[test]
fn river_crossing_uses_the_same_resolved_shore_as_area_colour() {
    for (sea_height, land_height, crossing_from) in [
        (-1, 100_000, TerrainKind::Sea),
        (-100_000, 1, TerrainKind::Land),
    ] {
        let mut cells = AreaCells::flat(Cell {
            terrain: TerrainKind::Sea,
            height: HeightMm::new(sea_height),
            ..Cell::default()
        });
        for y in 0..512 {
            for x in 101..512 {
                cells.set(
                    CellCoord::new(x, y).unwrap(),
                    Cell {
                        terrain: TerrainKind::Land,
                        height: HeightMm::new(land_height),
                        ..Cell::default()
                    },
                );
            }
        }
        let terrain = standalone_atlas(&cells);
        let scale = AreaImageScale::Custom(ImageQuality::new(4096).unwrap());
        let origin = GlobalCell { x: 0, y: 0 };
        let input = edge((100, 100), (101, 100), 2000);
        let mut base = AreaRaster::new_with_terrain(
            &cells,
            Some(&terrain),
            std::iter::empty(),
            origin,
            scale,
            &[],
        )
        .unwrap();
        let mut river =
            AreaRaster::new_with_terrain(&cells, Some(&terrain), [Ok(input)], origin, scale, &[])
                .unwrap();
        let py = 100 * 8 + 4;
        let base_row = base.row(py).unwrap();
        let river_row = river.row(py).unwrap();
        let mut crossed = 0;
        for px in 100 * 8..102 * 8 {
            let owner = CellCoord::new(u16::try_from(px / 8).unwrap(), 100).unwrap();
            let saved = cells.get(owner).terrain;
            let resolved = terrain
                .contour_class(
                    axis_kernel(u32::try_from(px).unwrap(), 4096).unwrap(),
                    axis_kernel(u32::try_from(py).unwrap(), 4096).unwrap(),
                    owner,
                    saved,
                )
                .unwrap();
            let start = px * 3;
            if saved == crossing_from && resolved != saved {
                crossed += 1;
                assert_eq!(
                    base_row[start..start + 3] != river_row[start..start + 3],
                    resolved == TerrainKind::Land
                );
            }
        }
        assert!(crossed > 0);
    }
}
