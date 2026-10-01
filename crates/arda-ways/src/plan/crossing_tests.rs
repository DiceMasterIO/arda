//! Unit tests of crossing planning.

use super::*;
use crate::input::{FnTerrain, Road};
use crate::standing::Standing;

/// Review round 2 #35: after a crossing straightens its way, that way's
/// chunk index matches its stations before the next crossing is planned.
#[test]
fn a_straightened_way_is_reindexed_before_the_next_crossing() {
    let terrain = FnTerrain {
        height: |x: f64, _y: f64| 10.0 + 0.004 * x,
        channels: vec![RiverChannel {
            id: 1,
            centreline: (-10..=20).map(|k| [50.0, f64::from(k) * 20.0]).collect(),
            width_m: 20.0,
            depth_m: 2.8,
        }],
    };
    let roads = vec![Road {
        id: 1,
        class: RoadClass::Highway,
        segments: vec![vec![[-1000, 52], [0, 40], [100, 34], [1100, 24]]],
        wealth: 170,
    }];
    let win = Window::new([0.0, 0.0], 64, 64).unwrap();
    let (mut ways, _) = super::super::plan_ways(win, &roads, &terrain, true);
    let mut channels: Vec<ChannelPlan> = terrain.channels.iter().map(ChannelPlan::new).collect();
    let c = Crossing {
        id: 1,
        kind: CrossingKind::Bridge,
        water: "river".into(),
        x_m: 50,
        y_m: 37,
        width_m: 20,
        order: 3,
        road_class: RoadClass::Highway,
        river: String::new(),
    };
    let before: Vec<_> = ways.iter().map(|w| w.dense.runs.clone()).collect();
    let plan = plan_one(
        &c,
        [50.0, 37.0],
        &mut ways,
        &mut channels,
        &terrain,
        &Standing::default(),
    );
    assert!(plan.is_some(), "the bridge is laid");
    assert!(
        ways.iter().zip(&before).any(|(w, b)| w.dense.runs != *b),
        "the bridge straightened its way"
    );
    for w in &ways {
        assert!(w.dense.index_is_fresh(), "a stale chunk index");
    }
}
