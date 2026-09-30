//! Shared fixtures: the four synthetic plans, built once per test binary.

#![allow(clippy::unwrap_used, clippy::expect_used)]
#![allow(dead_code)]

use arda_town::plan::TownPlan;
use arda_town::samples;
use std::sync::OnceLock;

/// World seed used by every test.
pub const SEED: u64 = 42;

/// Plans for every synthetic site, in `samples::NAMES` order.
pub fn plans() -> &'static [TownPlan] {
    static PLANS: OnceLock<Vec<TownPlan>> = OnceLock::new();
    PLANS.get_or_init(|| {
        samples::NAMES
            .iter()
            .map(|n| {
                let (site, terrain) = samples::by_name(n).expect("sample");
                arda_town::generate(&site, &terrain, SEED).expect("plan")
            })
            .collect()
    })
}

/// The plan of one site.
pub fn plan(name: &str) -> &'static TownPlan {
    let i = samples::NAMES
        .iter()
        .position(|n| *n == name)
        .expect("site");
    &plans()[i]
}

/// The placeholder library path.
pub fn library_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder")
}
