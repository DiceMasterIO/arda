//! The ways rules as `arda-scene`'s `RulesSidecar` format 2 (adapter A8).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation
)]

mod common;

use common::*;

/// Adapter A8 (logic/12 §scene-sidecar): the ways rules convert to
/// `arda-scene`'s format 2; a bridge square is walked at deck level, a
/// parapet edge blocks movement but not sight, and the ways extras ride in
/// `ext`.
#[test]
fn ways_rules_are_scene_sidecar_format_2() {
    use arda_scene::{build_scene, Movement, Sq};
    let (roads, crossings, t) = bridge_scene();
    let (mut l, out) = window([0.0, 0.0], 64, 64, &roads, &crossings, &t);
    assert_eq!(l.origin, Some([0, 0]));
    let r = &out.rules;
    assert_eq!(r.format_version, 2);
    assert_eq!(r.squares.len(), 64 * 64);
    let (bi, bridge) = r
        .squares
        .iter()
        .enumerate()
        .find(|(_, c)| c.deck == Some(true))
        .expect("a bridge deck square");
    let ext = bridge.ext.as_ref().unwrap();
    assert_eq!(ext["feature"], "bridge");
    assert!(ext.contains_key("deck_elevation_ft"));
    assert!(ext.contains_key("road_class"));
    assert!(r
        .edges
        .iter()
        .any(|e| e.role == arda_scene::EdgeRole::Parapet && e.blocks_movement && !e.blocks_sight));
    let (catalog, images) =
        arda_tactical::placeholders::generate(arda_tactical::placeholders::DEFAULT_SEED);
    let lib = arda_tactical::Library::from_parts(catalog, images).unwrap();
    let _ = arda_ways::fallback::degrade(&mut l, &lib);
    let s = build_scene(&l, &lib, 7, Some(r)).unwrap();
    let (x, y) = (bi as u32 % 64, bi as u32 / 64);
    assert_eq!(s.movement_at(Sq(x, y)), Movement::Normal);
    let json = serde_json::to_string(r).unwrap();
    assert_eq!(arda_scene::RulesSidecar::from_json(&json).unwrap(), *r);
}
