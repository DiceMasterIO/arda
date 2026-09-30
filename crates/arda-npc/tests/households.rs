//! Homes, households, ages and relationships are coherent.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::collections::{BTreeMap, BTreeSet};

use arda_npc::{generate_population, Npc, NpcId, RelationKind};
use common::{city, everyone, hamlet, port_town, srd, town, village, SEED};

#[test]
fn every_workplace_slot_is_filled_and_every_home_exists() {
    for (profile, buildings) in [hamlet(), village(), town(), port_town()] {
        let population = generate_population(SEED, &profile, &buildings).unwrap();
        let ids: BTreeSet<_> = buildings.iter().map(|b| b.id).collect();
        for b in buildings.iter().filter(|b| b.workplace_slots > 0) {
            let workers = population.workers(b.id);
            assert!(
                workers.len() >= usize::from(b.workplace_slots),
                "{}: {:?} #{} has {} of {} workers",
                profile.name,
                b.function,
                b.id.0,
                workers.len(),
                b.workplace_slots
            );
        }
        let mut housed = 0;
        for household in &population.households {
            assert!(ids.contains(&household.home));
            let capacity = buildings
                .iter()
                .find(|b| b.id == household.home)
                .unwrap()
                .capacity;
            assert!(population.residents(household.home).len() <= usize::from(capacity));
            housed += household.members.len();
        }
        assert_eq!(housed, usize::try_from(profile.population).unwrap());
        assert_eq!(population.roster.len(), housed);
    }
}

fn check_people(people: &[Npc]) {
    let races = srd("races.json");
    let by_id: BTreeMap<NpcId, &Npc> = people.iter().map(|n| (n.id, n)).collect();
    for npc in people {
        let race = races
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["key"] == npc.ancestry.as_str())
            .unwrap();
        let max_age = race["max_age"].as_u64().unwrap();
        assert!(
            u64::from(npc.age) < max_age,
            "{} is {} (max {max_age})",
            npc.name.full(),
            npc.age
        );
        assert_eq!(npc.home_building, npc.household.building);
        for rel in &npc.relationships {
            let other = by_id
                .get(&rel.other)
                .unwrap_or_else(|| panic!("dangling relation {rel:?}"));
            let back = other
                .relationships
                .iter()
                .any(|r| r.other == npc.id && r.kind == rel.kind.inverse());
            assert!(
                back,
                "{:?} of {} is not mirrored",
                rel.kind,
                npc.name.full()
            );
            assert_ne!(rel.other, npc.id, "self relation");
            match rel.kind {
                RelationKind::Parent => {
                    assert!(
                        other.age >= npc.age + 16,
                        "parent {} vs child {}",
                        other.age,
                        npc.age
                    );
                    assert_eq!(other.household, npc.household);
                }
                RelationKind::Spouse
                | RelationKind::Sibling
                | RelationKind::Landlord
                | RelationKind::Lodger => {
                    assert_eq!(other.household, npc.household);
                }
                RelationKind::Employer => {
                    assert_eq!(npc.employer, Some(rel.other));
                    assert_eq!(other.workplace_building, npc.workplace_building);
                }
                RelationKind::Friend | RelationKind::Rival => {
                    assert_ne!(other.household, npc.household);
                }
                RelationKind::Child | RelationKind::Employee => {}
            }
            if matches!(
                rel.kind,
                RelationKind::Spouse
                    | RelationKind::Child
                    | RelationKind::Sibling
                    | RelationKind::Parent
            ) && npc.name.family != other.name.family
            {
                panic!(
                    "{} and {} share a family but not a name",
                    npc.name.full(),
                    other.name.full()
                );
            }
        }
        let friends = npc
            .relationships
            .iter()
            .filter(|r| r.kind == RelationKind::Friend)
            .count();
        let rivals = npc
            .relationships
            .iter()
            .filter(|r| r.kind == RelationKind::Rival)
            .count();
        assert!(friends <= 1 && rivals <= 1);
    }
}

#[test]
fn households_ages_and_relationships_are_consistent() {
    for (profile, buildings) in [hamlet(), village(), town(), port_town()] {
        let people = everyone(&profile, &buildings);
        check_people(&people);
        let with_friend = people
            .iter()
            .filter(|n| {
                n.relationships
                    .iter()
                    .any(|r| r.kind == RelationKind::Friend)
            })
            .count();
        assert!(
            with_friend * 2 > people.len(),
            "{}: few friendships",
            profile.name
        );
    }
}

#[test]
fn household_records_match_the_people() {
    let (profile, buildings) = village();
    let population = generate_population(SEED, &profile, &buildings).unwrap();
    let people: BTreeMap<NpcId, Npc> = everyone(&profile, &buildings)
        .into_iter()
        .map(|n| (n.id, n))
        .collect();
    for household in &population.households {
        let head = &people[&household.members[0]];
        assert_eq!(head.name.family, household.family_name);
        for m in &household.members {
            assert_eq!(people[m].household, household.id);
        }
        for rel in &household.relations {
            let from = &people[&rel.from];
            assert!(from
                .relationships
                .iter()
                .any(|r| r.kind == rel.kind && r.other == rel.to));
        }
    }
}

#[test]
fn cities_hold_mixed_ancestries_and_families() {
    let (profile, buildings) = city(9_000);
    let people = everyone(&profile, &buildings);
    let ancestries: BTreeSet<&str> = people.iter().map(|n| n.ancestry.as_str()).collect();
    assert!(ancestries.len() >= 6, "{ancestries:?}");
    let humans = people.iter().filter(|n| n.ancestry == "human").count();
    assert!(humans * 2 > people.len(), "human majority expected");
    let children = people.iter().filter(|n| n.job.key == "child").count();
    assert!(children * 6 > people.len(), "only {children} children");
}
