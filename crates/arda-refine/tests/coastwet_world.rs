//! The calibration targets of logic/09 §biome signals, checked on a
//! generated MICRO world. Skips (and says why) when the fixture is absent:
//! `target/release/arda generate --seed 42 --micro --terrain fine --out out/micro42`.
#![allow(clippy::unwrap_used, missing_docs)]

use arda_refine::coastwet::{measure, Report};
use arda_refine::WorldSource;

fn report() -> Option<Report> {
    let dir = std::env::var("ARDA_REFINE_WORLD").unwrap_or_else(|_| "../../out/micro42".into());
    let path = std::path::Path::new(&dir);
    if !path.join("world.json").exists() {
        println!("skipped: no world at {dir}");
        return None;
    }
    let world = arda::World::load(path).unwrap();
    let src = WorldSource::new(&world).unwrap();
    let r = measure(&src, 8).unwrap();
    if !r.has_shore_layer {
        println!("skipped: {dir} has no terrain/shore.bin (recipe 5 or later)");
        return None;
    }
    Some(r)
}

#[test]
fn micro_world_shares_stay_inside_the_targets() {
    let Some(r) = report() else { return };
    // Marsh: 3-8 % of temperate lowland land.
    let low = &r.groups["temperate_lowland"];
    assert!(
        (3.0..8.0).contains(&low.marshy_pct),
        "marshy {}",
        low.marshy_pct
    );
    // Pools: under 1 % of the squares outside marsh cells.
    let all = &r.groups["all"];
    assert!(
        all.pools.pool_area_pct_outside_marsh < 1.0,
        "{:?}",
        all.pools
    );
    // Sand beaches are sandy and two to four cells deep; no sand under cliffs.
    let sand = &r.strip["sand_beach"];
    assert!(sand.sand_ground_pct >= 70.0, "{sand:?}");
    assert!((1.8..4.5).contains(&sand.mean_width), "{sand:?}");
    let cliff = &r.strip["cliff"];
    assert!(cliff.sand_ground_pct < 1e-9, "{cliff:?}");
    let narrow: u64 = cliff.width_hist.iter().take(3).sum();
    assert!(narrow * 100 >= cliff.cells * 80, "{cliff:?}");
}
