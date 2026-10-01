//! Review round 2 #25: way, crossing and channel ids are u64 like
//! `arda-settle`'s, and synthetic ids live far above them.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_ways::{Crossing, Road};
use common::*;

#[test]
fn settle_ids_beyond_u32_are_read_and_written_as_strings() {
    let road: Road = serde_json::from_str(
        r#"{"id": "4294967296", "class": "road", "segments": [[[0, 0], [10, 0]]]}"#,
    )
    .unwrap();
    assert_eq!(road.id, 1 << 32);
    let crossing: Crossing = serde_json::from_str(
        r#"{"id": 9007199254740993, "kind": "bridge", "x_m": 0, "y_m": 0,
            "width_m": 20, "road_class": "road"}"#,
    )
    .unwrap();
    assert_eq!(crossing.id, 9_007_199_254_740_993);
    let json = serde_json::to_value(&crossing).unwrap();
    assert_eq!(json["id"], "9007199254740993");
}

#[test]
fn a_record_id_with_bit_30_set_is_not_mistaken_for_a_synthetic_one() {
    // The highway bridge fixture gets its toll house whatever its id.
    for id in [1, (1 << 30) | 1, (1 << 31) | 1, (1 << 40) + 3] {
        let (roads, mut crossings, t) = bridge_scene();
        crossings[0].id = id;
        let (_, out) = window([0.0, 0.0], 64, 64, &roads, &crossings, &t);
        assert_eq!(out.report.houses.len(), 1, "id {id}");
        assert_eq!(out.report.crossings[0].id, id);
        assert_eq!(out.report.houses[0].0, id.to_string());
    }
}
