//! Towns are sized by the realm they end up in, and the city cap moves
//! people instead of deleting them (review 3.3-8, 3.3-9; logic/08
//! §settle-tiers, §realm-seats).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_settle::model::{Settlement, Tier, CITY_MIN};
use arda_settle::primacy::{self, MAX_CITIES};
use arda_settle::{run, synthetic, PlaceParams};

fn total(s: &[Settlement]) -> u64 {
    s.iter().map(|x| u64::from(x.population)).sum()
}

#[test]
fn the_city_cap_moves_people_to_the_realm_instead_of_deleting_them() {
    // One realm: the seat and two more cities, villages with room to grow.
    let mut s = vec![
        Settlement::bare(1, 0, 0, Tier::City, 12_000),
        Settlement::bare(2, 9, 0, Tier::City, 10_500),
        Settlement::bare(3, 19, 0, Tier::City, 9_000),
        Settlement::bare(4, 29, 0, Tier::Town, 3_000),
    ];
    for k in 0..40 {
        s.push(Settlement::bare(5 + k, 1, 1, Tier::Village, 300));
    }
    let before = total(&s);
    let slots = vec![0; s.len()];
    primacy::cap(&mut s, &[0], &slots);
    let cities = s.iter().filter(|x| x.tier == Tier::City).count();
    assert_eq!(cities, MAX_CITIES);
    assert_eq!(s[2].tier, Tier::Town);
    assert_eq!(s[2].population, CITY_MIN - 1);
    assert_eq!(total(&s), before, "nobody is deleted");
    assert!(s[4..]
        .iter()
        .all(|v| v.tier == Tier::Village && v.population <= 999));
    assert!(
        s[4..].iter().any(|v| v.population > 300),
        "villages took the surplus"
    );
}

#[test]
fn a_full_realm_still_keeps_everyone() {
    // Villages already at their ceiling: the surplus goes to the towns and
    // finally to the seat, never into thin air.
    let mut s = vec![
        Settlement::bare(1, 0, 0, Tier::City, 9_000),
        Settlement::bare(2, 9, 0, Tier::City, 8_500),
        Settlement::bare(3, 19, 0, Tier::City, 30_000),
        Settlement::bare(4, 29, 0, Tier::Village, 999),
    ];
    let before = total(&s);
    let slots = vec![0; s.len()];
    primacy::cap(&mut s, &[0], &slots);
    assert_eq!(
        s.iter().filter(|x| x.tier == Tier::City).count(),
        MAX_CITIES
    );
    assert_eq!(total(&s), before);
}

#[test]
fn every_seat_is_the_largest_settlement_of_the_realm_it_ends_up_in() {
    let params = PlaceParams {
        seed: 42,
        density_per_km2: 15,
    };
    let grid = synthetic::landscape(params.seed).unwrap();
    let soc = run(&grid, params).unwrap();
    for r in &soc.realms.realms {
        let seat = soc
            .settlements
            .iter()
            .find(|x| x.id.get() == r.seat)
            .unwrap();
        for s in soc.settlements.iter().filter(|x| x.realm_id.get() == r.id) {
            assert!(
                s.population <= seat.population,
                "realm {}: {} ({}) outgrows its seat {} ({})",
                r.id,
                s.id,
                s.population,
                seat.id,
                seat.population
            );
        }
        let cities = soc
            .settlements
            .iter()
            .filter(|x| x.realm_id.get() == r.id && x.tier == Tier::City)
            .count();
        assert!(cities <= MAX_CITIES);
    }
}
