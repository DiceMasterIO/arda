//! Seamlessness across a 3 × 3 window (goal 42, 46): on every shared edge
//! the elevation, the ground corners and the river crossing agree exactly
//! between blocks generated independently.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::unwrap_used,
    missing_docs
)]

mod common;
use arda_refine::fixed::Fixed;
use arda_refine::{refine, Block, CellKey};

fn blocks(centre: CellKey, seed: u64) -> Vec<Vec<Block>> {
    (-1..=1)
        .map(|dy| {
            (-1..=1)
                .map(|dx| {
                    // Every block gets its own fresh source: nothing is shared.
                    let src = common::world(seed);
                    refine(&src, centre.offset(dx, dy)).unwrap()
                })
                .collect()
        })
        .collect()
}

fn check_pair(a: &Block, b: &Block, east: bool) -> usize {
    let mut water = 0;
    for k in 0..64 {
        // `a`'s halo square beyond the edge is `b`'s first square, and the
        // other way round.
        let (ax, ay, bx, by) = if east { (64, k, 0, k) } else { (k, 64, k, 0) };
        let halo = *a.halo.get(a.origin.0 + ax, a.origin.1 + ay).unwrap();
        let i = by as usize * 64 + bx as usize;
        assert_eq!(halo.0, b.elevation_ft[i], "elevation at {k}");
        assert_eq!(halo.1, b.depth_ft[i], "water depth at {k}");
        let (ax2, ay2, bx2, by2) = if east { (63, k, -1, k) } else { (k, 63, k, -1) };
        let back = *b.halo.get(b.origin.0 + bx2, b.origin.1 + by2).unwrap();
        let j = ay2 as usize * 64 + ax2 as usize;
        assert_eq!(back.0, a.elevation_ft[j]);
        assert_eq!(back.1, a.depth_ft[j]);
        if b.fixed[i] == Fixed::Water {
            water += 1;
        }
    }
    // The shared corner line.
    for k in 0..=64usize {
        let (ai, bi) = if east {
            (k * 65 + 64, k * 65)
        } else {
            (64 * 65 + k, k)
        };
        assert_eq!(a.corners[ai], b.corners[bi], "corner {k}");
    }
    water
}

#[test]
fn a_three_by_three_river_window_is_seamless() {
    for seed in [42, 9] {
        let g = blocks(common::RIVER_CELL.offset(4, -1), seed);
        let mut crossings = 0;
        for row in &g {
            for pair in row.windows(2) {
                crossings += check_pair(&pair[0], &pair[1], true);
            }
        }
        for rows in g.windows(2) {
            for (a, b) in rows[0].iter().zip(&rows[1]) {
                crossings += check_pair(a, b, false);
            }
        }
        assert!(crossings > 0, "the window must contain a river crossing");
    }
}

#[test]
fn lake_coast_and_hill_windows_are_seamless() {
    for centre in [
        CellKey::new(11, 3),
        CellKey::new(5, 13),
        CellKey::new(3, 10),
    ] {
        let g = blocks(centre, 5);
        for row in &g {
            for pair in row.windows(2) {
                check_pair(&pair[0], &pair[1], true);
            }
        }
        for rows in g.windows(2) {
            for (a, b) in rows[0].iter().zip(&rows[1]) {
                check_pair(a, b, false);
            }
        }
    }
}

#[test]
fn rivers_enter_and_leave_on_both_sides() {
    let src = common::world(42);
    for x in 3..9 {
        let a = refine(&src, CellKey::new(x, common::RIVER_ROW)).unwrap();
        let west = (0..64).filter(|&y| a.depth_ft[y * 64] > 0).count();
        let east = (0..64).filter(|&y| a.depth_ft[y * 64 + 63] > 0).count();
        assert!(west > 0 && east > 0, "cell {x}: west {west} east {east}");
    }
}
