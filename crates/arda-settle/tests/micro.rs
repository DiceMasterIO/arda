//! The stage on the real seed-42 MICRO world, and its consumers:
//! - adapter A2: every settlement record deserialises straight into
//!   `arda-npc`'s `SettlementProfile`, with the shared fields equal;
//! - settle → society: `arda-society` reads the written `society/` files
//!   through `WorldSettlements::read_dir` and simulates them.
//!
//! World resolution: `$ARDA_TEST_WORLD`, then `<workspace>/out/micro42` (the
//! documented `arda generate --seed 42 --micro --terrain fine` output), else
//! the world is generated once into `target/arda-server-fixture/micro42`.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

#[path = "../../../tests/support/fixture_dir.rs"]
mod fixture_dir;

use arda_settle::grid::MEMORY_BUDGET;
use arda_settle::output::SettlementsFile;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

fn loads(dir: &Path) -> bool {
    arda::World::load(dir).is_ok_and(|w| w.manifest().fine_terrain.is_some() && w.seed() == 42)
}

fn world_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("ARDA_TEST_WORLD") {
        return PathBuf::from(dir);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = root.join("out/micro42");
    if loads(&out) {
        return out;
    }
    let dir = root.join("target/arda-server-fixture/micro42");
    fixture_dir::ensure(&dir, loads, |tmp| {
        arda::generate_from_fine_source(
            42,
            arda::GenerateConfig::MICRO,
            tmp,
            arda::FineDeliveryLimits::default(),
        )
        .expect("generating the MICRO fixture world");
    });
    dir
}

/// The stage's output for the MICRO world, written once per test process.
static SOCIETY_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let world = arda::World::load(&world_dir()).unwrap();
    let grid = arda_settle::load::load_grid(&world, MEMORY_BUDGET).unwrap();
    let params = arda_settle::PlaceParams {
        seed: world.seed(),
        density_per_km2: u32::from(world.manifest().config.mean_density_per_km2()),
    };
    let society = arda_settle::run(&grid, params).unwrap();
    let dir = std::env::temp_dir().join(format!("arda-settle-micro-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    arda_settle::write(&dir, &grid, params.seed, &society).unwrap();
    dir
});

#[test]
fn every_micro_settlement_record_is_an_npc_settlement_profile() {
    let dir = &*SOCIETY_DIR;
    let text = std::fs::read_to_string(dir.join("settlements.json")).unwrap();
    let file: SettlementsFile = serde_json::from_str(&text).unwrap();
    assert!(file.settlements.len() > 100, "{}", file.settlements.len());
    let raw: serde_json::Value = serde_json::from_str(&text).unwrap();
    let records = raw["settlements"].as_array().unwrap();
    assert_eq!(records.len(), file.settlements.len());
    for (record, s) in records.iter().zip(&file.settlements) {
        let p: arda_npc::SettlementProfile = serde_json::from_value(record.clone())
            .unwrap_or_else(|e| panic!("settlement {} is not a profile: {e}", s.id));
        assert_eq!(p.id.get(), s.id.get());
        assert_eq!(p.name, s.name);
        assert_eq!(p.population, s.population);
        assert_eq!(p.wealth, s.wealth);
        assert_eq!(p.culture, s.culture);
        assert_eq!(p.realm_id.get(), s.realm_id.get());
        assert_eq!(p.biome, s.biome);
        assert_eq!((p.coastal, p.riverine), (s.coastal, s.riverine));
        let tier = serde_json::to_value(p.tier).unwrap();
        assert_eq!(tier, serde_json::to_value(s.tier).unwrap());
        let functions = serde_json::to_value(&p.functions).unwrap();
        assert_eq!(functions, serde_json::to_value(&s.functions).unwrap());
    }
}

#[test]
fn society_simulates_the_micro_settlement_files() {
    let dir = &*SOCIETY_DIR;
    let world = arda_society::WorldSettlements::read_dir(dir).unwrap();
    assert!(world.settlements.len() > 100);
    assert!(!world.roads.is_empty() && !world.realms.is_empty());
    // settle writes realm members as `settlements`; society reads them.
    let listed: usize = world.realms.iter().map(|r| r.members.len()).sum();
    assert_eq!(listed, world.settlements.len());
    let society = arda_society::simulate_society(42, &world).unwrap();
    assert_eq!(society.settlements.len(), world.settlements.len());
    assert_eq!(society.realms.len(), world.realms.len());
    for s in &society.settlements {
        let head = s
            .roles
            .iter()
            .find(|r| matches!(r.kind.as_str(), "ruler" | "lord" | "reeve" | "elder"));
        assert!(head.is_some(), "settlement {} has no head role", s.id);
    }
    // History ends at the present partition, with the present seats.
    let replayed = society.history.replay_partition().unwrap();
    for w in &world.settlements {
        assert_eq!(replayed[&w.id], w.realm_id, "{} ends in its realm", w.name);
    }
    for r in &world.realms {
        let st = society.realms.iter().find(|x| x.state.id == r.id).unwrap();
        assert_eq!(st.state.seat, r.seat);
    }
    let again = arda_society::simulate_society(42, &world).unwrap();
    assert_eq!(
        arda_society::output::to_json(&society).unwrap(),
        arda_society::output::to_json(&again).unwrap()
    );
}
