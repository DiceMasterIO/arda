//! Saved geometry, explicit image scale, reload identity and read-only export.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda::{
    export_area, export_area_with_quality, export_area_with_scale, export_overview,
    export_overview_with_quality, AreaImageScale, ExportError, ExportFormat, ImageQuality, World,
};
use arda_core::hydrology::ChannelEdge;
use arda_core::{
    AreaCells, AreaObjects, Cell, DischargeMilli, GenerateConfig, GlobalCell, HeightMm, Manifest,
    TerrainKind, ValidationStats, FORMAT_VERSION,
};
use std::path::{Path, PathBuf};

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

#[test]
fn saved_geometry_exports_at_both_scales_and_survives_reload_without_writes() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    let world = World::load(&fixture.world()).unwrap();
    let preview = export_area(&world, 0, 0, &fixture.exports(), ExportFormat::Png).unwrap();
    let detail = export_area_with_scale(
        &world,
        0,
        0,
        &fixture.exports(),
        ExportFormat::Png,
        AreaImageScale::Detail,
    )
    .unwrap();
    assert_eq!(preview.file_name().unwrap(), "area_00_00.png");
    assert_eq!(detail.file_name().unwrap(), "area_00_00_detail.png");
    assert_eq!(dimensions(&preview), (512, 512));
    assert_eq!(dimensions(&detail), (4096, 4096));
    let original_png = std::fs::read(&detail).unwrap();
    let json = export_area(&world, 0, 0, &fixture.exports(), ExportFormat::Json).unwrap();
    let original_json = std::fs::read(&json).unwrap();
    drop(world);
    let reloaded = World::load(&fixture.world()).unwrap();
    export_area_with_scale(
        &reloaded,
        0,
        0,
        &fixture.exports(),
        ExportFormat::Png,
        AreaImageScale::Detail,
    )
    .unwrap();
    export_area(&reloaded, 0, 0, &fixture.exports(), ExportFormat::Json).unwrap();
    assert_eq!(std::fs::read(detail).unwrap(), original_png);
    assert_eq!(std::fs::read(json).unwrap(), original_json);
    assert_eq!(fixture.snapshot(), before);
}

#[test]
fn an_unreadable_overview_tile_fails_instead_of_becoming_ocean() {
    let fixture = Fixture::new();
    let world = World::load(&fixture.world()).unwrap();
    let error = export_overview(&world, &fixture.exports(), 48).unwrap_err();
    assert!(error
        .to_string()
        .contains(&Path::new("01_00").join("cells.bin").display().to_string()));
    assert!(!fixture.exports().join("overview.png").exists());
}

#[test]
fn non_preview_json_is_refused_before_tile_reads_or_output_writes() {
    let fixture = Fixture::new();
    let world = World::load(&fixture.world()).unwrap();
    for scale in [
        AreaImageScale::Detail,
        AreaImageScale::Custom(ImageQuality::DEFAULT),
    ] {
        let error =
            export_area_with_scale(&world, 1, 3, &fixture.exports(), ExportFormat::Json, scale)
                .unwrap_err();
        assert!(matches!(error, ExportError::InvalidImageScale));
    }
    assert_eq!(std::fs::read_dir(fixture.exports()).unwrap().count(), 0);
}

#[test]
fn custom_quality_stream_repeats_after_reload_without_changing_saved_layers() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    let world = World::load(&fixture.world()).unwrap();
    let quality = ImageQuality::new(513).unwrap();
    let path = export_area_with_quality(&world, 0, 0, &fixture.exports(), quality).unwrap();
    assert_eq!(dimensions(&path), (513, 513));
    let bytes = std::fs::read(&path).unwrap();
    let reloaded = World::load(&fixture.world()).unwrap();
    export_area_with_quality(&reloaded, 0, 0, &fixture.exports(), quality).unwrap();
    assert_eq!(std::fs::read(path).unwrap(), bytes);
    assert_eq!(fixture.snapshot(), before);
    assert_eq!(std::fs::read_dir(fixture.exports()).unwrap().count(), 1);
}

#[test]
fn failed_streamed_overview_preserves_a_completed_export() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    let world = World::load(&fixture.world()).unwrap();
    let path = fixture.exports().join("overview.png");
    std::fs::write(&path, b"previous complete PNG").unwrap();
    let error =
        export_overview_with_quality(&world, &fixture.exports(), ImageQuality::new(512).unwrap())
            .unwrap_err();
    assert!(error
        .to_string()
        .contains(&Path::new("01_00").join("cells.bin").display().to_string()));
    assert_eq!(std::fs::read(path).unwrap(), b"previous complete PNG");
    assert_eq!(std::fs::read_dir(fixture.exports()).unwrap().count(), 1);
    assert_eq!(fixture.snapshot(), before);
}
