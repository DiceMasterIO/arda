//! Realms on real MICRO worlds (goals 35 and 38): every realm has one or two
//! cities, the seats stand apart and reach the people, the realms share the
//! land and the people within bounds, and the towns still follow the
//! rank-size rule.
//!
//! Seed 42 runs by default (the fixture `tests/micro.rs` also uses). Seeds 3
//! and 7 generate two more worlds and are ignored by default:
//! `cargo test --release -p arda-settle --test realms_micro -- --ignored`.
//! World resolution per seed: `$ARDA_TEST_WORLD_<seed>`, then
//! `<workspace>/out/micro<seed>`, else generated once into
//! `target/arda-server-fixture/micro<seed>`.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_settle::grid::MEMORY_BUDGET;
use arda_settle::Society;
use std::path::PathBuf;

fn world_dir(seed: u64) -> PathBuf {
    if let Some(dir) = std::env::var_os(format!("ARDA_TEST_WORLD_{seed}")) {
        return PathBuf::from(dir);
    }
    let loads = |dir: &PathBuf| {
        arda::World::load(dir)
            .is_ok_and(|w| w.manifest().fine_terrain.is_some() && w.seed() == seed)
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = root.join(format!("out/micro{seed}"));
    if loads(&out) {
        return out;
    }
    let dir = root.join(format!("target/arda-server-fixture/micro{seed}"));
    if !loads(&dir) {
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
        arda::generate_from_fine_source(
            seed,
            arda::GenerateConfig::MICRO,
            &dir,
            arda::FineDeliveryLimits::default(),
        )
        .expect("generating a MICRO fixture world");
    }
    dir
}

fn settle(seed: u64) -> Society {
    let world = arda::World::load(&world_dir(seed)).unwrap();
    let grid = arda_settle::load::load_grid(&world, MEMORY_BUDGET).unwrap();
    let params = arda_settle::PlaceParams {
        seed: world.seed(),
        density_per_km2: u32::from(world.manifest().config.mean_density_per_km2()),
    };
    arda_settle::run(&grid, params).unwrap()
}

fn check(seed: u64) {
    let s = settle(seed);
    let st = &s.stats;
    let r = &st.realm;
    let n = s.realms.realms.len();
    eprintln!(
        "seed {seed}: {n} realms, {r:?}, rank-size {} r² {}",
        st.rank_size_slope, st.rank_size_r2
    );
    assert!(n >= 2, "seed {seed}: {n} realms");
    // Goal 35: one or two cities per realm, and the seat is one of them.
    assert_eq!(r.cities_per_realm.len(), n);
    for (k, &c) in r.cities_per_realm.iter().enumerate() {
        assert!(
            (1..=2).contains(&c),
            "seed {seed}: realm {} has {c} cities",
            k + 1
        );
    }
    for realm in &s.realms.realms {
        let seat = s
            .settlements
            .iter()
            .find(|x| x.id.get() == realm.seat)
            .unwrap();
        assert_eq!(
            seat.tier,
            arda_settle::model::Tier::City,
            "seed {seed}: seat of {}",
            realm.id
        );
        // Goal 38: the seat is its realm's largest settlement.
        let largest = realm
            .settlements
            .iter()
            .filter_map(|id| s.settlements.iter().find(|x| x.id.get() == *id))
            .map(|x| x.population)
            .max()
            .unwrap();
        assert_eq!(
            seat.population, largest,
            "seed {seed}: seat of realm {}",
            realm.id
        );
    }
    // Seat spread: no two seats closer than 0.4 of the even lattice
    // spacing, and nine people in ten within 0.85 of it of a seat.
    assert!(
        r.seat_nn_min_km >= 0.4 * r.seat_lattice_km,
        "seed {seed}: seats {} km apart, lattice {} km",
        r.seat_nn_min_km,
        r.seat_lattice_km
    );
    assert!(
        r.people_to_seat_p90_km <= 0.85 * r.seat_lattice_km,
        "seed {seed}: p90 {} km from a seat, lattice {} km",
        r.people_to_seat_p90_km,
        r.seat_lattice_km
    );
    // Balance: no realm holds more than twice its fair share of the land,
    // the largest holds at most 4.5 times the smallest, and people are
    // shared within a factor of 2.5.
    let fair = 1000 / u32::try_from(n).unwrap();
    assert!(r.land_share_max_pm <= 2 * fair, "seed {seed}: {r:?}");
    assert!(
        r.land_share_max_pm * 2 <= r.land_share_min_pm * 9,
        "seed {seed}: {r:?}"
    );
    assert!(
        r.people_share_max_pm * 2 <= r.people_share_min_pm * 5,
        "seed {seed}: {r:?}"
    );
    // Goal 35: the rank-size fit still holds over the whole land.
    assert!(
        (-1.2..=-0.8).contains(&st.rank_size_slope),
        "seed {seed}: slope {}",
        st.rank_size_slope
    );
    assert!(
        st.rank_size_r2 >= 0.8,
        "seed {seed}: r² {}",
        st.rank_size_r2
    );
}

#[test]
fn seed_42_realms_have_cities_spread_seats_and_balance() {
    check(42);
}

#[test]
#[ignore = "generates the seed-3 MICRO world"]
fn seed_3_realms_have_cities_spread_seats_and_balance() {
    check(3);
}

#[test]
#[ignore = "generates the seed-7 MICRO world"]
fn seed_7_realms_have_cities_spread_seats_and_balance() {
    check(7);
}
