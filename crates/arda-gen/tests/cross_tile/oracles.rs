//! Independent oracles: entering inflow, the continent filled and basin
//! surfaces and the shared lake surface, reimplemented without calling the
//! code under test.

use super::*;

/// Independent, test-local re-derivation of `bundle_for`'s entering-river
/// crossing/window/merge rule (feature 03 §Q3), reading only
/// `ctx.hydrology` and `boundary_height` — never `bundles::entering_rivers`
/// itself, which is private. Mirrors the crossing/window/tie-break logic
/// `continent::bundles::tests::merged_seeds_take_the_max_order_not_the_summed_order`
/// already re-derives independently for one tile, generalised to every
/// edge and extended to also sum discharge, so every `EnteringRiver`
/// field can be checked here, not just catchment.
pub(super) fn independent_entering(
    seed: u64,
    ctx: &Continent,
    area: AreaCoord,
) -> BTreeMap<CellCoord, (u64, u64, u8)> {
    let (w, h) = (ctx.grid.width(), ctx.grid.height());
    let (x0, y0) = (
        i64::from(area.x) * i64::from(N),
        i64::from(area.y) * i64::from(N),
    );
    let (x1, y1) = (x0 + i64::from(N), y0 + i64::from(N));
    let mut groups: BTreeMap<CellCoord, Vec<(u32, u64)>> = BTreeMap::new();

    for ky in 0..h {
        for kx in 0..w {
            let i = usize::try_from(ky * w + kx).unwrap();
            let c_km2 = ctx.hydrology.catchment_km2[i];
            if c_km2 < 3 {
                continue; // below the artifact's channel scale (§Q3)
            }
            let Some(d) = ctx.hydrology.downstream[i] else {
                continue;
            };
            let di = i64::from(d);
            let (dkx, dky) = (di % i64::from(w), di / i64::from(w));
            let (ocx, ocy) = (i64::from(kx) * 10 + 5, i64::from(ky) * 10 + 5);
            let (dcx, dcy) = (dkx * 10 + 5, dky * 10 + 5);
            if !(dcx > x0 && dcx < x1 && dcy > y0 && dcy < y1) {
                continue; // downstream centre not strictly inside the tile
            }

            let (win0, base, fixed_x, fixed_y) = if ocy < y0 && dcy > y0 {
                (dkx * 10, x0, None, Some(0i64)) // north line
            } else if ocy > y1 && dcy < y1 {
                (dkx * 10, x0, None, Some(i64::from(N) - 1)) // south line
            } else if ocx < x0 && dcx > x0 {
                (dky * 10, y0, Some(0i64), None) // west line
            } else if ocx > x1 && dcx < x1 {
                (dky * 10, y0, Some(i64::from(N) - 1), None) // east line
            } else {
                continue; // interior edge
            };

            let lo = (win0 - base).clamp(0, i64::from(N) - 1);
            let hi = (win0 + 10 - base).clamp(0, i64::from(N));
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
            let Some(cell) = CellCoord::new(
                u16::try_from(lx).unwrap_or(0),
                u16::try_from(ly).unwrap_or(0),
            ) else {
                continue;
            };
            groups
                .entry(cell)
                .or_default()
                .push((c_km2, ctx.hydrology.discharge_l_s[i]));
        }
    }

    groups
        .into_iter()
        .map(|(cell, parts)| {
            let catchment: u64 = parts.iter().map(|&(c, _)| u64::from(c)).sum();
            let discharge: u64 = parts.iter().map(|&(_, d)| d).sum();
            let order = parts
                .iter()
                .map(|&(c, _)| entering_order(c))
                .max()
                .unwrap_or(1);
            (cell, (catchment, discharge, order))
        })
        .collect()
}

/// Independent smoothstep-bilinear sample of a bundle's `filled_km` patch
/// at one local cell offset — `local_x`/`local_y` may equal [`AREA_CELLS`]
/// itself, naming the neighbour's first row/column, the same convention
/// [`abs_cell`] uses. Same interpolation as `arda_gen::area`'s private
/// `sample_km_patch`, reimplemented here rather than calling into it — both
/// it and `clamp_near_rim` are private to `arda-gen::area` — so a check
/// built on this cannot pass merely by calling back into the code under
/// test.
pub(super) fn independent_filled_km_sample(b: &TileBundle, local_x: i32, local_y: i32) -> i64 {
    let km0x = (b.area.x * 512).div_euclid(10);
    let km0y = (b.area.y * 512).div_euclid(10);
    let side = i32::try_from(PATCH_KM).unwrap_or(0);

    let smooth = |v: i32| -> i64 {
        let t = i64::from(v) * 65536 / 10;
        let t2 = (t * t) >> 16;
        let t3 = (t2 * t) >> 16;
        (3 * t2 - 2 * t3).clamp(0, 65536)
    };

    let (ax, ay) = abs_cell(
        b.area,
        u16::try_from(local_x).unwrap_or(0),
        u16::try_from(local_y).unwrap_or(0),
    );
    let (kx, ky) = (ax.div_euclid(10) - km0x, ay.div_euclid(10) - km0y);
    let (fx, fy) = (smooth(ax.rem_euclid(10)), smooth(ay.rem_euclid(10)));
    let at = |dx: i32, dy: i32| -> i64 {
        let px = usize::try_from((kx + dx).clamp(0, side - 1)).unwrap_or(0);
        let py = usize::try_from((ky + dy).clamp(0, side - 1)).unwrap_or(0);
        i64::from(b.filled_km[py * PATCH_KM + px])
    };
    let top = at(0, 0) + (((at(1, 0) - at(0, 0)) * fx) >> 16);
    let bottom = at(0, 1) + (((at(1, 1) - at(0, 1)) * fx) >> 16);
    top + (((bottom - top) * fy) >> 16)
}

/// Feature 03 §Q5 test oracle, ported from
/// `area::tests::continent_surface_at`: the max
/// [`independent_filled_km_sample`] over the near-rim cells among `cells`.
/// `None` when `cells` has no near-rim cell.
pub(super) fn independent_filled_km_surface(b: &TileBundle, cells: &[CellCoord]) -> Option<i32> {
    let best = cells
        .iter()
        .filter(|c| c.x() <= 1 || c.y() <= 1 || c.x() >= AREA_CELLS - 2 || c.y() >= AREA_CELLS - 2)
        .map(|c| independent_filled_km_sample(b, i32::from(c.x()), i32::from(c.y())))
        .max()?;
    Some(i32::try_from(best).unwrap_or(i32::MAX))
}

/// Nearest-cell lookup of `bundle.basin_km` at one tile-local cell —
/// same coordinate math as [`independent_filled_km_sample`], minus the
/// bilinear blend: `basin_km` is piecewise-constant per continent
/// depression (feature 02 §Q1 / open-items #12), so it is looked up by
/// nearest cell, never interpolated. Independent of `arda_gen::area`'s
/// private `nearest_km_patch`, which this mirrors rather than calls.
pub(super) fn independent_basin_km_sample(b: &TileBundle, local_x: i32, local_y: i32) -> i32 {
    let km0x = (b.area.x * 512).div_euclid(10);
    let km0y = (b.area.y * 512).div_euclid(10);
    let side = i32::try_from(PATCH_KM).unwrap_or(0);
    let (ax, ay) = abs_cell(
        b.area,
        u16::try_from(local_x).unwrap_or(0),
        u16::try_from(local_y).unwrap_or(0),
    );
    let (kx, ky) = (ax.div_euclid(10) - km0x, ay.div_euclid(10) - km0y);
    let px = usize::try_from(kx.clamp(0, side - 1)).unwrap_or(0);
    let py = usize::try_from(ky.clamp(0, side - 1)).unwrap_or(0);
    b.basin_km[py * PATCH_KM + px]
}

/// Feature 02 §Q1 / feature 03 §Q5 test oracle: like
/// [`independent_filled_km_surface`], but checked against continent-tier
/// lake identity FIRST — the max nearest-cell `basin_km` over the
/// near-rim cells among `cells`, `NO_BASIN` excluded — falling back to
/// `independent_filled_km_surface` only when no near-rim cell sees a
/// continent depression. Mirrors `clamp_near_rim`'s own preference order,
/// reimplemented independently (never calling `nearest_km_patch` /
/// `sample_km_patch` / `clamp_near_rim`, all private to `arda_gen::area`
/// regardless). `None` when `cells` has no near-rim cell.
pub(super) fn independent_lake_surface_at(b: &TileBundle, cells: &[CellCoord]) -> Option<i32> {
    let rim: Vec<&CellCoord> = cells
        .iter()
        .filter(|c| c.x() <= 1 || c.y() <= 1 || c.x() >= AREA_CELLS - 2 || c.y() >= AREA_CELLS - 2)
        .collect();
    if rim.is_empty() {
        return None;
    }
    let basin_best = rim
        .iter()
        .map(|c| independent_basin_km_sample(b, i32::from(c.x()), i32::from(c.y())))
        .filter(|&v| v != arda_gen::continent::hydrology::NO_BASIN)
        .max();
    if basin_best.is_some() {
        return basin_best;
    }
    independent_filled_km_surface(b, cells)
}
