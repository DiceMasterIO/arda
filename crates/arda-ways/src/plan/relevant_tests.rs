//! Review round 2 #36: planning only [`relevant_roads`] draws the window
//! exactly as planning every road does.

use super::relevant_roads;
use crate::input::{Crossing, CrossingKind, FnTerrain, RiverChannel, Road, RoadClass};
use crate::plan::{build_inner, Window};
use crate::standing::Standing;
use arda_tactical::TacticalLayout;

fn road(id: u64, class: RoadClass, pts: &[[i64; 2]]) -> Road {
    Road {
        id,
        class,
        segments: vec![pts.to_vec()],
        wealth: 120,
    }
}

/// Hilly ground, so far roads get switchbacks too.
fn hills(x: f64, y: f64) -> f64 {
    40.0 + 25.0 * (x / 90.0).sin() * (y / 70.0).cos() + 0.02 * y
}

fn scene() -> Vec<Road> {
    let mut roads = vec![
        // Through the window and on for 1.2 km: its far end is a junction.
        road(
            1,
            RoadClass::Highway,
            &[[-1000, 52], [0, 40], [100, 34], [1200, 24]],
        ),
        road(2, RoadClass::Track, &[[80, 35], [86, -40], [90, -300]]),
        // Starts on road 1's far end and runs away: it moves that end.
        road(3, RoadClass::Track, &[[1200, 25], [1600, 300], [2400, 900]]),
        // Its own far end snaps to road 3.
        road(4, RoadClass::Footpath, &[[2401, 901], [2600, 1500]]),
        // A highway far east of the window, planned before road 1, passing
        // through road 1's far end: it snaps that end.
        road(0, RoadClass::Highway, &[[1200, -3000], [1200, 3000]]),
        // A road just beyond the relevance margin, with switchbacks.
        road(5, RoadClass::Road, &[[-700, -700], [-900, -1400]]),
    ];
    // Far away: a steep network that must not cost the window anything.
    for k in 0..60_u32 {
        let x = 20_000 + 150 * i64::from(k);
        roads.push(road(
            100 + u64::from(k),
            RoadClass::Road,
            &[[x, 0], [x + 40, 900], [x, 1800]],
        ));
    }
    roads
}

fn draw(filter: bool, roads: &[Road], t: &FnTerrain<fn(f64, f64) -> f64>) -> (String, String) {
    let win = Window::new([0.0, 0.0], 64, 64).unwrap();
    let crossings = vec![Crossing {
        id: 1,
        kind: CrossingKind::Bridge,
        water: "river".into(),
        x_m: 50,
        y_m: 37,
        width_m: 20,
        order: 3,
        road_class: RoadClass::Highway,
        river: String::new(),
    }];
    let standing = Standing::default();
    let plan = build_inner(win, roads, &crossings, t, 7, &standing, filter);
    let mut l = TacticalLayout::new("t", 64, 64, "grass");
    let sidecar = crate::raster::apply(&mut l, &plan, t, 7, &standing);
    (
        serde_json::to_string(&l).unwrap(),
        serde_json::to_string(&sidecar).unwrap(),
    )
}

#[test]
fn planning_the_relevant_roads_draws_the_window_as_planning_all() {
    let t: FnTerrain<fn(f64, f64) -> f64> = FnTerrain {
        height: hills,
        channels: vec![RiverChannel {
            id: 1,
            centreline: (-10..=20).map(|k| [50.0, f64::from(k) * 20.0]).collect(),
            width_m: 20.0,
            depth_m: 2.8,
        }],
    };
    let roads = scene();
    let win = Window::new([0.0, 0.0], 64, 64).unwrap();
    let keep = relevant_roads(win, &roads);
    let mut kept: Vec<u64> = roads
        .iter()
        .zip(&keep)
        .filter(|(_, &k)| k)
        .map(|(r, _)| r.id)
        .collect();
    kept.sort_unstable();
    assert_eq!(
        kept,
        [0, 1, 2, 3, 4],
        "the snap chain is kept, the far roads are not"
    );
    assert_eq!(draw(true, &roads, &t), draw(false, &roads, &t));
}
