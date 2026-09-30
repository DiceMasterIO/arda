//! Regression tests for scene documents from untrusted sources and for
//! work that must stay near-linear in the map size (review round 2).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

mod common;

use arda_scene::{regions, Scene};
use common::L;
use std::time::Instant;

fn doc() -> serde_json::Value {
    let scene = L::new(3, 3)
        .h(0, 1, arda_tactical::catalog::WallRole::Run)
        .scene();
    serde_json::from_str(&scene.to_json().unwrap()).unwrap()
}

fn empty_layers(j: &mut serde_json::Value) {
    for k in [
        "movement",
        "climb",
        "cover",
        "obscured",
        "elevation_ft",
        "water_depth_ft",
    ] {
        j[k] = serde_json::json!([]);
    }
}

#[test]
fn a_zero_sized_scene_with_a_huge_side_is_refused() {
    // n = 0 matches the empty layers, but indexing allocates w × (h + 1)
    // edge slots: 13 GB for a four-billion-wide, zero-high scene.
    for (w, h) in [(u32::MAX, 0), (0, u32::MAX), (0, 0)] {
        let mut j = doc();
        j["width"] = w.into();
        j["height"] = h.into();
        empty_layers(&mut j);
        assert!(Scene::from_json(&j.to_string()).is_err(), "{w}x{h}");
    }
}

#[test]
fn walls_off_the_grid_or_diagonal_are_refused() {
    // A wall to x = u32::MAX expands to four billion unit edges on indexing.
    let mut j = doc();
    j["walls"][0]["points"] = serde_json::json!([[0, 0], [u32::MAX, 0]]);
    assert!(Scene::from_json(&j.to_string()).is_err());
    let mut j = doc();
    j["walls"][0]["points"] = serde_json::json!([[0, 0], [2, 2]]);
    assert!(Scene::from_json(&j.to_string()).is_err(), "diagonal");
    let mut j = doc();
    j["walls"][0]["points"] = serde_json::json!([[1, 1]]);
    assert!(Scene::from_json(&j.to_string()).is_err(), "one point");
    let ok = doc();
    assert!(Scene::from_json(&ok.to_string()).is_ok());
}

#[test]
fn an_unknown_scene_format_version_is_refused() {
    let mut j = doc();
    j["format_version"] = 2.into();
    assert!(Scene::from_json(&j.to_string()).is_err());
}

#[test]
fn tracing_a_checkerboard_is_near_linear() {
    // 131 072 one-square rings. Rescanning the consumed map prefix for each
    // new ring made this ~10^10 steps (minutes); it must take well under 5 s
    // even unoptimised.
    let (w, h) = (512u32, 512u32);
    let mask: Vec<bool> = (0..w * h).map(|i| (i % w + i / w) % 2 == 0).collect();
    let start = Instant::now();
    let rings = regions::trace(&mask, w, h);
    assert_eq!(rings.len(), (w * h / 2) as usize);
    assert!(start.elapsed().as_secs() < 5, "{:?}", start.elapsed());
}
