//! Crossings are where a road meets real water: no bridge over a ditch,
//! no road painted across the open sea (review 3.3-14, 3.3-15; logic/08
//! §crossings, Invariants 9).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda::{Cover, TerrainKind};
use arda_settle::cost::CostSurface;
use arda_settle::crossings::{self, CrossingKind};
use arda_settle::grid::{Grid, MEMORY_BUDGET};
use arda_settle::model::{Settlement, Tier};
use arda_settle::roads::{self, Network};

fn land(w: usize, h: usize) -> Grid {
    let mut g = Grid::sea(w, h, MEMORY_BUDGET).unwrap();
    for i in 0..g.len() {
        g.terrain[i] = TerrainKind::Land;
        g.cover[i] = Cover::Grass;
        g.height_mm[i] = 20_000;
        g.har_dm[i] = 50;
    }
    g
}

/// A road along row 10 over two channels: a 1 m ditch at x = 10 and a
/// 6 m stream at x = 20.
fn ditch_and_stream() -> (Grid, Network) {
    let mut g = land(30, 20);
    for (x, w_dm) in [(10, 10), (20, 60)] {
        for y in 0..20 {
            let i = g.at(x, y).unwrap();
            g.order[i] = 1;
            g.width_dm[i] = w_dm;
            g.har_dm[i] = 0;
        }
    }
    let mut net = Network {
        raster: vec![0; g.len()],
        ..Network::default()
    };
    for x in 0..30 {
        net.raster[g.at(x, 10).unwrap()] = 2;
    }
    (g, net)
}

#[test]
fn a_ditch_under_two_metres_is_not_a_crossing() {
    let (g, net) = ditch_and_stream();
    let (found, _) = crossings::crossings(&g, &net, &[]).unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].width_m, 6);
    assert_eq!(found[0].kind, CrossingKind::Ford);
}

#[test]
fn a_ferry_leg_is_not_written_as_road_on_open_water() {
    // Two shores split by a 2 km sound: the road between the towns ferries.
    let mut g = land(60, 20);
    for y in 0..20 {
        for x in 25..45 {
            let i = g.at(x, y).unwrap();
            g.terrain[i] = TerrainKind::Sea;
            g.height_mm[i] = -6_000;
        }
    }
    let cs = CostSurface {
        base: (0..g.len())
            .map(|i| if g.is_land(i) { 100 } else { 0 })
            .collect(),
    };
    let towns = vec![
        Settlement::bare(1, 10, 10, Tier::Town, 3000),
        Settlement::bare(2, 50, 10, Tier::Town, 3000),
    ];
    let net = roads::build(&g, &cs, &towns);
    assert!(net.unreachable.is_empty());
    for i in 0..g.len() {
        assert!(
            !(g.is_water(i) && net.raster[i] != 0),
            "road raster on open water at {:?}",
            g.xy(i)
        );
    }
    for r in &net.roads {
        for seg in &r.segments {
            for p in seg {
                let i = g.at(p[0] / 100, p[1] / 100).unwrap();
                assert!(g.is_land(i), "road {} drawn over water at {p:?}", r.id);
            }
        }
    }
    let (found, _) = crossings::crossings(&g, &net, &towns).unwrap();
    assert!(
        found
            .iter()
            .any(|c| c.kind == CrossingKind::Ferry && c.water == "sea"),
        "the sound is a ferry crossing: {found:?}"
    );
}
