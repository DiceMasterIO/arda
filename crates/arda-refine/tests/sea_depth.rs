//! Sea depth follows the sea floor square by square (review 3.4-3): deep
//! water is continuous across a block edge, as it is inside a block,
//! instead of jumping by the difference of two cells' depths.
#![allow(clippy::unwrap_used, missing_docs)]

mod common;
use arda_refine::{refine, Block, CellKey};

/// Largest depth step, feet, between deep-water neighbours: the synthetic
/// sea floor falls about 1.5 ft per square, so one step plus rounding.
const STEP_FT: i32 = 3;

/// Depth differences across the shared edge of `a` and the block east of
/// it (`east`) or south of it, over pairs where both squares are 5 ft deep
/// or more.
fn across(a: &Block, b: &Block, east: bool) -> Vec<i32> {
    (0..64usize)
        .filter_map(|k| {
            let (i, j) = if east {
                (k * 64 + 63, k * 64)
            } else {
                (63 * 64 + k, k)
            };
            let (da, db) = (a.depth_ft[i], b.depth_ft[j]);
            (da >= 5 && db >= 5).then(|| (i32::from(da) - i32::from(db)).abs())
        })
        .collect()
}

/// The same inside `a`, one column (or row) in from the edge.
fn inside(a: &Block, east: bool) -> Vec<i32> {
    (0..64usize)
        .filter_map(|k| {
            let (i, j) = if east {
                (k * 64 + 62, k * 64 + 63)
            } else {
                (62 * 64 + k, 63 * 64 + k)
            };
            let (da, db) = (a.depth_ft[i], a.depth_ft[j]);
            (da >= 5 && db >= 5).then(|| (i32::from(da) - i32::from(db)).abs())
        })
        .collect()
}

#[test]
fn deep_sea_depth_is_continuous_across_block_edges() {
    let src = common::world(42);
    let row = common::SEA_ROW;
    let (nw, ne) = (
        refine(&src, CellKey::new(6, row)).unwrap(),
        refine(&src, CellKey::new(7, row)).unwrap(),
    );
    let sw = refine(&src, CellKey::new(6, row + 1)).unwrap();
    let mut pairs = 0;
    for (a, b, east) in [(&nw, &ne, true), (&nw, &sw, false)] {
        let edge = across(a, b, east);
        let own = inside(a, east);
        assert!(!edge.is_empty() && !own.is_empty(), "no deep water");
        let worst = edge.iter().max().copied().unwrap_or(0);
        println!(
            "{} edge: {} deep pairs, worst step {worst} ft (inside: {} ft)",
            if east { "east" } else { "south" },
            edge.len(),
            own.iter().max().copied().unwrap_or(0)
        );
        assert!(worst <= STEP_FT, "a {worst} ft step across the block edge");
        pairs += edge.len();
    }
    assert!(pairs >= 64);
    // The sea floor is not flat: depth varies inside a block too.
    let depths: std::collections::BTreeSet<u8> =
        nw.depth_ft.iter().copied().filter(|&d| d >= 5).collect();
    assert!(depths.len() > 3, "sea depth is constant in the block");
}
