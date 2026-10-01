//! Exports after a reload: saved geometry at both scales, unreadable tiles,
//! refused previews and streamed overviews.
use super::*;

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
