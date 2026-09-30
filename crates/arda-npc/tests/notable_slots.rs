//! Society slots feed the notables (logic/14 §soc-offices, adapter A14),
//! and names come from `arda-names` in the settlement's tongue (A5, I18).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_npc::sample::market_town;
use arda_npc::{BuildingFunction, Generator, NotableSlot, Sex};

fn slot(id: &str, kind: &str, title: &str) -> NotableSlot {
    NotableSlot {
        id: id.into(),
        kind: kind.into(),
        title: title.into(),
        building: None,
        given_name: None,
        family_name: None,
        female: None,
    }
}

#[test]
fn slots_bind_to_their_building_with_the_names_history_gave() {
    let (seed, town, buildings) = market_town();
    let temple = buildings
        .iter()
        .find(|b| b.function == BuildingFunction::Temple)
        .unwrap()
        .id;
    let mut lord = slot("r7.lord", "lord", "Lord of Wendlebrook");
    lord.given_name = Some("Aldric".into());
    lord.family_name = Some("Varnhold".into());
    lord.female = Some(true);
    let mut priest = slot("r7.high_priest", "high_priest", "High Priest");
    priest.building = Some(temple);
    let slots = vec![lord, priest];
    let g = Generator::with_notables(seed, &town, &buildings, &slots).unwrap();
    let pop = g.population().unwrap();
    let held = pop
        .npcs
        .iter()
        .find(|n| n.job.title == "Lord of Wendlebrook")
        .expect("the lord is a stored notable");
    assert_eq!(held.name.given, "Aldric");
    assert_eq!(held.name.family, "Varnhold");
    assert_eq!(held.sex, Sex::Female);
    // The ruling family's name reaches the holder's household.
    let house = pop
        .households
        .iter()
        .find(|h| h.members.contains(&held.id))
        .unwrap();
    if house.members[0] == held.id {
        assert_eq!(house.family_name, "Varnhold");
        let kin = house
            .members
            .iter()
            .filter(|&&m| g.npc(m).unwrap().name.family == "Varnhold")
            .count();
        assert!(
            kin * 2 >= house.members.len(),
            "{kin} of {}",
            house.members.len()
        );
    }
    let priest = pop
        .npcs
        .iter()
        .find(|n| n.job.title == "High Priest")
        .expect("the priest is a stored notable");
    assert_eq!(priest.workplace_building, Some(temple));
    // Deterministic, and regeneration agrees with the stored notable.
    let again = Generator::with_notables(seed, &town, &buildings, &slots).unwrap();
    assert_eq!(again.npc(held.id).unwrap(), *held);
}

#[test]
fn names_follow_the_settlement_tongue() {
    let (seed, town, buildings) = market_town();
    let plain = Generator::new(seed, &town, &buildings).unwrap();
    let mut spoken = town.clone();
    spoken.tongue = Some(arda_names::Tongue {
        preset: arda_names::Preset::Coastal,
        language_seed: 11,
        substrate_seed: 12,
        dialect_seed: 13,
        width: 1000,
        height: 2000,
        x: 700,
        y: 900,
    });
    let local = Generator::new(seed, &spoken, &buildings).unwrap();
    let ids: Vec<_> = plain.ids().take(40).collect();
    let differ = ids
        .iter()
        .filter(|&&id| plain.npc(id).unwrap().name != local.npc(id).unwrap().name)
        .count();
    assert!(differ > 20, "{differ} of 40 names changed with the tongue");
    for &id in &ids {
        let n = local.npc(id).unwrap().name;
        assert!(!n.given.is_empty(), "{id}");
        assert_eq!(
            n,
            Generator::new(seed, &spoken, &buildings)
                .unwrap()
                .npc(id)
                .unwrap()
                .name
        );
    }
}
