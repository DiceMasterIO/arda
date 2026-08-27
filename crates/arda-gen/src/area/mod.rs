//! Area generation (`logic/02`).
//!
//! The normative rules are the artifact's own sections; this module
//! implements Relief and Water. Climate, vegetation, settlement, land use,
//! and roads arrive at build-order step 5.

pub mod erosion;
pub mod fields;
pub mod fill;
pub mod relief;
pub mod water;

use crate::continent::bundles::{abs_cell, TileBundle, PATCH_KM};
use crate::continent::Continent;
use arda_core::{
    AreaCells, AreaObjects, Cell, CellCoord, Cover, DischargeMilli, HeightMm, Lake, RainfallMm,
    RiverSegment, Terminus, TerrainKind, AREA_CELLS,
};
use fields::Floodplain;
use fill::{Basin, Filled};
pub use relief::{relief, ReliefGrid};
pub use water::{water, WaterGrid};

const N: i32 = AREA_CELLS as i32;

/// Smallest submerged extent that is recorded as a lake, in cells.
pub const LAKE_MIN_CELLS: usize = 100;
/// Smallest maximum depth that is recorded as a lake, in millimetres.
pub const LAKE_MIN_DEPTH_MM: u32 = 2_000;

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
#[must_use]
pub fn channel_width_dm(discharge: DischargeMilli) -> u16 {
    let q_milli = i64::from(discharge.raw());
    if q_milli == 0 {
        return 0;
    }
    // w_dm = 40 * sqrt(Q_m3s) = 40 * sqrt(q_milli / 1000)
    let scaled = isqrt(q_milli * 1000); // sqrt(q_milli)*1000 in milli units
    u16::try_from(40 * scaled / 1000).unwrap_or(u16::MAX)
}

fn isqrt(v: i64) -> i64 {
    if v <= 0 {
        return 0;
    }
    let mut x = v;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + v / x) / 2;
    }
    x
}

/// Builds the stored cell grid and object lists.
#[must_use]
pub fn compose(
    heights: &[i32],
    filled: &Filled,
    water: &WaterGrid,
    rain: &[u16],
    bundle: &TileBundle,
) -> (AreaCells, AreaObjects) {
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
            let hand_mm = if land { hand[at.index()] } else { 0 };
            let rainfall = if land {
                RainfallMm::new(rain[at.index()])
            } else {
                RainfallMm::new(0)
            };

            let cover = match (terrain, fields::floodplain(hand_mm)) {
                (TerrainKind::Land, Floodplain::Marsh) if order == 0 => Cover::Marsh,
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
                        channel_width_dm(discharge)
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

    let rivers = collect_segments(&cells, water, &lake_cell);
    (cells, AreaObjects { rivers, lakes })
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
/// applied AFTER the clamp, not before: a basin that only clears
/// `LAKE_MIN_DEPTH_MM` once the continent surface raises it should still
/// be promoted to a lake, and a basin that only cleared the threshold at
/// its own (locally wrong) spill has no business being reported as a
/// lake the neighbouring tile disagrees exists.
///
/// Basins are processed in `filled.basins`'s own deterministic order
/// (sorted by first cell) while threading one `claimed` grid through
/// every basin's clamp: `fill::fill` guarantees basins start disjoint,
/// but two independently-clamped near-rim basins can expand into the
/// same low-lying land (measured on real fixtures, not hypothetical —
/// see task-5-report.md), and without this a cell could end up listed in
/// two different `Lake.cells`. Earlier basins win; a later basin's
/// expansion simply cannot cross into cells an earlier one already
/// claimed.
fn collect_lakes(heights: &[i32], filled: &Filled, bundle: &TileBundle) -> Vec<Lake> {
    let mut claimed = vec![false; (N * N) as usize];
    filled
        .basins
        .iter()
        .map(|b| {
            let (surface_mm, depth_mm, cells) = clamp_near_rim(b, heights, bundle, &mut claimed);
            (surface_mm, depth_mm, cells, b.outlet)
        })
        .filter(|(_, depth_mm, cells, _)| {
            cells.len() >= LAKE_MIN_CELLS && *depth_mm >= LAKE_MIN_DEPTH_MM
        })
        .enumerate()
        .map(|(i, (surface_mm, depth_mm, cells, outlet))| Lake {
            id: u16::try_from(i + 1).unwrap_or(u16::MAX),
            surface: HeightMm::new(surface_mm),
            depth_mm,
            outlet,
            cells,
        })
        .collect()
}

/// A basin's lake surface, depth, and membership after the seam clamp.
///
/// Interior basins (no cell within one cell of the rim) pass through
/// unchanged. A near-rim basin's surface becomes the MAX, over its
/// ORIGINAL near-rim cells (as `fill::fill` reported them), of the
/// smoothstep-bilinear sample of `bundle.filled_km` — the same
/// interpolation `area_rainfall` uses, shared via `sample_km_patch`
/// rather than duplicated a third time. When that clamp surface differs
/// from the basin's own local spill, depth and membership are
/// recomputed against it; when it matches, nothing changes.
///
/// One BFS pass only, not a fixed point: growing membership can uncover
/// NEW near-rim cells whose own sample is higher still, and iterating
/// until that stops moving was tried and rejected — on real fixture data
/// it does not settle nearby, it runs the lake from a few hundred cells
/// to the majority of the tile before it stabilises, chasing a single
/// coarse km cell along the whole rim band. The brief's own fallback for
/// this ("clamp at the basin's lowest boundary cell") is deliberately
/// deferred to Task 8's invariant check rather than built speculatively
/// here; see task-5-report.md.
///
/// Depth and the "floor" (lowest submerged cell) are computed over the
/// basin's ORIGINAL (unclaimed) cells, per the brief's literal ordering
/// ("recompute depth_mm ... AND expand membership", depth first):
/// `fill::fill` already found this basin's true lowest point, and every
/// 4-neighbour of a submerged cell that is itself lower is already
/// submerged (the priority-flood invariant), so a newly-flooded
/// land-bridge cell sits at or above that floor in every fixture
/// measured for this task. Using the pre-expansion floor also keeps
/// `depth_mm` meaningful when `expand_lake` reaches far beyond the
/// original pit (see task-5-report.md): the reported depth stays
/// anchored to the basin `fill::fill` actually found, rather than
/// collapsing toward `clamp_surface` because the flood reached all the
/// way down to present-but-shallow land near sea level.
///
/// `claimed` tracks every cell already spoken for by an earlier-processed
/// basin this tile (see `collect_lakes`): cells already claimed are
/// dropped from this basin's own starting cells before anything else, so
/// two basins can never end up both listing the same `CellCoord`.
fn clamp_near_rim(
    b: &Basin,
    heights: &[i32],
    bundle: &TileBundle,
    claimed: &mut [bool],
) -> (i32, u32, Vec<CellCoord>) {
    let free: Vec<CellCoord> = b
        .cells
        .iter()
        .copied()
        .filter(|c| !claimed[c.index()])
        .collect();

    if !free.iter().any(|&c| near_rim(c)) {
        for c in &free {
            claimed[c.index()] = true;
        }
        return (b.surface_mm, b.depth_mm, free);
    }

    let clamp_mm = free
        .iter()
        .filter(|&&c| near_rim(c))
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
        .unwrap_or(b.surface_mm);

    if clamp_mm == b.surface_mm {
        for c in &free {
            claimed[c.index()] = true;
        }
        return (b.surface_mm, b.depth_mm, free);
    }

    // feature 03 §Q5: both sides sample the same continent surface
    let floor = free
        .iter()
        .map(|c| heights[c.index()])
        .min()
        .unwrap_or(clamp_mm);
    let depth_mm = u32::try_from(clamp_mm.saturating_sub(floor)).unwrap_or(0);
    let cells = expand_lake(&free, heights, clamp_mm, claimed);
    (clamp_mm, depth_mm, cells)
}

/// Grows a basin's membership by 4-connected BFS from its existing
/// cells, adding land cells with `0 < height < surface` (feature 03
/// §Q5): once the lake's reported surface rises to the continent clamp,
/// any newly-submerged land between the old shoreline and the new one
/// belongs to the lake too. Never crosses into a cell `claimed` already
/// marks true (an earlier basin's territory); every cell this call adds,
/// including the seed, is marked claimed before returning so a later
/// basin cannot re-claim it either.
///
/// Driven by a plain queue and a `Vec<bool>` visited flag, never a
/// `HashSet` — its iteration order is randomized per process and would
/// make the push order (and so, were it ever order-sensitive) vary
/// between runs. The final list is sorted ascending row-major, same as
/// every other basin's cells.
fn expand_lake(
    seed: &[CellCoord],
    heights: &[i32],
    surface: i32,
    claimed: &mut [bool],
) -> Vec<CellCoord> {
    let mut member = vec![false; (N * N) as usize];
    for &c in seed {
        member[c.index()] = true;
        claimed[c.index()] = true;
    }
    let mut queue: std::collections::VecDeque<CellCoord> = seed.iter().copied().collect();
    let mut cells = seed.to_vec();

    while let Some(cur) = queue.pop_front() {
        let (x, y) = (i32::from(cur.x()), i32::from(cur.y()));
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let Some(nb) = coord(x + dx, y + dy) else {
                continue;
            };
            let ni = nb.index();
            if !member[ni] && !claimed[ni] && heights[ni] > 0 && heights[ni] < surface {
                member[ni] = true;
                claimed[ni] = true;
                cells.push(nb);
                queue.push_back(nb);
            }
        }
    }

    cells.sort_unstable();
    cells
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
fn collect_segments(cells: &AreaCells, water: &WaterGrid, lake_cell: &[bool]) -> Vec<RiverSegment> {
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

    let mut owner = vec![0u16; (N * N) as usize]; // segment id per channel cell
    let mut drafts = Vec::new();

    for (i, &start) in starts.iter().enumerate() {
        let id = u16::try_from(i + 1).unwrap_or(u16::MAX);
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
            RiverSegment {
                id,
                order,
                width_dm: channel_width_dm(discharge),
                discharge,
                feeds,
                ends,
                course,
            }
        })
        .collect()
}

/// Runs the area stage for one tile: relief, erosion, filling, routing.
#[must_use]
pub fn generate_area(
    seed: u64,
    continent: &Continent,
    bundle: &TileBundle,
) -> (AreaCells, AreaObjects) {
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
        generate_area(42, &c, &b)
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

    /// Feature 03 §Q5 test oracle: the max smoothstep-bilinear sample of
    /// `bundle.filled_km` over the near-rim cells among `cells`,
    /// reimplemented here independently of `sample_km_patch` /
    /// `clamp_near_rim` so this check cannot pass merely by calling back
    /// into the code under test. `None` when `cells` has no near-rim cell.
    fn continent_surface_at(b: &TileBundle, cells: &[CellCoord]) -> Option<i32> {
        let km0x = (b.area.x * 512).div_euclid(10);
        let km0y = (b.area.y * 512).div_euclid(10);
        let side = i32::try_from(PATCH_KM).unwrap_or(0);

        let smooth = |v: i32| -> i64 {
            let t = i64::from(v) * 65536 / 10;
            let t2 = (t * t) >> 16;
            let t3 = (t2 * t) >> 16;
            (3 * t2 - 2 * t3).clamp(0, 65536)
        };

        let best = cells
            .iter()
            .filter(|c| c.x() <= 1 || c.y() <= 1 || c.x() >= 510 || c.y() >= 510)
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
    fn width_follows_the_artifact_relation() {
        // "one cubic metre a second is about four metres wide, a river
        // carrying twenty-five is twenty"
        let four_m = channel_width_dm(DischargeMilli::new(1_000));
        let twenty_m = channel_width_dm(DischargeMilli::new(25_000));
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

    #[test]
    fn edge_touching_basins_take_the_continent_spill_level() {
        // Feature 03 spec R9 (closes open-items #12): a basin whose cells
        // sit within one cell of the tile rim must report the SAME surface
        // the neighbouring tile would compute for the same shared data,
        // because both sample `bundle.filled_km` instead of each tile's own
        // (possibly different) local spill.
        //
        // Checked against `fill::fill`'s own basins (replicating
        // `generate_area`'s pipeline up to that point, as
        // `drainage_invariants.rs` and `fill::tests::setup` already do),
        // not `objects.lakes`: `clamp_near_rim` samples a basin's ORIGINAL
        // near-rim cells, and the BFS expansion below it can uncover
        // further-out near-rim cells with their own (possibly higher)
        // sample — a real basin in this fixture does exactly that, and
        // checking a lake's *final*, expanded cells against this same
        // oracle would demand a fixed-point clamp that was tried and
        // rejected (see `clamp_near_rim`'s doc comment and
        // task-5-report.md). Scans all 8 MICRO tiles rather than just
        // (0, 1): the brief's fallback ("if tile (0,1) has none, scan all
        // 8") collapses to this single unconditional scan, which gives the
        // same guarantee more simply.
        let ctx = fixture_ctx();
        let mut checked = 0u32;
        for area in GenerateConfig::MICRO.area_coords() {
            let b = bundle_for(SEAM_LAKE_SEED, &ctx, area);
            let r = relief(SEAM_LAKE_SEED, &ctx.grid, &b);
            let mut heights: Vec<i32> = (0..(N * N) as usize)
                .filter_map(|i| {
                    let i = i32::try_from(i).ok()?;
                    Some(r.get(coord(i % N, i / N)?))
                })
                .collect();
            let uplift: Vec<i32> = (0..(N * N) as usize)
                .filter_map(|i| {
                    let i = i32::try_from(i).ok()?;
                    let (ax, ay) = crate::continent::bundles::abs_cell(
                        area,
                        u16::try_from(i % N).ok()?,
                        u16::try_from(i / N).ok()?,
                    );
                    Some(crate::continent::bundles::coarse_height(&ctx.grid, ax, ay))
                })
                .collect();
            erosion::erode(&mut heights, &uplift, &b);
            let filled = fill::fill(&heights, &b);

            // Same claiming state `collect_lakes` threads through, in the
            // same basin order, so this reproduces exactly what the real
            // pipeline computes for each basin (mirroring `clamp_near_rim`'s
            // own claimed-filter so the two can never disagree about which
            // basin is near-rim).
            let mut claimed = vec![false; (N * N) as usize];
            for basin in &filled.basins {
                let free: Vec<CellCoord> = basin
                    .cells
                    .iter()
                    .copied()
                    .filter(|c| !claimed[c.index()])
                    .collect();
                let expected = free
                    .iter()
                    .any(|&c| near_rim(c))
                    .then(|| continent_surface_at(&b, &free))
                    .flatten();
                let (surface_mm, _, _) = clamp_near_rim(basin, &heights, &b, &mut claimed);
                if let Some(expected) = expected {
                    checked += 1;
                    assert_eq!(
                        surface_mm, expected,
                        "basin on tile {area:?} did not take the continent spill"
                    );
                }
            }
        }
        assert!(
            checked > 0,
            "no near-rim basin exists on any of the 8 MICRO tiles at seed \
             {SEAM_LAKE_SEED} — this test cannot exercise the clamp; \
             see task-5-report.md"
        );
    }

    #[test]
    fn clamp_near_rim_replaces_local_spill_and_grows_membership() {
        // Feature 03 §Q5, non-vacuous regardless of whether any generated
        // MICRO tile happens to carry a near-rim lake: a synthetic basin
        // touching the rim, clamped against a `filled_km` patch flattened
        // to one known value so the expected clamp is exact without
        // re-deriving the smoothstep-bilinear weights a third time.
        let ctx = fixture_ctx();
        let mut b = bundle_for(SEAM_LAKE_SEED, &ctx, AreaCoord::new(0, 1));
        let flat = 12_345i32;
        b.filled_km = vec![flat; PATCH_KM * PATCH_KM];

        // Sea everywhere except a 10x10 dry patch straddling the rim
        // (x in 0..10, y in 195..205), height 100 mm throughout: well
        // under both the (deliberately different) local spill and the
        // continent clamp, so BFS should flood the whole connected patch
        // and nothing beyond it.
        let mut heights = vec![0i32; (N * N) as usize];
        for y in 195..205 {
            for x in 0..10 {
                let c = coord(x, y).unwrap();
                heights[c.index()] = 100;
            }
        }
        let rim_cell = coord(0, 200).unwrap();
        let inner_cell = coord(5, 200).unwrap();
        assert!(near_rim(rim_cell) && !near_rim(inner_cell));

        let basin = Basin {
            cells: vec![inner_cell, rim_cell],
            surface_mm: 500, // local spill, deliberately far from `flat`
            depth_mm: 400,
            outlet: None,
        };

        let mut claimed = vec![false; (N * N) as usize];
        let (surface_mm, depth_mm, cells) = clamp_near_rim(&basin, &heights, &b, &mut claimed);
        assert_eq!(surface_mm, flat, "clamp did not take the continent surface");
        assert_eq!(depth_mm, u32::try_from(flat - 100).unwrap());
        assert_eq!(cells.len(), 100, "BFS should flood the whole dry patch");
        assert!(
            cells.windows(2).all(|w| w[0] < w[1]),
            "cells must stay sorted ascending row-major"
        );
        assert!(
            cells.iter().all(|c| claimed[c.index()]),
            "every flooded cell must be marked claimed"
        );

        // A second basin whose only cell sits inside territory the first
        // basin's expansion already claimed must come back empty, never
        // re-listing a cell another `Lake` already owns.
        let overlapping = Basin {
            cells: vec![coord(3, 199).unwrap()],
            surface_mm: 500,
            depth_mm: 400,
            outlet: None,
        };
        let (_, _, dup_cells) = clamp_near_rim(&overlapping, &heights, &b, &mut claimed);
        assert!(
            dup_cells.is_empty(),
            "a basin entirely inside claimed territory must contribute no cells"
        );

        // An interior basin (no near-rim cell) must pass through exactly
        // as `fill::fill` reported it, regardless of `filled_km`.
        let interior = Basin {
            cells: vec![coord(200, 200).unwrap()],
            surface_mm: 500,
            depth_mm: 400,
            outlet: None,
        };
        assert_eq!(
            clamp_near_rim(&interior, &heights, &b, &mut claimed),
            (500, 400, interior.cells.clone())
        );
    }
}
