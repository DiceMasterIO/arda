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
mod lakes;
pub mod relief;
mod segments;
#[cfg(test)]
mod terrain_correction_probe;
#[cfg(test)]
mod terrain_tests;
pub mod water;

use crate::continent::bundles::{abs_cell, TileBundle, PATCH_KM};
use crate::continent::Continent;
use crate::hydrology::budget::BudgetError;
use arda_core::{
    AreaCells, Cell, CellCoord, Cover, DischargeMilli, HeightMm, RainfallMm, TerrainKind,
    AREA_CELLS,
};
use fields::Floodplain;
use fill::Filled;
use lakes::collect_lakes;
use local_objects::LocalAreaObjects as AreaObjects;
pub use relief::{relief, ReliefGrid};
use segments::collect_segments;
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
mod tests;
