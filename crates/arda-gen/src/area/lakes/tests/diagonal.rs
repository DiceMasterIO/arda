//! Near-rim lake surveys and diagonally adjacent basins that share released
//! cells.

use super::*;

/// Hand-built `heights` for the diagonal-neighbour regression test
/// below (feature 03 §Q5, round-2 review fix): dry everywhere except
/// two near-rim basins, X and Y, positioned so exactly one cell of
/// each is an 8-neighbour of the other while neither basin is
/// 4-adjacent to the other at all — the shape `fill::find_basins`'
/// 4-connected grouping and the outlet search's 8-connected
/// (`fill::NEIGHBOURS`) search disagree on. Wall cells use the same
/// non-flat ramp as `cross_tile.rs`'s `walled_pit` (`WALL_BASE +`
/// chebyshev distance to the nearest edge `* WALL_STEP`), so
/// `fill::fill`'s priority-flood can never raise one above its own
/// height, keeping X and Y the only two basins the fixture produces.
///
/// Each basin is a "sacrificial tail plus surviving body," not a
/// single flat rectangle: the tail is the only part that needs to
/// touch `near_rim` (so `clamp_near_rim` applies to the whole basin)
/// and is deliberately the part the clamp trims away, so the
/// SURVIVING body never sits within one cell of the tile's literal
/// edge — the actual rim column/row is a flat, unbeatable
/// `WALL_BASE` floor (`fill::fill` seeds it directly, never raised),
/// which would otherwise always out-price whatever this test wants to
/// compare it against.
///
/// X: columns 1..=2 (the sacrificial, near-rim tail) plus column 3
/// (the surviving body), rows 2..=151 — 450 cells, all `DIAG_HIGH_MM`
/// on the tail and `DIAG_LOW_MM` on the body.
///
/// Y: a single cell at (4, 152) — diagonally adjacent to X's (3, 151)
/// (dx=1, dy=1) and 4-adjacent to nothing in X — plus columns 1..=3
/// (sacrificial tail) and column 4 (surviving body) for rows
/// 153..=302 — 601 cells total, same height split as X.
///
/// A flat `filled_km` clamp at `DIAG_CLAMP_MM`, between `DIAG_LOW_MM`
/// and `DIAG_HIGH_MM`, trims every tail cell from both basins,
/// including Y's lone (4, 152) — releasing it back to dry land
/// immediately next to X's surviving (3, 151).
fn diagonal_neighbour_pits() -> Vec<i32> {
    const WALL_BASE: i32 = 10_000;
    const WALL_STEP: i32 = 100;
    let mut heights = vec![0i32; usize::try_from(N * N).unwrap_or(0)];
    for y in 0..N {
        for x in 0..N {
            let dist = x.min(y).min(N - 1 - x).min(N - 1 - y);
            let i = usize::try_from(y * N + x).unwrap_or(0);
            heights[i] = WALL_BASE + dist * WALL_STEP;
        }
    }
    let mut set = |x: i32, y: i32, h: i32| {
        let i = usize::try_from(y * N + x).unwrap_or(0);
        heights[i] = h;
    };
    // Two submerged columns per basin, 150 rows: 300 cells each,
    // which is `LAKE_MIN_CELLS`. Column 1 stays high in both so each
    // basin keeps a sacrificial tail the clamp trims. `DIAG_LOW_MM`
    // sits 4.5 m under `DIAG_CLAMP_MM` so both clear
    // `LAKE_MIN_DEPTH_MM` as well; the test asserts both floors are
    // met so a future retune fails here with a readable message
    // rather than as a bare count mismatch.
    for y in 2..=151 {
        set(1, y, DIAG_HIGH_MM);
        set(2, y, DIAG_LOW_MM);
        set(3, y, DIAG_LOW_MM);
    }
    set(4, 152, DIAG_HIGH_MM);
    for y in 153..=302 {
        set(1, y, DIAG_HIGH_MM);
        set(2, y, DIAG_HIGH_MM);
        set(3, y, DIAG_LOW_MM);
        set(4, y, DIAG_LOW_MM);
    }
    heights
}

/// Survey helper: finds a (seed, tile) whose MICRO generation carries
/// a near-rim lake under the CURRENT thresholds. Run manually when
/// `LAKE_MIN_CELLS` / `LAKE_MIN_DEPTH_MM` change and
/// `edge_touching_basins_take_the_continent_spill_level` loses its
/// fixture.
#[test]
#[ignore = "near-rim lake survey, run manually after retuning the lake thresholds"]
fn survey_near_rim_lakes() {
    for seed in [123u64, 42, 7, 1, 2, 3, 5, 11, 17, 99, 436_342, 2024] {
        let ctx = build_continent(seed, GenerateConfig::MICRO, 0);
        for ay in 0..4 {
            for ax in 0..2 {
                let area = AreaCoord::new(ax, ay);
                let b = bundle_for(seed, &ctx, area);
                let (_, objects) = generate_area(seed, &ctx, &b).unwrap();
                for l in &objects.lakes {
                    if l.cells.iter().any(|&c| near_rim(c)) {
                        println!(
                            "HIT seed {seed} tile ({ax},{ay}) lake {} cells {} depth_mm {}",
                            l.id,
                            l.cells.len(),
                            l.depth_mm
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn diagonally_adjacent_basins_do_not_exclude_each_others_released_cells() {
    // Feature 03 §Q5, round-2 review fix (the gap round-1 left open):
    // `recompute_outlet` judged "is this candidate still submerged in
    // some OTHER basin" against `fill::fill`'s pre-clamp snapshot,
    // which is built once and never updated as sibling basins are
    // themselves clamped and trimmed. Basin membership comes from a
    // 4-connected flood (`fill::find_basins`) while the outlet search
    // is 8-connected (`fill::NEIGHBOURS`), so two basins can be
    // diagonally adjacent without ever merging — exactly the shape
    // `diagonal_neighbour_pits` builds: two near-rim basins, X and Y,
    // touching at exactly one diagonal pair of cells and nowhere
    // else, both down-clamped and trimmed by the same flat
    // `filled_km` surface.
    let ctx = fixture_ctx();
    let mut b = bundle_for(SEAM_LAKE_SEED, &ctx, AreaCoord::new(0, 1));
    b.filled_km = vec![DIAG_CLAMP_MM; PATCH_KM * PATCH_KM];

    let heights = diagonal_neighbour_pits();
    let filled = fill::fill(&heights, &b);
    let lakes = collect_lakes(&heights, &filled, &b);

    let x_tail = coord(1, 100).unwrap();
    let x_touch = coord(3, 151).unwrap();
    let y_touch = coord(4, 152).unwrap(); // trimmed away from Y
    let y_tail = coord(1, 200).unwrap();
    let y_body = coord(4, 200).unwrap();

    assert_eq!((i32::from(x_touch.x()) - i32::from(y_touch.x())).abs(), 1);
    assert_eq!((i32::from(x_touch.y()) - i32::from(y_touch.y())).abs(), 1);

    assert!(
        DIAG_CLAMP_MM - DIAG_LOW_MM >= i32::try_from(LAKE_MIN_DEPTH_MM).unwrap_or(i32::MAX),
        "fixture drifted: the synthetic pits are shallower than LAKE_MIN_DEPTH_MM \
         ({LAKE_MIN_DEPTH_MM} mm), so collect_lakes will discard them"
    );
    assert_eq!(lakes.len(), 2, "expected exactly the two synthetic lakes");
    for l in &lakes {
        assert!(
            l.cells.len() >= LAKE_MIN_CELLS,
            "fixture drifted: synthetic lake {} has {} cells, under LAKE_MIN_CELLS ({})",
            l.id,
            l.cells.len(),
            LAKE_MIN_CELLS
        );
    }
    let x_lake = lakes
        .iter()
        .find(|l| l.cells.contains(&x_touch))
        .expect("basin X must survive the clamp as a lake");
    let y_lake = lakes
        .iter()
        .find(|l| l.cells.contains(&y_body))
        .expect("basin Y must survive the clamp as a lake");
    assert_ne!(
        x_lake.id, y_lake.id,
        "X and Y must be reported as distinct lakes"
    );

    // Both basins actually lost cells to the trim -- "both trimmed",
    // not just a basin whose surface changed with no cell loss -- and
    // in particular Y's diagonal tip did.
    assert!(
        !x_lake.cells.contains(&x_tail),
        "X's sacrificial tail cell {x_tail:?} must have been trimmed by the clamp"
    );
    assert!(
        !y_lake.cells.contains(&y_tail),
        "Y's sacrificial tail cell {y_tail:?} must have been trimmed by the clamp"
    );
    assert!(
        !y_lake.cells.contains(&y_touch),
        "Y's diagonal tip {y_touch:?} must have been trimmed by the clamp -- it is the \
         cell the round-2 bug wrongly kept treating as still submerged"
    );

    // The core mechanism the bug got wrong: `fill::fill`'s pre-clamp
    // snapshot still shows Y's released tip as submerged (that is
    // exactly why the old per-basin check excluded it)...
    assert!(
        filled.get(y_touch) > heights[y_touch.index()],
        "fixture drifted: {y_touch:?} must still read as submerged in fill::fill's \
         pre-clamp snapshot for this to exercise the bug's exact mechanism"
    );
    // ...yet it belongs to neither surviving lake, so it is genuinely
    // free once every basin's fate (not just Y's) is known -- the
    // fact `recompute_outlet` now judges candidates against.
    assert!(
        !x_lake.cells.contains(&y_touch) && !y_lake.cells.contains(&y_touch),
        "{y_touch:?} must not belong to either surviving lake"
    );

    // Every surviving lake still has a valid, well-formed outlet: not
    // one of its own cells, not a cell of the other lake, and `Some`
    // given the tile's wall ramp always offers a dry neighbour.
    for (lake, other) in [(x_lake, y_lake), (y_lake, x_lake)] {
        let out = lake.outlet.unwrap_or_else(|| {
            panic!(
                "lake {} has a dry neighbour available (at least the tile's own wall \
                 ramp) so its outlet must be Some",
                lake.id
            )
        });
        assert!(
            !lake.cells.contains(&out),
            "lake {}'s outlet {out:?} must not be one of its own surviving cells",
            lake.id
        );
        assert!(
            !other.cells.contains(&out),
            "lake {}'s outlet {out:?} must not be a cell of the other lake",
            lake.id
        );
    }
}
