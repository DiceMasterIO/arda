//! Crossings on a rasterised river (`Terrain::rivers_rasterised`): bridges
//! and fords sit on the caller's water, span all of it, and never stand on
//! dry land; neighbouring windows agree.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

mod common;

use arda_tactical::TacticalLayout;
use arda_ways::{
    apply_ways, CrossingKind, Feature, RiverChannel, Road, RoadClass, Terrain, WaysOutput, SQUARE_M,
};
use common::{crossing, flat, road};

/// A meandering east–west river: its centre `y` at `x`, metres.
fn river_y(x: f64) -> f64 {
    100.0 + 15.0 * (x / 40.0).sin()
}

const HALF_M: f64 = 5.0;

/// The river as a caller's raster plus one guide channel, like arda-blocks'
/// refined rivers.
struct Raster {
    channels: Vec<RiverChannel>,
    wet: bool,
}

impl Raster {
    fn new(wet: bool) -> Self {
        let centreline = (-40..=80)
            .map(|k| {
                let x = f64::from(k) * 5.0;
                [x, river_y(x)]
            })
            .collect();
        Self {
            channels: vec![RiverChannel {
                id: 1,
                centreline,
                width_m: 2.0 * HALF_M,
                depth_m: 1.5,
            }],
            wet,
        }
    }
}

#[allow(clippy::cast_precision_loss)]
fn water(gx: i64, gy: i64) -> bool {
    let (x, y) = ((gx as f64 + 0.5) * SQUARE_M, (gy as f64 + 0.5) * SQUARE_M);
    (y - river_y(x)).abs() <= HALF_M
}

impl Terrain for Raster {
    fn height_m(&self, x: f64, y: f64) -> f64 {
        flat(x, y)
    }
    fn channels(&self) -> &[RiverChannel] {
        &self.channels
    }
    fn rivers_rasterised(&self) -> bool {
        true
    }
    fn river_water(&self, gx: i64, gy: i64) -> bool {
        water(gx, gy)
    }
    fn wet_ground(&self, _: i64, _: i64) -> bool {
        self.wet
    }
}

/// A road bridged by a record 30 m off the river (settle's straight line),
/// and a track that meets the river where no record is.
fn scene() -> Vec<Road> {
    vec![
        road(
            1,
            RoadClass::Road,
            100,
            &[[40, -300], [42, 0], [45, 200], [50, 500]],
        ),
        road(
            2,
            RoadClass::Track,
            90,
            &[[150, -300], [155, 100], [160, 500]],
        ),
    ]
}

fn run(origin_sq: [i64; 2], side: u32, t: &Raster) -> (TacticalLayout, WaysOutput) {
    let records = [crossing(
        1,
        CrossingKind::Bridge,
        [42, 80],
        10,
        RoadClass::Road,
    )];
    let mut l = TacticalLayout::new("wet", side, side, "grass");
    #[allow(clippy::cast_precision_loss)]
    let origin = [
        origin_sq[0] as f64 * SQUARE_M,
        origin_sq[1] as f64 * SQUARE_M,
    ];
    let out = apply_ways(&mut l, origin, &scene(), &records, t, 7).unwrap();
    (l, out)
}

fn way(f: Feature) -> bool {
    matches!(
        f,
        Feature::Road | Feature::Ruts | Feature::Bridge | Feature::Ford | Feature::Abutment
    )
}

#[test]
fn every_water_square_under_a_road_is_a_bridge_deck_or_a_ford() {
    let t = Raster::new(false);
    let side = 160;
    let (_, out) = run([0, 0], side, &t);
    let w = i64::from(side);
    let at = |x: i64, y: i64| &out.sidecar.squares[usize::try_from(y * w + x).unwrap()];
    let (mut bridges, mut fords) = (0, 0);
    for y in 0..w {
        for x in 0..w {
            let f = at(x, y).feature;
            match f {
                Feature::Bridge => bridges += 1,
                Feature::Ford => fords += 1,
                _ => {}
            }
            let deck_or_ford = matches!(f, Feature::Bridge | Feature::Ford);
            if water(x, y) {
                assert!(
                    deck_or_ford || matches!(f, Feature::None | Feature::Bank | Feature::GravelBar),
                    "{f:?} on the river at {x},{y}"
                );
            } else {
                assert!(!deck_or_ford, "{f:?} on dry land at {x},{y}");
            }
        }
    }
    assert!(
        bridges > 0 && fords > 0,
        "{bridges} bridge, {fords} ford squares"
    );
    // Each way is unbroken row by row across the river: wherever it runs
    // over water it does so on a deck or a ford.
    for (x0, x1) in [(15, 45), (85, 115)] {
        for y in 0..w {
            assert!(
                (x0..x1).any(|x| way(at(x, y).feature)),
                "the way between columns {x0} and {x1} breaks at row {y}"
            );
        }
    }
    let kinds: Vec<(String, CrossingKind)> = out
        .report
        .crossings
        .iter()
        .map(|c| (c.id.to_string(), c.kind))
        .collect();
    assert!(
        kinds.contains(&("1".into(), CrossingKind::Bridge)),
        "{kinds:?}"
    );
    assert!(kinds.iter().any(|k| k.1 == CrossingKind::Ford), "{kinds:?}");
    assert!(out.report.orphans.is_empty(), "{:?}", out.report.orphans);
}

#[test]
fn a_bridge_spans_the_whole_river_where_it_is_narrowest_nearby() {
    let t = Raster::new(false);
    let (_, out) = run([0, 0], 160, &t);
    let c = out
        .report
        .crossings
        .iter()
        .find(|c| c.id == 1)
        .expect("the record's bridge");
    assert_eq!(c.axis, "north_south");
    // The river is 10 m (6.4 squares) across and meanders: a skewed reach
    // would need more, abutments add two squares at each end.
    assert!(c.water_squares <= 9, "{c:?}");
    assert_eq!(c.span_squares, c.water_squares + 4, "{c:?}");
}

#[test]
fn neighbouring_windows_lay_the_same_crossings() {
    let t = Raster::new(false);
    let (la, a) = run([0, 0], 128, &t);
    let (lb, b) = run([64, 32], 128, &t);
    for y in 32..128 {
        for x in 64..128 {
            let i = usize::try_from(y * 128 + x).unwrap();
            let j = usize::try_from((y - 32) * 128 + (x - 64)).unwrap();
            assert_eq!(
                a.sidecar.squares[i].feature, b.sidecar.squares[j].feature,
                "square {x},{y}"
            );
            assert_eq!(la.squares[i].ground, lb.squares[j].ground, "square {x},{y}");
            assert_eq!(
                la.squares[i].water_depth_ft, lb.squares[j].water_depth_ft,
                "square {x},{y}"
            );
        }
    }
}

#[test]
fn ditches_hold_pools_only_on_wet_ground() {
    let highway = [road(
        3,
        RoadClass::Highway,
        150,
        &[[-300, 30], [100, 32], [500, 35]],
    )];
    let pools = |wet: bool| {
        let t = Raster::new(wet);
        let mut l = TacticalLayout::new("ditch", 128, 64, "grass");
        let out = apply_ways(&mut l, [0.0, 0.0], &highway, &[], &t, 7).unwrap();
        let ditches = out
            .sidecar
            .squares
            .iter()
            .filter(|q| q.feature == Feature::Ditch)
            .count();
        let wet_ditches = out
            .sidecar
            .squares
            .iter()
            .filter(|q| q.feature == Feature::Ditch && q.water_depth_ft > 0)
            .count();
        assert!(ditches > 100, "{ditches} ditch squares");
        wet_ditches
    };
    assert_eq!(pools(false), 0);
    assert!(pools(true) > 0);
}
