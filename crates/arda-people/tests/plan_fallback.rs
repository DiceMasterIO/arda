//! A plan that cannot house its settlement falls back to the building mix,
//! a city's plan sees enough ground, and off-plan cottages carry the
//! settlement's wealth (review 3.3-11; logic/21 §build-plan-site,
//! §build-buildings).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_people::town;
use arda_settle::model::{Settlement, Tier};

fn plan() -> arda_town::TownPlan {
    let (site, terrain) = arda_town::samples::aldermere();
    arda_town::generate(&site, &terrain, 7).unwrap()
}

#[test]
fn a_city_plan_sees_ground_in_proportion_to_its_people() {
    let small = Settlement::bare(1, 0, 0, Tier::City, 8_000);
    assert_eq!(
        town::plan_radius_for(&small),
        town::plan_radius_m(Tier::City)
    );
    let big = Settlement::bare(2, 0, 0, Tier::City, 120_000);
    let r = town::plan_radius_for(&big);
    assert!(r > town::plan_radius_m(Tier::City), "{r}");
    // Ground for 120,000 people at a city's density (logic/08 §landuse:
    // 150 a hectare) fits inside the window.
    let need_ha = 120_000.0 / 150.0;
    assert!(std::f64::consts::PI * r * r / 10_000.0 >= need_ha, "{r}");
}

#[test]
fn off_plan_cottages_carry_the_settlement_wealth() {
    let p = plan();
    let housed = town::housed(&p);
    let specs = town::plan_specs(&p, u32::try_from(housed).unwrap() + 40, 201);
    let off: Vec<_> = specs
        .iter()
        .filter(|b| b.tags.iter().any(|t| t == "off_plan"))
        .collect();
    assert!(!off.is_empty());
    assert!(off.iter().all(|b| b.wealth == 201));
}

#[test]
fn a_plan_housing_under_half_its_people_is_refused() {
    let p = plan();
    let housed = u32::try_from(town::housed(&p)).unwrap();
    assert!(town::usable(&p, housed));
    assert!(town::usable(&p, housed * 2));
    assert!(!town::usable(&p, housed * 2 + 2), "the mix must stand in");
}
