//! Shared fixtures: sample settlements of every tier and the raw SRD JSON.

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use arda_npc::input::{BuildingSpec, SettlementFunction as S, SettlementProfile, Tier};
use arda_npc::{sample, Generator, Npc};
use serde_json::Value;

pub const SEED: u64 = 0x5EED_0003;

pub fn hamlet() -> (SettlementProfile, Vec<BuildingSpec>) {
    make(sample::profile(
        1,
        "Thornby",
        Tier::Hamlet,
        60,
        &[S::Farming, S::Pastoral],
        70,
        "heartland",
    ))
}

pub fn village() -> (SettlementProfile, Vec<BuildingSpec>) {
    make(sample::profile(
        2,
        "Oakmere",
        Tier::Village,
        420,
        &[S::Farming, S::Market],
        110,
        "highland",
    ))
}

pub fn town() -> (SettlementProfile, Vec<BuildingSpec>) {
    make(sample::profile(
        3,
        "Wendlebrook",
        Tier::Town,
        1_500,
        &[S::Market, S::Crafting, S::Farming],
        140,
        "heartland",
    ))
}

pub fn port_town() -> (SettlementProfile, Vec<BuildingSpec>) {
    make(sample::profile(
        4,
        "Saltmarch",
        Tier::Town,
        2_400,
        &[S::Port, S::Fishing, S::Market],
        130,
        "coastal",
    ))
}

pub fn city(population: u32) -> (SettlementProfile, Vec<BuildingSpec>) {
    let functions = [S::Capital, S::Market, S::Crafting, S::Port, S::Fortress];
    make(sample::profile(
        5,
        "Highgate",
        Tier::City,
        population,
        &functions,
        170,
        "southern",
    ))
}

fn make(profile: SettlementProfile) -> (SettlementProfile, Vec<BuildingSpec>) {
    let buildings = sample::buildings(SEED, &profile);
    (profile, buildings)
}

/// Every inhabitant, expanded.
pub fn everyone(profile: &SettlementProfile, buildings: &[BuildingSpec]) -> Vec<Npc> {
    let generator = Generator::new(SEED, profile, buildings).unwrap();
    generator
        .ids()
        .map(|id| generator.npc(id).unwrap())
        .collect()
}

pub fn srd(file: &str) -> Value {
    let path = format!("{}/data/srd/{file}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

pub fn content(file: &str) -> Value {
    let path = format!("{}/data/content/{file}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// SRD modifier, recomputed independently of the crate.
pub fn modifier(score: u8) -> i8 {
    i8::try_from((i16::from(score) - 10).div_euclid(2)).unwrap()
}
