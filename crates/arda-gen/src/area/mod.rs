//! Area generation (`logic/02`).
//!
//! The normative rules are the artifact's own sections; this module
//! implements Relief and Water. Climate, vegetation, settlement, land use,
//! and roads arrive at build-order step 5.

pub mod erosion;
pub(crate) mod evolution;
pub mod local_objects;
mod mfd;
mod physical_spill;
pub(crate) mod prepare;
pub mod shared_compose;
pub mod temperature;

pub mod fields;
pub mod fill;
pub mod relief;
#[cfg(test)]
mod terrain_correction_probe;
#[cfg(test)]
mod terrain_tests;
pub mod water;

use crate::continent::bundles::{abs_cell, TileBundle, PATCH_KM};
use crate::continent::hydrology::NO_BASIN;
use crate::continent::Continent;
use crate::hydrology::budget::BudgetError;
use arda_core::{
    AreaCells, Cell, CellCoord, Cover, DischargeMilli, HeightMm, RainfallMm, Terminus, TerrainKind,
    AREA_CELLS,
};
use fields::Floodplain;
use fill::{Basin, Filled};
use local_objects::{
    LocalAreaObjects as AreaObjects, LocalLake as Lake, LocalRiverSegment as RiverSegment,
};
pub use relief::{relief, ReliefGrid};
pub use water::{water, WaterGrid};

const N: i32 = AREA_CELLS as i32;

/// Smallest submerged extent that is recorded as a lake, in cells.
///
/// Priority-flood finds every closed depression the height field happens
/// to contain, and most of those are not landforms — a fluvial landscape
/// drains its own hollows. Recording all of them made the map a speckle
/// of ponds: at 100 cells (1 km²) and 2 m, seed 436342 carried 206 lakes,
/// one per 1,149 km² of land, of which 85 were 1-2 km². For scale,
/// Poland runs about one lake per 312 km², Germany one per 4,000, France
/// one per 25,000.
///
/// Raising the floors barely touches how much water the map holds,
/// because area lives in the big lakes: 300 cells with a 4 m floor keeps
/// 2,462 km² of the original 2,712 (91%) while cutting the count to 79,
/// one per 2,995 km². It is the specks that go, not the lakes.
pub const LAKE_MIN_CELLS: usize = 300;
/// Smallest maximum depth that is recorded as a lake, in millimetres.
///
/// See [`LAKE_MIN_CELLS`] for the calibration. A basin shallower than
/// this is not open water — it is wet ground, and `compose` stores it as
/// land, where the marsh rule judges it on its own terms.
pub const LAKE_MIN_DEPTH_MM: u32 = 4_000;

fn coord(x: i32, y: i32) -> Option<CellCoord> {
    CellCoord::new(u16::try_from(x).ok()?, u16::try_from(y).ok()?)
}

/// Identical smoothstep weights to
/// [`crate::continent::bundles::coarse_height`]; see there for why
/// straight bilinear is not used. Hoisted out of [`sample_km_patch`]
/// rather than kept as a local closure: a closure calling `i64::from` on
/// a plain `i32` inside a function generic over `T: i64: From<T>`
/// resolves against that bound instead of the concrete `i32` impl and
/// fails to type-check.
fn smoothstep_km(v: i32) -> i64 {
    let t = i64::from(v) * 65536 / 10;
    let t2 = (t * t) >> 16;
    let t3 = (t2 * t) >> 16;
    (3 * t2 - 2 * t3).clamp(0, 65536)
}

/// Smoothstep-bilinear sample of a `PATCH_KM`-square km-resolution patch
/// (a [`TileBundle`] field such as [`TileBundle::rainfall_km`] or
/// [`TileBundle::filled_km`]) at one tile-local cell.
///
/// Generic over the patch's element type so the `u16` rainfall grid
/// (feature 03 §Q4) and the `i32` routing surface (§Q5, the seam-lake
/// clamp) share one interpolation instead of duplicating it a third
/// time.
fn sample_km_patch<T>(bundle: &TileBundle, patch: &[T], local_x: i32, local_y: i32) -> i64
where
    T: Copy,
    i64: From<T>,
{
    let km0x = (bundle.area.x * 512).div_euclid(10);
    let km0y = (bundle.area.y * 512).div_euclid(10);
    let side = i32::try_from(PATCH_KM).unwrap_or(0);

    let (ax, ay) = abs_cell(
        bundle.area,
        u16::try_from(local_x).unwrap_or(0),
        u16::try_from(local_y).unwrap_or(0),
    );
    // Patch coordinate of the km cell containing this 100 m cell,
    // relative to the patch origin `km0` (Task 3), plus its
    // fractional offset within that km cell.
    let (kx, ky) = (ax.div_euclid(10) - km0x, ay.div_euclid(10) - km0y);
    let (fx, fy) = (
        smoothstep_km(ax.rem_euclid(10)),
        smoothstep_km(ay.rem_euclid(10)),
    );

    let at = |dx: i32, dy: i32| -> i64 {
        let px = usize::try_from((kx + dx).clamp(0, side - 1)).unwrap_or(0);
        let py = usize::try_from((ky + dy).clamp(0, side - 1)).unwrap_or(0);
        i64::from(patch[py * PATCH_KM + px])
    };

    let top = at(0, 0) + (((at(1, 0) - at(0, 0)) * fx) >> 16);
    let bottom = at(0, 1) + (((at(1, 1) - at(0, 1)) * fx) >> 16);
    top + (((bottom - top) * fy) >> 16)
}

/// Nearest-cell lookup of a km-resolution patch: the patch cell containing
/// the given tile-local 100 m cell, with no blending against its
/// neighbours — the opposite of [`sample_km_patch`]'s interpolation.
///
/// [`TileBundle::basin_km`] is piecewise-constant per continent depression
/// (feature 02 §Q1 / open-items #12): interpolating it the way
/// [`sample_km_patch`] interpolates `filled_km`/`rainfall_km` would blur a
/// value across a depression's own edge and reintroduce exactly the
/// span-dependence this lookup exists to remove, so this helper stays
/// deliberately separate rather than folding into `sample_km_patch`.
fn nearest_km_patch(bundle: &TileBundle, patch: &[i32], local_x: i32, local_y: i32) -> i32 {
    let km0x = (bundle.area.x * 512).div_euclid(10);
    let km0y = (bundle.area.y * 512).div_euclid(10);
    let side = i32::try_from(PATCH_KM).unwrap_or(0);

    let (ax, ay) = abs_cell(
        bundle.area,
        u16::try_from(local_x).unwrap_or(0),
        u16::try_from(local_y).unwrap_or(0),
    );
    let (kx, ky) = (ax.div_euclid(10) - km0x, ay.div_euclid(10) - km0y);
    let px = usize::try_from(kx.clamp(0, side - 1)).unwrap_or(0);
    let py = usize::try_from(ky.clamp(0, side - 1)).unwrap_or(0);
    patch[py * PATCH_KM + px]
}

/// Resamples the tile's rainfall patch onto every area cell (feature 03
/// §Q4): a smoothstep-bilinear resample of the 1 km rainfall grid at
/// 100 m resolution, mirroring
/// [`crate::continent::bundles::coarse_height`]'s own interpolation
/// exactly, but reading the tile's [`TileBundle::rainfall_km`] patch
/// rather than the continent grid directly.
#[must_use]
pub fn area_rainfall(bundle: &TileBundle) -> Vec<u16> {
    let mut rain = Vec::with_capacity((N * N) as usize);
    for y in 0..N {
        for x in 0..N {
            let v = sample_km_patch(bundle, &bundle.rainfall_km, x, y);
            rain.push(u16::try_from(v.clamp(0, i64::from(u16::MAX))).unwrap_or(u16::MAX));
        }
    }
    rain
}

/// Channel width from discharge (artifact, Water).
///
/// "Width grows with the square root of discharge — a stream carrying one
/// cubic metre a second is about four metres wide, a river carrying
/// twenty-five is twenty." So `w = 4 * sqrt(Q)` metres, returned in
/// decimetres.
pub fn channel_width_dm(discharge: DischargeMilli) -> Result<u32, BudgetError> {
    arda_core::hydrology::channel_width_dm(discharge)
        .ok_or(BudgetError::Overflow("physical channel width"))
}

/// Builds the stored cell grid and object lists.
pub fn compose(
    heights: &[i32],
    filled: &Filled,
    water: &WaterGrid,
    rain: &[u16],
    bundle: &TileBundle,
) -> Result<(AreaCells, AreaObjects), BudgetError> {
    let mut cells = AreaCells::flat(Cell::default());

    // Which cells belong to a lake big enough to record.
    let lakes = collect_lakes(heights, filled, bundle);
    let mut lake_cell = vec![false; (N * N) as usize];
    for l in &lakes {
        for c in &l.cells {
            lake_cell[c.index()] = true;
        }
    }

    let hand = fields::hand(heights, water);

    for y in 0..N {
        for x in 0..N {
            let Some(at) = coord(x, y) else { continue };
            let h = heights[at.index()];
            let is_lake = lake_cell[at.index()];
            let terrain = if is_lake {
                TerrainKind::Lake
            } else if h > 0 {
                TerrainKind::Land
            } else {
                TerrainKind::Sea
            };

            let (slope_milli_deg, aspect_deg) = fields::slope_and_aspect(heights, x, y);
            let land = terrain == TerrainKind::Land;

            // Non-land cells carry no flow of their own; routing still
            // crosses them so land upstream reaches an outlet.
            let drainage = if land { water.drainage_at(at) } else { 0 };
            let discharge = if land {
                DischargeMilli::new(water.discharge_at(at))
            } else {
                DischargeMilli::new(0)
            };
            let order = if land { water.order_at(at) } else { 0 };
            let hand_mm = if land { hand.above_mm[at.index()] } else { 0 };
            let rainfall = if land {
                RainfallMm::new(rain[at.index()])
            } else {
                RainfallMm::new(0)
            };

            // Marsh is flat ground a river floods, so it takes both a low
            // stand above the watercourse and a watercourse worth
            // flooding — see `fields::MARSH_MIN_DISCHARGE_MILLI`.
            let carried = if land {
                hand.carried_milli[at.index()]
            } else {
                0
            };
            let cover = match (terrain, fields::floodplain(hand_mm)) {
                (TerrainKind::Land, Floodplain::Marsh)
                    if order == 0 && carried >= fields::MARSH_MIN_DISCHARGE_MILLI =>
                {
                    Cover::Marsh
                }
                (TerrainKind::Land, _) => Cover::Grass,
                _ => Cover::Bare,
            };

            cells.set(
                at,
                Cell {
                    height: HeightMm::new(h),
                    terrain,
                    cover,
                    slope_milli_deg,
                    aspect_deg,
                    rainfall,
                    drainage_area_cells: drainage,
                    discharge,
                    watercourse_order: order,
                    watercourse_width_dm: if order > 0 {
                        channel_width_dm(discharge)?
                    } else {
                        0
                    },
                    height_above_river_dm: u16::try_from(hand_mm / 100).unwrap_or(u16::MAX),
                    wetness: if land {
                        fields::wetness(drainage, slope_milli_deg)
                    } else {
                        255
                    },
                    ..Cell::default()
                },
            );
        }
    }

    let rivers = collect_segments(&cells, water, &lake_cell)?;
    Ok((cells, AreaObjects { rivers, lakes }))
}

/// A basin cell within one cell of the tile rim (feature 03 §Q5).
fn near_rim(c: CellCoord) -> bool {
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
fn collect_lakes(heights: &[i32], filled: &Filled, bundle: &TileBundle) -> Vec<Lake> {
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
fn clamp_near_rim(b: &Basin, heights: &[i32], bundle: &TileBundle) -> (i32, u32) {
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
fn clamp_and_trim(
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
fn recompute_outlet(cells: &[CellCoord], submerged: &[bool], filled: &Filled) -> Option<CellCoord> {
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

/// Whether a channel cell begins a segment: a head, or just below a
/// confluence (artifact: segments run "from a source or a junction").
fn is_segment_start(cells: &AreaCells, water: &WaterGrid, at: CellCoord) -> bool {
    let (x, y) = (i32::from(at.x()), i32::from(at.y()));
    let inflows = fill::NEIGHBOURS
        .iter()
        .filter_map(|(dx, dy)| coord(x + dx, y + dy))
        .filter(|&nb| cells.get(nb).watercourse_order > 0 && water.downstream_of(nb) == Some(at))
        .count();
    inflows != 1
}

/// Breaks the network into segments, each knowing what it feeds and how it
/// ends (artifact, Water).
#[allow(clippy::cast_possible_truncation)] // At most512² channel starts.
fn collect_segments(
    cells: &AreaCells,
    water: &WaterGrid,
    lake_cell: &[bool],
) -> Result<Vec<RiverSegment>, BudgetError> {
    // Pass 1: walk each segment, recording its cells and where it stopped.
    let mut starts = Vec::new();
    for y in 0..N {
        for x in 0..N {
            let Some(at) = coord(x, y) else { continue };
            if cells.get(at).watercourse_order > 0 && is_segment_start(cells, water, at) {
                starts.push(at);
            }
        }
    }

    let mut owner = vec![0u32; (N * N) as usize]; // segment id per channel cell
    let mut drafts = Vec::new();

    for (i, &start) in starts.iter().enumerate() {
        let id = (i + 1) as u32;
        let mut course = vec![start];
        owner[start.index()] = id;
        let mut cursor = start;

        let ends = loop {
            let Some(next) = water.downstream_of(cursor) else {
                break if water.is_outlet(cursor) {
                    Terminus::OffTile
                } else {
                    Terminus::Sea
                };
            };
            if lake_cell[next.index()] {
                break Terminus::Lake;
            }
            let c = cells.get(next);
            if c.terrain == TerrainKind::Sea {
                break Terminus::Sea;
            }
            if c.watercourse_order == 0 {
                break Terminus::Sea;
            }
            if is_segment_start(cells, water, next) {
                break Terminus::Junction;
            }
            course.push(next);
            owner[next.index()] = id;
            cursor = next;
        };

        let tail = cells.get(cursor);
        drafts.push((
            id,
            ends,
            cursor,
            course,
            tail.watercourse_order,
            tail.discharge,
        ));
    }

    // Pass 2: resolve which segment each one feeds.
    drafts
        .into_iter()
        .map(|(id, ends, tail_cell, course, order, discharge)| {
            let feeds = if ends == Terminus::Junction {
                water
                    .downstream_of(tail_cell)
                    .map(|d| owner[d.index()])
                    .filter(|&f| f != 0)
            } else {
                None
            };
            Ok(RiverSegment {
                id,
                order,
                width_dm: channel_width_dm(discharge)?,
                discharge,
                feeds,
                ends,
                course,
            })
        })
        .collect()
}

/// Runs the area stage for one tile: relief, erosion, filling, routing.
pub fn generate_area(
    seed: u64,
    continent: &Continent,
    bundle: &TileBundle,
) -> Result<(AreaCells, AreaObjects), BudgetError> {
    let r = relief(seed, &continent.grid, bundle);
    let mut heights: Vec<i32> = (0..(N * N) as usize)
        .filter_map(|i| {
            let i = i32::try_from(i).ok()?;
            Some(r.get(coord(i % N, i / N)?))
        })
        .collect();
    // Uplift follows the regional trend, not the per-cell detail: driving it
    // from `heights` amplifies every noise bump into a dam over the run.
    let uplift: Vec<i32> = (0..(N * N) as usize)
        .filter_map(|i| {
            let i = i32::try_from(i).ok()?;
            let (ax, ay) = crate::continent::bundles::abs_cell(
                bundle.area,
                u16::try_from(i % N).ok()?,
                u16::try_from(i / N).ok()?,
            );
            Some(crate::continent::bundles::coarse_height(
                &continent.grid,
                ax,
                ay,
            ))
        })
        .collect();

    erosion::erode(&mut heights, &uplift, bundle);

    let filled = fill::fill(&heights, bundle);
    let rain = area_rainfall(bundle);
    let w = water::water(&filled, bundle, &rain);
    compose(&heights, &filled, &w, &rain, bundle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continent::build_continent;
    use crate::continent::bundles::bundle_for;
    use arda_core::{AreaCoord, GenerateConfig};

    fn world(area: AreaCoord) -> (AreaCells, AreaObjects) {
        let c = build_continent(42, GenerateConfig::MICRO, 0);
        let b = bundle_for(42, &c, area);
        generate_area(42, &c, &b).unwrap()
    }

    /// `build_continent(123, MICRO, 0)`, memoized: the seam-lake tests
    /// below scan every MICRO tile, and continent generation is too
    /// expensive to redo per tile (mirrors
    /// `continent::bundles::tests::fixture_ctx`).
    ///
    /// Seed 123, not the usual 42: a survey of twelve seeds under MICRO
    /// (feature 03 §Q5 / open-items #12) found exactly one near-rim lake
    /// in 96 tile-generations, on seed 123 tile (1, 0); seed 42 has none.
    /// See task-5-report.md for the full survey.
    const SEAM_LAKE_SEED: u64 = 123;

    fn fixture_ctx() -> crate::continent::Continent {
        static CTX: std::sync::OnceLock<crate::continent::Continent> = std::sync::OnceLock::new();
        CTX.get_or_init(|| build_continent(SEAM_LAKE_SEED, GenerateConfig::MICRO, 0))
            .clone()
    }

    /// Feature 02 §Q1 / feature 03 §Q5 test oracle: the near-rim cells
    /// among `cells`, first checked for continent-tier lake identity
    /// (nearest-cell `bundle.basin_km`, `NO_BASIN` excluded) and, only if
    /// none maps to a depression, falling back to the max smoothstep-bilinear
    /// sample of `bundle.filled_km` — reimplemented here independently of
    /// `nearest_km_patch` / `sample_km_patch` / `clamp_near_rim` so this
    /// check cannot pass merely by calling back into the code under test.
    /// `None` when `cells` has no near-rim cell.
    fn continent_surface_at(b: &TileBundle, cells: &[CellCoord]) -> Option<i32> {
        let km0x = (b.area.x * 512).div_euclid(10);
        let km0y = (b.area.y * 512).div_euclid(10);
        let side = i32::try_from(PATCH_KM).unwrap_or(0);

        let rim: Vec<&CellCoord> = cells
            .iter()
            .filter(|c| c.x() <= 1 || c.y() <= 1 || c.x() >= 510 || c.y() >= 510)
            .collect();
        if rim.is_empty() {
            return None;
        }

        let basin_best = rim
            .iter()
            .map(|c| {
                let (ax, ay) = abs_cell(b.area, c.x(), c.y());
                let (kx, ky) = (ax.div_euclid(10) - km0x, ay.div_euclid(10) - km0y);
                let px = usize::try_from(kx.clamp(0, side - 1)).unwrap_or(0);
                let py = usize::try_from(ky.clamp(0, side - 1)).unwrap_or(0);
                b.basin_km[py * PATCH_KM + px]
            })
            .filter(|&v| v != crate::continent::hydrology::NO_BASIN)
            .max();
        if let Some(v) = basin_best {
            return Some(v);
        }

        let smooth = |v: i32| -> i64 {
            let t = i64::from(v) * 65536 / 10;
            let t2 = (t * t) >> 16;
            let t3 = (t2 * t) >> 16;
            (3 * t2 - 2 * t3).clamp(0, 65536)
        };

        let best = rim
            .iter()
            .map(|c| {
                let (ax, ay) = abs_cell(b.area, c.x(), c.y());
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
            })
            .max()?;
        Some(i32::try_from(best).unwrap_or(i32::MAX))
    }

    #[test]
    fn physical_width_supports_the_maximum_world_and_rejects_unrepresentable_inputs() {
        let width = channel_width_dm(DischargeMilli::new(33_249_619_483)).unwrap();
        assert!(width > u32::from(u16::MAX));
        assert_eq!(width, 230_649);
        assert!(channel_width_dm(DischargeMilli::new(u64::MAX)).is_err());
    }

    #[test]
    fn width_follows_the_artifact_relation() {
        // "one cubic metre a second is about four metres wide, a river
        // carrying twenty-five is twenty"
        let four_m = channel_width_dm(DischargeMilli::new(1_000)).unwrap();
        let twenty_m = channel_width_dm(DischargeMilli::new(25_000)).unwrap();
        assert!((38..=42).contains(&four_m), "1 m3/s gave {four_m} dm");
        assert!(
            (190..=210).contains(&twenty_m),
            "25 m3/s gave {twenty_m} dm"
        );
    }

    #[test]
    fn area_generation_is_deterministic() {
        assert_eq!(world(AreaCoord::new(1, 1)), world(AreaCoord::new(1, 1)));
    }

    #[test]
    fn segments_know_how_they_end() {
        let (_, o) = world(AreaCoord::new(0, 1));
        assert!(!o.rivers.is_empty(), "no segments emitted");
        for s in &o.rivers {
            if s.ends == Terminus::Junction {
                assert!(
                    s.feeds.is_some(),
                    "segment {} ends at a junction but feeds nothing",
                    s.id
                );
            } else {
                assert_eq!(
                    s.feeds, None,
                    "segment {} ends at {:?} yet feeds",
                    s.id, s.ends
                );
            }
        }
    }

    #[test]
    fn segments_partition_the_channel_network() {
        let (c, o) = world(AreaCoord::new(0, 1));
        let mut seen = std::collections::HashSet::new();
        for s in &o.rivers {
            for cell in &s.course {
                assert!(seen.insert(*cell), "cell {cell:?} is in two segments");
            }
        }
        for y in 0..N {
            for x in 0..N {
                let Some(at) = coord(x, y) else { continue };
                if c.get(at).watercourse_order > 0 {
                    assert!(seen.contains(&at), "channel cell {x},{y} is in no segment");
                }
            }
        }
    }

    #[test]
    fn non_land_cells_carry_no_flow() {
        let (c, _) = world(AreaCoord::new(0, 0));
        for y in 0..N {
            for x in 0..N {
                let Some(at) = coord(x, y) else { continue };
                let cell = c.get(at);
                if cell.terrain != TerrainKind::Land {
                    assert_eq!(cell.drainage_area_cells, 0);
                    assert_eq!(cell.discharge.raw(), 0);
                    assert_eq!(cell.watercourse_order, 0);
                }
            }
        }
    }

    #[test]
    fn slope_and_aspect_are_populated() {
        let (c, _) = world(AreaCoord::new(0, 1));
        let sloped = (0..N)
            .flat_map(|y| (0..N).map(move |x| (x, y)))
            .filter_map(|(x, y)| coord(x, y))
            .filter(|&at| c.get(at).slope_milli_deg > 0)
            .count();
        assert!(sloped > 1_000, "only {sloped} cells have a slope");
    }

    #[test]
    fn rainfall_is_sampled_onto_every_land_cell() {
        let (c, _) = world(AreaCoord::new(0, 1));
        let mut wet = 0;
        for y in 0..N {
            for x in 0..N {
                let Some(at) = coord(x, y) else { continue };
                let cell = c.get(at);
                match cell.terrain {
                    TerrainKind::Land => wet += u32::from(cell.rainfall.raw() > 0),
                    _ => assert_eq!(cell.rainfall.raw(), 0),
                }
            }
        }
        assert!(wet > 10_000, "only {wet} land cells got rain");
    }

    /// A rim-touching basin takes the continent's spill level, and its
    /// membership stays exactly what `fill::fill` found (feature 03 §Q5,
    /// closes open-items #12).
    ///
    /// Built from a synthetic pit rather than generated relief. It used to
    /// scan seed 123 tile (1, 0), the single near-rim lake a 12-seed
    /// survey turned up; raising `LAKE_MIN_CELLS`/`LAKE_MIN_DEPTH_MM`
    /// leaves no near-rim lake anywhere in a MICRO world (re-surveyed 12
    /// seeds x 8 tiles with `survey_near_rim_lakes`: zero hits — a MICRO
    /// tile is too small to hold a 3 km2 lake against its own rim). A test
    /// that quietly finds nothing to check is worse than one that builds
    /// its own case, so it now builds one.
    #[test]
    fn edge_touching_basins_take_the_continent_spill_level() {
        let ctx = fixture_ctx();
        let mut b = bundle_for(SEAM_LAKE_SEED, &ctx, AreaCoord::new(0, 1));
        b.filled_km = vec![DIAG_CLAMP_MM; PATCH_KM * PATCH_KM];

        let heights = rim_touching_pit();
        let filled = fill::fill(&heights, &b);
        let lakes = collect_lakes(&heights, &filled, &b);

        let mut checked = 0u32;
        for lake in &lakes {
            if !lake.cells.iter().any(|&c| near_rim(c)) {
                continue;
            }
            checked += 1;

            let expected = continent_surface_at(&b, &lake.cells)
                .expect("a near-rim lake must have at least one near-rim cell");
            assert_eq!(
                lake.surface.raw(),
                expected,
                "lake {} did not take the continent spill",
                lake.id
            );

            // Basins are disjoint and non-empty, so the first cell
            // (ascending row-major, per `fill::fill`) identifies the basin
            // this lake came from.
            let basin = filled
                .basins
                .iter()
                .find(|basin| basin.cells.first() == lake.cells.first())
                .unwrap_or_else(|| {
                    panic!(
                        "lake {} has no fill::fill basin starting at the same cell",
                        lake.id
                    )
                });
            assert_eq!(
                &basin.cells, &lake.cells,
                "lake {} cell set drifted from fill::fill's basin \u{2014} membership must \
                 stay local to the tile (\u{a7}Q5)",
                lake.id
            );
        }
        assert!(
            checked > 0,
            "the synthetic rim basin must yield a near-rim lake for this to check anything"
        );
    }

    /// A pit pressed against the tile's west rim, sized to clear both lake
    /// floors.
    ///
    /// Column 0 stays wall: `fill::fill` seeds the border at its own
    /// height, so a cell ON the edge is never submerged and could not form
    /// a depression. `near_rim` reaches x <= 1, so column 1 is what makes
    /// this basin near-rim.
    fn rim_touching_pit() -> Vec<i32> {
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
        // 4 columns x 100 rows = 400 cells, over LAKE_MIN_CELLS, and
        // DIAG_CLAMP_MM - DIAG_LOW_MM = 4.5 m clears LAKE_MIN_DEPTH_MM.
        for y in 100..=199 {
            for x in 1..=4 {
                let i = usize::try_from(y * N + x).unwrap_or(0);
                heights[i] = DIAG_LOW_MM;
            }
        }
        heights
    }

    #[test]
    fn clamp_near_rim_replaces_local_spill_but_not_membership() {
        // Feature 03 §Q5, non-vacuous regardless of whether any generated
        // MICRO tile happens to carry a near-rim lake: a synthetic basin
        // touching the rim, clamped against a `filled_km` patch flattened
        // to one known value so the expected clamp is exact without
        // re-deriving the smoothstep-bilinear weights a third time.
        let ctx = fixture_ctx();
        let mut b = bundle_for(SEAM_LAKE_SEED, &ctx, AreaCoord::new(0, 1));
        let flat = 12_345i32;
        b.filled_km = vec![flat; PATCH_KM * PATCH_KM];

        let rim_cell = coord(0, 200).unwrap();
        let inner_cell = coord(5, 200).unwrap();
        assert!(near_rim(rim_cell) && !near_rim(inner_cell));
        let mut heights = vec![0i32; (N * N) as usize];
        heights[rim_cell.index()] = 100;
        heights[inner_cell.index()] = 300;

        let basin = Basin {
            cells: vec![inner_cell, rim_cell],
            surface_mm: 500, // local spill, deliberately far from `flat`
            depth_mm: 400,
        };
        assert_eq!(
            clamp_near_rim(&basin, &heights, &b),
            (flat, u32::try_from(flat - 100).unwrap()),
            "clamp must take the continent surface and recompute depth against \
             it and the basin's own floor, without touching membership"
        );

        // An interior basin (no near-rim cell) must pass through exactly
        // as `fill::fill` reported it, regardless of `filled_km`.
        let interior = Basin {
            cells: vec![coord(200, 200).unwrap()],
            surface_mm: 500,
            depth_mm: 400,
        };
        assert_eq!(clamp_near_rim(&interior, &heights, &b), (500, 400));
    }

    /// A west-rim physical basin with 100 high bank cells and 300 low cells.
    /// Both controls pass this terrain through the real local priority flood.
    /// A constant coarse clamp makes trim expectations analytic; no natural
    /// erosion pit or sampled continent-depression location is required.
    fn explicit_seam_basin() -> (Vec<i32>, TileBundle, Filled) {
        let ctx = fixture_ctx();
        let mut b = bundle_for(SEAM_LAKE_SEED, &ctx, AreaCoord::new(0, 1));
        b.basin_km.fill(NO_BASIN);
        b.filled_km.fill(DIAG_CLAMP_MM);
        let mut heights = rim_touching_pit();
        for y in 100..=199 {
            heights[coord(1, y).unwrap().index()] = DIAG_HIGH_MM;
        }
        let filled = fill::fill(&heights, &b);
        assert_eq!(
            filled.basins.len(),
            1,
            "only the physical pit may be submerged"
        );
        (heights, b, filled)
    }

    #[test]
    fn seam_clamp_trims_cells_the_shared_surface_leaves_dry() {
        // The former seed 99 pit vanished under the approved physical erosion
        // correction. Preserve the down-clamp/trim/outlet contract with a real
        // explicit 400-cell pit: its 100 high bank cells become dry at 5.5 m,
        // while 300 cells at 1 m remain submerged with 4.5 m depth.
        let (heights, b, filled) = explicit_seam_basin();
        let target = coord(1, 100).unwrap();
        let basin = filled
            .basins
            .iter()
            .find(|basin| basin.cells.first() == Some(&target))
            .expect("the physical west-rim pit must be found by the real fill");
        // Four cardinal steps from the 10 m physical rim give the legacy
        // routing flood's 10,004 mm maximum; walls already descend without fill.
        assert_eq!((basin.cells.len(), basin.surface_mm), (400, 10_004));

        let (surface_mm, depth_mm, cells) =
            clamp_and_trim(basin, &heights, &b).expect("the clamp does not empty this basin");
        assert!(
            surface_mm < basin.surface_mm,
            "this case is meant to pin a DOWN clamp"
        );
        assert_eq!((surface_mm, depth_mm, cells.len()), (5_500, 4_500, 300));

        for &c in &cells {
            assert!(
                heights[c.index()] < surface_mm,
                "cell {c:?} survived the trim but sits at or above the clamped surface"
            );
        }
        for &c in &basin.cells {
            if !cells.contains(&c) {
                assert!(
                    heights[c.index()] >= surface_mm,
                    "cell {c:?} was trimmed despite sitting below the clamped surface"
                );
            }
        }

        // Round-2 review fix: the outlet now comes from a second pass over
        // the tile's final lake set (`collect_lakes`), not from
        // `clamp_and_trim` itself. Mirror that pass here for this single
        // basin: a `submerged` grid marking exactly its surviving `cells`,
        // as `collect_lakes` would build if this were the tile's only
        // lake.
        let mut submerged = vec![false; (N * N) as usize];
        for &c in &cells {
            submerged[c.index()] = true;
        }
        let outlet = recompute_outlet(&cells, &submerged, &filled);

        // The outlet moves to a released bank cell, remains outside surviving
        // membership, and still physically touches the trimmed lake.
        let out = outlet.expect("the 300 surviving cells have a released adjacent bank");
        assert!(
            basin.cells.contains(&out),
            "the outlet uses the released bank"
        );
        assert!(
            !cells.contains(&out),
            "outlet {out:?} must not be one of the surviving lake's own cells"
        );
        assert!(
            cells.iter().any(|&c| {
                let dx = i32::from(c.x()) - i32::from(out.x());
                let dy = i32::from(c.y()) - i32::from(out.y());
                dx.abs() <= 1 && dy.abs() <= 1
            }),
            "outlet {out:?} is not 4/8-adjacent to any surviving cell"
        );
    }

    #[test]
    fn seam_clamp_drops_a_basin_the_trim_empties() {
        // At the 1 m floor, every cell in the explicit physical basin becomes
        // dry (equality is not submerged). This replaces the vanished seed 42
        // two-cell natural pit without weakening the empty-trim contract.
        let (heights, mut b, filled) = explicit_seam_basin();
        b.filled_km.fill(DIAG_LOW_MM);
        let target = coord(1, 100).unwrap();
        let basin = filled
            .basins
            .iter()
            .find(|basin| basin.cells.first() == Some(&target))
            .expect("the physical west-rim pit must be found by the real fill");
        assert_eq!((basin.cells.len(), basin.surface_mm), (400, 10_004));
        assert!(basin
            .cells
            .iter()
            .all(|c| heights[c.index()] >= DIAG_LOW_MM));

        assert_eq!(
            clamp_and_trim(basin, &heights, &b),
            None,
            "every cell in this basin sits at or above the clamped surface, so trimming \
             must empty it and the basin must not become a lake"
        );
        assert!(collect_lakes(&heights, &filled, &b).is_empty());
    }

    /// Heights used only by
    /// `diagonally_adjacent_basins_do_not_exclude_each_others_released_cells`
    /// below.
    const DIAG_LOW_MM: i32 = 1_000;
    const DIAG_HIGH_MM: i32 = 9_000;
    const DIAG_CLAMP_MM: i32 = 5_500;

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
}
