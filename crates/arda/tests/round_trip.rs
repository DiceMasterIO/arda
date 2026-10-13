//! End-to-end: generate a micro-continent, load it back, read it
//! (`logic/05`, `mockup/04`).
//!
//! Integration tests compile as their own crate, so the lib's cfg(test) allow
//! does not reach here; `code-prefs.md` §Q1 permits unwrap in tests.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda::{generate, GenerateConfig, Manifest, World};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

struct MicroWorld {
    manifest: Manifest,
    directories: Vec<PathBuf>,
    files: Vec<(PathBuf, Vec<u8>)>,
}

impl MicroWorld {
    fn capture(&mut self, root: &Path, dir: &Path) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            if entry.file_type().unwrap().is_dir() {
                self.directories.push(relative);
                self.capture(root, &path);
            } else {
                self.files.push((relative, std::fs::read(path).unwrap()));
            }
        }
    }
}

// Generate once per test process; keep only immutable bytes so mutations in a
// test cannot affect another test, and the source directory is always cleaned up.
static MICRO_WORLD: LazyLock<MicroWorld> = LazyLock::new(|| {
    let dir = TempDir::new("shared-micro");
    let mut snapshot = MicroWorld {
        manifest: generate(42, GenerateConfig::MICRO, dir.path()).unwrap(),
        directories: Vec::new(),
        files: Vec::new(),
    };
    snapshot.capture(dir.path(), dir.path());
    snapshot
});

fn micro_world(tag: &str) -> TempDir {
    let dir = TempDir::new(tag);
    for relative in &MICRO_WORLD.directories {
        std::fs::create_dir_all(dir.path().join(relative)).unwrap();
    }
    for (relative, bytes) in &MICRO_WORLD.files {
        std::fs::write(dir.path().join(relative), bytes).unwrap();
    }
    dir
}

struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("arda-rt-{}-{tag}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn generate_then_load_round_trips() {
    let dir = micro_world("round-trip");
    let manifest = &MICRO_WORLD.manifest;
    assert_eq!(manifest.seed, 42);
    assert_eq!(manifest.stats.area_count, 8);

    let world = World::load(dir.path()).unwrap();
    assert_eq!(world.seed(), 42);
    assert_eq!(world.areas(), 8);
    assert_eq!(world.size_km().width, 102);
}

#[test]
fn world_directory_matches_the_mockup_layout() {
    // world.json, continent/, areas/<ax>_<ay>/; no blocks/ since 1.0.
    let dir = micro_world("layout");

    assert!(dir.path().join("world.json").is_file());
    assert!(dir.path().join("areas").is_dir());
    assert!(dir.path().join("areas/00_00/cells.bin").is_file());
    assert!(dir.path().join("areas/00_00/objects.bin").is_file());
    assert!(dir.path().join("areas/01_03/cells.bin").is_file());
    assert!(
        !dir.path().join("blocks").exists(),
        "blocks/ was retired in 1.0"
    );
}

#[test]
fn cells_survive_the_disk_round_trip() {
    let dir = micro_world("cells");
    let world = World::load(dir.path()).unwrap();

    let area = world.area(1, 1).unwrap();
    let cell = area.cell(300, 128).unwrap();
    // The field list from mockup/04 is readable.
    let _ = cell.height;
    let _ = cell.terrain;
    let _ = cell.watercourse_order;
    assert!(area.rivers().iter().all(|r| r.order >= 1));
}

#[test]
fn out_of_range_area_returns_a_range_error_carrying_the_bounds() {
    let dir = micro_world("range");
    let world = World::load(dir.path()).unwrap();

    let err = world.area(9, 9).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("outside the world"), "message was: {msg}");
}

#[test]
fn a_partial_world_is_refused() {
    // 04-data-flow.md: manifest absent means loaders refuse the directory.
    let dir = micro_world("partial");
    std::fs::remove_file(dir.path().join("world.json")).unwrap();

    let err = World::load(dir.path()).unwrap_err();
    assert!(err.to_string().contains("did not finish"));
}

#[test]
fn generating_into_a_non_empty_directory_is_refused() {
    // mockup/01 States: refuse rather than overwrite.
    let dir = micro_world("occupied");
    let err = generate(42, GenerateConfig::MICRO, dir.path()).unwrap_err();
    assert!(err.to_string().contains("not empty"));
}

#[test]
fn the_same_seed_produces_byte_identical_worlds() {
    // architecture-interview.md §Q4, the one-way door.
    let a = micro_world("det-a");
    let b = TempDir::new("det-b");
    // Keep an independent generation to exercise determinism against the fixture.
    generate(42, GenerateConfig::MICRO, b.path()).unwrap();

    for rel in [
        "world.json",
        "areas/00_00/cells.bin",
        "areas/01_03/objects.bin",
    ] {
        assert_eq!(
            std::fs::read(a.path().join(rel)).unwrap(),
            std::fs::read(b.path().join(rel)).unwrap(),
            "{rel} differs between runs"
        );
    }
}

#[test]
fn different_seeds_produce_different_worlds() {
    let a = micro_world("seed-42");
    let b = TempDir::new("seed-43");
    generate(43, GenerateConfig::MICRO, b.path()).unwrap();
    assert_ne!(
        std::fs::read(a.path().join("areas/00_00/cells.bin")).unwrap(),
        std::fs::read(b.path().join("areas/00_00/cells.bin")).unwrap()
    );
}

#[test]
fn a_world_that_still_has_a_retired_blocks_folder_loads_and_exports() {
    // Worlds written before 1.0 carry blocks/; readers ignore it.
    let src = micro_world("legacy-blocks-src");
    let dir = TempDir::new("legacy-blocks");
    copy_tree(src.path(), dir.path());
    std::fs::create_dir_all(dir.path().join("blocks")).unwrap();
    std::fs::write(dir.path().join("blocks/00_00.tiles.zst"), b"retired").unwrap();

    let world = World::load(dir.path()).unwrap();
    assert!(world.area(0, 0).is_ok());
    let out = TempDir::new("legacy-blocks-out");
    let path = arda::export_area(&world, 0, 0, out.path(), arda::ExportFormat::Json).unwrap();
    assert!(path.is_file());
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}
