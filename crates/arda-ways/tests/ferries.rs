//! Long crossings (review 3.5-9): a ferry over open water spans all of it,
//! between landings on both shores, the same from either shore's window;
//! a river wider than 150 m is still crossed.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

mod common;

use arda_tactical::TacticalLayout;
use arda_ways::{apply_ways, CrossingKind, Feature, RoadClass, Terrain, SQUARE_M};
use common::{crossing, flat, road};

/// Global squares of the open water: a 300 m strait, x in 100..400 m.
const SEA: std::ops::Range<i64> = 64..256;

/// Open water the terrain knows everywhere (the refined sea).
struct Strait;

impl Terrain for Strait {
    fn height_m(&self, x: f64, y: f64) -> f64 {
        flat(x, y)
    }

    fn open_water(&self, gx: i64, _gy: i64) -> Option<bool> {
        Some(SEA.contains(&gx))
    }
}

/// A `w × h` window at `origin_m` whose sea squares hold water, as the
/// refined layout gives them.
#[allow(clippy::cast_possible_truncation)]
fn sea_window(origin_m: [f64; 2], w: u32, h: u32) -> TacticalLayout {
    let mut l = TacticalLayout::new("sea", w, h, "grass");
    let gx0 = (origin_m[0] / SQUARE_M).round() as i64;
    for y in 0..h {
        for x in 0..w {
            if SEA.contains(&(gx0 + i64::from(x))) {
                let sq = l.square_mut(x, y).unwrap();
                sq.water_depth_ft = 12;
                sq.ground = "water_deep".into();
            }
        }
    }
    l
}

#[test]
fn a_sea_ferry_spans_the_strait_from_either_shore() {
    let roads = vec![road(1, RoadClass::Road, 90, &[[-500, 50], [900, 50]])];
    let mut ferry = crossing(7, CrossingKind::Ferry, [250, 50], 0, RoadClass::Road);
    ferry.water = "sea".into();
    let mut spans = Vec::new();
    for origin in [[0.0, 0.0], [350.0, 0.0]] {
        let mut l = sea_window(origin, 128, 64);
        let out = apply_ways(&mut l, origin, &roads, &[ferry.clone()], &Strait, 7).unwrap();
        let c = out
            .report
            .crossings
            .iter()
            .find(|c| c.id == 7)
            .expect("the ferry is laid");
        assert!(
            c.water_squares >= 190,
            "a {}-square ferry over a 192-square strait",
            c.water_squares
        );
        let landings = out
            .sidecar
            .squares
            .iter()
            .filter(|r| r.feature == Feature::Landing)
            .count();
        assert!(landings > 0, "no landing in the window at {origin:?}");
        spans.push((c.water_squares, c.span_squares));
    }
    assert_eq!(spans[0], spans[1], "the two shores disagree");
}

/// A rasterised river 250 m wide (160 squares), x in 100..350 m.
struct Wide;

impl Terrain for Wide {
    fn height_m(&self, x: f64, y: f64) -> f64 {
        flat(x, y)
    }

    fn rivers_rasterised(&self) -> bool {
        true
    }

    fn river_water(&self, gx: i64, _gy: i64) -> bool {
        (64..224).contains(&gx)
    }
}

#[test]
fn a_river_wider_than_150_m_is_crossed() {
    let roads = vec![road(2, RoadClass::Highway, 170, &[[-500, 50], [900, 50]])];
    let mut l = TacticalLayout::new("wide", 128, 64, "grass");
    let out = apply_ways(&mut l, [0.0, 0.0], &roads, &[], &Wide, 7).unwrap();
    let c = out.report.crossings.first().expect("the river is crossed");
    assert!(c.water_squares >= 160, "{} squares", c.water_squares);
}

/// Global squares of a wide sea: x in 100..1037.5 m (600 squares).
const WIDE_SEA: std::ops::Range<i64> = 64..664;

/// The wide sea, known everywhere.
struct Sound;

impl Terrain for Sound {
    fn height_m(&self, x: f64, y: f64) -> f64 {
        flat(x, y)
    }

    fn open_water(&self, gx: i64, _gy: i64) -> Option<bool> {
        Some(WIDE_SEA.contains(&gx))
    }
}

#[test]
fn a_sea_route_too_wide_to_rope_lands_on_both_shores() {
    let roads = vec![road(1, RoadClass::Road, 90, &[[-500, 50], [1600, 50]])];
    let mut ferry = crossing(8, CrossingKind::Ferry, [600, 50], 0, RoadClass::Road);
    ferry.water = "sea".into();
    #[allow(clippy::cast_possible_truncation)]
    for origin in [[0.0, 0.0], [962.5, 0.0]] {
        let mut l = TacticalLayout::new("sound", 128, 64, "grass");
        let gx0 = (origin[0] / SQUARE_M).round() as i64;
        for y in 0..64 {
            for x in 0..128 {
                if WIDE_SEA.contains(&(gx0 + i64::from(x))) {
                    let sq = l.square_mut(x, y).unwrap();
                    sq.water_depth_ft = 12;
                    sq.ground = "water_deep".into();
                }
            }
        }
        let out = apply_ways(&mut l, origin, &roads, &[ferry.clone()], &Sound, 7).unwrap();
        let landings = out
            .sidecar
            .squares
            .iter()
            .filter(|r| r.feature == Feature::Landing)
            .count();
        assert!(landings >= 3, "{landings} landing squares at {origin:?}");
        let ids: Vec<String> = l
            .placements
            .iter()
            .map(|p| format!("{:?}", p.asset))
            .collect();
        assert!(ids.iter().any(|i| i.contains("ferry_boat")), "{origin:?}");
        assert!(!ids.iter().any(|i| i.contains("ferry_rope")), "{origin:?}");
    }
}
