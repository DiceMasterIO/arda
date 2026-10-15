//! The 1.5 m flood rule holds where the stored HAND is the generator's
//! "unknown" placeholder (review 3.3-3; logic/08 §settle-refusals rule 4).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda::{Cover, TerrainKind};
use arda_settle::grid::{Grid, MEMORY_BUDGET};
use arda_settle::hand::{self, UNKNOWN_DM};
use arda_settle::tags;

/// Sea in the west third, a lake in the north-east corner, low grass shore
/// rising 1 m a cell eastwards; every land cell carries the placeholder.
fn shore() -> Grid {
    let (w, h) = (30, 20);
    let mut g = Grid::sea(w, h, MEMORY_BUDGET).unwrap();
    for i in 0..g.len() {
        let (x, y) = g.xy(i);
        if x < 10 {
            continue;
        }
        if x >= 26 && y < 4 {
            g.terrain[i] = TerrainKind::Lake;
            g.height_mm[i] = 15_000;
            continue;
        }
        g.terrain[i] = TerrainKind::Land;
        g.cover[i] = Cover::Grass;
        g.height_mm[i] = i32::try_from((x - 10) * 1000 + 500).unwrap();
        g.temp_cc[i] = 1100;
        g.moisture[i] = 150;
        g.har_dm[i] = UNKNOWN_DM;
    }
    g
}

#[test]
fn a_placeholder_shore_half_a_metre_up_is_refused_and_not_arable() {
    let mut g = shore();
    let low = g.at(10, 10).unwrap();
    let r = hand::resolve(&mut g).unwrap();
    assert!(r.measured > 0 && r.refused == 0, "{r:?}");
    assert_eq!(g.har_dm[low], 5, "0.5 m above the sea");
    assert!(tags::refused(&g, low), "below the 1.5 m flood line");
    assert!(!tags::arable(&g, low), "flooded shore is not arable");
    let up = g.at(16, 10).unwrap();
    assert_eq!(g.har_dm[up], 65, "6.5 m above the sea");
    assert!(!tags::refused(&g, up));
    // Beside the lake the height is taken above its spill level, not its bed.
    let by_lake = g.at(25, 1).unwrap();
    let spill = (0..g.len())
        .filter(|&j| g.is_land(j) && g.neighbours4(j).any(|k| g.terrain[k] == TerrainKind::Lake))
        .map(|j| g.height_mm[j])
        .min()
        .unwrap();
    assert_eq!(
        i64::from(g.har_dm[by_lake]),
        i64::from(g.height_mm[by_lake] - spill) / 100
    );
}

#[test]
fn land_without_any_water_is_refused_not_waved_through() {
    let mut g = Grid::sea(4, 4, MEMORY_BUDGET).unwrap();
    for i in 0..g.len() {
        g.terrain[i] = TerrainKind::Land;
        g.cover[i] = Cover::Grass;
        g.temp_cc[i] = 1100;
        g.har_dm[i] = UNKNOWN_DM;
    }
    let r = hand::resolve(&mut g).unwrap();
    assert_eq!(r.refused, 16);
    assert!((0..g.len()).all(|i| tags::refused(&g, i)));
}

#[test]
fn measured_heights_are_left_alone() {
    let mut g = shore();
    let i = g.at(20, 10).unwrap();
    g.har_dm[i] = 7;
    hand::resolve(&mut g).unwrap();
    assert_eq!(g.har_dm[i], 7);
}
