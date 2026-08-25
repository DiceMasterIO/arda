//! End-to-end: generate a micro-continent, load it back, read it
//! (`logic/05`, `mockup/04`).
//!
//! Integration tests compile as their own crate, so the lib's cfg(test) allow
//! does not reach here; `code-prefs.md` §Q1 permits unwrap in tests.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda::{generate, GenerateConfig, World};

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
    let dir = TempDir::new("round-trip");
    let manifest = generate(42, GenerateConfig::MICRO, dir.path()).unwrap();
    assert_eq!(manifest.seed, 42);
    assert_eq!(manifest.stats.area_count, 8);

    let world = World::load(dir.path()).unwrap();
    assert_eq!(world.seed(), 42);
    assert_eq!(world.areas(), 8);
    assert_eq!(world.size_km().width, 102);
}

#[test]
fn world_directory_matches_the_mockup_layout() {
    // mockup/02: world.json, continent/, areas/<ax>_<ay>/, blocks/.
    let dir = TempDir::new("layout");
    generate(42, GenerateConfig::MICRO, dir.path()).unwrap();

    assert!(dir.path().join("world.json").is_file());
    assert!(dir.path().join("areas").is_dir());
    assert!(dir.path().join("areas/00_00/cells.bin").is_file());
    assert!(dir.path().join("areas/00_00/objects.bin").is_file());
    assert!(dir.path().join("areas/01_03/cells.bin").is_file());
    assert!(dir.path().join("blocks/00_00.tiles.zst").is_file());
}

#[test]
fn cells_survive_the_disk_round_trip() {
    let dir = TempDir::new("cells");
    generate(42, GenerateConfig::MICRO, dir.path()).unwrap();
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
    let dir = TempDir::new("range");
    generate(42, GenerateConfig::MICRO, dir.path()).unwrap();
    let world = World::load(dir.path()).unwrap();

    let err = world.area(9, 9).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("outside the world"), "message was: {msg}");
}

#[test]
fn a_partial_world_is_refused() {
    // 04-data-flow.md: manifest absent means loaders refuse the directory.
    let dir = TempDir::new("partial");
    generate(42, GenerateConfig::MICRO, dir.path()).unwrap();
    std::fs::remove_file(dir.path().join("world.json")).unwrap();

    let err = World::load(dir.path()).unwrap_err();
    assert!(err.to_string().contains("did not finish"));
}

#[test]
fn generating_into_a_non_empty_directory_is_refused() {
    // mockup/01 States: refuse rather than overwrite.
    let dir = TempDir::new("occupied");
    generate(42, GenerateConfig::MICRO, dir.path()).unwrap();
    let err = generate(42, GenerateConfig::MICRO, dir.path()).unwrap_err();
    assert!(err.to_string().contains("not empty"));
}

#[test]
fn the_same_seed_produces_byte_identical_worlds() {
    // architecture-interview.md §Q4, the one-way door.
    let a = TempDir::new("det-a");
    let b = TempDir::new("det-b");
    generate(42, GenerateConfig::MICRO, a.path()).unwrap();
    generate(42, GenerateConfig::MICRO, b.path()).unwrap();

    for rel in [
        "world.json",
        "areas/00_00/cells.bin",
        "areas/01_03/objects.bin",
        "blocks/00_00.tiles.zst",
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
    let a = TempDir::new("seed-42");
    let b = TempDir::new("seed-43");
    generate(42, GenerateConfig::MICRO, a.path()).unwrap();
    generate(43, GenerateConfig::MICRO, b.path()).unwrap();
    assert_ne!(
        std::fs::read(a.path().join("areas/00_00/cells.bin")).unwrap(),
        std::fs::read(b.path().join("areas/00_00/cells.bin")).unwrap()
    );
}

#[test]
fn every_area_materialises_blocks_where_it_has_sampled_land() {
    // Guards the block stride: identical block archives across tiles are only
    // legitimate when both are genuinely empty.
    let dir = TempDir::new("blocks-present");
    generate(42, GenerateConfig::MICRO, dir.path()).unwrap();
    let world = World::load(dir.path()).unwrap();

    let mut report = Vec::new();
    for ay in 0..4 {
        for ax in 0..2 {
            let area = world.area(ax, ay).unwrap();
            let mut land = 0;
            let mut blocks = 0;
            let mut y = 0u16;
            while y < 512 {
                let mut x = 0u16;
                while x < 512 {
                    if area.cell(x, y).unwrap().terrain == arda::TerrainKind::Land {
                        land += 1;
                        if world.block(ax, ay, x, y).is_ok() {
                            blocks += 1;
                        }
                    }
                    x += 64;
                }
                y += 64;
            }
            report.push(format!("{ax},{ay}: sampled land {land}, blocks {blocks}"));
            assert_eq!(land, blocks, "tile {ax},{ay} has land without blocks");
        }
    }
    println!("{}", report.join("\n"));
    assert!(
        report.iter().any(|r| !r.ends_with("blocks 0")),
        "no tile materialised any block"
    );
}
