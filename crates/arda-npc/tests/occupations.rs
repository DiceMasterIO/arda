//! The occupation mix is realistic for tier, functions and wealth.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_npc::input::{BuildingFunction, BuildingSpec, SettlementProfile};
use arda_npc::{generate_population, JobCategory, Population};
use common::{city, hamlet, port_town, town, village, SEED};

struct Mix {
    population: Population,
    working: u32,
}

impl Mix {
    fn new(profile: &SettlementProfile, buildings: &[BuildingSpec]) -> Self {
        let population = generate_population(SEED, profile, buildings).unwrap();
        let working = population
            .category_counts()
            .iter()
            .filter(|(c, _)| *c != JobCategory::Dependent)
            .map(|&(_, n)| n)
            .sum();
        Self {
            population,
            working,
        }
    }

    fn share(&self, category: JobCategory) -> f64 {
        let n = self
            .population
            .category_counts()
            .iter()
            .find(|(c, _)| *c == category)
            .map_or(0, |&(_, n)| n);
        f64::from(n) / f64::from(self.working)
    }

    fn count(&self, job: &str) -> usize {
        self.population.by_job(job).len()
    }
}

fn buildings_of(buildings: &[BuildingSpec], function: BuildingFunction) -> usize {
    buildings.iter().filter(|b| b.function == function).count()
}

#[test]
fn farming_dominates_hamlets_and_villages() {
    for (profile, buildings) in [hamlet(), village()] {
        let mix = Mix::new(&profile, &buildings);
        let farming = mix.share(JobCategory::Agriculture);
        assert!(
            (0.60..=0.85).contains(&farming),
            "{}: farming share {farming:.2}",
            profile.name
        );
    }
}

#[test]
fn towns_and_cities_are_not_farm_villages() {
    let (profile, buildings) = town();
    let mix = Mix::new(&profile, &buildings);
    let farming = mix.share(JobCategory::Agriculture);
    assert!(
        (0.05..=0.35).contains(&farming),
        "town farming share {farming:.2}"
    );
    assert!(
        mix.share(JobCategory::Craft) >= 0.05,
        "town crafts {:.2}",
        mix.share(JobCategory::Craft)
    );
    let (profile, buildings) = city(10_000);
    let mix = Mix::new(&profile, &buildings);
    let farming = mix.share(JobCategory::Agriculture);
    assert!(farming <= 0.10, "city farming share {farming:.2}");
}

#[test]
fn port_towns_live_on_the_water() {
    let (profile, buildings) = port_town();
    let mix = Mix::new(&profile, &buildings);
    let maritime = mix.share(JobCategory::Maritime);
    assert!(
        (0.15..=0.55).contains(&maritime),
        "maritime share {maritime:.2}"
    );
    assert!(mix.count("fisher") > 0 && mix.count("dockhand") > 0 && mix.count("harbourmaster") > 0);
}

#[test]
fn specialists_come_in_plausible_numbers() {
    for (profile, buildings) in [village(), town(), port_town(), city(10_000)] {
        let mix = Mix::new(&profile, &buildings);
        let people = f64::from(profile.population);
        let smiths =
            mix.count("smith") + mix.count("journeyman_smith") + mix.count("apprentice_smith");
        let masters = mix.count("smith");
        assert!(masters >= 1, "{}: no smith", profile.name);
        assert!(
            people / masters as f64 >= 150.0,
            "{}: one smith per {:.0}",
            profile.name,
            people / masters as f64
        );
        assert!(smiths <= masters * 3);
        assert_eq!(
            mix.count("innkeeper"),
            buildings_of(&buildings, BuildingFunction::Inn),
            "{}",
            profile.name
        );
        assert_eq!(
            mix.count("high_priest"),
            buildings_of(&buildings, BuildingFunction::Temple),
            "{}",
            profile.name
        );
        let clergy = mix.count("priest") + mix.count("acolyte");
        assert!(
            clergy >= buildings_of(&buildings, BuildingFunction::Temple),
            "{}: temples unstaffed",
            profile.name
        );
    }
}

#[test]
fn children_and_elders_have_no_real_job() {
    let (profile, buildings) = town();
    for npc in common::everyone(&profile, &buildings) {
        let race = &npc.ancestry;
        if npc.job.category == JobCategory::Dependent {
            assert!(
                npc.workplace_building.is_none(),
                "{race} dependant with a workplace"
            );
        }
        if npc.job.key == "child" {
            assert!(npc
                .relationships
                .iter()
                .all(|r| r.kind != arda_npc::RelationKind::Employer));
        }
    }
}

#[test]
fn notable_counts_follow_the_tier() {
    let cases = [
        (hamlet(), 2, 4),
        (village(), 6, 12),
        (town(), 20, 60),
        (port_town(), 20, 60),
        (city(12_000), 60, 200),
    ];
    for ((profile, buildings), lo, hi) in cases {
        let population = generate_population(SEED, &profile, &buildings).unwrap();
        let n = population.npcs.len();
        assert!(
            (lo..=hi).contains(&n),
            "{}: {n} notables, want {lo}–{hi}",
            profile.name
        );
        assert!(population
            .npcs
            .iter()
            .all(|npc| npc.notable && npc.sheet.level() > 0));
    }
}

#[test]
fn notable_levels_scale_with_tier() {
    let cases = [
        (hamlet(), 1, 5),
        (village(), 1, 6),
        (town(), 1, 11),
        (city(12_000), 2, 15),
    ];
    for ((profile, buildings), lo, hi) in cases {
        let population = generate_population(SEED, &profile, &buildings).unwrap();
        let levels: Vec<u8> = population.npcs.iter().map(|n| n.sheet.level()).collect();
        assert!(
            levels.iter().all(|l| (lo..=hi).contains(l)),
            "{}: levels {levels:?}",
            profile.name
        );
        let top = levels.iter().max().copied().unwrap_or(0);
        match profile.tier {
            arda_npc::Tier::Town => assert!(top >= 5, "town leaders reach {top}"),
            arda_npc::Tier::City => assert!(top >= 8, "city elites reach {top}"),
            _ => assert!(top <= 6, "{} top level {top}", profile.name),
        }
    }
}
