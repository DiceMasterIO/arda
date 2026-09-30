//! `BuildingSpec.function` is a plain snake_case string and a workshop's
//! craft is a free `craft:<key>` tag (vocabulary I7).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_npc::input::label;
use arda_npc::{generate_population, BuildingFunction, BuildingSpec, Craft, JobCategory};
use common::{town, SEED};
use serde_json::json;

#[test]
fn functions_are_plain_strings_with_craft_tags() {
    let (_, buildings) = town();
    let workshop = buildings
        .iter()
        .find(|b| b.function == BuildingFunction::Workshop)
        .unwrap();
    let v = serde_json::to_value(workshop).unwrap();
    assert_eq!(v["function"], json!("workshop"));
    assert!(v["tags"][0].as_str().unwrap().starts_with("craft:"));
    assert!(workshop.craft_tag().is_some());
    let house = buildings
        .iter()
        .find(|b| b.function == BuildingFunction::House)
        .unwrap();
    let v = serde_json::to_value(house).unwrap();
    assert_eq!(v["function"], json!("house"));
    assert!(v.get("tags").is_none(), "{v}");
    let back: BuildingSpec = serde_json::from_value(v).unwrap();
    assert_eq!(&back, house);
    assert_eq!(
        label(BuildingFunction::Workshop, Some(Craft::Weaving)),
        "weaving workshop"
    );
    assert_eq!(label(BuildingFunction::MarketHall, None), "market hall");
}

#[test]
fn an_untagged_workshop_still_gets_a_stable_craft() {
    let (profile, mut buildings) = town();
    for b in &mut buildings {
        b.tags.clear();
    }
    let a = generate_population(SEED, &profile, &buildings).unwrap();
    let b = generate_population(SEED, &profile, &buildings).unwrap();
    assert_eq!(a, b);
    let crafters = a
        .npcs
        .iter()
        .filter(|n| n.job.category == JobCategory::Craft && n.job.title.starts_with("Master "))
        .count();
    assert!(crafters > 0, "no craft masters among notables");
}
