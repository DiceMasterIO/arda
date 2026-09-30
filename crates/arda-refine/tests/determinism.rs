//! Determinism: identical bytes on repeated runs, independent of the order
//! in which blocks are generated (goal 42, §8).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::unwrap_used,
    missing_docs
)]

mod common;
use arda_refine::{refine_block, refine_window, CellKey};

#[test]
fn repeated_runs_give_identical_bytes() {
    let src = common::world(42);
    let a = refine_block(&src, common::RIVER_CELL).unwrap();
    let b = refine_block(&src, common::RIVER_CELL).unwrap();
    assert_eq!(a.layout_json().unwrap(), b.layout_json().unwrap());
    assert_eq!(a.rules_json().unwrap(), b.rules_json().unwrap());
    assert_eq!(a.meta_json().unwrap(), b.meta_json().unwrap());
}

#[test]
fn generation_order_does_not_matter() {
    let cells = [
        common::RIVER_CELL,
        CellKey::new(11, 4),
        common::FOREST_CELL,
        CellKey::new(3, 11),
    ];
    let forward = common::world(7);
    let a: Vec<String> = cells
        .iter()
        .map(|&c| refine_block(&forward, c).unwrap().layout_json().unwrap())
        .collect();
    // A fresh source, the reverse order, and a window generated first.
    let backward = common::world(7);
    let _ = refine_window(&backward, 64 * 5, 64 * 6, 192, 192).unwrap();
    let mut b: Vec<String> = cells
        .iter()
        .rev()
        .map(|&c| refine_block(&backward, c).unwrap().layout_json().unwrap())
        .collect();
    b.reverse();
    assert_eq!(a, b);
}

#[test]
fn a_window_equals_its_blocks() {
    let src = common::world(3);
    let win = refine_window(&src, 64 * 6, 64 * 7, 128, 64).unwrap();
    for (i, cx) in [6, 7].iter().enumerate() {
        let one = refine_block(&src, CellKey::new(*cx, 7)).unwrap();
        for y in 0..64 {
            for x in 0..64 {
                let w = &win.layout.squares[y * 128 + i * 64 + x];
                assert_eq!(w, &one.layout.squares[y * 64 + x]);
            }
        }
    }
}

#[test]
fn different_seeds_differ() {
    let a = refine_block(&common::world(1), common::FOREST_CELL).unwrap();
    let b = refine_block(&common::world(2), common::FOREST_CELL).unwrap();
    assert_ne!(a.layout_json().unwrap(), b.layout_json().unwrap());
}
