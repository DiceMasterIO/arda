//! The settled seed-42 MICRO world the society tests compose:
//! `$ARDA_TEST_WORLD`, then `<workspace>/out/micro42` when it holds
//! `society/`, else a MICRO world generated and settled once into
//! `target/arda-people-fixture/micro42` (shared with arda-people).

use arda_blocks::society::SocietyOverlays;
use arda_blocks::Pipeline;
use arda_tactical::Library;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

#[path = "../../../../tests/support/fixture_dir.rs"]
mod fixture_dir;

fn ready(dir: &Path) -> bool {
    dir.join("society/settlements.json").is_file()
        && dir.join("society/landuse.bin").is_file()
        && arda::World::load(dir)
            .is_ok_and(|w| w.manifest().fine_terrain.is_some() && w.seed() == 42)
}

/// The fixture world's directory.
pub fn world_dir() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        if let Some(dir) = std::env::var_os("ARDA_TEST_WORLD") {
            return PathBuf::from(dir);
        }
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let out = root.join("out/micro42");
        if ready(&out) {
            return out;
        }
        let dir = root.join("target/arda-people-fixture/micro42");
        fixture_dir::ensure(&dir, ready, |tmp| {
            arda::generate_from_fine_source(
                42,
                arda::GenerateConfig::MICRO,
                tmp,
                arda::FineDeliveryLimits::default(),
            )
            .expect("generating the MICRO fixture world");
            arda_settle::generate(tmp, arda_settle::grid::MEMORY_BUDGET).expect("settle");
        });
        dir
    })
    .clone()
}

/// The committed placeholder library.
pub fn lib() -> Library {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
    Library::load(&dir).unwrap()
}

/// The world, its society overlays and a pipeline over it.
pub fn open() -> (Arc<arda_people::World>, SocietyOverlays, Pipeline) {
    let world = Arc::new(arda_people::World::open(&world_dir()).unwrap());
    let society = SocietyOverlays::open(Arc::clone(&world)).unwrap();
    let pipeline = Pipeline::new(Box::new(world.src.clone()), 64);
    (world, society, pipeline)
}

/// The cell holding world position `(x_m, y_m)`.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn cell_at(x_m: i64, y_m: i64) -> (i64, i64) {
    let sq = |m: i64| (m as f64 / (100.0 / 64.0)).floor() as i64;
    (sq(x_m).div_euclid(64), sq(y_m).div_euclid(64))
}
