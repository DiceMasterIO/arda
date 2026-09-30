//! Performance (goal 50): a block in under 250 ms, a quarter-block battle
//! map in under 100 ms. Timed as the best of several runs so a loaded test
//! machine does not flake; run `cargo test --release` for the real figure.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::unwrap_used,
    missing_docs
)]

mod common;
use arda_refine::{refine_block, refine_window, CellKey};
use std::time::{Duration, Instant};

fn best<F: FnMut()>(mut f: F) -> Duration {
    (0..5)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed()
        })
        .min()
        .unwrap()
}

#[test]
fn a_block_generates_in_under_250_ms() {
    let src = common::world(42);
    for k in [common::RIVER_CELL, common::FOREST_CELL, CellKey::new(3, 10)] {
        let t = best(|| {
            refine_block(&src, k).unwrap();
        });
        println!("block {k:?}: {t:?}");
        assert!(t < Duration::from_millis(250), "{k:?} took {t:?}");
    }
}

#[test]
fn a_quarter_block_generates_in_under_100_ms() {
    let src = common::world(42);
    let (x0, y0) = (
        common::RIVER_CELL.x * 64 + 16,
        common::RIVER_CELL.y * 64 + 16,
    );
    let t = best(|| {
        refine_window(&src, x0, y0, 32, 32).unwrap();
    });
    println!("quarter block: {t:?}");
    assert!(t < Duration::from_millis(100), "took {t:?}");
}
