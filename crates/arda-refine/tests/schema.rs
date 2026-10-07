//! The emitted JSON parses into the consumer types themselves:
//! `arda-tactical`'s `TacticalLayout` and `arda-scene`'s `RulesSidecar`
//! format 2, and the layout validates against the placeholder library and
//! yields a scene.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::unwrap_used,
    missing_docs,
    dead_code
)]

mod common;
use arda_refine::refine_window;
use serde::Deserialize;

mod tactical {
    //! The consumer's own types (adapter A7): no mirror left to drift.
    pub use arda_tactical::layout::*;
}

mod scene {
    pub use arda_scene::{CoverLevel, RulesSidecar};
}

const GROUND: [&str; 23] = [
    "trail",
    "grass",
    "dirt",
    "mud",
    "sand",
    "gravel",
    "water_shallow",
    "water_deep",
    "meadow",
    "forest_floor",
    "leaf_litter",
    "heath",
    "scrub",
    "moss",
    "scree",
    "rock",
    "cliff",
    "snow",
    "ice",
    "marsh",
    "reed_bed",
    "salt_crust",
    "mudflat",
];

const VEG: [&str; 42] = [
    "tree_alder",
    "tree_stunted",
    "juniper",
    "rock_outcrop",
    "tree_oak",
    "tree_elm",
    "tree_birch",
    "tree_fruit",
    "bush",
    "bush_flowering",
    "reeds",
    "boulder",
    "stones",
    "tree_pine",
    "tree_spruce",
    "tree_willow",
    "tree_dead",
    "fallen_log",
    "stump",
    "fern",
    "mushroom_ring",
    "heather",
    "flower_patch",
    "tall_grass",
    "cattail",
    "lily_pads",
    "rock_small",
    "rock_large",
    "scree_patch",
    "driftwood",
    "sea_rock",
    "tide_pool",
    "dune_grass",
    "lichen_rock",
    "rock_snow",
    "alpine_flowers",
    "tussock",
    "krummholz",
    "sagebrush",
    "dry_grass",
    "sedge",
    "marsh_flowers",
];

#[derive(Deserialize)]
struct Meta {
    width: u32,
    height: u32,
    placements: Vec<serde_json::Value>,
}

#[test]
fn layout_rules_and_meta_parse_into_the_consumer_schemas() {
    let src = common::world(42);
    let (x0, y0) = (64 * 9, 64 * 6);
    let map = refine_window(&src, x0, y0, 128, 96).unwrap();
    let layout: tactical::TacticalLayout =
        serde_json::from_str(&map.layout_json().unwrap()).unwrap();
    assert_eq!((layout.width, layout.height), (128, 96));
    assert_eq!(layout.squares.len(), 128 * 96);
    for sq in &layout.squares {
        assert!(GROUND.contains(&sq.ground.as_str()), "ground {}", sq.ground);
        assert_eq!(sq.elevation_ft % 5, 0, "elevation is on 5-ft steps");
        let deep = sq.water_depth_ft >= 5;
        match sq.ground.as_str() {
            "water_deep" => assert!(deep),
            "water_shallow" => assert!(sq.water_depth_ft > 0 && !deep),
            _ => assert_eq!(sq.water_depth_ft, 0),
        }
    }
    assert!(!layout.placements.is_empty());
    for p in &layout.placements {
        let tactical::AssetRef::Id(id) = &p.asset else {
            panic!("placements use vocabulary ids");
        };
        let name = id.strip_prefix("veg.").unwrap();
        assert!(VEG.contains(&name), "veg id {id}");
        assert!([0, 90, 180, 270].contains(&p.rotation));
        assert!(p.x >= 0.0 && p.x < 128.0 && p.y >= 0.0 && p.y < 96.0);
        assert_eq!((p.x * 64.0).fract(), 0.0, "fixed-point 1/64 square");
    }
    let rules: scene::RulesSidecar = serde_json::from_str(&map.rules_json().unwrap()).unwrap();
    assert_eq!(rules.format_version, 2);
    assert_eq!((rules.width, rules.height), (128, 96));
    assert_eq!(rules.squares.len(), layout.squares.len());
    for (r, s) in rules.squares.iter().zip(&layout.squares) {
        assert_eq!(r.water_depth_ft, Some(s.water_depth_ft));
        if (1..5).contains(&s.water_depth_ft) {
            assert_eq!(r.difficult, Some(true), "wading is difficult terrain");
        }
    }
    assert!(rules
        .squares
        .iter()
        .any(|r| r.cover == Some(scene::CoverLevel::ThreeQuarters)));
    let meta: Meta = serde_json::from_str(&map.meta_json().unwrap()).unwrap();
    assert_eq!(layout.origin, Some([x0, y0]));
    assert_eq!((meta.width, meta.height), (128, 96));
    assert_eq!(meta.placements.len(), layout.placements.len());
}

#[test]
fn a_refined_block_validates_and_builds_a_scene() {
    let src = common::world(42);
    let map = arda_refine::refine_block(&src, arda_refine::CellKey::new(9, 6)).unwrap();
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
    let lib = arda_tactical::Library::load(&dir).unwrap();
    map.layout.check(&lib).unwrap();
    let scene = arda_scene::build_scene(&map.layout, &lib, 7, Some(&map.rules)).unwrap();
    assert_eq!((scene.width, scene.height), (64, 64));
    assert_eq!(map.layout.origin, Some([64 * 9, 64 * 6]));
}
