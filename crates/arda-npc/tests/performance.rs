//! A 20,000-person city generates in under a second (release), and storage
//! follows the notable count, not the population.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::time::{Duration, Instant};

use arda_npc::generate_population;
use common::{city, SEED};

#[test]
fn city_of_twenty_thousand_is_fast_and_small() {
    let (profile, buildings) = city(20_000);
    let _warm = generate_population(SEED, &profile, &buildings[..0]).err();
    let start = Instant::now();
    let population = generate_population(SEED, &profile, &buildings).unwrap();
    let elapsed = start.elapsed();
    let limit = if cfg!(debug_assertions) {
        Duration::from_secs(5)
    } else {
        Duration::from_secs(1)
    };
    assert!(elapsed < limit, "20k city took {elapsed:?}");
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
