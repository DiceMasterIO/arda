//! Same inputs give the same people, whatever order the buildings arrive in.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_npc::{generate_population, Generator};
use common::{town, village, SEED};

#[test]
fn identical_across_runs() {
    let (profile, buildings) = town();
    let a = generate_population(SEED, &profile, &buildings).unwrap();
    let b = generate_population(SEED, &profile, &buildings).unwrap();
    assert_eq!(a, b);
    let json_a = serde_json::to_string(&a).unwrap();
    let json_b = serde_json::to_string(&b).unwrap();
    assert_eq!(json_a, json_b);
}

#[test]
fn building_order_does_not_matter() {
    let (profile, buildings) = village();
    let mut reversed = buildings.clone();
    reversed.reverse();
    let mut interleaved: Vec<_> = buildings
        .iter()
        .step_by(2)
        .chain(buildings.iter().skip(1).step_by(2))
        .cloned()
        .collect();
    interleaved.rotate_left(7);
    let base = generate_population(SEED, &profile, &buildings).unwrap();
    assert_eq!(
        base,
        generate_population(SEED, &profile, &reversed).unwrap()
    );
    assert_eq!(
        base,
        generate_population(SEED, &profile, &interleaved).unwrap()
    );
}

#[test]
fn expansion_order_does_not_matter() {
    let (profile, buildings) = village();
    let generator = Generator::new(SEED, &profile, &buildings).unwrap();
    let ids: Vec<_> = generator.ids().collect();
    let forward: Vec<_> = ids.iter().map(|&id| generator.npc(id).unwrap()).collect();
    let mut backward: Vec<_> = ids
        .iter()
        .rev()
        .map(|&id| generator.npc(id).unwrap())
        .collect();
    backward.reverse();
    assert_eq!(forward, backward);
}

#[test]
fn seed_changes_the_people() {
    let (profile, buildings) = village();
    let a = generate_population(SEED, &profile, &buildings).unwrap();
    let b = generate_population(SEED + 1, &profile, &buildings).unwrap();
    assert_ne!(a.npcs, b.npcs);
}
