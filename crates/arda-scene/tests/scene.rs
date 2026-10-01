//! Whole-scene behaviour: regions, spawn hints, secret doors, JSON and
//! determinism over the arda-tactical test layouts.
#![allow(clippy::unwrap_used, missing_docs)]

mod common;

use arda_scene::{
    build_scene, scene_debug_png, Edge, RegionKind, RulesSidecar, Scene, Sq, WallKind,
};
use arda_tactical::catalog::WallRole;
use arda_tactical::{layouts, Library};
use common::{lib, L};
use std::collections::BTreeMap;

#[test]
fn difficult_squares_merge_into_one_region() {
    let mut l = L::new(6, 4);
    for (x, y) in [(1, 1), (2, 1), (3, 1), (1, 2), (2, 2), (3, 2)] {
        l = l.put("veg.bush", x, y);
    }
    let s = l.water(5, 0, 2).water(5, 1, 2).scene();
    let diff = s
        .regions
        .iter()
        .find(|r| r.kind == RegionKind::Difficult)
        .unwrap();
    assert_eq!(diff.rings, vec![vec![[1, 1], [4, 1], [4, 3], [1, 3]]]);
    let wet = s
        .regions
        .iter()
        .find(|r| r.kind == RegionKind::ShallowWater)
        .unwrap();
    assert_eq!(wet.rings, vec![vec![[5, 0], [6, 0], [6, 2], [5, 2]]]);
    assert!(!s.regions.iter().any(|r| r.kind == RegionKind::DeepWater));
}

#[test]
fn spawn_hints_cover_edges_and_entrances() {
    let mut l = L::new(6, 5);
    for x in 0..6 {
        l = l.h(x, 2, WallRole::Run);
    }
    let s = l.h(3, 2, WallRole::Door).put("prop.barrel", 0, 0).scene();
    let hints = &s.spawn_hints;
    let north: Vec<_> = hints.exits.iter().filter(|e| e.edge == Edge::N).collect();
    assert_eq!(north.len(), 1);
    assert_eq!((north[0].from, north[0].to), (Sq(1, 0), Sq(5, 0)));
    assert_eq!(hints.exits.iter().filter(|e| e.edge == Edge::S).count(), 1);
    assert_eq!(hints.entrances.len(), 1);
    assert_eq!(hints.entrances[0].squares, vec![Sq(3, 1), Sq(3, 2)]);
    assert!(s.walls[hints.entrances[0].wall].kind == WallKind::Door);
    // Open squares have eight clear, reachable neighbours.
    assert!(hints.open.contains(&Sq(2, 3)));
    assert!(!hints.open.contains(&Sq(2, 2)), "beside the wall");
    assert!(!hints.open.contains(&Sq(1, 1)), "beside the barrel");
}

/// The placeholder library with `wall.timber.door` tagged secret.
fn secret_library() -> Library {
    let base = lib();
    let mut catalog = base.catalog.clone();
    for a in &mut catalog.assets {
        if a.id == "wall.timber.door" {
            a.tags.free.push(arda_scene::walls::SECRET_TAG.into());
        }
    }
    let images: BTreeMap<_, _> = catalog
        .assets
        .iter()
        .filter_map(|a| base.image(&a.id).map(|i| (a.id.clone(), i.clone())))
        .collect();
    Library::from_parts(catalog, images).unwrap()
}

#[test]
fn doors_of_a_secret_tagged_kit_are_secret_doors() {
    let mut l = L::new(4, 3);
    l.0.walls.push(arda_tactical::layout::WallSegment {
        x: 1,
        y: 1,
        axis: arda_tactical::layout::EdgeAxis::Horizontal,
        kind: WallRole::Door,
        kit: "timber".into(),
    });
    let mut s = build_scene(&l.0, &secret_library(), 1, None).unwrap();
    assert_eq!(s.walls[0].kind, WallKind::Secret);
    assert!(s.walls[0].blocks_sight);
    s.set_open(0, true).unwrap();
    assert!(!s.walls[0].blocks_movement);
    let plain = build_scene(&l.0, lib(), 1, None).unwrap();
    assert_eq!(plain.walls[0].kind, WallKind::Door);
}

fn all_scenes(seed: u64) -> Vec<Scene> {
    layouts::all()
        .iter()
        .map(|l| build_scene(l, lib(), seed, None).unwrap())
        .collect()
}

#[test]
fn scenes_are_deterministic() {
    let a = all_scenes(7);
    let b = all_scenes(7);
    assert!(a.len() >= 3);
    for (x, y) in a.iter().zip(&b) {
        assert_eq!(x.to_json().unwrap(), y.to_json().unwrap());
        assert_eq!(
            scene_debug_png(x, 16).unwrap(),
            scene_debug_png(y, 16).unwrap()
        );
    }
}

#[test]
fn scenes_round_trip_through_compact_json() {
    for s in all_scenes(3) {
        let json = s.to_json().unwrap();
        assert!(!json.contains('\n'));
        let back = Scene::from_json(&json).unwrap();
        assert_eq!(back, s);
        assert_eq!(Scene::from_json(&s.to_json_pretty().unwrap()).unwrap(), s);
    }
}

#[test]
fn truncated_layers_are_refused() {
    let s = L::new(3, 3).scene();
    let json = s
        .to_json()
        .unwrap()
        .replace("[[9,\"normal\"]]", "[[8,\"normal\"]]");
    assert!(Scene::from_json(&json).is_err());
}

#[test]
fn every_test_layout_yields_useful_scene_data() {
    for s in all_scenes(7) {
        // Natural layouts (forest glade, marsh, scree, farm field) have no
        // wall kit; every built one does.
        let natural = ["forest_glade", "marsh", "mountain_scree", "farm_field"];
        assert!(
            natural.contains(&s.name.as_str()) || !s.walls.is_empty(),
            "{}",
            s.name
        );
        assert!(!s.spawn_hints.open.is_empty(), "{}", s.name);
        assert!(!s.spawn_hints.exits.is_empty(), "{}", s.name);
        // Merging never produces fewer unit edges than distinct layout edges.
        let edges: usize = s
            .walls
            .iter()
            .map(|w| arda_scene::walls::unit_edges(w).len())
            .sum();
        let layout = layouts::by_name(&s.name).unwrap();
        let distinct: std::collections::BTreeSet<_> =
            layout.walls.iter().map(|w| (w.axis, w.x, w.y)).collect();
        assert_eq!(edges, distinct.len(), "{}", s.name);
    }
}

#[test]
fn the_riverside_scene_has_water_and_lights() {
    let l = layouts::by_name("riverside").unwrap();
    let s = build_scene(&l, lib(), 7, Some(&RulesSidecar::empty(l.width, l.height))).unwrap();
    assert!(s.regions.iter().any(|r| r.kind == RegionKind::DeepWater));
    assert!(s.regions.iter().any(|r| r.kind == RegionKind::ShallowWater));
    assert!(!s.lights.is_empty());
    assert!(!s.vision_blockers.is_empty());
    // An empty sidecar changes nothing.
    assert_eq!(s, build_scene(&l, lib(), 7, None).unwrap());
}

#[test]
fn seeds_serialise_as_strings_and_tokens_default_empty() {
    let l = L::new(2, 2);
    let s = build_scene(&l.0, lib(), u64::MAX, None).unwrap();
    let json = s.to_json().unwrap();
    assert!(json.contains(r#""seed":"18446744073709551615""#));
    assert!(json.contains(r#""tokens":[]"#));
    let mut back = Scene::from_json(&json).unwrap();
    assert_eq!(back.seed, u64::MAX);
    back.tokens.push(arda_scene::TokenSlot {
        npc_id: "9007199254740993".into(),
        x: 0.5,
        y: 1.5,
        facing: 90,
    });
    let again = Scene::from_json(&back.to_json().unwrap()).unwrap();
    assert_eq!(again, back);
    // Scenes written before tokens existed still load.
    let old = json.replace(r#","tokens":[]"#, "");
    assert!(Scene::from_json(&old).unwrap().tokens.is_empty());
}

#[test]
fn a_secret_door_is_no_spawn_entrance() {
    // Review round 2 #39: a secret door looks like wall until found.
    let mut l = L::new(4, 3);
    l.0.walls.push(arda_tactical::layout::WallSegment {
        x: 1,
        y: 1,
        axis: arda_tactical::layout::EdgeAxis::Horizontal,
        kind: WallRole::Door,
        kit: "timber".into(),
    });
    let secret = build_scene(&l.0, &secret_library(), 1, None).unwrap();
    assert_eq!(secret.walls[0].kind, WallKind::Secret);
    assert!(secret.spawn_hints.entrances.is_empty());
    let plain = build_scene(&l.0, lib(), 1, None).unwrap();
    assert_eq!(plain.spawn_hints.entrances.len(), 1);
}
