//! Review round 2 #34: a window between `1.5 w + 150` m and a wide
//! crossing's synthetic-channel reach planned no crossing, so it painted no
//! water beside a neighbour that did. Any window now matches the same
//! squares of a larger window holding the crossing.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

mod common;

use arda_ways::{CrossingKind, RoadClass, SQUARE_M};
use common::*;

#[test]
fn a_wide_crossing_paints_the_same_water_in_every_window() {
    // A road east–west at y = 50 m, a 100 m ferry at x = 500 m, no channel:
    // the synthetic channel runs north–south 300 m either side of the foot.
    let roads = vec![road(1, RoadClass::Road, 90, &[[-500, 50], [1500, 50]])];
    let crossings = vec![crossing(
        1,
        CrossingKind::Ferry,
        [500, 50],
        100,
        RoadClass::Road,
    )];
    let t = terrain(flat, Vec::new());
    let (big, _) = window([400.0, 0.0], 128, 270, &roads, &crossings, &t);
    // 300 × 230 squares from the origin: about 309 m south of the record,
    // past the old 300 m threshold but inside the channel's reach.
    let (sx, sy) = (300_u32, 230_u32);
    let origin = [f64::from(sx) * SQUARE_M, f64::from(sy) * SQUARE_M];
    let (small, _) = window(origin, 32, 32, &roads, &crossings, &t);
    let (dx, dy) = (sx - 256, sy);
    let mut water = 0;
    for y in 0..32 {
        for x in 0..32 {
            let a = big.square(i64::from(x + dx), i64::from(y + dy));
            let b = small.square(i64::from(x), i64::from(y));
            assert_eq!(a.water_depth_ft, b.water_depth_ft, "({x}, {y})");
            assert_eq!(a.ground, b.ground, "({x}, {y})");
            water += usize::from(b.water_depth_ft > 0);
        }
    }
    assert!(
        water > 100,
        "the window holds the channel's end ({water} squares)"
    );
}
