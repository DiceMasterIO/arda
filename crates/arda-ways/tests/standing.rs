//! Review round 2 #33: water already in the layout (a lake or the sea) is
//! never paved, keeps its depth, ground and elevation, and a crossing over
//! it gets no synthetic river painted across the land.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

mod common;

use arda_tactical::TacticalLayout;
use arda_ways::sidecar::Feature;
use arda_ways::{apply_ways, CrossingKind, RoadClass};
use common::*;

const W: u32 = 64;

/// A grass window with a round lake of radius 10 squares at (32, 32).
fn lake_window() -> TacticalLayout {
    let mut l = TacticalLayout::new("lake", W, W, "grass");
    for y in 0..W {
        for x in 0..W {
            let (dx, dy) = (f64::from(x) - 32.0, f64::from(y) - 32.0);
            if dx.hypot(dy) <= 10.0 {
                let sq = l.square_mut(x, y).unwrap();
                sq.ground = "water_deep".into();
                sq.water_depth_ft = 8;
                sq.elevation_ft = 5;
            }
        }
    }
    l
}

fn lake(x: u32, y: u32) -> bool {
    (f64::from(x) - 32.0).hypot(f64::from(y) - 32.0) <= 10.0
}

#[test]
fn roads_never_pave_standing_water() {
    let mut l = lake_window();
    // A highway straight through the lake, east to west, at y = 50 m.
    let roads = vec![road(1, RoadClass::Highway, 170, &[[-500, 50], [600, 50]])];
    let t = terrain(flat, Vec::new());
    let out = apply_ways(&mut l, [0.0, 0.0], &roads, &[], &t, 7).unwrap();
    let mut paved = 0;
    for y in 0..W {
        for x in 0..W {
            let sq = l.square(i64::from(x), i64::from(y));
            let rule = out.sidecar.at(x, y).unwrap();
            if lake(x, y) {
                assert_eq!(sq.water_depth_ft, 8, "({x}, {y}) depth");
                assert_eq!(sq.ground, "water_deep", "({x}, {y}) ground");
                assert_eq!(sq.elevation_ft, 5, "({x}, {y}) elevation");
                assert_ne!(rule.feature, Feature::Road, "({x}, {y}) paved");
            } else if rule.feature == Feature::Road {
                paved += 1;
            }
        }
    }
    assert!(
        paved > 40,
        "the road still runs on dry land ({paved} squares)"
    );
}

#[test]
fn a_lake_ferry_paints_no_river_across_the_land() {
    let mut l = lake_window();
    let roads = vec![road(1, RoadClass::Road, 90, &[[-500, 50], [600, 50]])];
    let mut ferry = crossing(1, CrossingKind::Ferry, [50, 50], 0, RoadClass::Road);
    ferry.water = "lake".into();
    let t = terrain(flat, Vec::new());
    apply_ways(&mut l, [0.0, 0.0], &roads, &[ferry], &t, 7).unwrap();
    for y in 0..W {
        for x in 0..W {
            let sq = l.square(i64::from(x), i64::from(y));
            if !lake(x, y) {
                assert_eq!(sq.water_depth_ft, 0, "({x}, {y}) became water");
            }
        }
    }
}
