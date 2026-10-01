//! Integration tests: determinism, seams, bridge spans, fords, widths,
//! switchbacks and the sidecar (goals 37, 43, 46, 48).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]

mod common;

use arda_tactical::layout::{EdgeAxis, Placement, WallSegment};
use arda_tactical::TacticalLayout;
use arda_ways::sidecar::{EdgeRole, Feature};
use arda_ways::{CrossingKind, RoadClass, Sidecar, WaysError, SQUARE_M};
use common::*;

#[path = "ways/refusals.rs"]
mod refusals;

fn squares_equal(
    big: &TacticalLayout,
    bs: &Sidecar,
    part: &TacticalLayout,
    ps: &Sidecar,
    dx: u32,
    dy: u32,
) {
    for y in 0..part.height {
        for x in 0..part.width {
            let a = big.square(i64::from(x + dx), i64::from(y + dy));
            let b = part.square(i64::from(x), i64::from(y));
            assert_eq!(
                a,
                b,
                "square ({}, {}) differs across the seam",
                x + dx,
                y + dy
            );
            assert_eq!(
                bs.at(x + dx, y + dy),
                ps.at(x, y),
                "sidecar ({}, {})",
                x + dx,
                y + dy
            );
        }
    }
}

fn placements_in(l: &TacticalLayout, dx: f32, dy: f32) -> Vec<String> {
    let mut v: Vec<String> = l
        .placements
        .iter()
        .filter(|p| p.x >= 0.0 && p.y >= 0.0 && p.x < l.width as f32 && p.y < l.height as f32)
        .map(|p: &Placement| format!("{:?}@{},{}r{}", p.asset, p.x + dx, p.y + dy, p.rotation))
        .collect();
    v.sort();
    v
}

fn walls_of(l: &TacticalLayout, dx: u32, dy: u32) -> Vec<(u32, u32, EdgeAxis, String)> {
    l.walls
        .iter()
        .map(|w: &WallSegment| (w.x + dx, w.y + dy, w.axis, format!("{:?}{}", w.kind, w.kit)))
        .collect()
}

fn assert_seam(split_x: bool) {
    let (roads, crossings, t) = if split_x {
        let (mut r, _, _) = bridge_scene();
        r.truncate(1);
        let c = vec![crossing(
            1,
            CrossingKind::Bridge,
            [98, 34],
            20,
            RoadClass::Highway,
        )];
        (r, c, terrain(flat, vec![river_ns(1, 98.0, 20.0, 2.8)]))
    } else {
        let r = vec![road(
            6,
            RoadClass::Road,
            90,
            &[[40, -300], [40, -100], [40, 100], [40, 300]],
        )];
        (r, Vec::new(), terrain(slope, Vec::new()))
    };
    let (w, h) = if split_x { (64, 48) } else { (64, 64) };
    let (bw, bh) = if split_x { (128, 48) } else { (64, 128) };
    let second = if split_x { [100.0, 0.0] } else { [0.0, 100.0] };
    let (big, bs) = window([0.0, 0.0], bw, bh, &roads, &crossings, &t);
    let (a, sa) = window([0.0, 0.0], w, h, &roads, &crossings, &t);
    let (b, sb) = window(second, w, h, &roads, &crossings, &t);
    let (dx, dy) = if split_x { (64, 0) } else { (0, 64) };
    squares_equal(&big, &bs.sidecar, &a, &sa.sidecar, 0, 0);
    squares_equal(&big, &bs.sidecar, &b, &sb.sidecar, dx, dy);
    let mut union = placements_in(&a, 0.0, 0.0);
    union.extend(placements_in(&b, dx as f32, dy as f32));
    union.sort();
    assert_eq!(placements_in(&big, 0.0, 0.0), union, "placements agree");
    let mut wu = walls_of(&a, 0, 0);
    wu.extend(walls_of(&b, dx, dy));
    wu.sort();
    wu.dedup();
    let mut wb = walls_of(&big, 0, 0);
    wb.sort();
    assert_eq!(wb, wu, "walls agree");
}

#[test]
fn a_bridged_highway_matches_exactly_across_an_east_west_seam() {
    assert_seam(true);
}

#[test]
fn a_switchback_road_matches_exactly_across_a_north_south_seam() {
    assert_seam(false);
}

#[test]
fn output_is_deterministic() {
    let (roads, crossings, t) = bridge_scene();
    let (a, sa) = window([0.0, 0.0], 64, 48, &roads, &crossings, &t);
    let (b, sb) = window([0.0, 0.0], 64, 48, &roads, &crossings, &t);
    assert_eq!(a.to_json().unwrap(), b.to_json().unwrap());
    assert_eq!(sa.sidecar.to_json().unwrap(), sb.sidecar.to_json().unwrap());
    assert_eq!(sa.report, sb.report);
}

#[test]
fn the_bridge_decks_every_water_square_of_its_span_and_rests_on_abutments() {
    let (roads, crossings, t) = bridge_scene();
    let (l, out) = window([0.0, 0.0], 64, 48, &roads, &crossings, &t);
    let s = &out.sidecar;
    let rows: Vec<u32> = (0..l.height)
        .filter(|&y| (0..l.width).any(|x| s.at(x, y).unwrap().feature == Feature::Bridge))
        .collect();
    assert_eq!(rows.len(), 4, "a highway bridge deck is four squares wide");
    for &y in &rows {
        let deck: Vec<u32> = (0..l.width).filter(|&x| s.at(x, y).unwrap().deck).collect();
        let (x0, x1) = (deck[0], deck[deck.len() - 1]);
        // Contiguous, and the whole water run of the row is decked.
        assert_eq!(deck.len() as u32, x1 - x0 + 1);
        for x in 0..l.width {
            if l.square(i64::from(x), i64::from(y)).water_depth_ft > 0 && (x0..=x1).contains(&x) {
                assert!(s.at(x, y).unwrap().deck);
            }
        }
        for x in [x0 - 1, x1 + 1] {
            assert_eq!(
                l.square(i64::from(x), i64::from(y)).water_depth_ft,
                0,
                "dry beyond the deck"
            );
        }
        for x in [x0 - 2, x0 - 1, x1 + 1, x1 + 2] {
            assert_eq!(s.at(x, y).unwrap().feature, Feature::Abutment);
        }
        // Water continues beneath the deck, and the deck stands above it.
        let mid = s.at(u32::midpoint(x0, x1), y).unwrap();
        assert!(
            l.square(i64::from(u32::midpoint(x0, x1)), i64::from(y))
                .water_depth_ft
                >= 5
        );
        assert!(
            mid.deck_elevation_ft.unwrap()
                >= l.square(i64::from(x0 - 1), i64::from(y)).elevation_ft
        );
    }
    // The river is 20 m, so the deck spans at least 20 / 1.5625 squares.
    let n = (0..l.width)
        .filter(|&x| s.at(x, rows[0]).unwrap().deck)
        .count();
    assert!(n as f64 >= 20.0 / SQUARE_M - 1.0, "deck of {n} squares");
    // Parapets on both sides block movement but not sight.
    let parapets: Vec<_> = s
        .edges
        .iter()
        .filter(|e| e.role == EdgeRole::Parapet)
        .collect();
    assert!(!parapets.is_empty());
    assert!(parapets
        .iter()
        .all(|e| e.blocks_movement && !e.blocks_sight));
    let ys: std::collections::BTreeSet<u32> = parapets.iter().map(|e| e.y).collect();
    assert_eq!(ys, [rows[0], rows[3] + 1].into_iter().collect());
    assert_eq!(
        out.report.houses.len(),
        1,
        "a toll house at the highway bridge"
    );
}

#[test]
fn a_ford_is_shallow_gravel_and_difficult() {
    let roads = vec![road(
        3,
        RoadClass::Track,
        60,
        &[[40, -300], [42, 0], [44, 300]],
    )];
    let crossings = vec![crossing(
        2,
        CrossingKind::Ford,
        [42, 40],
        12,
        RoadClass::Track,
    )];
    let river = arda_ways::RiverChannel {
        id: 2,
        centreline: vec![[-200.0, 40.0], [0.0, 41.0], [100.0, 39.0], [300.0, 40.0]],
        width_m: 12.0,
        depth_m: 1.6,
    };
    let t = terrain(flat, vec![river]);
    let (l, out) = window([0.0, 0.0], 64, 48, &roads, &crossings, &t);
    let s = &out.sidecar;
    let mut fords = 0;
    for y in 0..l.height {
        for x in 0..l.width {
            let r = s.at(x, y).unwrap();
            if r.feature == Feature::Ford {
                fords += 1;
                let d = l.square(i64::from(x), i64::from(y)).water_depth_ft;
                assert!((1..=2).contains(&d), "ford depth {d} ft");
                assert!(r.difficult);
                assert_eq!(l.square(i64::from(x), i64::from(y)).ground, "gravel");
            }
        }
    }
    // Two squares wide, across the whole 12 m channel.
    assert!(fords >= 2 * 7, "{fords} ford squares");
    // The unforded river beside it is deeper.
    assert!(l.squares.iter().any(|q| q.water_depth_ft >= 4));
    assert_eq!(out.report.crossings[0].width_squares, 2);
}

#[test]
fn a_ferry_has_landings_a_boat_and_a_rope() {
    let roads = vec![road(
        5,
        RoadClass::Road,
        110,
        &[[-400, 30], [0, 36], [100, 38], [400, 34]],
    )];
    let crossings = vec![crossing(
        3,
        CrossingKind::Ferry,
        [50, 37],
        40,
        RoadClass::Road,
    )];
    let t = terrain(flat, vec![river_ns(3, 50.0, 40.0, 4.0)]);
    let (l, out) = window([0.0, 0.0], 64, 48, &roads, &crossings, &t);
    let landings = out
        .sidecar
        .squares
        .iter()
        .filter(|r| r.feature == Feature::Landing)
        .count();
    assert_eq!(landings, 2 * 3 * 3, "two 3 × 3 landing stages");
    let ids: Vec<String> = l
        .placements
        .iter()
        .map(|p| format!("{:?}", p.asset))
        .collect();
    assert!(ids.iter().any(|i| i.contains("ferry_boat")));
    assert!(ids.iter().filter(|i| i.contains("ferry_rope")).count() >= 20);
}

fn width_of(class: RoadClass, wealth: u8, y_m: i64) -> usize {
    let roads = vec![road(1, class, wealth, &[[-500, y_m], [600, y_m]])];
    let t = terrain(flat, Vec::new());
    let (_, out) = window([0.0, 0.0], 64, 48, &roads, &[], &t);
    (0..48)
        .filter(|&y| {
            matches!(
                out.sidecar.at(20, y).unwrap().feature,
                Feature::Road | Feature::Ruts
            )
        })
        .count()
}

#[test]
fn road_width_follows_the_class() {
    for y_m in [30, 31, 37] {
        assert_eq!(
            width_of(RoadClass::Highway, 128, y_m),
            5,
            "highway at y = {y_m}"
        );
        assert_eq!(width_of(RoadClass::Road, 128, y_m), 4, "road at y = {y_m}");
        assert_eq!(
            width_of(RoadClass::Road, 20, y_m),
            4,
            "poor road at y = {y_m}"
        );
        assert_eq!(
            width_of(RoadClass::Track, 128, y_m),
            2,
            "track at y = {y_m}"
        );
        assert_eq!(
            width_of(RoadClass::Footpath, 128, y_m),
            1,
            "footpath at y = {y_m}"
        );
    }
}

#[test]
fn highways_have_verges_and_ditches_and_tracks_have_ruts() {
    let t = terrain(flat, Vec::new());
    let roads = vec![road(1, RoadClass::Highway, 128, &[[-500, 37], [600, 37]])];
    let (_, out) = window([0.0, 0.0], 64, 48, &roads, &[], &t);
    let col: Vec<Feature> = (0..48)
        .map(|y| out.sidecar.at(20, y).unwrap().feature)
        .collect();
    assert_eq!(col.iter().filter(|f| **f == Feature::Verge).count(), 2);
    assert_eq!(col.iter().filter(|f| **f == Feature::Ditch).count(), 2);
    let roads = vec![road(1, RoadClass::Track, 128, &[[-500, 37], [600, 37]])];
    let (l, out) = window([0.0, 0.0], 64, 48, &roads, &[], &t);
    let ruts: Vec<&str> = (0..64)
        .filter(|&x| out.sidecar.at(x, 23).unwrap().feature == Feature::Ruts)
        .map(|x| l.square(i64::from(x), 23).ground.as_str())
        .collect();
    assert!(
        ruts.contains(&"dirt") && ruts.contains(&"mud"),
        "rutted dirt: {ruts:?}"
    );
}

#[test]
fn switchbacks_keep_the_grade_and_a_continuous_elevation() {
    let roads = vec![road(
        6,
        RoadClass::Road,
        90,
        &[[40, -300], [40, -100], [40, 100], [40, 300]],
    )];
    let t = terrain(slope, Vec::new());
    let (l, out) = window([0.0, 0.0], 64, 64, &roads, &[], &t);
    assert!(out.report.switchbacks > 0);
    assert_eq!(out.report.over_grade, 0);
    let s = &out.sidecar;
    let road = |x: u32, y: u32| s.at(x, y).is_some_and(|r| r.feature == Feature::Road);
    let mut pairs = 0;
    for y in 0..63 {
        for x in 0..63 {
            if !road(x, y) {
                continue;
            }
            let e = l.square(i64::from(x), i64::from(y)).elevation_ft;
            for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                if road(nx, ny) {
                    let d = (l.square(i64::from(nx), i64::from(ny)).elevation_ft - e).abs();
                    assert!(d <= 5, "step of {d} ft at ({x}, {y})");
                    assert_eq!(d % 5, 0, "elevations are 5-ft steps");
                    pairs += 1;
                }
            }
        }
    }
    assert!(pairs > 200);
    // A straight climb would have been 25 % against a 10 % limit.
    let road_squares = s
        .squares
        .iter()
        .filter(|r| r.feature == Feature::Road)
        .count();
    assert!(
        road_squares > 3 * 64 * 2,
        "the road winds: {road_squares} squares"
    );
}

#[test]
fn the_sidecar_matches_the_layout_size_and_round_trips() {
    let (roads, crossings, t) = bridge_scene();
    let (l, out) = window([0.0, 0.0], 64, 48, &roads, &crossings, &t);
    assert_eq!((out.sidecar.width, out.sidecar.height), (l.width, l.height));
    assert_eq!(out.sidecar.squares.len(), l.squares.len());
    let back: Sidecar = serde_json::from_str(&out.sidecar.to_json().unwrap()).unwrap();
    assert_eq!(back, out.sidecar);
}

#[test]
fn a_junction_gets_a_signpost_and_milestones_mark_kilometres() {
    let (roads, crossings, t) = bridge_scene();
    let (l, out) = window([-8.0 * SQUARE_M, 0.0], 64, 48, &roads, &crossings, &t);
    assert_eq!(out.report.junctions, 1);
    let ids: Vec<String> = l
        .placements
        .iter()
        .map(|p| format!("{:?}", p.asset))
        .collect();
    assert!(ids.iter().any(|i| i.contains("signpost")));
    assert!(ids.iter().any(|i| i.contains("milestone")));
}

#[test]
fn records_use_the_canonical_codes_and_string_ids() {
    use arda_ways::{Crossing, Road};
    assert_eq!(
        [
            RoadClass::None,
            RoadClass::Track,
            RoadClass::Road,
            RoadClass::Highway,
            RoadClass::Footpath
        ]
        .map(RoadClass::code),
        [0, 1, 2, 3, 4]
    );
    let r: Road =
        serde_json::from_str(r#"{"id":"12","class":"footpath","segments":[[[0,0],[5,5]]]}"#)
            .unwrap();
    assert_eq!((r.id, r.class, r.wealth), (12, RoadClass::Footpath, 128));
    let r: Road =
        serde_json::from_str(r#"{"id":7,"class":"highway","from":"3","segments":[]}"#).unwrap();
    assert_eq!(r.id, 7);
    assert!(serde_json::to_string(&r).unwrap().contains(r#""id":"7""#));
    let c: Crossing = serde_json::from_str(
        r#"{"id":"4","kind":"ford","water":"river","x_m":1,"y_m":2,"width_m":8,"order":2,"road_class":"track","river":"Esk"}"#,
    )
    .unwrap();
    assert_eq!(
        (c.id, c.kind, c.road_class),
        (4, CrossingKind::Ford, RoadClass::Track)
    );
}

#[test]
fn shallow_water_is_wading_and_difficult_but_decks_are_not() {
    let (roads, crossings, t) = bridge_scene();
    let (l, out) = window([0.0, 0.0], 64, 48, &roads, &crossings, &t);
    for (q, r) in l.squares.iter().zip(&out.sidecar.squares) {
        assert_eq!(q.water_depth_ft, r.water_depth_ft);
        assert_eq!(q.elevation_ft % 5, 0);
        if r.deck {
            assert!(!r.difficult);
        } else if (1..5).contains(&q.water_depth_ft) {
            assert!(r.difficult);
        }
    }
}

#[test]
fn crossing_highways_leave_no_ditch_water_or_difficulty_on_the_carriageway() {
    // Highway 2 is painted after highway 1 and takes over squares of its
    // ditches and cut faces; those must not keep difficult terrain or pools.
    let roads = vec![
        road(1, RoadClass::Highway, 128, &[[-500, 37], [600, 37]]),
        road(2, RoadClass::Highway, 128, &[[50, -300], [50, 300]]),
    ];
    let t = terrain(flat, Vec::new());
    let (_, out) = window([0.0, 0.0], 64, 48, &roads, &[], &t);
    let mut checked = 0;
    for sq in &out.sidecar.squares {
        if matches!(sq.feature, Feature::Road | Feature::Verge) {
            checked += 1;
            assert!(!sq.difficult && sq.water_depth_ft == 0, "{sq:?}");
        }
    }
    assert!(checked > 0);
}
