//! The town WFC over a generated world's plans (goal 47: the relaxed-fill
//! rate on seed-42 MICRO towns stays below 1 %). Needs a settled world
//! with its society: set `ARDA_WORLD` (for example `out/micro42`).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_town::block::wfc::{report, Report};
use std::path::PathBuf;

#[test]
#[ignore = "needs ARDA_WORLD, a generated, settled world with society"]
fn relaxed_fill_rate_is_below_one_percent() {
    let dir = PathBuf::from(std::env::var("ARDA_WORLD").expect("ARDA_WORLD"));
    let world = arda_people::World::open(&dir).unwrap();
    let ids: Vec<u64> = world
        .files
        .settlements
        .settlements
        .iter()
        .map(|s| s.id.get())
        .collect();
    let mut total = Report::default();
    for id in ids {
        if let Some(plan) = world.plan(id).unwrap().as_ref() {
            total.add(&report(plan));
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let rate = total.relaxed() as f64 / total.problems().max(1) as f64;
    println!(
        "{total:?}\nrelaxed {} of {} ({:.3} %)",
        total.relaxed(),
        total.problems(),
        rate * 100.0
    );
    assert!(rate < 0.01, "relaxed-fill rate {rate}");
}
