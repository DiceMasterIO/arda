//! Goal 64 on the real seed-42 MICRO world: the city's homes are furnished
//! with varied interiors. Few homes share a layout with any other home, and
//! no two adjacent homes share one.
//!
//! World resolution: `$ARDA_TEST_WORLD`, then `<workspace>/out/micro42`
//! when it holds `society/`, else a MICRO world generated and settled once
//! into `target/arda-people-fixture/micro42`.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_people::files::SocietyFiles;
use arda_people::shared::SharedSource;
use arda_people::town;
use arda_settle::model::Tier;
use arda_town::block::interior::variety;
use std::path::{Path, PathBuf};

/// Most homes whose interior may equal another home's (goal 64).
const MAX_REPEATED: f64 = 0.05;

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
    if !ready(&dir) {
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
        arda::generate_from_fine_source(
            42,
            arda::GenerateConfig::MICRO,
            &dir,
            arda::FineDeliveryLimits::default(),
        )
        .expect("generating the MICRO fixture world");
        arda_settle::generate(&dir, arda_settle::grid::MEMORY_BUDGET).expect("settle");
    }
    dir
}

fn city_plan() -> (String, arda_town::TownPlan) {
    let dir = world_dir();
    let src = SharedSource::open(&dir).unwrap();
    let files = SocietyFiles::read(&dir.join("society")).unwrap();
    let (s, record) = files
        .settlements
        .settlements
        .iter()
        .zip(&files.records)
        .filter(|(s, _)| s.tier == Tier::City)
        .max_by_key(|(s, _)| (s.population, std::cmp::Reverse(s.id.get())))
        .expect("the seed-42 world has a city");
    let plan = town::plan(&src, s, record, &files.roads.roads)
        .unwrap()
        .expect("the city has a plan");
    (s.name.clone(), plan)
}

/// Ordered furniture (logic/10 §town-wfc): the city's barracks beds,
/// warehouse racks, library bookshelves, market-hall tables and temple
/// pews stand in aligned rows.
#[test]
fn city_ordered_furniture_stands_in_rows() {
    use arda_town::block::wfc::{indoor, rows};
    use arda_town::BuildingFunction as F;
    let (name, plan) = city_plan();
    let cases = [
        (F::Barracks, "dormitory", "prop.bed"),
        (F::Warehouse, "hall", "rack"),
        (F::Library, "reading_room", "prop.bookshelf"),
        (F::MarketHall, "arcade", "prop.table"),
        (F::Temple, "nave", "prop.pew"),
    ];
    for (f, tag, piece) in cases {
        let mut rooms = 0;
        for b in plan.buildings.iter().filter(|b| b.function == f) {
            let Ok(Some(p)) = indoor::solve(&plan, b) else {
                continue;
            };
            for (_, laid) in rows::laid_out(&p, tag, piece) {
                if laid.len() >= 2 {
                    assert!(rows::lined_up(&laid), "{name} {f:?} {}: {laid:?}", b.id.0);
                    rooms += 1;
                }
            }
        }
        println!("{name}: {rooms} {f:?} rooms with {piece} in rows");
        assert!(rooms > 0, "{name}: no {f:?} with {piece} rows");
    }
}

#[test]
fn city_homes_have_varied_interiors() {
    let dir = world_dir();
    let src = SharedSource::open(&dir).unwrap();
    let files = SocietyFiles::read(&dir.join("society")).unwrap();
    let (s, record) = files
        .settlements
        .settlements
        .iter()
        .zip(&files.records)
        .filter(|(s, _)| s.tier == Tier::City)
        .max_by_key(|(s, _)| (s.population, std::cmp::Reverse(s.id.get())))
        .expect("the seed-42 world has a city");
    let plan = town::plan(&src, s, record, &files.roads.roads)
        .unwrap()
        .expect("the city has a plan");
    let d = variety::diversity(&plan);
    println!(
        "{} ({}): {} homes, {} distinct interiors, {} repeated ({:.1} %), {} adjacent equal pairs",
        s.name,
        s.id,
        d.homes,
        d.distinct,
        d.repeated,
        100.0 * d.repeated_fraction(),
        d.adjacent_equal
    );
    assert!(d.homes > 200, "{} homes", d.homes);
    let wfc = arda_town::block::wfc::indoor::diversity(&plan);
    println!(
        "WFC: {} distinct, {} repeated ({:.1} %), {} adjacent equal pairs",
        wfc.distinct,
        wfc.repeated,
        100.0 * wfc.repeated_fraction(),
        wfc.adjacent_equal
    );
    assert_eq!(
        wfc.adjacent_equal, 0,
        "adjacent WFC homes share an interior"
    );
    assert!(
        wfc.repeated_fraction() < MAX_REPEATED,
        "{:.1} % of WFC homes repeat another's interior",
        100.0 * wfc.repeated_fraction()
    );
    assert_eq!(d.adjacent_equal, 0, "adjacent homes share an interior");
    assert!(
        d.repeated_fraction() < MAX_REPEATED,
        "{:.1} % of homes repeat another's interior",
        100.0 * d.repeated_fraction()
    );
}
