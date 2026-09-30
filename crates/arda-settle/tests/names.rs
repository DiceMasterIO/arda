//! Names come from `arda-names`: deterministic, glossed and unique across
//! the whole world.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_names::scope::skeleton;
use arda_settle::{run, synthetic, PlaceParams, Society};
use std::collections::BTreeSet;

const PARAMS: PlaceParams = PlaceParams {
    seed: 7,
    density_per_km2: 15,
};

/// Every (kind, native, gloss) the society names.
fn all_names(s: &Society) -> Vec<(&'static str, String, String)> {
    let mut v = Vec::new();
    for x in &s.settlements {
        v.push(("settlement", x.name.clone(), x.name_gloss.clone()));
    }
    for x in &s.rivers {
        v.push(("river", x.name.clone(), x.name_gloss.clone()));
    }
    for x in &s.mountains {
        v.push(("mountain", x.name.clone(), x.name_gloss.clone()));
    }
    for x in &s.passes {
        v.push(("pass", x.name.clone(), x.name_gloss.clone()));
    }
    for x in &s.realms.realms {
        v.push(("realm", x.name.clone(), x.name_gloss.clone()));
    }
    for x in &s.regions {
        v.push(("region", x.name.clone(), x.name_gloss.clone()));
    }
    v
}

#[test]
fn names_are_deterministic_glossed_and_unique_in_the_world() {
    let grid = synthetic::landscape(PARAMS.seed).unwrap();
    let a = run(&grid, PARAMS).unwrap();
    let b = run(&grid, PARAMS).unwrap();
    let (na, nb) = (all_names(&a), all_names(&b));
    assert_eq!(na, nb, "names differ between runs");
    assert!(na.len() > 50);
    let mut seen = BTreeSet::new();
    for (kind, native, gloss) in &na {
        assert!(!native.is_empty() && !gloss.is_empty(), "empty {kind} name");
        assert!(
            native
                .chars()
                .all(|c| c.is_ascii_alphabetic() || " '-".contains(c)),
            "{kind} {native} is not plain romanisation"
        );
        assert!(
            seen.insert(skeleton(native)),
            "{kind} {native} is taken twice"
        );
    }
    for r in &a.realms.realms {
        assert!(
            r.name_gloss.to_ascii_lowercase().contains("mark"),
            "realm gloss {} is not a mark",
            r.name_gloss
        );
    }
    // The history hook reads the river's native name.
    let rivers: BTreeSet<&str> = a.rivers.iter().map(|r| r.name.as_str()).collect();
    assert!(a
        .settlements
        .iter()
        .filter_map(|s| s.history.rsplit("the ").next())
        .any(|tail| rivers.contains(tail)));
}
