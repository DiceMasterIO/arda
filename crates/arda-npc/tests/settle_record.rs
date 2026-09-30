//! A settlement record written by `arda-settle` (feat/settlements-roads,
//! `society/settlements.json`) deserialises straight into a
//! `SettlementProfile` (integration plan A2): ids are JSON strings, the
//! biome is the closed snake_case list, and the extra record fields are
//! ignored. Ids and seeds serialise as strings (vocabulary I5).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_npc::{
    generate_population, Biome, BuildingId, NpcId, RealmId, SettlementFunction as S, SettlementId,
    SettlementProfile, Tier,
};
use serde_json::{json, Value};

/// One record in the format of feat/settlements-roads `model::Settlement`.
const RECORD: &str = r#"{
  "id": "6",
  "name": "Stowndcote",
  "tier": "village",
  "population": 374,
  "functions": ["farming", "fishing", "port"],
  "wealth": 134,
  "culture": "heartland",
  "realm_id": "2",
  "biome": "coastal",
  "coastal": true,
  "riverine": false,
  "x_m": 5650,
  "y_m": 20850,
  "cell_x": 56,
  "cell_y": 208,
  "height_m": 160,
  "rank": 0,
  "site_tags": ["harbour", "defensible", "fish", "coast", "hill"],
  "history": "grew around a sheltered harbour",
  "buildings": {"boathouse": 2, "cottage": 46, "dock": 1, "farmhouse": 37}
}"#;

#[test]
fn a_settle_record_deserialises_into_a_profile() {
    let profile: SettlementProfile = serde_json::from_str(RECORD).unwrap();
    assert_eq!(profile.id, SettlementId(6));
    assert_eq!(profile.name, "Stowndcote");
    assert_eq!(profile.tier, Tier::Village);
    assert_eq!(profile.population, 374);
    assert_eq!(profile.functions, [S::Farming, S::Fishing, S::Port]);
    assert_eq!(profile.realm_id, RealmId(2));
    assert_eq!(profile.biome, Biome::Coastal);
    assert!(profile.coastal && !profile.riverine);
    assert_eq!(profile.ancestry_mix, None);
    // The profile is usable as-is.
    let buildings = arda_npc::sample::buildings(common::SEED, &profile);
    let population = generate_population(common::SEED, &profile, &buildings).unwrap();
    assert_eq!(population.population, 374);
}

#[test]
fn every_settle_biome_is_accepted_and_free_text_is_refused() {
    let mut record: Value = serde_json::from_str(RECORD).unwrap();
    for biome in Biome::ALL {
        record["biome"] = json!(biome.key());
        let profile: SettlementProfile = serde_json::from_value(record.clone()).unwrap();
        assert_eq!(profile.biome, biome);
    }
    record["biome"] = json!("boreal forest");
    assert!(serde_json::from_value::<SettlementProfile>(record).is_err());
}

#[test]
fn ids_are_json_strings_and_numbers_still_read() {
    let mut record: Value = serde_json::from_str(RECORD).unwrap();
    let profile: SettlementProfile = serde_json::from_value(record.clone()).unwrap();
    let out = serde_json::to_value(&profile).unwrap();
    assert_eq!(out["id"], json!("6"));
    assert_eq!(out["realm_id"], json!("2"));
    record["id"] = json!(6);
    record["realm_id"] = json!(2);
    assert_eq!(
        serde_json::from_value::<SettlementProfile>(record).unwrap(),
        profile
    );
    let id = NpcId::from_parts(SettlementId(u64::MAX), BuildingId((1 << 53) + 1), 7);
    let text = serde_json::to_value(id).unwrap();
    assert!(text.is_string(), "{text}");
    assert_eq!(serde_json::from_value::<NpcId>(text).unwrap(), id);
}
