//! Recorded lakes: basins clamped to the shared continent surface at the
//! tile rim, trimmed, filtered by the lake floors, then given outlets
//! against the final surviving lake set (logic/02 Water; feature 03 §Q5).

use super::*;
use crate::continent::bundles::TileBundle;
use crate::continent::hydrology::NO_BASIN;
use arda_core::{CellCoord, HeightMm, AREA_CELLS};
use fill::{Basin, Filled};
use local_objects::LocalLake as Lake;

/// A basin cell within one cell of the tile rim (feature 03 §Q5).
pub(super) fn near_rim(c: CellCoord) -> bool {
    c.x() <= 1 || c.y() <= 1 || c.x() >= AREA_CELLS - 2 || c.y() >= AREA_CELLS - 2
}

/// Basins large and deep enough to record as lakes.
///
/// Feature 03 §Q5 (closes open-items #12): `fill::fill` only sees this
/// tile's own relief, so a basin straddling the rim can reach a
/// different local spill on each side of the seam. A basin with a
/// near-rim cell instead takes its surface from `bundle.filled_km`, the
/// continent's own routing surface, which both neighbouring tiles sample
/// identically — so both sides agree by construction, the same way
/// `area_rainfall` already does for rainfall (§Q4).
///
/// The size/depth thresholds (`LAKE_MIN_CELLS`, `LAKE_MIN_DEPTH_MM`) are
/// applied AFTER the clamp and trim (see [`clamp_and_trim`]), not before:
/// a basin that only clears `LAKE_MIN_DEPTH_MM` once the continent
/// surface raises it should still be promoted to a lake, and a basin
/// that only cleared the threshold at its own (locally wrong) spill has
/// no business being reported as a lake the neighbouring tile disagrees
/// exists.
///
/// `clamp_and_trim` only ever removes cells from a basin, never adds any
/// — a review measured 27 of 63 near-rim basins clamped DOWN across a
/// 5-seed x 8-tile MICRO sweep (task-5-report.md), so the trim path is
/// exercised, not speculative. Because it is a pure shrink, `fill::fill`'s
/// basins — already pairwise disjoint by construction
/// (`fill::tests::basins_are_disjoint`) — stay disjoint here too: a
/// subset of a disjoint set is still disjoint. No cross-basin claiming is
/// needed to keep a cell out of two `Lake`s.
///
/// Outlets are computed in a second pass, after every basin's clamp,
/// trim, and threshold filter has run (feature 03 §Q5, round-2 review
/// fix): [`recompute_outlet`] needs to know the FINAL surviving lake set
/// tile-wide, not just this one basin's own before/after, so it cannot
/// run until every basin here has already been decided.
#[allow(clippy::cast_possible_truncation)] // At most512² disjoint fragments.
pub(super) fn collect_lakes(heights: &[i32], filled: &Filled, bundle: &TileBundle) -> Vec<Lake> {
    // Pass 1: clamp, trim, and threshold-filter every basin. Outlets wait
    // for pass 2 — computing one here could only consult this basin's own
    // surviving cells, not the tile's final lake set.
    let survivors: Vec<(i32, u32, Vec<CellCoord>)> = filled
        .basins
        .iter()
        .filter_map(|b| clamp_and_trim(b, heights, bundle))
        .filter(|(_, depth_mm, cells)| {
            cells.len() >= LAKE_MIN_CELLS && *depth_mm >= LAKE_MIN_DEPTH_MM
        })
        .collect();

    // Every cell belonging to a basin that survived pass 1 — the tile's
    // final lake membership. A basin the threshold dropped is not a lake,
    // so its cells are dry for this purpose; that is the only coherent
    // definition once the filter has run.
    let mut submerged = vec![false; (N * N) as usize];
    for (_, _, cells) in &survivors {
        for c in cells {
            submerged[c.index()] = true;
        }
    }

    // Pass 2: now that final membership is settled tile-wide, each
    // surviving lake's outlet can be judged against it.
    survivors
        .into_iter()
        .enumerate()
        .map(|(i, (surface_mm, depth_mm, cells))| {
            let outlet = recompute_outlet(&cells, &submerged, filled);
            Lake {
                id: (i + 1) as u32,
                surface: HeightMm::new(surface_mm),
                depth_mm,
                outlet,
                cells,
            }
        })
        .collect()
}

/// A basin's lake surface and depth after the seam clamp.
///
/// feature 03 §Q5: both sides of a seam sample the same continent
/// surface, so the recorded level agrees. Membership stays local —
/// flooding every connected cell under the clamped surface drowned up
/// to 88% of a tile in measurement, which the "minimal blast radius"
/// decision excludes.
///
/// Interior basins (no cell within one cell of the rim) pass through
/// unchanged: `(b.surface_mm, b.depth_mm)`. A near-rim basin's surface
/// prefers continent-tier lake identity (feature 02 §Q1, the mechanism
/// open-items #12 needs): if
/// any near-rim cell maps, by NEAREST 1 km cell, onto a continent
/// depression (`bundle.basin_km` != `NO_BASIN`), the surface is the MAX of
/// those values. That value is constant across the WHOLE continent
/// depression (`ContinentHydrology::basin_surface`), so two fragments of
/// the same depression agree exactly regardless of which cells each
/// fragment's own contact span happens to cover — the residual
/// span-dependence a bilinear sample of `filled_km` could not remove
/// (measured: 723 mm on a synthetic straddling case, exactly 0 once both
/// fragments see one shared depression).
///
/// **Two residues, both unobserved and both recorded in `open-items.md`
/// #12 rather than guarded here.** A fragment whose rim abuts two
/// *different* depressions takes the max of two constants, which is
/// span-dependent again if its sibling abuts only one. And no fixture has
/// yet produced a straddling basin whose cells see a continent depression
/// at all — 0 in a 4,280 seed/seam sweep — so the exact path is proven on
/// constructed input, not on natural data. The lookup is
/// nearest-cell, not bilinear — see [`nearest_km_patch`]'s own doc for
/// why interpolating a piecewise-constant field would be wrong here.
///
/// Only when NO near-rim cell sees a continent depression does the
/// surface fall back to the MAX, over the near-rim cells, of the
/// smoothstep-bilinear sample of `bundle.filled_km` — the same
/// interpolation `area_rainfall` uses, shared via `sample_km_patch` rather
/// than duplicated a third time; unchanged from before this fix, so
/// behaviour is identical wherever the continent tier sees no depression.
///
/// Depth is then recomputed as `surface` minus the basin's own floor (its
/// lowest cell's height), saturating (floored) at 0. Cell membership does
/// not appear in this function's signature at all: the caller keeps
/// exactly the cells `fill::fill` gave the basin.
pub(super) fn clamp_near_rim(b: &Basin, heights: &[i32], bundle: &TileBundle) -> (i32, u32) {
    if !b.cells.iter().any(|&c| near_rim(c)) {
        return (b.surface_mm, b.depth_mm);
    }

    let rim_cells: Vec<CellCoord> = b.cells.iter().copied().filter(|&c| near_rim(c)).collect();

    let basin_surface_mm = rim_cells
        .iter()
        .map(|c| nearest_km_patch(bundle, &bundle.basin_km, i32::from(c.x()), i32::from(c.y())))
        .filter(|&v| v != NO_BASIN)
        .max();

    let surface_mm = match basin_surface_mm {
        Some(v) => v,
        None => rim_cells
            .iter()
            .map(|c| {
                sample_km_patch(
                    bundle,
                    &bundle.filled_km,
                    i32::from(c.x()),
                    i32::from(c.y()),
                )
            })
            .max()
            .and_then(|v| i32::try_from(v).ok())
            .unwrap_or(b.surface_mm),
    };

    let floor = b
        .cells
        .iter()
        .map(|c| heights[c.index()])
        .min()
        .unwrap_or(surface_mm);
    let depth_mm = u32::try_from(surface_mm.saturating_sub(floor)).unwrap_or(0);
    (surface_mm, depth_mm)
}

/// A basin's cells, surface, and depth once cells the shared surface
/// leaves dry are trimmed out (feature 03 §Q5).
///
/// `clamp_near_rim` alone can move a near-rim basin's surface DOWN — even
/// below the basin's own floor — so a cell `fill::fill` recorded as
/// submerged at the basin's local spill can end up sitting at or above
/// the shared surface (measured: 27 of 63 near-rim basins clamped DOWN
/// across a 5-seed x 8-tile MICRO sweep; see task-5-report.md). Depth is
/// then recomputed against the *surviving* cells' own floor, not the
/// original basin's — `clamp_near_rim`'s own depth is discarded here.
/// Returns `None` when every cell is trimmed: a basin with no submerged
/// cell left is not a lake.
///
/// Round-2 review fix: outlet computation does not happen here any more.
/// It used to (round-1's fix recomputed it per-basin, in step with the
/// surface and membership this function already tracks), but a per-basin
/// outlet can only ever consult `fill::fill`'s pre-clamp snapshot for
/// "is this candidate still submerged in some OTHER basin" — and that
/// snapshot goes stale the moment a sibling basin is itself clamped and
/// trimmed (see [`recompute_outlet`]'s doc for the failure this allowed).
/// The outlet now waits for a second pass over every basin's surviving
/// cells, run once by [`collect_lakes`] after this function has decided
/// every basin's fate.
pub(super) fn clamp_and_trim(
    b: &Basin,
    heights: &[i32],
    bundle: &TileBundle,
) -> Option<(i32, u32, Vec<CellCoord>)> {
    let (surface_mm, _) = clamp_near_rim(b, heights, bundle);
    // feature 03 §Q5: the shared surface governs; a cell above it is not
    // submerged, so it leaves the lake. Trim only — never flood (see the
    // ballooning measured at plan time).
    let cells: Vec<CellCoord> = b
        .cells
        .iter()
        .copied()
        .filter(|c| heights[c.index()] < surface_mm)
        .collect();
    let floor = cells.iter().map(|c| heights[c.index()]).min()?;
    let depth_mm = u32::try_from(surface_mm.saturating_sub(floor)).unwrap_or(0);
    Some((surface_mm, depth_mm, cells))
}

/// A lake's outlet: the lowest cell 8-adjacent to `cells` that belongs to
/// no SURVIVING lake — this one or any other (feature 03 §Q5, round-2
/// review fix).
///
/// Mirrors the deleted `fill::spill_cell`'s rule — lowest neighbour keyed
/// by the routing surface `filled.get`, ties to the smaller [`CellCoord`].
/// That is NOT "unchanged for a basin that was never clamped", even
/// though it sounds like it should be: `submerged` marks only the FINAL,
/// POST-THRESHOLD lake set (`LAKE_MIN_CELLS`/`LAKE_MIN_DEPTH_MM` already
/// applied, see [`collect_lakes`]), while `spill_cell` read `fill::fill`'s
/// raw per-basin submersion directly, which knows nothing of that filter
/// and treats a cell as wet the moment ANY basin — however small or
/// shallow — claims it. A neighbour belonging to a genuine basin that
/// never cleared the threshold is therefore now a valid outlet candidate,
/// for every basin and not just a clamped one, where `spill_cell` would
/// have turned it away as still submerged. That is a correctness
/// improvement, not a regression to paper over: a basin that misses the
/// threshold is not in the surviving set, so `compose` persists every one
/// of its cells as `TerrainKind::Land` (or `Sea`), never `Lake` — treating
/// it as fair game for an outlet is exactly consistent with what the
/// world actually stores.
///
/// `submerged` itself is also new relative to `spill_cell`: a tile-wide
/// grid [`collect_lakes`] builds from that same FINAL lake set, rather
/// than consulting `fill::fill`'s pre-clamp snapshot the way round-1's fix
/// did — the round-2 fix the rest of this doc comment covers below.
///
/// Round-1's fix reimplemented `spill_cell` per basin, over that basin's
/// own surviving cells, but still consulted `fill::fill`'s pre-clamp
/// `Filled` to decide whether a candidate belonged to some OTHER basin.
/// That snapshot is built once, before any basin is clamped, and is never
/// updated as siblings are trimmed. Basin *membership* comes from a
/// 4-connected flood (`fill::find_basins`), but this search — like
/// `fill::fill`'s own flood and `spill_cell` — is 8-connected
/// (`fill::NEIGHBOURS`), so two basins can be diagonally adjacent without
/// ever merging into one. A cell released by ONE basin's trim can sit
/// diagonally against a surviving cell of a DIFFERENT, also-near-rim
/// basin; the stale snapshot still shows it raised at that other basin's
/// pre-clamp spill, so round-1's per-basin check wrongly turned it away —
/// yielding an outlet that was too high, or `None` when a valid, lower
/// outlet existed. Judging every candidate against one final `submerged`
/// grid, built only after every basin's fate is settled, removes the
/// staleness: a cell no longer needs a special "was this MY basin's own
/// raised ground" exception, because a basin's own trimmed-away cells are
/// — correctly, uniformly — not `submerged` either.
///
/// Returns `None` when every 8-neighbour of `cells` either belongs to a
/// surviving lake or falls off the tile.
pub(super) fn recompute_outlet(
    cells: &[CellCoord],
    submerged: &[bool],
    filled: &Filled,
) -> Option<CellCoord> {
    let mut best: Option<(i32, CellCoord)> = None;
    for c in cells {
        let (x, y) = (i32::from(c.x()), i32::from(c.y()));
        for (dx, dy) in fill::NEIGHBOURS {
            let Some(nb) = coord(x + dx, y + dy) else {
                continue;
            };
            if submerged[nb.index()] {
                continue; // belongs to a surviving lake -- this one or another
            }
            let h = filled.get(nb);
            if best.is_none_or(|(bh, bc)| h < bh || (h == bh && nb < bc)) {
                best = Some((h, nb));
            }
        }
    }
    best.map(|(_, c)| c)
}

#[cfg(test)]
mod tests;
