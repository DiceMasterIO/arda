//! Plans keep to their rivers: no building on the water, every street
//! square over the water a bridge deck, every deck spanning the channel
//! from bank to bank, and the same plan every time.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_town::plan::check;
use arda_town::plan::grid::SQUARE_M;
use arda_town::samples;
use arda_town::site::TerrainInput;
use common::{plan, SEED};

#[allow(clippy::cast_precision_loss)]
fn water(t: &TerrainInput) -> impl Fn(i64, i64) -> bool + '_ {
    move |x, y| {
        (t.water)(arda_town::geom::v2(
            (x as f64 + 0.5) * SQUARE_M,
            (y as f64 + 0.5) * SQUARE_M,
        ))
    }
}

#[test]
fn plans_keep_to_their_rivers() {
    for name in samples::NAMES {
        let (_, terrain) = samples::by_name(name).unwrap();
        let p = plan(name);
        let problems = check::water(p, &water(&terrain));
        assert!(problems.is_empty(), "{name}: {problems:?}");
    }
}

#[test]
fn the_river_town_bridges_its_river() {
    let p = plan("aldermere");
    assert!(!p.bridges.is_empty(), "no bridge in Aldermere");
    for b in &p.bridges {
        assert!(!b.rows.is_empty());
    }
}

#[test]
fn river_plans_are_deterministic() {
    for name in samples::NAMES {
        let (site, terrain) = samples::by_name(name).unwrap();
        let a = arda_town::generate(&site, &terrain, SEED).unwrap();
        let b = arda_town::generate(&site, &terrain, SEED).unwrap();
        assert_eq!(a.bridges, b.bridges, "{name}");
        assert_eq!(a.grid.kind, b.grid.kind, "{name}");
    }
}
