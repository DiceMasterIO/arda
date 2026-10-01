use super::*;
use crate::orchestrator::{
    fine_input::FineInputError, generate_world_from_fine_terrain, GenError, HydrologyLimits,
};
use arda_core::{read_manifest, GenerateConfig, ValidationStats, FORMAT_VERSION};
use std::sync::atomic::{AtomicU64, Ordering};

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "arda-publication-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn manifest() -> Manifest {
    Manifest {
        format_version: FORMAT_VERSION,
        arda_version: "test".to_owned(),
        seed: 42,
        config: GenerateConfig::MICRO,
        areas_wide: 2,
        areas_high: 4,
        stats: ValidationStats {
            land_fraction_permille: 500,
            area_count: 8,
            settlement_count: 0,
            named_river_count: 0,
            river_count: 1,
        },
        fine_terrain: None,
    }
}

#[test]
fn partial_output_stays_unloadable_and_requires_a_clean_rerun() {
    let dir = Directory::new();
    let output = WorldOutput::begin(&dir.0).unwrap();
    output
        .write_layer(Path::new("areas/0_0/cells.bin"), b"first")
        .unwrap();
    assert!(matches!(
        read_manifest(&dir.0),
        Err(LoadError::ManifestMissing { .. })
    ));
    drop(output);
    assert!(matches!(
        WorldOutput::begin(&dir.0),
        Err(PublicationError::Occupied(_))
    ));
    assert_eq!(
        fs::read(dir.0.join("areas/0_0/cells.bin")).unwrap(),
        b"first"
    );
}

#[test]
fn commit_removes_only_runtime_scratch_and_publishes_identical_manifests() {
    let a = Directory::new();
    let b = Directory::new();
    for directory in [&a, &b] {
        let output = WorldOutput::begin(&directory.0).unwrap();
        fs::create_dir(output.scratch().join("nested")).unwrap();
        fs::write(output.scratch().join("nested/flow.bin"), b"private").unwrap();
        output
            .write_layer(Path::new("hydrology/metadata.bin"), b"complete")
            .unwrap();
        output.commit(&manifest()).unwrap();
        assert_eq!(read_manifest(&directory.0).unwrap(), manifest());
        assert!(!directory.0.join(SCRATCH).exists());
        assert!(!directory.0.join(MARKER).exists());
        assert!(!directory.0.join(PENDING).exists());
        assert_eq!(
            fs::read(directory.0.join("hydrology/metadata.bin")).unwrap(),
            b"complete"
        );
    }
    assert_eq!(
        fs::read(a.0.join(MANIFEST_NAME)).unwrap(),
        fs::read(b.0.join(MANIFEST_NAME)).unwrap()
    );
}

#[test]
fn existing_data_and_actual_filesystem_errors_are_preserved() {
    let dir = Directory::new();
    fs::write(dir.0.join("original"), b"keep").unwrap();
    assert!(matches!(
        WorldOutput::begin(&dir.0),
        Err(PublicationError::Occupied(_))
    ));
    match WorldOutput::begin(&dir.0.join("original")) {
        Err(PublicationError::Io { source, .. }) => {
            assert_eq!(source.kind(), std::io::ErrorKind::AlreadyExists);
        }
        _ => panic!("expected original create-directory error"),
    }
    assert_eq!(fs::read(dir.0.join("original")).unwrap(), b"keep");
}

#[test]
fn fine_world_rejects_resources_before_output_and_corruption_before_manifest() {
    let directory = Directory::new();
    let source = directory.0.join("source.terrain");
    fs::write(&source, b"not a terrain file").unwrap();
    let out = directory.0.join("world");
    let tiny = HydrologyLimits {
        ram_bytes: 0,
        ..HydrologyLimits::default()
    };
    assert!(generate_world_from_fine_terrain(
        42,
        fine_descriptor(),
        GenerateConfig::MICRO,
        &source,
        &out,
        tiny
    )
    .is_err());
    assert!(!out.exists());
    assert!(matches!(
        generate_world_from_fine_terrain(
            42,
            fine_descriptor(),
            GenerateConfig::MICRO,
            &source,
            &out,
            HydrologyLimits::default()
        ),
        Err(GenError::FineInput(FineInputError::File(_)))
    ));
    assert!(!out.join("world.json").exists());
    assert_eq!(fs::read(&source).unwrap(), b"not a terrain file");
    assert!(matches!(
        generate_world_from_fine_terrain(
            42,
            fine_descriptor(),
            GenerateConfig::MICRO,
            &source,
            &out,
            HydrologyLimits::default()
        ),
        Err(GenError::OutputNotEmpty { .. })
    ));
}

#[test]
fn fine_world_refuses_valid_checksum_with_wrong_recipe_spacing() {
    let directory = Directory::new();
    let source = directory.0.join("source.terrain");
    let out = directory.0.join("world");
    let mut writer = arda_core::TerrainFileWriter::new(
        fs::File::create(&source).unwrap(),
        arda_core::TerrainPoint { x_um: 0, y_um: 0 },
        1_000_000_000,
        104,
        206,
    )
    .unwrap();
    for _ in 0..206 {
        writer
            .write_row(&vec![arda_core::HeightMm::new(100_000); 104])
            .unwrap();
    }
    drop(writer.finish().unwrap());
    let error = generate_world_from_fine_terrain(
        42,
        fine_descriptor(),
        GenerateConfig::MICRO,
        &source,
        &out,
        HydrologyLimits::default(),
    )
    .unwrap_err();
    assert!(matches!(error, GenError::Validation { check } if check.contains("spacing")));
    assert!(!out.join("world.json").exists());
}

#[test]
fn layer_conflict_and_reserved_paths_never_publish() {
    let dir = Directory::new();
    let output = WorldOutput::begin(&dir.0).unwrap();
    for invalid in [
        "",
        "/absolute",
        "../escape",
        "world.json",
        ".arda-generating",
        ".arda-hydrology-scratch/x",
        ".arda-manifest-pending",
    ] {
        assert!(matches!(
            output.write_layer(Path::new(invalid), b"bad"),
            Err(PublicationError::InvalidLayer)
        ));
    }
    output.write_layer(Path::new("a/b"), b"once").unwrap();
    assert!(matches!(
        output.write_layer(Path::new("a/b"), b"twice"),
        Err(PublicationError::Io { .. })
    ));
    assert!(matches!(
        output.write_layer(Path::new("a/b/c"), b"bad"),
        Err(PublicationError::Io { .. })
    ));
    assert!(!dir.0.join(MANIFEST_NAME).exists());
    assert_eq!(fs::read(dir.0.join("a/b")).unwrap(), b"once");
}

#[test]
fn manifest_write_failure_keeps_completion_absent() {
    let dir = Directory::new();
    let output = WorldOutput::begin(&dir.0).unwrap();
    fs::create_dir(output.scratch().join(MANIFEST_NAME)).unwrap();
    assert!(matches!(
        output.commit(&manifest()),
        Err(PublicationError::Manifest(_))
    ));
    assert!(!dir.0.join(MANIFEST_NAME).exists());
}

#[test]
fn prepublication_cleanup_failure_keeps_completion_absent() {
    let dir = Directory::new();
    let output = WorldOutput::begin(&dir.0).unwrap();
    fs::remove_file(dir.0.join(MARKER)).unwrap();
    fs::create_dir(dir.0.join(MARKER)).unwrap();
    assert!(matches!(
        output.commit(&manifest()),
        Err(PublicationError::Io { .. })
    ));
    assert!(!dir.0.join(MANIFEST_NAME).exists());
    assert!(dir.0.join(PENDING).is_file());
    assert!(matches!(
        WorldOutput::begin(&dir.0),
        Err(PublicationError::Occupied(_))
    ));
}

fn fine_descriptor() -> arda_core::FineTerrainDescriptor {
    arda_core::FineTerrainDescriptor {
        recipe_version: arda_core::FINE_TERRAIN_RECIPE_VERSION,
        attempt: 0,
    }
}

#[test]
fn fine_world_rejects_unknown_recipe_before_output() {
    let directory = Directory::new();
    let out = directory.0.join("world");
    for recipe_version in [0, arda_core::FINE_TERRAIN_LATEST_RECIPE_VERSION + 1] {
        let result = generate_world_from_fine_terrain(
            42,
            arda_core::FineTerrainDescriptor {
                recipe_version,
                attempt: 0,
            },
            GenerateConfig::MICRO,
            &directory.0.join("absent"),
            &out,
            HydrologyLimits::default(),
        );
        assert!(matches!(result, Err(GenError::Validation { .. })));
        assert!(!out.exists());
    }
}
