//! `arda settle` stamps its output with the world and code it came from
//! (review 3.3-4, §4 item 4).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_settle::stamp::{Provenance, DETACHED};
use arda_settle::{encode, encode_stamped, run, synthetic, PlaceParams};
use std::path::Path;

#[test]
fn the_stamp_names_the_world_and_changes_with_any_output() {
    let params = PlaceParams {
        seed: 42,
        density_per_km2: 15,
    };
    let g = synthetic::landscape(params.seed).unwrap();
    let s = run(&g, params).unwrap();
    let prov = Provenance {
        seed: 42,
        world_arda_version: "0.8.1".into(),
        recipe_version: Some(7),
        world_json: "00".repeat(16),
    };
    let dir = Path::new("society");
    let files = encode_stamped(dir, &g, &s, &prov).unwrap();
    let read = |files: &[(&str, Vec<u8>)]| {
        let f: arda_settle::output::SettlementsFile = serde_json::from_slice(&files[0].1).unwrap();
        f.stamp.unwrap()
    };
    let stamp = read(&files);
    assert_eq!(stamp.seed, "42");
    assert_eq!(stamp.recipe_version, Some(7));
    assert_eq!(stamp.world_json, prov.world_json);
    assert_eq!(stamp.id.len(), 32);
    assert_eq!(
        read(&encode_stamped(dir, &g, &s, &prov).unwrap()).id,
        stamp.id,
        "deterministic"
    );
    // Another world, or any changed output, gives another id.
    let other = Provenance {
        world_json: "11".repeat(16),
        ..prov.clone()
    };
    assert_ne!(
        read(&encode_stamped(dir, &g, &s, &other).unwrap()).id,
        stamp.id
    );
    let mut moved = s.clone();
    moved.settlements[0].population += 1;
    assert_ne!(
        read(&encode_stamped(dir, &g, &moved, &prov).unwrap()).id,
        stamp.id
    );
    // A run over a bare grid says so.
    assert_eq!(read(&encode(dir, &g, 42, &s).unwrap()).world_json, DETACHED);
}
