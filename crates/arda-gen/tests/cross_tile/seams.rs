//! Seam entries and crossings: inflow at a seam matches independent
//! hydrology and lines up with the upstream tile's outlet.

use super::*;

/// Spec R12 (a): seam continuity on tile (0,2) -> tile (0,3).
///
/// The original fixture had 31 north-edge crossings, all with an upstream
/// outlet in the matching absolute 10-cell window. Candidate 04 had 12
/// crossings; the current creep-only coarse terrain leaves six on that same
/// seam, still all matched. Their entering fields are checked independently,
/// together with every other MICRO tile's entries. The count of receiving
/// drainage values exceeding the independently computed upstream local
/// catchment is reported as a diagnostic. Conservation is instead checked
/// exactly against the added coarse catchments/discharges whose unseeded
/// routing paths reach each entry. Derivation: `cross-tile-c06-fixtures.md`.
#[test]
fn seam_entries_match_independent_hydrology_and_the_upstream_outlet() {
    let a_coord = AreaCoord::new(0, 2);
    let b_coord = AreaCoord::new(0, 3);
    let a = tile(a_coord);
    let b = tile(b_coord);

    // Every entering entry on every MICRO tile equals an independent
    // re-derivation from ctx.hydrology and the shared boundary surface.
    // This remains an invariant when corrected terrain moves crossings.
    let mut independent_entries = 0;
    for t in tiles() {
        let want = independent_entering(42, ctx(), t.coord);
        assert_eq!(
            t.bundle.entering.len(),
            want.len(),
            "tile {:?} entering count drifted from the independent re-derivation",
            t.coord
        );
        for e in &t.bundle.entering {
            let got = (u64::from(e.catchment_km2), e.discharge.raw(), e.order);
            let expected = want.get(&e.cell).unwrap_or_else(|| {
                panic!(
                    "independent re-derivation has no entry at seed {:?}",
                    e.cell
                )
            });
            assert_eq!(
                got, *expected,
                "tile {:?} seed {:?}: (catchment, discharge, order) diverges from the independent re-derivation",
                t.coord, e.cell
            );
            independent_entries += 1;
        }
    }

    // A's water grid marks an outlet within the same absolute-aligned
    // 10-cell window for every one of B's north-edge crossings — water
    // does leave where it should enter.
    let north: Vec<_> = b
        .bundle
        .entering
        .iter()
        .filter(|e| e.cell.y() == 0)
        .collect();
    let origin_x = a_coord.x * N;
    let mut ge_holds = 0u32;
    let (unseeded, _) = without_entering(b);
    let independent_sources = independent_entering(42, ctx(), b_coord);
    for e in &north {
        let (ax, _) = abs_cell(b_coord, e.cell.x(), e.cell.y());
        let win_abs_lo = ax.div_euclid(10) * 10;
        let win_lo = (win_abs_lo - origin_x).clamp(0, N - 1);
        let win_hi = (win_abs_lo + 10 - origin_x).clamp(0, N);

        let mut best: Option<u32> = None;
        for j in win_lo..win_hi {
            let at = cc(j, N - 1); // A's south row: A sits north of B
            if a.water.is_outlet(at) {
                let d = a.water.drainage_at(at);
                best = Some(best.map_or(d, |v| v.max(d)));
            }
        }
        let outlet_drainage = best.unwrap_or_else(|| {
            panic!(
                "seed {:?}: A marks no outlet in the matching window [{win_lo},{win_hi}) — \
                 water does not leave A where B expects to receive it",
                e.cell
            )
        });
        if b.water.drainage_at(e.cell) >= outlet_drainage {
            ge_holds += 1;
        }

        // One km² contains exactly 100 of the 100 m terrain cells. Follow
        // independently derived sources on the unseeded graph and sum their
        // physical loads. Omitting or duplicating any contributing inflow
        // violates this equality; a comparison to A's separately generated
        // fine catchment cannot establish the retained coarse API's contract.
        let mut added_cells = 0_u64;
        let mut added_discharge = 0_u64;
        for (&source, &(catchment_km2, discharge_l_s, _)) in &independent_sources {
            if path_reaches(&unseeded, source, e.cell) {
                added_cells += catchment_km2 * 100;
                added_discharge += discharge_l_s;
            }
        }
        assert!(added_cells > 0, "the entry's own inflow must contribute");
        assert_eq!(
            u64::from(b.water.drainage_at(e.cell)),
            u64::from(unseeded.drainage_at(e.cell)) + added_cells,
            "seed {:?}: entered catchment was omitted or counted more than once",
            e.cell
        );
        assert_eq!(
            b.water.discharge_at(e.cell),
            unseeded.discharge_at(e.cell) + added_discharge,
            "seed {:?}: entered discharge was omitted or counted more than once",
            e.cell
        );
        println!(
            "legacy seam entry {:?}: catchment_km2={}, discharge_l_s={}, order={}, upstream_outlet_drainage={}, receiving_drainage={}, unseeded_drainage={}, independently_added_cells={added_cells}, independently_added_discharge={added_discharge}",
            e.cell, e.catchment_km2, e.discharge.raw(), e.order, outlet_drainage,
            b.water.drainage_at(e.cell), unseeded.drainage_at(e.cell)
        );
    }
    println!("legacy seam totals: independently_checked_entries={independent_entries}, north_entries={}, matched_outlets={}, receiving_ge_upstream={ge_holds}", north.len(), north.len());
    assert_eq!(
        north.len(),
        6,
        "tile (0,3) north-edge entering count drifted; re-derive the independent crossing fixture"
    );
}

/// Distance, in local cells along a shared boundary line, from `pos` to
/// the window `[lo, hi)` — 0 when `pos` already sits inside it.
fn distance_to_window(pos: i32, lo: i32, hi: i32) -> i32 {
    if pos >= lo && pos < hi {
        0
    } else if pos < lo {
        lo - pos
    } else {
        pos - hi + 1
    }
}

#[test]
fn nonmatching_outlet_distance_uses_the_nearest_boundary_position() {
    let window = (20, 30);
    let outlets = [3, 7, 42, 47];
    let nearest = outlets
        .into_iter()
        .map(|j| distance_to_window(j, window.0, window.1))
        .min();
    assert_eq!(nearest, Some(13));
    assert_eq!(distance_to_window(20, window.0, window.1), 0);
    assert_eq!(distance_to_window(29, window.0, window.1), 0);
    assert_eq!(distance_to_window(30, window.0, window.1), 1);
}

/// The MICRO tile in compass direction `edge` from `coord`, or `None` past
/// the grid's own edge (`GenerateConfig::MICRO`'s real extent, not a
/// hardcoded 2x4).
fn neighbour_area(coord: AreaCoord, edge: char) -> Option<AreaCoord> {
    let (nx, ny) = match edge {
        'N' => (coord.x, coord.y - 1),
        'S' => (coord.x, coord.y + 1),
        'E' => (coord.x + 1, coord.y),
        _ => (coord.x - 1, coord.y), // 'W'
    };
    let cfg = GenerateConfig::MICRO;
    let in_range = (0..cfg.areas_wide()).contains(&nx) && (0..cfg.areas_high()).contains(&ny);
    in_range.then_some(AreaCoord::new(nx, ny))
}

/// Spec R12 (a): the retained coarse/fine outlet-window approximation.
///
/// Every interior seam of the three predetermined MICRO contexts is surveyed.
/// A crossing matches when the
/// upstream local water grid has an outlet inside its absolute-aligned
/// ten-cell coarse window. For non-matches, distance is measured to the
/// nearest actual upstream outlet on that same boundary.
///
/// With candidate 06's regional detail correction, the same fixed panel has
/// 119 crossings, 15 non-matches, and 12/13 large matches (92%), with median
/// non-match distance 4 cells. Seed 42's first accepted attempt matches
/// 3/4 large crossings; passing the aggregate gate does not imply exact
/// local continuity. Candidate 05 with actual outside neighbors had 14
/// non-matches, 11/13 large matches (84%), a 2-cell median and 2/4 large
/// matches in first-accepted seed 42. Before the neighbor fix, its two-context sample had
/// 91 crossings, 33 non-matches, 6/9 large matches (66%) and a 50-cell median.
/// The added first-accepted seed-42 context was selected before scoring it;
/// no replacement sample was searched. Candidate 04's 107 crossings,
/// 43 non-matches, 15/21 large matches (71%) and 20-cell median remain recorded
/// in `cross-tile-c04-fixtures.md`. Older measurements and interpretation are
/// in `.superpowers/sdd/task-8-report.md`; current derivation is in
/// `lake-district-correction/cross-tile-c06-fixtures.md`. The public shared fine solver does not use
/// these independently generated local outlet guesses.
#[test]
fn seam_crossings_align_with_upstream_outlets() {
    const MIN_CATCHMENT_KM2: u32 = 30;
    const MIN_MATCH_PCT_ABOVE_MIN_CATCHMENT: u32 = 70; // historical 92% (12/13); gate retained
    const MAX_MEDIAN_NONMATCH_DIST: i32 = 20; // historical 4 cells; gate retained

    let mut total = 0u32;
    let mut big_total = 0u32;
    let mut big_matched = 0u32;
    let mut nonmatch_dists: Vec<i32> = Vec::new();

    for (seed, attempt, continent, sample) in [
        (42, 2, ctx(), tiles()),
        (99, 0, survey_ctx(), survey_tiles()),
        (42, 0, first_accepted_ctx(), first_accepted_tiles()),
    ] {
        let before = (total, big_total, big_matched, nonmatch_dists.len());
        assert!(
            (250..=900).contains(&continent.grid.land_fraction_permille()),
            "seed {seed} attempt {attempt} must retain an accepted land fraction"
        );
        for t in sample {
            let expected = independent_entering(seed, continent, t.coord);
            assert_eq!(
                t.bundle.entering.len(),
                expected.len(),
                "seed {seed} tile {:?}: entering count disagrees with the independent oracle",
                t.coord
            );
            for e in &t.bundle.entering {
                assert_eq!(
                expected.get(&e.cell),
                Some(&(u64::from(e.catchment_km2), e.discharge.raw(), e.order)),
                "seed {seed} tile {:?} entry {:?}: independent catchment, discharge or order differs",
                t.coord,
                e.cell
            );
                // Which edge this crossing sits on — always exactly one, by
                // construction of `entering_rivers`' fixed_x/fixed_y windows.
                // Horizontal wins a literal corner cell, matching that same
                // tie rule (§Q3).
                let edge = if e.cell.y() == 0 {
                    'N'
                } else if e.cell.y() == AREA_CELLS - 1 {
                    'S'
                } else if e.cell.x() == 0 {
                    'W'
                } else {
                    'E'
                };
                let Some(up_coord) = neighbour_area(t.coord, edge) else {
                    continue; // the continent rim is forced ocean, so a real
                              // fixture never hits this; skip rather than
                              // panic if that ever changes
                };
                let up = tile_in(sample, up_coord);

                let (ax, ay) = abs_cell(t.coord, e.cell.x(), e.cell.y());
                let (win_abs_lo, up_origin, x_axis) = match edge {
                    'N' | 'S' => (ax.div_euclid(10) * 10, up_coord.x * N, true),
                    _ => (ay.div_euclid(10) * 10, up_coord.y * N, false),
                };
                let win_lo = (win_abs_lo - up_origin).clamp(0, N - 1);
                let win_hi = (win_abs_lo + 10 - up_origin).clamp(0, N);
                let fixed = match edge {
                    'N' => N - 1, // upstream sits north: check its south row
                    'S' => 0,     // upstream sits south: check its north row
                    'W' => N - 1, // upstream sits west: check its east column
                    _ => 0,       // upstream sits east: check its west column
                };
                let at_of = |j: i32| if x_axis { cc(j, fixed) } else { cc(fixed, j) };

                let is_match = (win_lo..win_hi).any(|j| up.water.is_outlet(at_of(j)));
                let is_big = e.catchment_km2 >= MIN_CATCHMENT_KM2;

                total += 1;
                big_total += u32::from(is_big);
                if is_match {
                    big_matched += u32::from(is_big);
                } else {
                    let d = (0..N)
                        .filter(|&j| up.water.is_outlet(at_of(j)))
                        .map(|j| distance_to_window(j, win_lo, win_hi))
                        .min()
                        .unwrap_or_else(|| {
                            panic!(
                                "tile {:?} seed {:?}: upstream tile {up_coord:?} marks no outlet \
                             at all on the shared boundary line",
                                t.coord, e.cell
                            )
                        });
                    nonmatch_dists.push(d);
                }
            }
        }
        let mut sample_dists = nonmatch_dists[before.3..].to_vec();
        sample_dists.sort_unstable();
        println!(
            "legacy survey sample: seed={seed}, attempt={attempt}, land_permille={}, total={}, large={}, large_matched={}, large_match_percent={:?}, nonmatches={}, median_nonmatch_distance={:?}",
            continent.grid.land_fraction_permille(), total - before.0, big_total - before.1,
            big_matched - before.2, ((big_matched - before.2) * 100).checked_div(big_total - before.1),
            sample_dists.len(), sample_dists.get(sample_dists.len() / 2)
        );
    }

    nonmatch_dists.sort_unstable();
    println!(
        "legacy combined survey before gates: total={total}, large={big_total}, large_matched={big_matched}, large_match_percent={:?}, nonmatches={}, median_nonmatch_distance={:?}",
        (big_matched * 100).checked_div(big_total), nonmatch_dists.len(), nonmatch_dists.get(nonmatch_dists.len() / 2)
    );

    assert!(
        total >= 100,
        "only {total} entering crossings surveyed across MICRO seed-42 attempt-2 and both \
         first-accepted seed-42/99 attempt-0 contexts; re-survey before trusting the bucketed rate below"
    );
    assert!(
        big_total >= 10,
        "only {big_total} crossings have catchment >= {MIN_CATCHMENT_KM2} km2; too few to judge \
         a match rate (measured baseline 24)"
    );
    let big_pct = big_matched * 100 / big_total;
    assert!(
        big_pct >= MIN_MATCH_PCT_ABOVE_MIN_CATCHMENT,
        "only {big_pct}% of the {big_total} crossings with catchment >= {MIN_CATCHMENT_KM2} km2 \
         land an outlet in the upstream window (measured baseline 87%, 21/24)"
    );

    if let Some(&median) = nonmatch_dists.get(nonmatch_dists.len() / 2) {
        println!("legacy all-seam survey: total={total}, large={big_total}, large_matched={big_matched}, large_match_percent={big_pct}, nonmatches={}, median_nonmatch_distance={median}", nonmatch_dists.len());
        assert!(
            median <= MAX_MEDIAN_NONMATCH_DIST,
            "median non-match distance is {median} cells, past the {MAX_MEDIAN_NONMATCH_DIST}-cell \
             approximation bar (measured baseline 8)"
        );
    } else {
        // Every sampled crossing matched an upstream outlet in its window.
        // No empirical non-match median exists; the separate negative control
        // above exercises distance measurement when outlets miss a window.
        assert_eq!(big_matched, big_total);
    }
}
