//! Fine-recipe compatibility gate (`goal-prompt.md` §8: new behaviour goes
//! behind recipe versions; logic/02 §fine-formation recipes).
//!
//! Recipe 5 must regenerate exactly as v0.1 (commit 7f32695) did and
//! recipe 6 exactly as v0.2.0 did: every terrain, area and hydrology file,
//! and the Atlas 4K overview. `world.json` carries the build version, so it
//! is checked field by field instead of hashed.
//!
//! Regenerating a fixture: run with `ARDA_BLESS=1`, then inspect the diff.
//! The recipe-5 hashes are the v0.1 output and must never be re-blessed.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda::{FineDeliveryLimits, GenerateConfig, ImageQuality, MapStyle, World};
use std::path::{Path, PathBuf};

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let p = std::env::temp_dir().join(format!("arda-recipe-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        Self(p)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Every file under `root` except the manifest, sorted, as
/// `<relative path>  <blake3 hex>`.
fn fingerprint(root: &Path) -> Vec<String> {
    fn collect(root: &Path, dir: &Path, out: &mut Vec<String>) {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        paths.sort();
        for path in paths {
            if path.is_dir() {
                collect(root, &path, out);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if rel != "world.json" {
                    let bytes = std::fs::read(&path).unwrap();
                    out.push(format!("{rel}  {}", blake3::hash(&bytes).to_hex()));
                }
            }
        }
    }
    let mut out = Vec::new();
    collect(root, root, &mut out);
    out
}

/// Generates MICRO seed 42 with `recipe`, renders its Atlas 4K overview and
/// compares both against `tests/golden/micro-42-recipe<recipe>.txt`.
fn check(recipe: u16) {
    let tmp = TempDir::new(&format!("r{recipe}"));
    let world_dir = tmp.0.join("world");
    let manifest = arda::generate_from_fine_recipe(
        42,
        GenerateConfig::MICRO,
        &world_dir,
        FineDeliveryLimits::default(),
        recipe,
    )
    .expect("generation failed");
    let fine = manifest.fine_terrain.expect("a fine world");
    assert_eq!((fine.recipe_version, fine.attempt), (recipe, 0));
    // Only recipe 6 publishes the shore layer and the water forms.
    let optional = world_dir.join("terrain/shore.bin").exists()
        || world_dir.join("areas/00_00/water.bin").exists();
    assert_eq!(optional, recipe >= 6, "recipe {recipe} optional layers");

    let world = World::load(&world_dir).expect("load");
    let render = tmp.0.join("render");
    std::fs::create_dir_all(&render).unwrap();
    let png = arda::export_overview_with_quality_and_style(
        &world,
        &render,
        ImageQuality::new(4_096).unwrap(),
        MapStyle::Atlas,
    )
    .expect("Atlas overview");
    let mut actual = fingerprint(&world_dir);
    actual.push(format!(
        "atlas-overview-4k.png  {}",
        blake3::hash(&std::fs::read(png).unwrap()).to_hex()
    ));
    let actual = actual.join("\n");

    let golden = format!("tests/golden/micro-42-recipe{recipe}.txt");
    if std::env::var("ARDA_BLESS").is_ok() {
        std::fs::write(&golden, &actual).expect("cannot write fixture");
        return;
    }
    let expected = std::fs::read_to_string(&golden)
        .unwrap_or_else(|_| panic!("{golden} is missing; run with ARDA_BLESS=1 to create it"));
    assert_eq!(
        actual.trim(),
        expected.trim(),
        "recipe {recipe} output changed: new behaviour must go behind a new recipe"
    );
}

#[test]
fn recipe_5_replays_v0_1_byte_for_byte() {
    check(5);
}

#[test]
fn recipe_6_replays_v0_2_byte_for_byte() {
    check(6);
}

#[test]
fn unknown_recipes_are_refused_before_output() {
    let tmp = TempDir::new("r9");
    for recipe in [0, 1, 3, 7] {
        let err = arda::generate_from_fine_recipe(
            42,
            GenerateConfig::MICRO,
            &tmp.0,
            FineDeliveryLimits::default(),
            recipe,
        )
        .unwrap_err();
        assert!(err.to_string().contains("recipe"), "{err}");
        assert!(!tmp.0.exists(), "recipe {recipe} wrote output");
    }
}
