//! Every mirror reads the domain JSON and writes it back unchanged
//! (`logic/16` §api-bindings: "a test per mirror").

use super::*;
use arda_npc::sample::{self, market_town};
use arda_npc::{Generator, SettlementFunction as S, Tier as T};
use serde::de::DeserializeOwned;

fn same<M: DeserializeOwned + Serialize, D: Serialize>(domain: &D) -> M {
    let json = serde_json::to_value(domain).unwrap();
    let mirror: M = serde_json::from_value(json.clone())
        .unwrap_or_else(|e| panic!("{e}: {}", std::any::type_name::<M>()));
    assert_eq!(serde_json::to_value(&mirror).unwrap(), json);
    mirror
}

#[test]
fn demo_inputs_and_population_match_their_mirrors() {
    let (seed, settlement, buildings) = market_town();
    let _: SettlementProfile = same(&settlement);
    let mut spoken = settlement.clone();
    spoken.tongue = Some(arda_names::Tongue {
        preset: arda_names::Preset::Sylvan,
        language_seed: u64::MAX,
        substrate_seed: 2,
        dialect_seed: 3,
        width: 1024,
        height: 2048,
        x: 10,
        y: 20,
    });
    let _: SettlementProfile = same(&spoken);
    let _: Vec<BuildingSpec> = same(&buildings);
    let population = Generator::new(seed, &settlement, &buildings)
        .unwrap()
        .population()
        .unwrap();
    let mirror: Population = same(&population);
    assert!(!mirror.npcs.is_empty());
    assert!(mirror.npcs.iter().any(|n| n.sheet.spellcasting.is_some()));
    assert!(mirror
        .npcs
        .iter()
        .any(|n| matches!(n.sheet.kind, SheetKind::Class { .. })));
}

#[test]
fn commoners_and_stat_block_sheets_match_their_mirrors() {
    let profile = sample::profile(
        3,
        "Mirrorby",
        T::Village,
        300,
        &[S::Farming],
        90,
        "heartland",
    );
    let buildings = sample::buildings(11, &profile);
    let generator = Generator::new(11, &profile, &buildings).unwrap();
    let mut stat_blocks = 0;
    for id in generator.ids() {
        let npc = generator.npc(id).unwrap();
        let mirror: Npc = same(&npc);
        stat_blocks += usize::from(matches!(mirror.sheet.kind, SheetKind::StatBlock { .. }));
    }
    assert!(stat_blocks > 0);
}

#[test]
fn ids_are_strings_in_the_mirrors() {
    let (_, settlement, buildings) = market_town();
    let json = serde_json::to_value(&settlement).unwrap();
    assert!(json["id"].is_string() && json["realm_id"].is_string());
    let json = serde_json::to_value(&buildings[0]).unwrap();
    assert!(json["id"].is_string() && json["settlement_id"].is_string());
}
