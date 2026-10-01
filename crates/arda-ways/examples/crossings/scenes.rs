//! The synthetic scenes: roads, crossings, channels and terrain.

use arda_ways::{Crossing, CrossingKind, RiverChannel, Road, RoadClass, SQUARE_M};

/// One synthetic window.
pub struct Scene {
    /// Output name.
    pub name: &'static str,
    /// North-west corner in world metres.
    pub origin_m: [f64; 2],
    /// Width in squares.
    pub width: u32,
    /// Height in squares.
    pub height: u32,
    /// Roads.
    pub roads: Vec<Road>,
    /// Crossings.
    pub crossings: Vec<Crossing>,
    /// Channels.
    pub channels: Vec<RiverChannel>,
    /// Terrain height in metres.
    pub height_fn: fn(f64, f64) -> f64,
    /// Base ground.
    pub ground: &'static str,
}

fn road(id: u64, class: RoadClass, wealth: u8, pts: &[[i64; 2]]) -> Road {
    Road {
        id,
        class,
        segments: vec![pts.to_vec()],
        wealth,
    }
}

fn crossing(id: u64, kind: CrossingKind, at: [i64; 2], width_m: u32, class: RoadClass) -> Crossing {
    Crossing {
        id,
        kind,
        water: "river".into(),
        x_m: at[0],
        y_m: at[1],
        width_m,
        order: 4,
        road_class: class,
        river: String::new(),
    }
}

/// A soft valley bump for rivers: lower near `d = 0`.
fn valley(d: f64, depth: f64, half: f64) -> f64 {
    -depth * (-(d / half) * (d / half)).exp()
}

fn bridge_height(x: f64, y: f64) -> f64 {
    let river_x = 50.0 + 3.0 * (y / 40.0).sin();
    20.0 + 0.01 * x + 0.4 * (x / 23.0).sin() * (y / 31.0).cos() + valley(x - river_x, 2.5, 22.0)
}

fn ford_height(x: f64, y: f64) -> f64 {
    let river_y = 42.0 + 4.0 * (x / 30.0).sin();
    12.0 + 0.3 * (x / 17.0).cos() * (y / 21.0).sin() + valley(y - river_y, 1.5, 14.0)
}

fn ferry_height(x: f64, y: f64) -> f64 {
    8.0 + 0.3 * (y / 19.0).sin() + valley(x - 52.0, 1.8, 40.0)
}

fn mountain_height(x: f64, y: f64) -> f64 {
    // A 25 % slope rising to the north, with a gentle cross-fall.
    400.0 - 0.25 * y + 0.05 * x + 1.2 * (x / 13.0).sin() * (y / 17.0).cos()
}

/// The four scenes of the example.
#[must_use]
pub fn all() -> Vec<Scene> {
    let sq = SQUARE_M;
    vec![
        Scene {
            name: "highway_stone_bridge",
            origin_m: [-8.0 * sq, 0.0],
            width: 64,
            height: 48,
            roads: vec![
                road(
                    1,
                    RoadClass::Highway,
                    170,
                    &[[-1000, 52], [0, 40], [100, 34], [1100, 24]],
                ),
                road(2, RoadClass::Track, 90, &[[80, 35], [86, -40], [90, -300]]),
            ],
            crossings: vec![crossing(
                1,
                CrossingKind::Bridge,
                [50, 37],
                20,
                RoadClass::Highway,
            )],
            channels: vec![RiverChannel {
                id: 1,
                centreline: (-4..=6)
                    .map(|k| {
                        let y = f64::from(k) * 20.0;
                        [50.0 + 3.0 * (y / 40.0).sin(), y]
                    })
                    .collect(),
                width_m: 20.0,
                depth_m: 2.8,
            }],
            height_fn: bridge_height,
            ground: "grass",
        },
        Scene {
            name: "track_ford",
            origin_m: [0.0, 0.0],
            width: 64,
            height: 48,
            roads: vec![
                road(
                    3,
                    RoadClass::Track,
                    50,
                    &[[30, -300], [40, -100], [55, 0], [60, 100], [45, 300]],
                ),
                road(
                    4,
                    RoadClass::Footpath,
                    60,
                    &[[-200, 22], [0, 20], [100, 18], [300, 24]],
                ),
            ],
            crossings: vec![crossing(
                2,
                CrossingKind::Ford,
                [58, 44],
                11,
                RoadClass::Track,
            )],
            channels: vec![RiverChannel {
                id: 2,
                centreline: (-4..=10)
                    .map(|k| {
                        let x = f64::from(k) * 15.0;
                        [x, 42.0 + 4.0 * (x / 30.0).sin()]
                    })
                    .collect(),
                width_m: 11.0,
                depth_m: 1.4,
            }],
            height_fn: ford_height,
            ground: "grass",
        },
        Scene {
            name: "river_ferry",
            origin_m: [0.0, 0.0],
            width: 64,
            height: 48,
            roads: vec![road(
                5,
                RoadClass::Road,
                110,
                &[[-400, 30], [0, 36], [100, 38], [400, 34]],
            )],
            crossings: vec![crossing(
                3,
                CrossingKind::Ferry,
                [52, 37],
                42,
                RoadClass::Road,
            )],
            channels: vec![RiverChannel {
                id: 3,
                centreline: vec![
                    [48.0, -100.0],
                    [52.0, 0.0],
                    [53.0, 40.0],
                    [50.0, 100.0],
                    [46.0, 200.0],
                ],
                width_m: 42.0,
                depth_m: 4.0,
            }],
            height_fn: ferry_height,
            ground: "grass",
        },
        Scene {
            name: "mountain_switchbacks",
            origin_m: [-10.0 * sq, 0.0],
            width: 64,
            height: 64,
            roads: vec![road(
                6,
                RoadClass::Road,
                50,
                &[
                    [40, -300],
                    [40, -200],
                    [40, -100],
                    [40, 0],
                    [40, 100],
                    [40, 200],
                    [40, 300],
                ],
            )],
            crossings: Vec::new(),
            channels: Vec::new(),
            height_fn: mountain_height,
            ground: "scrub",
        },
    ]
}
