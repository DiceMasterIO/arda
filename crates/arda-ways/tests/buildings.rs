//! Goal 45, review 3.5-12: a toll house or waystation is a building with a
//! `BuildingId`, the same whichever window draws it.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

mod common;

use common::{bridge_scene, window};
use std::collections::BTreeSet;

fn ids(rules: &arda_scene::RulesSidecar) -> BTreeSet<(String, String)> {
    let mut out = BTreeSet::new();
    for c in &rules.squares {
        let Some(ext) = &c.ext else { continue };
        if ext.get("feature").and_then(|v| v.as_str()) != Some("building") {
            continue;
        }
        let id = ext.get("building").and_then(|v| v.as_str());
        let function = ext.get("building_function").and_then(|v| v.as_str());
        out.insert((
            id.expect("a building square without an id").to_owned(),
            function.expect("a building without a function").to_owned(),
        ));
    }
    out
}

#[test]
fn a_toll_house_has_one_building_id_in_every_window() {
    let (roads, crossings, t) = bridge_scene();
    let (_, a) = window([0.0, 0.0], 64, 64, &roads, &crossings, &t);
    assert_eq!(a.report.houses.len(), 1);
    let here = ids(&a.rules);
    assert_eq!(here.len(), 1, "{here:?}");
    let (id, function) = here.iter().next().unwrap();
    assert_eq!(function, "toll_house");
    let n: u64 = id.parse().unwrap();
    assert!(arda_ids::BuildingId(n).is_rural());
    // A window shifted by 8 squares names the same building.
    let (_, b) = window([-12.5, -12.5], 64, 64, &roads, &crossings, &t);
    assert!(ids(&b.rules).contains(&(id.clone(), function.clone())));
}
