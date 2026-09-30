//! `npc()` regenerates any person exactly as the full population has them.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_npc::{commoner, generate_population, npc, Generator, NpcError, NpcId};
use common::{city, hamlet, port_town, town, SEED};

#[test]
fn every_notable_regenerates_identically() {
    for (profile, buildings) in [hamlet(), town(), port_town()] {
        let population = generate_population(SEED, &profile, &buildings).unwrap();
        assert!(!population.npcs.is_empty());
        let generator = Generator::new(SEED, &profile, &buildings).unwrap();
        for stored in &population.npcs {
            let (home, index) = generator.locate(stored.id).unwrap();
            assert_eq!(home, stored.home_building);
            assert_eq!(stored.id, NpcId::from_parts(profile.id, home, index));
            let again = npc(SEED, &profile, &buildings, stored.id).unwrap();
            assert_eq!(&again, stored, "notable {:?} differs", stored.id);
        }
    }
}

#[test]
fn sampled_commoners_regenerate_identically() {
    let (profile, buildings) = town();
    let generator = Generator::new(SEED, &profile, &buildings).unwrap();
    let population = generator.population().unwrap();
    let mut checked = 0;
    for entry in population.roster.iter().filter(|r| !r.notable).step_by(23) {
        let full = generator.npc(entry.id).unwrap();
        let fresh = npc(SEED, &profile, &buildings, entry.id).unwrap();
        let (building, index) = generator.locate(entry.id).unwrap();
        assert_eq!(entry.id, NpcId::from_parts(profile.id, building, index));
        let by_building = commoner(SEED, &profile, &buildings, building, index).unwrap();
        assert_eq!(full, fresh);
        assert_eq!(full, by_building);
        assert!(!full.notable);
        assert_eq!(population.job_of(entry.id), Some(full.job.key.as_str()));
        checked += 1;
    }
    assert!(checked > 50, "only {checked} commoners sampled");
}

#[test]
fn commoners_of_a_large_city_regenerate() {
    let (profile, buildings) = city(12_000);
    let generator = Generator::new(SEED, &profile, &buildings).unwrap();
    for id in generator.ids().step_by(997) {
        assert_eq!(
            generator.npc(id).unwrap(),
            npc(SEED, &profile, &buildings, id).unwrap()
        );
    }
}

#[test]
fn unknown_ids_are_refused() {
    let (profile, buildings) = hamlet();
    let bad_index = NpcId::from_parts(profile.id, buildings[0].id, 9_999);
    assert_eq!(
        npc(SEED, &profile, &buildings, bad_index),
        Err(NpcError::UnknownNpc(bad_index))
    );
    assert_eq!(
        commoner(SEED, &profile, &buildings, buildings[0].id, 9_999),
        Err(NpcError::UnknownNpc(bad_index))
    );
    let bad_settlement = NpcId::from_parts(arda_npc::SettlementId(99), buildings[0].id, 0);
    assert!(matches!(
        npc(SEED, &profile, &buildings, bad_settlement),
        Err(NpcError::UnknownNpc(_))
    ));
}

#[test]
fn bad_inputs_are_refused() {
    let (mut profile, mut buildings) = hamlet();
    profile.population = 100_000;
    assert!(matches!(
        generate_population(SEED, &profile, &buildings),
        Err(NpcError::NotEnoughHousing { .. })
    ));
    let (profile, _) = hamlet();
    let duplicate = buildings[0].clone();
    buildings.push(duplicate);
    assert!(matches!(
        generate_population(SEED, &profile, &buildings),
        Err(NpcError::DuplicateBuilding(_))
    ));
}
