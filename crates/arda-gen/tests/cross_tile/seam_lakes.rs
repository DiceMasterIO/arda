//! Seam lakes: the shared surface, trimming below it, and agreement across
//! the seam for adjacent and straddling basins.

use super::*;

/// Spec R12 (d): seam lakes.
///
/// Synthetic. This used to scan seed 123 tile (1,0), the one near-rim lake
/// a 12-seed x 8-tile MICRO survey turned up at feature-03 §Q5's plan time
/// (task-5-report.md). Raising `LAKE_MIN_CELLS` to 300 and
/// `LAKE_MIN_DEPTH_MM` to 4 m leaves no near-rim lake anywhere in a MICRO
/// world — re-surveyed 12 seeds x 8 tiles
/// (`area::tests::survey_near_rim_lakes`), zero hits, because a MICRO tile
/// is too small to hold a 3 km² lake against its own rim. Rather than let
/// the loop find nothing and pass, the fixture now builds its own
/// rim-touching lake and runs it through the real
/// `fill` -> `water` -> `compose` path.
#[test]
fn seam_lakes_take_the_shared_surface_and_trim_below_it() {
    const SEAM_LAKE_SEED: u64 = 123;
    let area = AreaCoord::new(1, 0);
    let lake_ctx = build_continent(SEAM_LAKE_SEED, GenerateConfig::MICRO, 0);
    let bundle = bundle_for(SEAM_LAKE_SEED, &lake_ctx, area);

    // Touches the near-rim column x=1 (x=0 is a `fill::fill` border seed
    // and can never be submerged), 11 x 30 = 330 cells, over
    // `LAKE_MIN_CELLS`.
    let heights = walled_pit(1..12, 200..230);
    let filled = fill::fill(&heights, &bundle);
    let rain = area_rainfall(&bundle);
    let water = arda_gen::area::water(&filled, &bundle, &rain);
    let (cells, objects) = compose(&heights, &filled, &water, &rain, &bundle).unwrap();

    let near_rim = |c: CellCoord| {
        c.x() <= 1 || c.y() <= 1 || c.x() >= AREA_CELLS - 2 || c.y() >= AREA_CELLS - 2
    };

    let mut checked = 0u32;
    for lake in &objects.lakes {
        if !lake.cells.iter().any(|&c| near_rim(c)) {
            continue;
        }
        checked += 1;

        // Both sides of a seam would compute the same level: it comes
        // only from shared continent-tier data — `bundle.basin_km` where
        // the continent tier sees a depression (feature 02 §Q1 /
        // open-items #12), else the shared `bundle.filled_km` patch.
        let expected = independent_lake_surface_at(&bundle, &lake.cells)
            .expect("a near-rim lake must have at least one near-rim cell");
        assert_eq!(
            lake.surface.raw(),
            expected,
            "lake {} surface diverges from the independently-recomputed shared continent level",
            lake.id
        );

        // The trim holds: every recorded cell sits strictly below that
        // surface.
        for &c in &lake.cells {
            assert!(
                cells.get(c).height.raw() < lake.surface.raw(),
                "lake {} cell {c:?} sits at or above its own recorded surface",
                lake.id
            );
        }
    }
    assert!(
        checked > 0,
        "the synthetic rim-touching pit must yield a near-rim lake for this to check anything"
    );
}

/// Spec R12 (d), task-8 fix-wave-1 item 2: a real pairwise cross-tile
/// comparison, not only a same-tile oracle.
///
/// `seam_lakes_take_the_shared_surface_and_trim_below_it` above checks one
/// tile's lake against a same-tile independent recomputation; the brief's
/// actual requirement is that TWO ADJACENT tiles agree. This test builds
/// both.
///
/// P = tile (0,0), Q = tile (1,0), seed 123 attempt 0 — chosen because
/// Q's own near-rim lake (the fixture the test above uses) sits on Q's
/// WEST edge (local x in {0,1}, y in [236,237]), i.e. exactly on the P/Q
/// seam, making this the pair most likely to exercise a straddling lake,
/// not an arbitrary choice.
///
/// First, the shared-data property directly (feature 03 §Q5): every one
/// of the 512 absolute 100 m positions along the seam samples the
/// identical smoothstep-bilinear `filled_km` value from P's own bundle
/// (its east edge) and from Q's own bundle (its west edge). This holds by
/// construction — both edges name the same absolute cells
/// (`abs_cell`'s pinned-edge guarantee) and both patches sample the same
/// underlying `continent.hydrology.filled` grid — and it is testable
/// regardless of whether either side actually has a lake; it is WHY a
/// straddling lake's surface would agree.
///
/// Second, the straddling-lake case itself, written generically (not
/// hardcoded to "none found", so it would catch a real disagreement if the
/// fixture ever grows one): at seed 123 attempt 0, P has 3 lakes, none
/// near-rim at all; Q has exactly 1 near-rim lake, only on its west
/// (seam-facing) side, and nothing on P's matching east-facing side
/// overlaps it. **No straddling pair exists in this fixture** — checked
/// below, not assumed; documented rather than faked (task-8-report.md
/// "Fix wave 1").
#[test]
fn adjacent_tiles_agree_on_the_shared_seam_surface() {
    const SEAM_SEED: u64 = 123;
    let p_coord = AreaCoord::new(0, 0);
    let q_coord = AreaCoord::new(1, 0);
    let seam_ctx = build_continent(SEAM_SEED, GenerateConfig::MICRO, 0);
    let p_bundle = bundle_for(SEAM_SEED, &seam_ctx, p_coord);
    let q_bundle = bundle_for(SEAM_SEED, &seam_ctx, q_coord);
    let (_, p_objects) = arda_gen::area::generate_area(SEAM_SEED, &seam_ctx, &p_bundle).unwrap();
    let (_, q_objects) = arda_gen::area::generate_area(SEAM_SEED, &seam_ctx, &q_bundle).unwrap();

    // The shared-data property: P's east edge (local x = N) and Q's west
    // edge (local x = 0) name the same absolute cells, so sampling either
    // bundle's `filled_km` patch at the seam must agree exactly.
    for local_y in 0..N {
        let p_sample = independent_filled_km_sample(&p_bundle, N, local_y);
        let q_sample = independent_filled_km_sample(&q_bundle, 0, local_y);
        assert_eq!(
            p_sample, q_sample,
            "seam local_y={local_y}: P's east-edge filled_km sample disagrees with \
             Q's west-edge sample"
        );
    }

    // The straddling-lake case: relevant only where a lake's near-rim
    // cells sit on the seam-facing side of BOTH tiles at an overlapping
    // absolute y.
    let mut straddling_pairs = 0u32;
    for pl in &p_objects.lakes {
        let p_seam_ys: Vec<u16> = pl
            .cells
            .iter()
            .filter(|c| c.x() >= AREA_CELLS - 2)
            .map(|c| c.y())
            .collect();
        if p_seam_ys.is_empty() {
            continue;
        }
        for ql in &q_objects.lakes {
            let overlaps = ql
                .cells
                .iter()
                .any(|c| c.x() <= 1 && p_seam_ys.contains(&c.y()));
            if !overlaps {
                continue;
            }
            straddling_pairs += 1;
            assert_eq!(
                pl.surface.raw(),
                ql.surface.raw(),
                "straddling lakes P#{} / Q#{} disagree on their shared surface",
                pl.id,
                ql.id
            );
        }
    }
    // Measured: 0 (see this test's doc comment). Asserted, not just
    // claimed, so a future fixture change that grows a straddling pair
    // forces this comment to be revisited rather than silently going
    // vacuously true forever.
    assert_eq!(
        straddling_pairs, 0,
        "a straddling lake pair now exists on seed {SEAM_SEED} tiles {p_coord:?}/{q_coord:?} — \
         update this test's doc comment, it no longer describes the fixture"
    );
}

/// Hand-built `heights` for the synthetic straddling-basin test below: dry
/// everywhere except one solid rectangular pit. Every dry ("wall") cell
/// sits at `WALL_BASE + (chebyshev distance to the nearest tile edge) *
/// WALL_STEP` -- a genuine (non-flat) ramp descending to the rim on every
/// side, so `fill::fill`'s priority-flood can never raise a wall cell
/// above its own height: walking straight from any wall cell to its
/// nearest edge is already a non-increasing path, which is the best any
/// path can do, so the priority-flood (a min-max-path algorithm) assigns
/// that cell exactly its own height. The pit is therefore the only basin
/// `fill::fill` can find; its near-rim contact is exactly the pit cells
/// whose `x`/`y` falls in `near_rim`'s band. `x_range`/`y_range` are
/// tile-local, end-exclusive.
fn walled_pit(x_range: std::ops::Range<i32>, y_range: std::ops::Range<i32>) -> Vec<i32> {
    const WALL_BASE: i32 = 10_000;
    const WALL_STEP: i32 = 100;
    const PIT_HEIGHT: i32 = 5_000;
    let mut heights = vec![0i32; usize::try_from(N * N).unwrap_or(0)];
    for y in 0..N {
        for x in 0..N {
            let dist = x.min(y).min(N - 1 - x).min(N - 1 - y);
            let in_pit = x_range.contains(&x) && y_range.contains(&y);
            let i = usize::try_from(y * N + x).unwrap_or(0);
            heights[i] = if in_pit {
                PIT_HEIGHT
            } else {
                WALL_BASE + dist * WALL_STEP
            };
        }
    }
    heights
}

/// SYNTHETIC regression test for feature 03 §Q5 / feature 02 §Q1
/// (open-items #12) -- verification-gap task, closed exactly by the
/// continent-tier lake identity `ContinentHydrology::basin_surface` adds.
///
/// The branch review: the feature claimed #12 closed (a basin straddling
/// a tile seam reaching different spill levels on each side), but no
/// fixture ever produced a straddling lake pair, so the then-shipped rule
/// (max bilinear sample of `filled_km`, still the fallback below) was
/// never observed working on the real scenario. A documented sweep of
/// 4,280 (seed, seam) combinations -- 300 MICRO seeds x 10 seam-pairs,
/// plus 40 seeds of a 200x400 km continent x 32 seam-pairs -- found zero
/// natural fixtures where both sides of a seam independently grow a
/// large-enough lake touching it (see
/// `.superpowers/sdd/straddling-lakes-report.md`), so this test is
/// synthetic: two real `TileBundle`s (`ctx()`'s seed 42 attempt 2, tiles
/// (0,1)/(1,1)), each fed a `walled_pit` `heights` array that forces
/// exactly one basin reaching the shared seam -- at a DIFFERENT (only
/// partially overlapping) span of local y on each side, the exact risk
/// under test.
///
/// Two phases:
///
/// Phase 1 uses the real `fill` -> `water` -> `compose` path with each
/// bundle's real `basin_km`. Neither near-rim contact sees a continent
/// depression, so the bilinear fallback applies. Independent bundle sampling
/// derives surfaces 120,025 and 119,747 mm, a 278 mm gap. Candidate 04 measured
/// 299,106 / 300,786 mm (1,680 mm gap); the earlier physical fixture was
/// 306,426 / 308,937 mm (2,511 mm gap). This is the documented
/// limitation of the retained local API: a synthetic fine-only pit has no
/// shared continent depression to make both contact spans choose one level.
/// The exact fixture is kept as a diagnostic alongside the independent oracle.
///
/// Phase 2, new: the identical P/Q fragments (same heights, same
/// differing contact spans), but with `basin_km` overridden on both
/// bundles to one shared flat value -- constructing the one precondition
/// no fixture, natural or synthetic-heights, could supply on its own
/// (`fill::fill`'s output depends only on `heights`, already computed in
/// phase 1, never on `bundle` -- see `fill::fill`'s own signature -- so
/// this phase reuses phase 1's `filled`/`water`/`rain` unchanged and only
/// recomposes with the mutated bundles). Under that constructed premise
/// the fix's actual claim holds: both sides read back the identical
/// constant regardless of their differing contact spans, so the gap is
/// exactly 0 mm -- proof the mechanism itself is correct, even though no
/// known fixture exercises it end-to-end without this construction.
#[test]
fn straddling_basins_agree_on_their_surface_across_the_seam() {
    let p_coord = AreaCoord::new(0, 1);
    let q_coord = AreaCoord::new(1, 1);
    let p_bundle = bundle_for(42, ctx(), p_coord);
    let q_bundle = bundle_for(42, ctx(), q_coord);

    // P's fragment touches the seam (its own near-rim column, x=510) at
    // local y in [425,455); Q's fragment touches ITS near-rim column
    // (x=1) at local y in [429,459) -- overlapping on [429,455) (26 of 30
    // cells) but not identical spans. 11 columns x 30 rows = 330 cells,
    // over `LAKE_MIN_CELLS`; the span was 12 rows until that floor rose,
    // which is why phase 1's pinned surfaces were re-derived.
    let p_heights = walled_pit(500..511, 425..455);
    let q_heights = walled_pit(1..12, 429..459);

    let p_filled = fill::fill(&p_heights, &p_bundle);
    let q_filled = fill::fill(&q_heights, &q_bundle);
    let p_rain = area_rainfall(&p_bundle);
    let q_rain = area_rainfall(&q_bundle);
    let p_water = arda_gen::area::water(&p_filled, &p_bundle, &p_rain);
    let q_water = arda_gen::area::water(&q_filled, &q_bundle, &q_rain);
    let (_, p_objects) = compose(&p_heights, &p_filled, &p_water, &p_rain, &p_bundle).unwrap();
    let (_, q_objects) = compose(&q_heights, &q_filled, &q_water, &q_rain, &q_bundle).unwrap();

    assert_eq!(
        p_objects.lakes.len(),
        1,
        "P's walled pit must produce exactly the one synthetic lake"
    );
    assert_eq!(
        q_objects.lakes.len(),
        1,
        "Q's walled pit must produce exactly the one synthetic lake"
    );
    let p_lake = &p_objects.lakes[0];
    let q_lake = &q_objects.lakes[0];
    assert_eq!(
        p_lake.cells.len(),
        330,
        "P must retain the entire synthetic pit"
    );
    assert_eq!(
        q_lake.cells.len(),
        330,
        "Q must retain the entire synthetic pit"
    );
    assert!(
        p_lake.cells.iter().any(|c| c.x() == 510),
        "P's lake must actually touch the seam-facing near-rim column"
    );
    assert!(
        q_lake.cells.iter().any(|c| c.x() == 1),
        "Q's lake must actually touch the seam-facing near-rim column"
    );

    // Phase 1: confirm the diagnosis directly -- neither fragment's
    // near-rim cells see a continent depression at this synthetic
    // location, so the new preference rule cannot engage and the
    // fallback path governs, unchanged.
    let no_basin = arda_gen::continent::hydrology::NO_BASIN;
    let seam_sees_no_basin = |bundle: &TileBundle, seam_x: u16, ys: std::ops::Range<i32>| {
        ys.map(|y| u16::try_from(y).unwrap()).all(|y| {
            independent_basin_km_sample(bundle, i32::from(seam_x), i32::from(y)) == no_basin
        })
    };
    assert!(
        seam_sees_no_basin(&p_bundle, 510, 425..455),
        "P's near-rim cells unexpectedly see a continent depression; the fixture drifted -- \
         re-derive phase 1's pinned numbers"
    );
    assert!(
        seam_sees_no_basin(&q_bundle, 1, 429..459),
        "Q's near-rim cells unexpectedly see a continent depression; the fixture drifted -- \
         re-derive phase 1's pinned numbers"
    );

    let p_expected = independent_lake_surface_at(&p_bundle, &p_lake.cells)
        .expect("P's synthetic lake must touch the near rim");
    let q_expected = independent_lake_surface_at(&q_bundle, &q_lake.cells)
        .expect("Q's synthetic lake must touch the near rim");
    assert_eq!(
        p_lake.surface.raw(),
        p_expected,
        "P must use the independently sampled fallback surface"
    );
    assert_eq!(
        q_lake.surface.raw(),
        q_expected,
        "Q must use the independently sampled fallback surface"
    );
    let gap = p_lake.surface.raw().abs_diff(q_lake.surface.raw());
    println!("legacy straddling fallback: independently_sampled_mm=({p_expected},{q_expected}), composed_mm=({},{}), gap_mm={gap}, lake_cell_counts=({},{})", p_lake.surface.raw(), q_lake.surface.raw(), p_lake.cells.len(), q_lake.cells.len());

    // Phase 2: construct the fix's actual precondition -- both fragments'
    // near-rim cells mapping into ONE shared continent depression -- and
    // confirm the surfaces then agree EXACTLY, over the SAME differing
    // contact spans phase 1 used. `fill`/`water`/`rain` do not depend on
    // `bundle.basin_km` (see the doc comment above), so they carry over
    // unchanged; only `compose` (which runs `collect_lakes`) needs re-running.
    const SHARED_DEPRESSION_MM: i32 = 500_000;
    let mut p_shared = p_bundle.clone();
    let mut q_shared = q_bundle.clone();
    p_shared.basin_km = vec![SHARED_DEPRESSION_MM; PATCH_KM * PATCH_KM];
    q_shared.basin_km = vec![SHARED_DEPRESSION_MM; PATCH_KM * PATCH_KM];

    let (_, p_objects2) = compose(&p_heights, &p_filled, &p_water, &p_rain, &p_shared).unwrap();
    let (_, q_objects2) = compose(&q_heights, &q_filled, &q_water, &q_rain, &q_shared).unwrap();
    assert_eq!(
        p_objects2.lakes.len(),
        1,
        "P's shared-depression lake must still be exactly one"
    );
    assert_eq!(
        q_objects2.lakes.len(),
        1,
        "Q's shared-depression lake must still be exactly one"
    );
    let p_lake2 = &p_objects2.lakes[0];
    let q_lake2 = &q_objects2.lakes[0];

    assert_eq!(
        p_lake2.surface.raw(),
        SHARED_DEPRESSION_MM,
        "P did not take the shared continent depression's surface"
    );
    assert_eq!(
        q_lake2.surface.raw(),
        SHARED_DEPRESSION_MM,
        "Q did not take the shared continent depression's surface"
    );
    assert_eq!(
        p_lake2.surface.raw(),
        q_lake2.surface.raw(),
        "straddling fragments of the SAME continent depression must agree EXACTLY, whatever \
         their differing contact spans -- this is what closes open-items #12 exactly"
    );
    println!(
        "legacy straddling shared depression: composed_mm=({},{}), gap_mm=0",
        p_lake2.surface.raw(),
        q_lake2.surface.raw()
    );
    assert_eq!(
        (p_lake.surface.raw(), q_lake.surface.raw()),
        (120_025, 119_747),
        "the corrected fallback pair drifted; re-derive the independent surface fixture"
    );
    assert_eq!(
        gap, 278,
        "the legacy fallback gap drifted from 278 mm; re-derive the independent surface fixture"
    );
}
