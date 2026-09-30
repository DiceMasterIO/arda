//! WFC legality (goal 47): every neighbour pairing is legal after a normal
//! fill, and a forced contradiction ends in a flagged relaxed fill.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::unwrap_used,
    missing_docs
)]

mod common;
use arda_refine::classes::{Class, COUNT, LAND_MASK};
use arda_refine::tiles::{corners_of, legal_neighbours, tile_of, Corners};
use arda_refine::wfc::{solve, Problem};
use arda_refine::{refine, Block, CellKey};

fn corners(b: &Block, x: usize, y: usize) -> Corners {
    let c = &b.corners;
    [
        c[y * 65 + x],
        c[y * 65 + x + 1],
        c[(y + 1) * 65 + x + 1],
        c[(y + 1) * 65 + x],
    ]
}

#[test]
fn every_pairing_is_legal_after_a_normal_fill() {
    let src = common::world(42);
    let cells = [
        common::RIVER_CELL,
        common::FOREST_CELL,
        common::GRASS_CELL,
        CellKey::new(11, 2),
        CellKey::new(5, 13),
        CellKey::new(3, 10),
        CellKey::new(8, 10),
        CellKey::new(6, 5),
    ];
    for k in cells {
        let b = refine(&src, k).unwrap();
        assert!(!b.relaxed, "{k:?} needed the relaxed fill");
        for y in 0..64 {
            for x in 0..64 {
                let t =
                    tile_of(corners(&b, x, y)).unwrap_or_else(|| panic!("{k:?} ({x},{y}) no tile"));
                let c = corners_of(t).unwrap();
                if x + 1 < 64 {
                    assert!(legal_neighbours(c, corners(&b, x + 1, y), true));
                }
                if y + 1 < 64 {
                    assert!(legal_neighbours(c, corners(&b, x, y + 1), false));
                }
            }
        }
        assert!(b.rules.iter().all(|r| !r.review));
    }
}

#[test]
fn a_forced_contradiction_is_relaxed_and_flagged() {
    // Snow and water may never share a tile, yet the border forces them
    // onto opposite corners of every square along the diagonal.
    let n = 9;
    let mut masks = vec![LAND_MASK | Class::Water.bit(); n * n];
    for i in 0..n {
        masks[i] = Class::Snow.bit();
        masks[(n - 1) * n + i] = Class::Water.bit();
        masks[i * n] = Class::Snow.bit();
        masks[i * n + n - 1] = Class::Water.bit();
    }
    let p = Problem {
        n,
        masks,
        weights: vec![[1.0; COUNT]; n * n],
        seed: 1,
        key: (0, 0),
    };
    let s = solve(&p);
    assert!(s.relaxed);
    assert!(!s.illegal.is_empty());
    assert_eq!(s, solve(&p), "the relaxed fill is deterministic");
    // Fixed corners are kept.
    assert_eq!(s.classes[0], Class::Snow);
    assert_eq!(s.classes[n * n - 1], Class::Water);
}

#[test]
fn an_open_lattice_never_needs_the_relaxed_fill() {
    for seed in 0..6 {
        let n = 65;
        let p = Problem {
            n,
            masks: vec![LAND_MASK; n * n],
            weights: (0..n * n)
                .map(|i| {
                    let mut w = [0.05; COUNT];
                    w[1 + (i * 7 + seed as usize) % (COUNT - 1)] = 3.0;
                    w
                })
                .collect(),
            seed,
            key: (3, 4),
        };
        let s = solve(&p);
        assert!(!s.relaxed, "seed {seed}");
    }
}
