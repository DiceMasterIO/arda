//! Storage follows the notable count, not the population; and (ignored,
//! release gate) a 20,000-person city generates in under a second.
//!
//! The wall-clock bound lives in its own `#[ignore]` test: on a loaded box
//! running the whole workspace in parallel it flaked (review round 1 #18).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::time::{Duration, Instant};

use arda_npc::generate_population;
use common::{city, SEED};

#[test]
fn city_of_twenty_thousand_stores_few_records() {
    let (profile, buildings) = city(20_000);
    let population = generate_population(SEED, &profile, &buildings).unwrap();
    assert_eq!(population.population, 20_000);
    assert!(
        population.npcs.len() <= 200,
        "{} notables stored",
        population.npcs.len()
    );
    assert_eq!(
        population.npcs.len() + usize::try_from(population.commoner_count).unwrap(),
        20_000
    );

    let (small_profile, small_buildings) = city(8_000);
    let small = generate_population(SEED, &small_profile, &small_buildings).unwrap();
    // 2.5× the people must not mean 2.5× the stored records.
    assert!(
        population.npcs.len() < small.npcs.len() * 2,
        "{} vs {}",
        population.npcs.len(),
        small.npcs.len()
    );
}

/// Run with `cargo test --release -p arda-npc --test performance -- --ignored`
/// on a quiet machine: the best of three runs stays under a second.
#[test]
#[ignore = "wall-clock timing; release gate"]
fn city_of_twenty_thousand_is_fast() {
    let (profile, buildings) = city(20_000);
    let _warm = generate_population(SEED, &profile, &buildings[..0]).err();
    let best = (0..3)
        .map(|_| {
            let start = Instant::now();
            let population = generate_population(SEED, &profile, &buildings).unwrap();
            assert_eq!(population.population, 20_000);
            start.elapsed()
        })
        .min()
        .unwrap();
    let limit = if cfg!(debug_assertions) {
        Duration::from_secs(5)
    } else {
        Duration::from_secs(1)
    };
    assert!(best < limit, "20k city took {best:?}");
}
