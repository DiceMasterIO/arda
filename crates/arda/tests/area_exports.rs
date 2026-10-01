//! Saved geometry, explicit image scale, reload identity and read-only export.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda::{
    export_area, export_area_with_quality, export_area_with_quality_and_style,
    export_area_with_scale, export_overview, export_overview_with_quality,
    export_overview_with_quality_and_style, AreaImageScale, ExportError, ExportFormat,
    ImageQuality, MapStyle, World,
};
use arda_core::hydrology::ChannelEdge;
use arda_core::{
    AreaCells, AreaObjects, Cell, DischargeMilli, GenerateConfig, GlobalCell, HeightMm, Manifest,
    TerrainKind, ValidationStats, FORMAT_VERSION,
};
use arda_render::{AtlasHalo, AtlasNeighbor, AtlasTerrain};
use std::path::{Path, PathBuf};

#[path = "area_exports/reload.rs"]
mod reload;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "arda-export-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(path.join("world/areas/00_00")).unwrap();
        std::fs::create_dir_all(path.join("exports")).unwrap();
        let manifest = Manifest {
            format_version: FORMAT_VERSION,
            arda_version: "0.1.0".into(),
            seed: 42,
            config: GenerateConfig::MICRO,
            areas_wide: 2,
            areas_high: 4,
            stats: ValidationStats {
                land_fraction_permille: 500,
                area_count: 8,
                settlement_count: 0,
                named_river_count: 0,
                river_count: 0,
            },
            fine_terrain: None,
        };
        arda_core::write_manifest(&path.join("world"), &manifest).unwrap();
        let cells = AreaCells::flat(Cell {
            height: HeightMm::new(123_000),
            terrain: TerrainKind::Land,
            ..Cell::default()
        });
        let mut objects = AreaObjects::empty();
        objects.channel_edges.push(ChannelEdge {
            from: GlobalCell { x: 20, y: 20 },
            to: GlobalCell { x: 21, y: 21 },
            from_width_dm: 10,
            to_width_dm: 20,
            discharge: DischargeMilli::new(1_000),
        });
        std::fs::write(
            path.join("world/areas/00_00/cells.bin"),
            arda_core::encode_cells(&cells),
        )
        .unwrap();
        std::fs::write(
            path.join("world/areas/00_00/objects.bin"),
            arda_core::encode_objects(&objects).unwrap(),
        )
        .unwrap();
        Self(path)
    }
    fn world(&self) -> PathBuf {
        self.0.join("world")
    }
    fn exports(&self) -> PathBuf {
        self.0.join("exports")
    }
    fn snapshot(&self) -> Vec<Vec<u8>> {
        [
            "world.json",
            "areas/00_00/cells.bin",
            "areas/00_00/objects.bin",
        ]
        .map(|name| std::fs::read(self.world().join(name)).unwrap())
        .into()
    }

    fn new_grid(areas_wide: i32, areas_high: i32) -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "arda-atlas-grid-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(path.join("world/areas")).unwrap();
        std::fs::create_dir_all(path.join("exports")).unwrap();
        let size = arda::SizeKm::new(
            u32::try_from(areas_wide).unwrap() * 51,
            u32::try_from(areas_high).unwrap() * 51,
        );
        let config = GenerateConfig::new(size, arda::LatitudeBand::new(35, 55), 15).unwrap();
        let manifest = Manifest {
            format_version: FORMAT_VERSION,
            arda_version: "0.1.0".into(),
            seed: 42,
            config,
            areas_wide,
            areas_high,
            stats: ValidationStats {
                land_fraction_permille: 500,
                area_count: u32::try_from(areas_wide * areas_high).unwrap(),
                settlement_count: 0,
                named_river_count: 0,
                river_count: 0,
            },
            fine_terrain: None,
        };
        arda_core::write_manifest(&path.join("world"), &manifest).unwrap();
        for ay in 0..areas_high {
            for ax in 0..areas_wide {
                let area_dir = path.join("world/areas").join(format!("{ax:02}_{ay:02}"));
                std::fs::create_dir_all(&area_dir).unwrap();
                let mut cells = AreaCells::flat(Cell {
                    height: HeightMm::new(100_000),
                    terrain: TerrainKind::Land,
                    ..Cell::default()
                });
                for y in 0..arda_core::AREA_CELLS {
                    for x in 0..arda_core::AREA_CELLS {
                        let at = arda_core::CellCoord::new(x, y).unwrap();
                        let global_x = ax * 512 + i32::from(x);
                        let global_y = ay * 512 + i32::from(y);
                        let mut cell = *cells.get(at);
                        // The cross-term distinguishes diagonal two-deep corner samples.
                        cell.height = HeightMm::new(
                            100_000 + global_x * 17 + global_y * 29 + global_x * global_y,
                        );
                        cells.set(at, cell);
                    }
                }
                std::fs::write(area_dir.join("cells.bin"), arda_core::encode_cells(&cells))
                    .unwrap();
                std::fs::write(
                    area_dir.join("objects.bin"),
                    arda_core::encode_objects(&AreaObjects::empty()).unwrap(),
                )
                .unwrap();
            }
        }
        Self(path)
    }

    fn remove_area(&self, ax: i32, ay: i32) {
        std::fs::remove_dir_all(self.world().join("areas").join(format!("{ax:02}_{ay:02}")))
            .unwrap();
    }

    fn corrupt_cells(&self, ax: i32, ay: i32) -> PathBuf {
        let path = self
            .world()
            .join("areas")
            .join(format!("{ax:02}_{ay:02}"))
            .join("cells.bin");
        std::fs::write(&path, b"corrupt neighbor").unwrap();
        path
    }

    fn all_source_bytes(&self) -> Vec<(PathBuf, Vec<u8>)> {
        fn collect(root: &Path, dir: &Path, files: &mut Vec<(PathBuf, Vec<u8>)>) {
            let mut entries: Vec<_> = std::fs::read_dir(dir)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect();
            entries.sort();
            for path in entries {
                if path.is_dir() {
                    collect(root, &path, files);
                } else {
                    files.push((
                        path.strip_prefix(root).unwrap().to_path_buf(),
                        std::fs::read(path).unwrap(),
                    ));
                }
            }
        }
        let mut files = Vec::new();
        collect(&self.world(), &self.world(), &mut files);
        files
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn dimensions(path: &Path) -> (u32, u32) {
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(&bytes[12..16], b"IHDR");
    (
        u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
        u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
    )
}

const NEIGHBORS: [(AtlasNeighbor, i32, i32); 8] = [
    (AtlasNeighbor::North, 1, 0),
    (AtlasNeighbor::NorthEast, 2, 0),
    (AtlasNeighbor::East, 2, 1),
    (AtlasNeighbor::SouthEast, 2, 2),
    (AtlasNeighbor::South, 1, 2),
    (AtlasNeighbor::SouthWest, 0, 2),
    (AtlasNeighbor::West, 0, 1),
    (AtlasNeighbor::NorthWest, 0, 0),
];

fn direct_center_terrain(world: &World) -> AtlasTerrain {
    let center = world.read_area(1, 1).unwrap();
    let mut halo = AtlasHalo::new();
    for (direction, ax, ay) in NEIGHBORS {
        let neighbor = world.read_area(ax, ay).unwrap();
        halo.copy_neighbor(direction, neighbor.cells()).unwrap();
    }
    AtlasTerrain::new(center.cells(), halo).unwrap()
}

fn direct_corner_terrain(world: &World) -> AtlasTerrain {
    let corner = world.read_area(0, 0).unwrap();
    let mut halo = AtlasHalo::new();
    for direction in [
        AtlasNeighbor::North,
        AtlasNeighbor::NorthEast,
        AtlasNeighbor::West,
        AtlasNeighbor::NorthWest,
        AtlasNeighbor::SouthWest,
    ] {
        halo.mark_world_edge(direction).unwrap();
    }
    for (direction, ax, ay) in [
        (AtlasNeighbor::East, 1, 0),
        (AtlasNeighbor::SouthEast, 1, 1),
        (AtlasNeighbor::South, 0, 1),
    ] {
        let neighbor = world.read_area(ax, ay).unwrap();
        halo.copy_neighbor(direction, neighbor.cells()).unwrap();
    }
    AtlasTerrain::new(corner.cells(), halo).unwrap()
}

fn render_direct(
    world: &World,
    ax: i32,
    ay: i32,
    quality: ImageQuality,
    terrain: &AtlasTerrain,
) -> Vec<u8> {
    let area = world.read_area(ax, ay).unwrap();
    let mut bytes = Vec::new();
    arda_render::render_area_png_to_atlas(
        area.cells(),
        area.objects(),
        GlobalCell {
            x: u32::try_from(ax).unwrap() * 512,
            y: u32::try_from(ay).unwrap() * 512,
        },
        if quality.pixels() == 512 {
            AreaImageScale::Preview
        } else {
            AreaImageScale::Custom(quality)
        },
        terrain,
        &mut bytes,
    )
    .unwrap();
    bytes
}

#[test]
fn classic_styled_entry_points_match_existing_quality_bytes() {
    let fixture = Fixture::new_grid(2, 2);
    let world = World::load(&fixture.world()).unwrap();
    let quality = ImageQuality::new(513).unwrap();
    let old = export_area_with_quality(&world, 0, 0, &fixture.exports(), quality).unwrap();
    let expected = std::fs::read(&old).unwrap();
    let styled = export_area_with_quality_and_style(
        &world,
        0,
        0,
        &fixture.exports(),
        quality,
        MapStyle::Classic,
    )
    .unwrap();
    assert_eq!(std::fs::read(styled).unwrap(), expected);

    let old = export_overview_with_quality(&world, &fixture.exports(), quality).unwrap();
    let expected = std::fs::read(&old).unwrap();
    let styled = export_overview_with_quality_and_style(
        &world,
        &fixture.exports(),
        quality,
        MapStyle::Classic,
    )
    .unwrap();
    assert_eq!(std::fs::read(styled).unwrap(), expected);
    assert_eq!(MapStyle::default(), MapStyle::Classic);
}

#[test]
fn atlas_facade_routes_all_eight_neighbors_and_two_deep_diagonal_context() {
    let fixture = Fixture::new_grid(3, 3);
    let world = World::load(&fixture.world()).unwrap();
    let quality = ImageQuality::new(513).unwrap();
    let actual = export_area_with_quality_and_style(
        &world,
        1,
        1,
        &fixture.exports(),
        quality,
        MapStyle::Atlas,
    )
    .unwrap();
    let expected = render_direct(&world, 1, 1, quality, &direct_center_terrain(&world));
    assert_eq!(std::fs::read(actual).unwrap(), expected);
}

#[test]
fn atlas_facade_marks_each_outside_direction_at_a_world_corner() {
    let fixture = Fixture::new_grid(2, 2);
    let world = World::load(&fixture.world()).unwrap();
    let quality = ImageQuality::new(513).unwrap();
    let actual = export_area_with_quality_and_style(
        &world,
        0,
        0,
        &fixture.exports(),
        quality,
        MapStyle::Atlas,
    )
    .unwrap();
    let expected = render_direct(&world, 0, 0, quality, &direct_corner_terrain(&world));
    assert_eq!(std::fs::read(actual).unwrap(), expected);
}

#[test]
fn atlas_area_and_overview_repeat_without_changing_saved_world() {
    let fixture = Fixture::new_grid(2, 2);
    let before = fixture.all_source_bytes();
    let world = World::load(&fixture.world()).unwrap();
    let quality = ImageQuality::new(513).unwrap();
    let area = export_area_with_quality_and_style(
        &world,
        0,
        0,
        &fixture.exports(),
        quality,
        MapStyle::Atlas,
    )
    .unwrap();
    let area_bytes = std::fs::read(&area).unwrap();
    export_area_with_quality_and_style(&world, 0, 0, &fixture.exports(), quality, MapStyle::Atlas)
        .unwrap();
    assert_eq!(std::fs::read(area).unwrap(), area_bytes);

    let overview = export_overview_with_quality_and_style(
        &world,
        &fixture.exports(),
        quality,
        MapStyle::Atlas,
    )
    .unwrap();
    let overview_bytes = std::fs::read(&overview).unwrap();
    export_overview_with_quality_and_style(&world, &fixture.exports(), quality, MapStyle::Atlas)
        .unwrap();
    assert_eq!(std::fs::read(overview).unwrap(), overview_bytes);
    assert_eq!(fixture.all_source_bytes(), before);
}

#[test]
fn every_in_world_halo_direction_propagates_a_missing_neighbor() {
    for (_, ax, ay) in NEIGHBORS {
        let fixture = Fixture::new_grid(3, 3);
        fixture.remove_area(ax, ay);
        let world = World::load(&fixture.world()).unwrap();
        let error = export_area_with_quality_and_style(
            &world,
            1,
            1,
            &fixture.exports(),
            ImageQuality::new(512).unwrap(),
            MapStyle::Atlas,
        )
        .unwrap_err();
        assert!(error.to_string().contains(&format!("{ax:02}_{ay:02}")));
        assert_eq!(std::fs::read_dir(fixture.exports()).unwrap().count(), 0);
    }
}

#[test]
fn corrupt_diagonal_neighbor_propagates_the_named_layer() {
    let fixture = Fixture::new_grid(3, 3);
    let cells = fixture.corrupt_cells(2, 0);
    let world = World::load(&fixture.world()).unwrap();
    let error = export_area_with_quality_and_style(
        &world,
        1,
        1,
        &fixture.exports(),
        ImageQuality::new(512).unwrap(),
        MapStyle::Atlas,
    )
    .unwrap_err();
    assert!(error.to_string().contains(&cells.display().to_string()));
    assert_eq!(std::fs::read_dir(fixture.exports()).unwrap().count(), 0);
}

#[test]
fn failed_atlas_overview_cleans_partial_and_preserves_completed_png() {
    let fixture = Fixture::new_grid(2, 2);
    fixture.remove_area(1, 1);
    let world = World::load(&fixture.world()).unwrap();
    let path = fixture.exports().join("overview.png");
    std::fs::write(&path, b"previous complete PNG").unwrap();
    let error = export_overview_with_quality_and_style(
        &world,
        &fixture.exports(),
        ImageQuality::new(512).unwrap(),
        MapStyle::Atlas,
    )
    .unwrap_err();
    assert!(error.to_string().contains("01_01"));
    assert_eq!(std::fs::read(path).unwrap(), b"previous complete PNG");
    assert_eq!(std::fs::read_dir(fixture.exports()).unwrap().count(), 1);
}
