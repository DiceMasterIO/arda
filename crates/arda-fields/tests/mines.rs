//! Review round 2 #37: a mine or quarry never cuts a road; arda-ways paints
//! the road through the cell, so the compound moves off it (or, if no site
//! nearby is clear, gives up the squares on it).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_fields::compounds::{enumerate, CompoundKind};
use arda_fields::input::{Road, RoadClass};
use arda_fields::linear::RoadNet;
use arda_fields::synthetic;

const SEED: u64 = 42;

fn worked(roads: &[Road]) -> Vec<arda_fields::compounds::Compound> {
    let mut s = synthetic::quarry();
    s.roads.extend_from_slice(roads);
    let inputs = s.inputs();
    let net = RoadNet::new(&s.roads);
    enumerate(&inputs, &net, SEED, (19, 18, 24, 22))
        .into_iter()
        .filter(|c| matches!(c.kind, CompoundKind::Mine | CompoundKind::Quarry))
        .collect()
}

#[test]
fn mines_and_quarries_leave_roads_through_their_cells_alone() {
    // Roads straight through the middle of both resource cells.
    let roads = [
        Road {
            class: RoadClass::Road,
            points: vec![[1900.0, 2050.0], [2200.0, 2050.0]],
        },
        Road {
            class: RoadClass::Track,
            points: vec![[2250.0, 1850.0], [2250.0, 2100.0]],
        },
    ];
    let net = RoadNet::new(&roads);
    let mines = worked(&roads);
    assert_eq!(mines.len(), 2, "both resource cells are still worked");
    for m in &mines {
        assert!(m.squares.len() > 100, "{:?} keeps its works", m.kind);
        for s in m.squares.keys() {
            assert!(
                !net.near(s.centre(), 0.0),
                "{:?} cuts the road at {s:?}",
                m.kind
            );
        }
    }
    // Without a road in the way the sites are as before.
    let alone = worked(&[]);
    assert_eq!(alone.len(), 2);
}
