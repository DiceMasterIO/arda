use super::*;
use arda_core::GenerateConfig;

fn fixture() -> Continent {
    crate::continent::build_continent(42, GenerateConfig::MICRO, 0)
}

/// `fixture()`, memoized: several new tests below build bundles over
/// the same MICRO continent, and continent generation (plates,
/// tectonics, climate, hydrology) is too expensive to redo per test.
fn fixture_ctx() -> Continent {
    static CTX: std::sync::OnceLock<Continent> = std::sync::OnceLock::new();
    CTX.get_or_init(|| crate::continent::build_continent(42, GenerateConfig::MICRO, 0))
        .clone()
}

#[test]
fn abs_cell_maps_tiles_without_overlap() {
    assert_eq!(abs_cell(AreaCoord::new(0, 0), 0, 0), (0, 0));
    assert_eq!(abs_cell(AreaCoord::new(0, 0), 511, 511), (511, 511));
    assert_eq!(abs_cell(AreaCoord::new(1, 0), 0, 0), (512, 0));
    assert_eq!(abs_cell(AreaCoord::new(1, 3), 0, 0), (512, 1536));
}

#[test]
fn boundary_height_is_deterministic() {
    let c = fixture();
    assert_eq!(
        boundary_height(42, &c.grid, 700, 1200),
        boundary_height(42, &c.grid, 700, 1200)
    );
}

/// The invariant this whole task exists for: tile (0, y)'s east edge and
/// tile (1, y)'s west edge name the same absolute cells, so they must be
/// the same numbers — `implementation.md` "Pinned edges".
#[test]
fn neighbouring_tiles_agree_on_their_shared_vertical_edge() {
    let c = fixture();
    let left = bundle_for(42, &c, AreaCoord::new(0, 1));
    let right = bundle_for(42, &c, AreaCoord::new(1, 1));
    assert_eq!(left.east, right.west);
    assert_eq!(left.east.len(), AREA_CELLS as usize);
}

#[test]
fn neighbouring_tiles_agree_on_their_shared_horizontal_edge() {
    let c = fixture();
    let top = bundle_for(42, &c, AreaCoord::new(1, 1));
    let bottom = bundle_for(42, &c, AreaCoord::new(1, 2));
    assert_eq!(top.south, bottom.north);
}

#[test]
fn outside_samples_add_the_missing_ring_without_changing_existing_edges_or_mean() {
    let c = fixture_ctx();
    let area = AreaCoord::new(1, 2);
    let b = bundle_for(42, &c, area);
    let (x0, y0) = abs_cell(area, 0, 0);
    let n = i32::from(AREA_CELLS);
    assert_eq!(b.north_outside.len(), usize::from(AREA_CELLS));
    assert_eq!(b.west_outside.len(), usize::from(AREA_CELLS));
    let mut old_edge_sum = 0_i64;
    for j in 0..n {
        let i = usize::try_from(j).unwrap();
        let old_samples = [(j, 0), (j, n), (n, j), (0, j)]
            .map(|(x, y)| boundary_height(42, &c.grid, x0 + x, y0 + y));
        assert_eq!([b.north[i], b.south[i], b.east[i], b.west[i]], old_samples);
        old_edge_sum += old_samples.into_iter().map(i64::from).sum::<i64>();
        assert_eq!(
            b.north_outside[i],
            boundary_height(42, &c.grid, x0 + j, y0 - 1)
        );
        assert_eq!(
            b.west_outside[i],
            boundary_height(42, &c.grid, x0 - 1, y0 + j)
        );
    }
    assert_eq!(
        i64::from(b.mean_height_mm),
        old_edge_sum / (4 * i64::from(n))
    );
    assert_ne!(b.north_outside, b.north);
    assert_ne!(b.west_outside, b.west);
    let expected = [(-1, -1), (n, -1), (-1, n), (n, n)]
        .map(|(x, y)| boundary_height(42, &c.grid, x0 + x, y0 + y));
    assert_eq!(b.outside_corners, expected);
    println!("outside allocation: samples={}, vector_payload_bytes={}, corner_bytes={}, vector_headers_bytes={}, bundle_size={}",
        b.north_outside.len() + b.west_outside.len() + b.outside_corners.len(),
        (b.north_outside.len() + b.west_outside.len()) * std::mem::size_of::<i32>(),
        std::mem::size_of_val(&b.outside_corners), 2 * std::mem::size_of::<Vec<i32>>(), std::mem::size_of::<TileBundle>());
}

#[test]
fn legacy_outside_samples_cannot_change_shared_prepared_terrain() {
    use crate::area::prepare::{prepare_area_terrain, SharedTerrain};
    use crate::hydrology::prepared_domain::PreparedDomain;
    use arda_core::SizeKm;

    // A bounded real shared preparation, without a world solve or export.
    // Its evolved terrain has no TileBundle input; every field that its
    // immutable area slice does consume must be independent of this halo.
    let c = fixture_ctx();
    let config = GenerateConfig::new(
        SizeKm::new(64, 64),
        GenerateConfig::MICRO.latitude_band(),
        15,
    )
    .unwrap();
    let domain = PreparedDomain::for_config(config).unwrap();
    let terrain = SharedTerrain::build(42, &c, domain).unwrap();
    let entry = domain.entry(0).unwrap();
    let b = bundle_for(42, &c, entry.area);
    let before = prepare_area_terrain(&terrain, &c, &b, entry.valid).unwrap();
    let mut changed = b.clone();
    changed.north_outside.fill(i32::MIN);
    changed.west_outside.fill(i32::MAX);
    changed.outside_corners = [i32::MIN, i32::MAX, 0, 1];
    let after = prepare_area_terrain(&terrain, &c, &changed, entry.valid).unwrap();
    assert_eq!(before, after);
    println!("shared preparation unchanged by outside samples: domain={}x{}, area={:?}, exact_cells={}, compared=height/rain/temperature/extent/identity", domain.width(), domain.height(), entry.area, before.heights.len());
}

#[test]
fn bundles_do_not_depend_on_the_order_they_are_built() {
    let c = fixture();
    let forward: Vec<_> = GenerateConfig::MICRO
        .area_coords()
        .map(|a| bundle_for(42, &c, a))
        .collect();
    let mut coords: Vec<_> = GenerateConfig::MICRO.area_coords().collect();
    coords.reverse();
    let mut backward: Vec<_> = coords.iter().map(|&a| bundle_for(42, &c, a)).collect();
    backward.reverse();
    assert_eq!(forward, backward);
}

#[test]
fn every_micro_tile_gets_a_bundle() {
    let c = fixture();
    let bundles: Vec<_> = GenerateConfig::MICRO
        .area_coords()
        .map(|a| bundle_for(42, &c, a))
        .collect();
    assert_eq!(bundles.len(), 8);
}

#[test]
fn entering_order_is_the_floor_log_map() {
    // Feature 03 §Q3: g(c) = 1 + ilog2(c/3)/2, ratio 4, base 3 km².
    for (c, o) in [
        (3, 1),
        (11, 1),
        (12, 2),
        (48, 3),
        (192, 4),
        (768, 5),
        (3_072, 6),
    ] {
        assert_eq!(entering_order(c), o, "catchment {c}");
    }
    assert_eq!(entering_order(u32::MAX), 12, "clamped at 12");
}

#[test]
fn patches_cover_the_tile_plus_one() {
    let ctx = fixture_ctx();
    let b = bundle_for(42, &ctx, AreaCoord::new(0, 1));
    assert_eq!(b.rainfall_km.len(), 53 * 53);
    assert_eq!(b.regime_km.len(), 53 * 53);
    assert_eq!(b.filled_km.len(), 53 * 53);
    assert_eq!(b.basin_km.len(), 53 * 53);
    assert_eq!(b.west_moisture.len(), 53);
    // The patch matches a direct climate lookup at a spot inside.
    let (kx0, ky0) = (0i32, 51i32); // tile (0,1) starts at abs cell 512 → km 51
    let i_patch = 2 * 53 + 3;
    let i_grid = usize::try_from((ky0 + 2) * ctx.grid.width() + (kx0 + 3)).unwrap();
    assert_eq!(b.rainfall_km[i_patch], ctx.climate.rainfall[i_grid]);
    assert_eq!(b.filled_km[i_patch], ctx.hydrology.filled[i_grid]);
    assert_eq!(b.basin_km[i_patch], ctx.hydrology.basin_surface[i_grid]);
}

#[test]
fn tile_entries_match_an_independent_crossing_sum() {
    // Feature 03 spec R4 / §Q9(a): the crossing set is a pure function
    // of shared continent data. Re-derive tile (1,1)'s west-line
    // crossings directly from hydrology, independently of the bundle
    // code, and compare total catchment and discharge (sums are robust
    // to same-seed merging).
    let ctx = fixture_ctx();
    let b = AreaCoord::new(1, 1);
    let bb = bundle_for(42, &ctx, b);
    let west: Vec<_> = bb.entering.iter().filter(|e| e.cell.x() == 0).collect();

    let (w, h) = (ctx.grid.width(), ctx.grid.height());
    let (x0, x1) = (i64::from(b.x) * 512, i64::from(b.x + 1) * 512);
    let (y0, y1) = (i64::from(b.y) * 512, i64::from(b.y + 1) * 512);
    let (mut catchment, mut discharge) = (0u64, 0u64);
    for ky in 0..h {
        for kx in 0..w {
            let i = usize::try_from(ky * w + kx).unwrap();
            if ctx.hydrology.catchment_km2[i] < 3 {
                continue;
            }
            let Some(d) = ctx.hydrology.downstream[i] else {
                continue;
            };
            let di = i64::from(d);
            let (dkx, dky) = (di % i64::from(w), di / i64::from(w));
            let (ocx, ocy) = (i64::from(kx) * 10 + 5, i64::from(ky) * 10 + 5);
            let (dcx, dcy) = (dkx * 10 + 5, dky * 10 + 5);
            let d_inside = dcx > x0 && dcx < x1 && dcy > y0 && dcy < y1;
            // West-line crossing NOT stolen by the horizontal tie rule.
            let crosses_ns = (ocy < y0 && dcy > y0) || (ocy > y1 && dcy < y1);
            if d_inside && ocx < x0 && dcx > x0 && !crosses_ns {
                // Apply the same all-sea-window drop rule the bundle
                // uses, via the same public height source.
                let lo = (dky * 10 - y0).clamp(0, 511);
                let hi = (dky * 10 + 10 - y0).clamp(0, 512);
                let any_land = (lo..hi).any(|j| {
                    let (ax, ay) = (i32::try_from(x0).unwrap(), i32::try_from(y0 + j).unwrap());
                    boundary_height(42, &ctx.grid, ax, ay) > 0
                });
                if any_land {
                    catchment += u64::from(ctx.hydrology.catchment_km2[i]);
                    discharge += ctx.hydrology.discharge_l_s[i];
                }
            }
        }
    }
    // Bundle-side sums over west entries whose window was not all-sea.
    let got_c: u64 = west.iter().map(|e| u64::from(e.catchment_km2)).sum();
    let got_d: u64 = west.iter().map(|e| e.discharge.raw()).sum();
    if west.is_empty() {
        // Legal only when the seam genuinely has no qualifying land
        // crossing; the independent sum must then be 0 too, or every
        // window was sea (assert the weaker direction loudly).
        assert_eq!(catchment, got_c, "bundle dropped land crossings");
    } else {
        assert_eq!((got_c, got_d), (catchment, discharge));
    }
}

#[test]
fn crossings_below_three_km2_are_dropped() {
    let ctx = fixture_ctx();
    for area in [AreaCoord::new(0, 1), AreaCoord::new(1, 2)] {
        for e in bundle_for(42, &ctx, area).entering {
            assert!(e.catchment_km2 >= 3);
            assert!(e.discharge.raw() > 0);
            // Not `e.order == entering_order(e.catchment_km2)`: that
            // identity only holds for unmerged seeds. Merged seeds
            // (see `merged_seeds_take_the_max_order_not_the_summed_order`)
            // legitimately take the max per-edge order, which is not
            // `entering_order` of the summed catchment.
            assert!(e.order >= 1 && e.order <= 12);
        }
    }
}

fn independent_seed_parts(
    seed: u64,
    ctx: &Continent,
    area: AreaCoord,
) -> std::collections::BTreeMap<(u16, u16), Vec<(u32, u64)>> {
    let (w, h) = (ctx.grid.width(), ctx.grid.height());
    let n = i64::from(AREA_CELLS);
    let (x0, y0) = (i64::from(area.x) * n, i64::from(area.y) * n);
    let (x1, y1) = (x0 + n, y0 + n);
    let mut groups: std::collections::BTreeMap<(u16, u16), Vec<(u32, u64)>> =
        std::collections::BTreeMap::new();
    for ky in 0..h {
        for kx in 0..w {
            let i = usize::try_from(ky * w + kx).unwrap();
            let c_km2 = ctx.hydrology.catchment_km2[i];
            if c_km2 < 3 {
                continue;
            }
            let Some(d) = ctx.hydrology.downstream[i] else {
                continue;
            };
            let di = i64::from(d);
            let (dkx, dky) = (di % i64::from(w), di / i64::from(w));
            let (ocx, ocy) = (i64::from(kx) * 10 + 5, i64::from(ky) * 10 + 5);
            let (dcx, dcy) = (dkx * 10 + 5, dky * 10 + 5);
            if !(dcx > x0 && dcx < x1 && dcy > y0 && dcy < y1) {
                continue; // downstream center not strictly inside
            }
            let (win0, base, fixed_x, fixed_y) = if ocy < y0 && dcy > y0 {
                (dkx * 10, x0, None, Some(0i64))
            } else if ocy > y1 && dcy < y1 {
                (dkx * 10, x0, None, Some(n - 1))
            } else if ocx < x0 && dcx > x0 {
                (dky * 10, y0, Some(0i64), None)
            } else if ocx > x1 && dcx < x1 {
                (dky * 10, y0, Some(n - 1), None)
            } else {
                continue; // interior edge
            };
            let lo = (win0 - base).clamp(0, n - 1);
            let hi = (win0 + 10 - base).clamp(0, n);
            let mut best: Option<(i32, i64)> = None;
            for j in lo..hi {
                let (lx, ly) = (fixed_x.unwrap_or(j), fixed_y.unwrap_or(j));
                let (ax, ay) = abs_cell(
                    area,
                    u16::try_from(lx).unwrap_or(0),
                    u16::try_from(ly).unwrap_or(0),
                );
                let hgt = boundary_height(seed, &ctx.grid, ax, ay);
                if hgt <= 0 {
                    continue; // sea cell cannot seed
                }
                if best.is_none_or(|(bh, _)| hgt < bh) {
                    best = Some((hgt, j));
                }
            }
            let Some((_, j)) = best else {
                continue; // all-sea window
            };
            let (lx, ly) = (fixed_x.unwrap_or(j), fixed_y.unwrap_or(j));
            let (Ok(cx), Ok(cy)) = (u16::try_from(lx), u16::try_from(ly)) else {
                continue;
            };
            groups
                .entry((cx, cy))
                .or_default()
                .push((c_km2, ctx.hydrology.discharge_l_s[i]));
        }
    }
    groups
}

#[test]
fn merged_seeds_take_the_max_order_not_the_summed_order() {
    // The crossing/window/tie-break oracle is independent of
    // `entering_rivers`; the exact landing cell may move within the tile.
    let seed = 0;
    let ctx = crate::continent::build_continent(seed, GenerateConfig::MICRO, 0);
    let area = AreaCoord::new(1, 0);
    let bundle = bundle_for(seed, &ctx, area);
    let merged = independent_seed_parts(seed, &ctx, area)
        .into_iter()
        .filter(|(_, parts)| parts.len() >= 2);
    let mut merged_count = 0;
    for ((cx, cy), parts) in merged {
        merged_count += 1;
        let cell = CellCoord::new(cx, cy).unwrap();
        let want_catchment: u32 = parts.iter().map(|&(c, _)| c).sum();
        let want_discharge: u64 = parts.iter().map(|&(_, q)| q).sum();
        let want_order = parts.iter().map(|&(c, _)| entering_order(c)).max().unwrap();
        let got = bundle
            .entering
            .iter()
            .find(|e| e.cell == cell)
            .unwrap_or_else(|| panic!("bundle has no entry at merged seed {cell:?}"));
        assert_eq!(got.catchment_km2, want_catchment);
        assert_eq!(got.discharge.raw(), want_discharge);
        assert_eq!(got.order, want_order);
    }
    assert!(
        merged_count > 0,
        "no physical merged seed in MICRO seed 0 tile (1,0)"
    );
}

#[test]
fn a_merged_seed_keeps_the_max_part_order_not_the_summed_order() {
    // A distinguishing natural merge catches recomputing order from the
    // summed catchment, even when both rules happen to agree elsewhere.
    let seed = 2;
    let ctx = crate::continent::build_continent(seed, GenerateConfig::MICRO, 0);
    let area = AreaCoord::new(0, 1);
    let bundle = bundle_for(seed, &ctx, area);
    let mut distinguished = 0;
    for ((cx, cy), parts) in independent_seed_parts(seed, &ctx, area) {
        if parts.len() < 2 {
            continue;
        }
        let catchment_sum: u32 = parts.iter().map(|&(c, _)| c).sum();
        let max_part_order = parts.iter().map(|&(c, _)| entering_order(c)).max().unwrap();
        if entering_order(catchment_sum) == max_part_order {
            continue;
        }
        distinguished += 1;
        let cell = CellCoord::new(cx, cy).unwrap();
        let entries: Vec<_> = bundle.entering.iter().filter(|e| e.cell == cell).collect();
        assert_eq!(entries.len(), 1, "contributors must become one entry");
        let entry = entries[0];
        assert_eq!(entry.catchment_km2, catchment_sum);
        assert_eq!(
            entry.discharge.raw(),
            parts.iter().map(|&(_, q)| q).sum::<u64>()
        );
        assert_eq!(entry.order, max_part_order);
        assert_ne!(entry.order, entering_order(entry.catchment_km2));
    }
    assert!(
        distinguished > 0,
        "no natural merge distinguishes the two order rules"
    );
}
