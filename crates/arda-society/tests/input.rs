//! Input handling: the settlement stage's JSON shape loads as is (string or
//! numeric ids, road class names), explicit buildings are honoured, and
//! inconsistent worlds are refused.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_society::input::{BuildingSpec, RoadClass, WorldSettlements};
use arda_society::{simulate_society, SocietyError};

const SAMPLE: &str = r#"{
  "format_version": 1,
  "settlements": [
    {"id": "1", "name": "Oakford", "tier": "town", "population": 1800, "functions": ["farming", "market", "crafting", "crossing", "capital"],
     "wealth": 150, "culture": "heartland", "realm_id": "7", "biome": "temperate", "coastal": false, "riverine": true,
     "x_m": 10000, "y_m": 10000, "site_tags": ["ford", "river"], "history": "Grew at the ford of the Wend.",
     "buildings": {"house": 300, "temple": 1, "inn": 1, "smithy": 1, "keep": 1, "guardhouse": 1, "market_hall": 1, "workshop": 4}},
    {"id": 2, "name": "Ashby", "tier": "village", "population": 300, "functions": ["farming", "pastoral"],
     "wealth": 80, "culture": "heartland", "realm_id": 7, "biome": "temperate", "coastal": false, "riverine": false,
     "x_m": 16000, "y_m": 12000, "buildings": {"farmhouse": 30, "cottage": 30, "shrine": 1, "tavern": 1, "smithy": 1}},
    {"id": "3", "name": "Hollin", "tier": "hamlet", "population": 40, "functions": ["pastoral"],
     "wealth": 40, "culture": "unknown-culture", "realm_id": "7", "biome": "highland", "coastal": false, "riverine": false,
     "x_m": 20000, "y_m": 18000, "buildings": {"farmhouse": 6}}
  ],
  "roads": [
    {"id": 1, "class": "road", "from": "1", "to": "2", "length_m": 8000},
    {"id": 2, "class": "track", "from": "2", "to": "3", "length_m": 9000},
    {"id": 3, "class": "none", "from": "3", "to_edge": "east", "length_m": 5000}
  ],
  "realms": [{"id": "7", "name": "Wendmark", "seat": "1"}]
}"#;

#[test]
fn settlement_stage_json_loads_and_simulates() {
    let world: WorldSettlements = serde_json::from_str(SAMPLE).unwrap();
    assert_eq!(world.roads[2].class, RoadClass::None);
    assert_eq!(RoadClass::Footpath.code(), 4);
    let s = simulate_society(9, &world).unwrap();
    assert_eq!(s.realms.len(), 1);
    assert!(s.relations.is_empty());
    let town = &s.settlements[0];
    assert!(town
        .history_hooks
        .iter()
        .any(|h| h.text.contains("ford of the Wend")));
    assert!(town.roles.iter().any(|r| r.kind == "ruler"));
    assert!(s
        .settlements
        .iter()
        .all(|x| (3..=5).contains(&x.hooks.len())));
}

#[test]
fn explicit_buildings_replace_the_mix() {
    let mut world: WorldSettlements = serde_json::from_str(SAMPLE).unwrap();
    let spec = |id, function: &str, tags: Vec<String>| BuildingSpec {
        id,
        settlement_id: 2,
        function: function.to_string(),
        tags,
    };
    world.buildings = vec![
        spec(900, "temple", vec![]),
        spec(901, "farmhouse", vec![]),
        spec(902, "workshop", vec!["craft:weaver".into()]),
    ];
    let s = simulate_society(9, &world).unwrap();
    let v = &s.settlements[1];
    assert_eq!(
        v.buildings.iter().map(|b| b.id).collect::<Vec<_>>(),
        vec![900, 901, 902]
    );
    assert_eq!(v.buildings[2].craft(), Some("weaver"));
    assert!(v
        .roles
        .iter()
        .any(|r| r.kind == "high_priest" && r.building == Some(900)));
}

#[test]
fn inconsistent_worlds_are_refused() {
    let base: WorldSettlements = serde_json::from_str(SAMPLE).unwrap();
    let mut dup = base.clone();
    dup.settlements[1].id = 1;
    assert!(matches!(
        simulate_society(1, &dup),
        Err(SocietyError::Input(_))
    ));
    let mut bad_seat = base.clone();
    bad_seat.realms[0].seat = 99;
    assert!(matches!(
        simulate_society(1, &bad_seat),
        Err(SocietyError::Input(_))
    ));
    let mut bad_road = base.clone();
    bad_road.roads[0].to = Some(99);
    assert!(matches!(
        simulate_society(1, &bad_road),
        Err(SocietyError::Input(_))
    ));
    let mut stray = base;
    stray.settlements[2].realm_id = 8;
    let s = simulate_society(1, &stray).unwrap();
    assert_eq!(
        s.realms.len(),
        2,
        "a realm missing from the file is derived"
    );
}

#[test]
fn road_ids_are_u64_strings_as_the_settlement_stage_writes_them() {
    // arda-settle writes `"id": "1"` (I5); a u32 field rejected every file.
    let text = SAMPLE.replace(
        r#"{"id": 1, "class""#,
        r#"{"id": "18446744073709551615", "class""#,
    );
    let world: WorldSettlements = serde_json::from_str(&text).unwrap();
    assert_eq!(world.roads[0].id, u64::MAX);
    let s = simulate_society(9, &world).unwrap();
    let json = serde_json::to_string(&s).unwrap();
    assert!(
        !json.contains("18446744073709551615,"),
        "road ids leave as strings"
    );
}

#[test]
fn snake_case_biomes_keep_their_modifiers_and_unknown_ones_are_refused() {
    // arda-settle's Biome serialises snake_case (`temperate_forest`).
    for biome in ["temperate_forest", "boreal_forest", "warm_temperate"] {
        let text = SAMPLE.replacen(
            r#""biome": "temperate""#,
            &format!(r#""biome": "{biome}""#),
            1,
        );
        let world: WorldSettlements = serde_json::from_str(&text).unwrap();
        assert!(simulate_society(9, &world).is_ok(), "{biome}");
    }
    let text = SAMPLE.replacen(
        r#""biome": "temperate""#,
        r#""biome": "temperate forest""#,
        1,
    );
    let world: WorldSettlements = serde_json::from_str(&text).unwrap();
    assert!(matches!(
        simulate_society(9, &world),
        Err(SocietyError::Input(_))
    ));
}

#[test]
fn building_counts_beyond_the_population_are_refused() {
    // Each derived building is materialised: four billion houses would
    // exhaust memory instead of returning an error.
    let text = SAMPLE.replace(r#"{"farmhouse": 6}"#, r#"{"farmhouse": 4000000000}"#);
    let world: WorldSettlements = serde_json::from_str(&text).unwrap();
    assert!(matches!(
        simulate_society(9, &world),
        Err(SocietyError::Input(_))
    ));
}

#[test]
fn duplicate_or_orphan_explicit_buildings_are_refused() {
    let base: WorldSettlements = serde_json::from_str(SAMPLE).unwrap();
    let spec = |id, settlement_id| BuildingSpec {
        id,
        settlement_id,
        function: "house".to_string(),
        tags: vec![],
    };
    let mut dup = base.clone();
    dup.buildings = vec![spec(5, 2), spec(5, 2)];
    assert!(matches!(
        simulate_society(9, &dup),
        Err(SocietyError::Input(_))
    ));
    let mut orphan = base;
    orphan.buildings = vec![spec(5, 99)];
    assert!(matches!(
        simulate_society(9, &orphan),
        Err(SocietyError::Input(_))
    ));
}

#[test]
fn settle_realm_member_lists_are_read_and_cross_checked() {
    // arda-settle writes the member list as `settlements` (review round 2, #27).
    let ok = SAMPLE.replace(
        r#""seat": "1"}"#,
        r#""seat": "1", "settlements": ["1", "2", "3"]}"#,
    );
    let world: WorldSettlements = serde_json::from_str(&ok).unwrap();
    assert_eq!(world.realms[0].members, vec![1, 2, 3]);
    assert!(simulate_society(3, &world).is_ok());
    let bad = SAMPLE.replace(
        r#""seat": "1"}"#,
        r#""seat": "1", "settlements": ["1", "2"]}"#,
    );
    let world: WorldSettlements = serde_json::from_str(&bad).unwrap();
    assert!(matches!(
        simulate_society(3, &world),
        Err(SocietyError::Input(_))
    ));
}
