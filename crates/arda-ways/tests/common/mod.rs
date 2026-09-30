//! Shared fixtures for the integration tests.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use arda_tactical::TacticalLayout;
use arda_ways::{
    apply_ways, Crossing, CrossingKind, FnTerrain, RiverChannel, Road, RoadClass, WaysOutput,
};

pub fn road(id: u32, class: RoadClass, wealth: u8, pts: &[[i64; 2]]) -> Road {
    Road {
        id,
        class,
        segments: vec![pts.to_vec()],
        wealth,
    }
}

pub fn crossing(
    id: u32,
    kind: CrossingKind,
    at: [i64; 2],
    width_m: u32,
    class: RoadClass,
) -> Crossing {
    Crossing {
        id,
        kind,
        water: "river".into(),
        x_m: at[0],
        y_m: at[1],
        width_m,
        order: 3,
        road_class: class,
        river: String::new(),
    }
}

/// A north–south river at `x` metres with a gentle meander.
pub fn river_ns(id: u32, x: f64, width_m: f64, depth_m: f64) -> RiverChannel {
    RiverChannel {
        id,
        centreline: (-10..=20)
            .map(|k| {
                let y = f64::from(k) * 20.0;
                [x + 2.0 * (y / 45.0).sin(), y]
            })
            .collect(),
        width_m,
        depth_m,
    }
}

pub fn flat(x: f64, y: f64) -> f64 {
    10.0 + 0.004 * x + 0.3 * (x / 21.0).sin() * (y / 17.0).cos()
}

pub fn slope(x: f64, y: f64) -> f64 {
    300.0 - 0.25 * y + 0.03 * x
}

pub type Terrain = FnTerrain<fn(f64, f64) -> f64>;

pub fn terrain(h: fn(f64, f64) -> f64, channels: Vec<RiverChannel>) -> Terrain {
    FnTerrain {
        height: h,
        channels,
    }
}

/// Applies ways to a fresh grass window.
pub fn window(
    origin: [f64; 2],
    w: u32,
    h: u32,
    roads: &[Road],
    crossings: &[Crossing],
    t: &Terrain,
) -> (TacticalLayout, WaysOutput) {
    let mut l = TacticalLayout::new("test", w, h, "grass");
    let out = apply_ways(&mut l, origin, roads, crossings, t, 7).unwrap();
    (l, out)
}

/// The highway stone-bridge fixture: a river at x = 50 m, a highway across it.
pub fn bridge_scene() -> (Vec<Road>, Vec<Crossing>, Terrain) {
    let roads = vec![
        road(
            1,
            RoadClass::Highway,
            170,
            &[[-1000, 52], [0, 40], [100, 34], [1100, 24]],
        ),
        road(2, RoadClass::Track, 90, &[[80, 35], [86, -40], [90, -300]]),
    ];
    let crossings = vec![crossing(
        1,
        CrossingKind::Bridge,
        [50, 37],
        20,
        RoadClass::Highway,
    )];
    (
        roads,
        crossings,
        terrain(flat, vec![river_ns(1, 50.0, 20.0, 2.8)]),
    )
}
