//! The pipeline over arda-refine's synthetic world: refined windows match
//! arda-refine, overlays compose by precedence, and windows composed
//! independently agree where they overlap (logic/09 Invariants 2, 3, 7).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

use arda_blocks::demo::DemoOverlays;
use arda_blocks::{Owner, Pipeline};
use arda_refine::synthetic::{self, GRASS_CELL, RIVER_CELL};
use arda_refine::{refine_block, CellKey};
use arda_tactical::Library;
use std::path::Path;
use std::sync::OnceLock;

fn lib() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
        Library::load(&dir).unwrap()
    })
}

fn pipeline() -> Pipeline {
    Pipeline::new(Box::new(synthetic::world(42)), 64)
}

fn cell_window(c: CellKey) -> (i64, i64, u32, u32) {
    (c.x * 64, c.y * 64, 64, 64)
}

#[test]
fn a_cell_window_is_the_refined_block_with_its_origin() {
    let p = pipeline();
    let got = p.window(cell_window(RIVER_CELL), lib(), None).unwrap();
    let want = refine_block(&synthetic::world(42), RIVER_CELL).unwrap();
    assert_eq!(got.layout.squares, want.layout.squares);
    assert_eq!(got.layout.placements, want.layout.placements);
    // The pipeline folds the placements' catalogue rules into refine's
    // (logic/12 §scene-sidecar, the rules contract), those anchored up to
    // the compose margin outside the cell included (review 3.5-8).
    let m = i64::from(arda_blocks::COMPOSE_MARGIN);
    let (x0, y0) = (RIVER_CELL.x * 64 - m, RIVER_CELL.y * 64 - m);
    let side = u32::try_from(64 + 2 * m).unwrap();
    let src = synthetic::world(42);
    let blocks: Vec<_> = (-1..=1)
        .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
        .map(|(dx, dy)| {
            let c = CellKey::new(RIVER_CELL.x + dx, RIVER_CELL.y + dy);
            arda_refine::refine(&src, c).unwrap()
        })
        .collect();
    let grown = arda_refine::output::assemble("g", &blocks, x0, y0, side, side);
    let mut folded = grown.rules.clone();
    let placed = arda_scene::squares::resolve_known(&grown.layout, lib(), p.art_seed());
    arda_scene::squares::fold_placements(&mut folded, &placed);
    let cut = arda_blocks::Cut {
        x: 8,
        y: 8,
        w: 64,
        h: 64,
    };
    assert_eq!(got.rules, arda_blocks::crop::rules(&folded, cut));
    assert!(got
        .rules
        .squares
        .iter()
        .any(|c| c.blocks_movement == Some(true)));
    assert_eq!(
        got.layout.origin,
        Some([RIVER_CELL.x * 64, RIVER_CELL.y * 64])
    );
    assert_eq!(got.layout.name, "cell_6_7");
    assert!(got.owners.contains(&Owner::Water));
    assert!(got.overlays.is_empty());
    got.layout.check(lib()).unwrap();
    arda_scene::build_scene(&got.layout, lib(), 1, Some(&got.rules)).unwrap();
}

#[test]
fn a_window_equals_its_cells_square_for_square() {
    let p = pipeline();
    let (x0, y0) = (5 * 64, 7 * 64);
    let wide = p.window((x0, y0, 192, 64), lib(), None).unwrap();
    for k in 0..3_i64 {
        let one = p.window((x0 + 64 * k, y0, 64, 64), lib(), None).unwrap();
        for y in 0..64_usize {
            let a = &wide.layout.squares[y * 192 + 64 * k as usize..][..64];
            let b = &one.layout.squares[y * 64..][..64];
            assert_eq!(a, b, "cell {k} row {y}");
            let ra = &wide.rules.squares[y * 192 + 64 * k as usize..][..64];
            assert_eq!(ra, &one.rules.squares[y * 64..][..64]);
        }
    }
}

#[test]
fn demo_overlays_compose_in_precedence_order_and_are_deterministic() {
    let p = pipeline();
    let demo = DemoOverlays::at_cell(GRASS_CELL.x, GRASS_CELL.y);
    let (x0, y0) = ((GRASS_CELL.x - 1) * 64, (GRASS_CELL.y - 1) * 64);
    let win = (x0, y0, 192, 192);
    let a = p.window(win, lib(), Some(&demo)).unwrap();
    let b = p.window(win, lib(), Some(&demo)).unwrap();
    assert_eq!(a, b);
    assert!(a.overlays.contains(&"town"), "{:?}", a.overlays);
    assert!(a.owners.contains(&Owner::Town));
    let base = p.window(win, lib(), None).unwrap();
    for (i, o) in a.owners.iter().enumerate() {
        // Water is never dried by a layer that neither keeps it nor decks it.
        if base.layout.squares[i].water_depth_ft > 0 && *o != Owner::Water {
            let wet = a.layout.squares[i].water_depth_ft > 0;
            assert!(wet || a.rules.squares[i].deck == Some(true), "square {i}");
        }
        // A claimed square carries only its owner's rules.
        if *o == Owner::Town {
            assert!(a.rules.squares[i].deck.is_some());
        }
    }
    a.layout.check(lib()).unwrap();
    arda_scene::build_scene(&a.layout, lib(), 1, Some(&a.rules)).unwrap();
}

#[test]
fn overlapping_demo_windows_agree_where_they_overlap() {
    let p = pipeline();
    let demo = DemoOverlays::at_cell(GRASS_CELL.x, GRASS_CELL.y);
    let (x0, y0) = ((GRASS_CELL.x - 1) * 64, (GRASS_CELL.y - 1) * 64);
    let big = p.window((x0, y0, 128, 128), lib(), Some(&demo)).unwrap();
    let off = (64_usize, 32_usize);
    let small = p
        .window((x0 + 64, y0 + 32, 64, 64), lib(), Some(&demo))
        .unwrap();
    for y in 0..64 {
        for x in 0..64 {
            let i = (y + off.1) * 128 + x + off.0;
            let j = y * 64 + x;
            assert_eq!(big.layout.squares[i], small.layout.squares[j], "({x}, {y})");
            assert_eq!(big.owners[i], small.owners[j], "({x}, {y})");
            assert_eq!(big.rules.squares[i], small.rules.squares[j], "({x}, {y})");
        }
    }
    // Walls, edge rules, placements and lights too, the small window's
    // border edges included (review §5: only squares were compared).
    let cut = arda_blocks::Cut {
        x: 64,
        y: 32,
        w: 64,
        h: 64,
    };
    let from_big = big.cut(cut, &small.layout.name);
    assert_eq!(from_big.layout, small.layout);
    assert_eq!(from_big.rules, small.rules);
    assert_eq!(from_big.owners, small.owners);
    assert!(!small.layout.walls.is_empty() || !small.layout.placements.is_empty());
}

#[test]
fn oversized_and_outside_windows_are_refused() {
    let p = pipeline();
    assert!(p.window((0, 0, 0, 64), lib(), None).is_err());
    assert!(p.window((0, 0, 1000, 64), lib(), None).is_err());
    assert!(p.window((-1, 0, 64, 64), lib(), None).is_err());
    assert!(p.window((16 * 64 - 32, 0, 64, 64), lib(), None).is_err());
}
