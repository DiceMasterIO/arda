//! Same inputs give byte-identical output; the JSON round-trips and uses
//! string ids (vocabulary.md I5, I17).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_society::output::to_json;
use arda_society::{simulate_society, Society};

#[test]
fn same_seed_same_bytes() {
    for seed in common::SEEDS {
        let (world, a) = common::build(seed);
        let b = simulate_society(seed, &world).unwrap();
        assert_eq!(to_json(&a).unwrap(), to_json(&b).unwrap(), "seed {seed}");
    }
}

#[test]
fn different_seeds_differ() {
    let (world, a) = common::build(42);
    let b = simulate_society(43, &world).unwrap();
    assert_ne!(to_json(&a).unwrap(), to_json(&b).unwrap());
}

#[test]
fn json_round_trips() {
    let (_, s) = common::build(42);
    let text = to_json(&s).unwrap();
    let back: Society = serde_json::from_str(&text).unwrap();
    assert_eq!(back, s);
}

#[test]
fn ids_and_seed_are_json_strings() {
    let (_, s) = common::build(42);
    let v: serde_json::Value = serde_json::from_str(&to_json(&s).unwrap()).unwrap();
    assert!(v["seed"].is_string());
    assert!(v["settlements"][0]["id"].is_string());
    assert!(v["settlements"][0]["realm_id"].is_string());
    assert!(v["settlements"][0]["buildings"][0]["id"].is_string());
    assert!(v["realms"][0]["id"].is_string());
    assert!(v["realms"][0]["seat"].is_string());
    assert!(v["relations"][0]["a"].is_string());
}
