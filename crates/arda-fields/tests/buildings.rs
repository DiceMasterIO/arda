//! Goal 45, review 3.5-12: the houses, barns, stables and mills of the
//! fields' compounds are buildings with `BuildingId`s.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_fields::{generate, synthetic};
use std::collections::BTreeMap;

/// Squares per `(building id, function)`.
fn buildings(rules: &arda_scene::RulesSidecar) -> BTreeMap<(String, String), usize> {
    let mut out = BTreeMap::new();
    for c in &rules.squares {
        let Some(ext) = &c.ext else { continue };
        let (Some(id), Some(f)) = (
            ext.get("building").and_then(|v| v.as_str()),
            ext.get("building_function").and_then(|v| v.as_str()),
        ) else {
            continue;
        };
        *out.entry((id.to_owned(), f.to_owned())).or_insert(0) += 1;
    }
    out
}

#[test]
fn farmsteads_and_mills_are_buildings_with_ids() {
    for (name, want) in [("orchard_farmstead", "farmhouse"), ("watermill", "mill")] {
        let s = synthetic::by_name(name).unwrap();
        let win = generate(&s.inputs(), s.origin_m, s.w, s.h, 42).unwrap();
        let b = buildings(&win.rules);
        assert!(
            b.keys().any(|(_, f)| f == want),
            "{name}: no {want} among {b:?}"
        );
        for ((id, f), n) in &b {
            let id: u64 = id.parse().unwrap();
            assert!(arda_ids::BuildingId(id).is_rural(), "{name}: {f} {id}");
            assert!(*n >= 4, "{name}: {f} has {n} squares");
        }
    }
}
