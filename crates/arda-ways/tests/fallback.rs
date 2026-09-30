//! Degrading canonical keys to the placeholder library.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_tactical::{placeholders, render, Library, RenderOptions};
use arda_ways::fallback::{degrade, FallbackKind};
use arda_ways::{CrossingKind, RoadClass};
use common::*;

/// Ids the full placeholder library now has, removed again so the fallback
/// path stays exercised.
const TRIMMED: [&str; 5] = [
    "prop.bridge_deck_stone",
    "prop.ferry_boat",
    "prop.signpost",
    "ground.flagstone.0",
    "ground.flagstone.1",
];

#[test]
fn the_full_placeholder_library_covers_every_canonical_way_key() {
    let (catalog, images) = placeholders::generate(placeholders::DEFAULT_SEED);
    let lib = Library::from_parts(catalog, images).unwrap();
    let (mut roads, mut crossings, _) = bridge_scene();
    roads.push(road(9, RoadClass::Road, 90, &[[-400, 90], [400, 90]]));
    crossings.push(crossing(
        9,
        CrossingKind::Ferry,
        [50, 90],
        20,
        RoadClass::Road,
    ));
    let t = terrain(flat, vec![river_ns(1, 50.0, 20.0, 2.8)]);
    let (l, _) = window([0.0, 0.0], 64, 72, &roads, &crossings, &t);
    l.check(&lib).unwrap();
}

#[test]
fn missing_assets_degrade_to_the_nearest_and_are_recorded() {
    let (mut catalog, mut images) = placeholders::generate(placeholders::DEFAULT_SEED);
    let gone: Vec<String> = catalog
        .assets
        .iter()
        .filter(|a| TRIMMED.contains(&a.id.as_str()))
        .map(|a| a.image.clone())
        .collect();
    catalog.assets.retain(|a| !TRIMMED.contains(&a.id.as_str()));
    images.retain(|k, _| !gone.contains(k) && !TRIMMED.contains(&k.as_str()));
    let lib = Library::from_parts(catalog, images).unwrap();
    let (mut roads, mut crossings, _) = bridge_scene();
    roads.push(road(9, RoadClass::Road, 90, &[[-400, 90], [400, 90]]));
    crossings.push(crossing(
        9,
        CrossingKind::Ferry,
        [50, 90],
        20,
        RoadClass::Road,
    ));
    let t = terrain(flat, vec![river_ns(1, 50.0, 20.0, 2.8)]);
    let (mut l, _) = window([0.0, 0.0], 64, 72, &roads, &crossings, &t);
    assert!(
        l.check(&lib).is_err(),
        "canonical keys are not all in the placeholders"
    );
    let log = degrade(&mut l, &lib).records();
    l.check(&lib).unwrap();
    let has = |kind, wanted: &str, used: &str| {
        log.iter()
            .any(|f| f.kind == kind && f.wanted == wanted && f.used == used && f.count > 0)
    };
    assert!(has(
        FallbackKind::Asset,
        "prop.bridge_deck_stone",
        "prop.dock_planks"
    ));
    assert!(has(FallbackKind::Asset, "prop.ferry_boat", "prop.rowboat"));
    assert!(has(FallbackKind::Asset, "prop.signpost", "prop.fence"));
    assert!(has(FallbackKind::Ground, "flagstone", "stone_floor"));
    // Substitutes keep only rotations they allow (fences: 0 and 90).
    let img = render(
        &l,
        &lib,
        1,
        &RenderOptions {
            ppsq: 16,
            grid: false,
            lighting: false,
        },
    )
    .unwrap();
    assert_eq!((img.width, img.height), (64 * 16, 72 * 16));
}
