//! Town plans on the real seed-42 MICRO world keep to the refined rivers
//! arda-refine draws: no building footprint on river water, every street
//! square over it a bridge deck, every deck spanning the channel from bank
//! to bank, and the same plan every time.
//!
//! World resolution: `$ARDA_TEST_WORLD`, then `<workspace>/out/micro42`
//! when it holds `society/`, else a MICRO world generated and settled once
//! into `target/arda-people-fixture/micro42`.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

#[path = "../../../tests/support/fixture_dir.rs"]
mod fixture_dir;

use arda_people::files::SocietyFiles;
use arda_people::shared::SharedSource;
use arda_people::town;
use arda_refine::water::RiverWater;
use arda_refine::{CellKey, Source};
use arda_settle::model::Tier;
use arda_town::plan::check;
use arda_town::TownPlan;
use std::path::{Path, PathBuf};

fn ready(dir: &Path) -> bool {
    dir.join("society/settlements.json").is_file()
        && arda::World::load(dir)
            .is_ok_and(|w| w.manifest().fine_terrain.is_some() && w.seed() == 42)
}

fn world_dir() -> PathBuf {
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
}

/// Refined river water over the plan's grid (and a cell around it), from
/// arda-refine directly.
fn water_of(src: &dyn Source, plan: &TownPlan) -> RiverWater {
    let (gx0, gy0) = plan.origin();
    let (w, h) = plan.size();
    let cells: Vec<CellKey> = (gy0.div_euclid(64) - 1..=(gy0 + h).div_euclid(64) + 1)
        .flat_map(|cy| {
            (gx0.div_euclid(64) - 1..=(gx0 + w).div_euclid(64) + 1)
                .map(move |cx| CellKey::new(cx, cy))
        })
        .collect();
    RiverWater::of_cells(src, &cells).unwrap()
}

#[test]
fn micro_town_plans_keep_to_the_refined_rivers() {
    let dir = world_dir();
    let src = SharedSource::open(&dir).unwrap();
    let files = SocietyFiles::read(&dir.join("society")).unwrap();
    // Every riverine town and city, and the largest riverine villages.
    let mut picked: Vec<_> = files
        .settlements
        .settlements
        .iter()
        .zip(&files.records)
        .filter(|(s, _)| s.riverine)
        .collect();
    picked.sort_by_key(|(s, _)| (std::cmp::Reverse(s.population), s.id.get()));
    let picked: Vec<_> = picked
        .into_iter()
        .enumerate()
        .filter(|(i, (s, _))| *i < 12 || matches!(s.tier, Tier::Town | Tier::City))
        .map(|(_, x)| x)
        .collect();
    let (mut bridges, mut planned) = (0, 0);
    for (s, record) in picked {
        let plan = town::plan(&src, s, record, &files.roads.roads).unwrap();
        let Some(plan) = plan else { continue };
        planned += 1;
        let water = water_of(&src, &plan);
        let problems = check::water(&plan, &|x, y| water.is_water(x, y));
        assert!(problems.is_empty(), "{} ({}): {problems:?}", s.name, s.id);
        // Deterministic: the same plan again, deck for deck.
        let again = town::plan(&src, s, record, &files.roads.roads)
            .unwrap()
            .unwrap();
        assert_eq!(again.to_json().unwrap(), plan.to_json().unwrap());
        assert_eq!(again.grid.kind, plan.grid.kind);
        bridges += plan.bridges.len();
    }
    assert!(planned >= 10, "{planned} riverine plans");
    assert!(bridges > 0, "no town bridge in {planned} riverine plans");
    println!("{planned} riverine plans, {bridges} bridges");
}
