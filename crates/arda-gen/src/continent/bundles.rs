//! Per-tile input bundles (`logic/01` step 10).
//!
//! A bundle is a pure function of coarse continent data plus the seed — never
//! of any area's fine output. Adjacent tiles are therefore mirror-consistent
//! by construction, and areas may generate in any order (`logic/01`
//! invariant).

use super::{Continent, ContinentGrid};
use arda_core::{AreaCoord, CellCoord, ClimateRegime, DischargeMilli, AREA_CELLS};

/// Absolute cell coordinates of a tile-local offset.
///
/// `local` may equal [`AREA_CELLS`], which addresses the neighbour's first
/// cell — that is how a shared edge is named identically by both tiles.
#[must_use]
pub fn abs_cell(area: AreaCoord, local_x: u16, local_y: u16) -> (i32, i32) {
    (
        area.x * i32::from(AREA_CELLS) + i32::from(local_x),
        area.y * i32::from(AREA_CELLS) + i32::from(local_y),
    )
}

/// Initial terrain elevation at an absolute cell, in millimetres.
///
/// Shared preparation and the local diagnostic API use the same initial relief.
/// Absolute coordinates identify the same sample from any tile. Production
/// subsequently evolves the whole modeled rectangle before slicing it; internal
/// publication edges do not pin the final bed (`logic/02`, shared terrain).
#[must_use]
pub fn boundary_height(seed: u64, continent: &ContinentGrid, abs_x: i32, abs_y: i32) -> i32 {
    let coarse = coarse_height(continent, abs_x, abs_y);
    let relief = refinement_relief(coarse, |dx, dy| {
        coarse_height(
            continent,
            abs_x.saturating_add(dx),
            abs_y.saturating_add(dy),
        )
    });
    refine_height(seed, coarse, relief, abs_x, abs_y)
}

/// Regional detail correction (`logic/02`): the largest regional height
/// change to a neighbor 1 km away. Sampling the same continuous surface at
/// translated absolute coordinates avoids a new envelope jump at grid cuts.
/// The full i32 height difference fits i64, including opposite extremes.
pub(crate) fn refinement_relief(
    coarse: i32,
    mut sample_offset: impl FnMut(i32, i32) -> i32,
) -> i64 {
    const OFFSETS: [(i32, i32); 8] = [
        (0, -10),
        (10, -10),
        (10, 0),
        (10, 10),
        (0, 10),
        (-10, 10),
        (-10, 0),
        (-10, -10),
    ];
    OFFSETS
        .into_iter()
        .map(|(dx, dy)| (i64::from(sample_offset(dx, dy)) - i64::from(coarse)).abs())
        .max()
        .unwrap_or(0)
}

/// Add bounded area detail to an already sampled regional elevation.
pub(crate) fn refine_height(seed: u64, coarse: i32, relief: i64, abs_x: i32, abs_y: i32) -> i32 {
    // Regional detail correction (`logic/02`): local relief sets roughness.
    // Raising the same regional surface above sea level must not amplify
    // its detail into closed depressions. Preserve the existing absolute
    // amplitude cap and the regional land/sea sign below (§Q8).
    let amplitude = if coarse > 0 {
        relief.clamp(0, 90_000)
    } else {
        1_500
    };
    let detail = i64::from(super::area_detail::sample(seed ^ 0x00A1_2EA5, abs_x, abs_y));

    #[allow(clippy::cast_possible_truncation)]
    {
        let detailed = i64::from(coarse) + ((detail * amplitude) >> 15);
        let bounded = if coarse > 0 {
            detailed.max(1)
        } else {
            detailed.min(0)
        };
        bounded.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    }
}

/// The bounded affine-preserving regional surface sampled from the 1 km
/// continent grid, with no area-scale detail added.
///
/// This is the uplift pattern. The artifact raises land "fastest near the
/// mountain edge, slowly in the lowland" — a regional trend. Driving uplift
/// from the noise-refined relief instead amplifies every per-cell bump for
/// the whole run, and the bumps grow into dams that close off basins.
#[must_use]
pub fn coarse_height(continent: &ContinentGrid, abs_x: i32, abs_y: i32) -> i32 {
    crate::terrain_interpolation::sample(abs_x, abs_y, |x, y| continent.get(x, y).raw())
}

/// Prevailing wind octant (uniform westerlies today — logic/01 §Q6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompassOctant {
    /// Wind blows out of the west; the only octant modelled today.
    West,
}

/// One watercourse crossing into the tile (logic/01 step 10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnteringRiver {
    /// Boundary cell the river is seeded at, the point of lowest relief
    /// in its crossing window.
    pub cell: CellCoord,
    /// Catchment feeding this entry point, summed over every continent
    /// cell that crosses in at the same seed cell.
    pub catchment_km2: u32,
    /// Discharge feeding this entry point, summed the same way.
    pub discharge: DischargeMilli,
    /// Strahler floor for this entry point.
    ///
    /// For a single contributing edge this is `entering_order(catchment_km2)`
    /// (`self.catchment_km2` re-applied to [`entering_order`]). For a
    /// MERGED seed (two or more continent edges landing on the same cell)
    /// it is instead the MAX, over the contributing edges, of each edge's
    /// own `entering_order(its own catchment)` — deliberately NOT
    /// `entering_order` of the summed `catchment_km2` on this same struct.
    /// Order continuity must hold edge-by-edge (feature 03 §Q3: "order
    /// takes the max"), so re-deriving the floor from the total after
    /// summing would understate it whenever the edges' catchments straddle
    /// an `entering_order` step differently than their sum does.
    pub order: u8,
}

/// One area tile's inputs, computed from coarse data and the seed only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileBundle {
    /// Which tile this bundle feeds.
    pub area: AreaCoord,
    /// Heights along the tile's northern edge row, west to east.
    pub north: Vec<i32>,
    /// Heights along the row just past the tile's southern edge.
    pub south: Vec<i32>,
    /// Heights along the column just past the tile's eastern edge.
    pub east: Vec<i32>,
    /// Heights along the tile's western edge column, north to south.
    pub west: Vec<i32>,
    /// Heights one cell north of the tile, at local y=-1, west to east.
    /// Unlike `north`, these are actual outside neighbors for legacy routing.
    pub north_outside: Vec<i32>,
    /// Heights one cell west of the tile, at local x=-1, north to south.
    /// Unlike `west`, these are actual outside neighbors for legacy routing.
    pub west_outside: Vec<i32>,
    /// Outside diagonal heights in northwest, northeast, southwest, southeast
    /// order: local (-1,-1), (512,-1), (-1,512), (512,512).
    pub outside_corners: [i32; 4],
    /// Mean elevation across the sampled edges, for cheap summaries.
    pub mean_height_mm: i32,
    /// Watercourses crossing into the tile from the continent drainage
    /// tree (`logic/01` step 10).
    pub entering: Vec<EnteringRiver>,
    /// Rainfall patch: the tile's 52 km plus the +1 bilinear row/column,
    /// row-major ([`PATCH_KM`] per side).
    pub rainfall_km: Vec<u16>,
    /// Climate regime patch, same footprint as [`TileBundle::rainfall_km`].
    pub regime_km: Vec<ClimateRegime>,
    /// Routing-surface patch, same footprint as
    /// [`TileBundle::rainfall_km`].
    pub filled_km: Vec<i32>,
    /// Continent-tier lake-identity patch (feature 02 §Q1 deferred this;
    /// added to close open-items #12 exactly — see
    /// `continent::hydrology::ContinentHydrology::basin_surface`), same
    /// footprint as [`TileBundle::rainfall_km`], copied with the same
    /// clamped indexing as [`TileBundle::filled_km`].
    /// `continent::hydrology::NO_BASIN` marks a patch cell outside any
    /// continent depression.
    pub basin_km: Vec<i32>,
    /// Prevailing wind for this tile.
    pub wind: CompassOctant,
    /// Final advection moisture store along the tile's western patch
    /// edge, [`PATCH_KM`] long.
    pub west_moisture: Vec<u16>,
}

/// Builds one tile's bundle.
///
/// The south and east edges are sampled at local offset [`AREA_CELLS`] — the
/// neighbour's first row/column — so `south` and the neighbour's `north` name
/// the same absolute cells and are equal by construction.
#[must_use]
pub fn bundle_for(seed: u64, continent: &Continent, area: AreaCoord) -> TileBundle {
    let n = AREA_CELLS;

    let row = |local_y: u16| -> Vec<i32> {
        (0..n)
            .map(|local_x| {
                let (ax, ay) = abs_cell(area, local_x, local_y);
                boundary_height(seed, &continent.grid, ax, ay)
            })
            .collect()
    };
    let column = |local_x: u16| -> Vec<i32> {
        (0..n)
            .map(|local_y| {
                let (ax, ay) = abs_cell(area, local_x, local_y);
                boundary_height(seed, &continent.grid, ax, ay)
            })
            .collect()
    };

    let north = row(0);
    let south = row(n);
    let west = column(0);
    let east = column(n);

    // Logic/02 "Legacy outside-neighbor correction": legacy routing needs
    // actual adjacent cells. Preserve existing shared-edge identities.
    let (x0, y0) = abs_cell(area, 0, 0);
    let north_outside = (0..n)
        .map(|x| boundary_height(seed, &continent.grid, x0 + i32::from(x), y0 - 1))
        .collect();
    let west_outside = (0..n)
        .map(|y| boundary_height(seed, &continent.grid, x0 - 1, y0 + i32::from(y)))
        .collect();
    let outside_corners = [
        (-1, -1),
        (i32::from(n), -1),
        (-1, i32::from(n)),
        (i32::from(n), i32::from(n)),
    ]
    .map(|(x, y)| boundary_height(seed, &continent.grid, x0 + x, y0 + y));

    let sum: i64 = north
        .iter()
        .chain(south.iter())
        .chain(east.iter())
        .chain(west.iter())
        .map(|&h| i64::from(h))
        .sum();
    #[allow(clippy::cast_possible_truncation)]
    let mean_height_mm = (sum / (4 * i64::from(n))) as i32;

    // Patch side: the tile's 52 km plus the +1 bilinear row/column, rooted
    // at the km cell containing the tile's own origin.
    let km0x = (area.x * 512).div_euclid(10);
    let km0y = (area.y * 512).div_euclid(10);

    let mut rainfall_km = Vec::with_capacity(PATCH_KM * PATCH_KM);
    let mut regime_km = Vec::with_capacity(PATCH_KM * PATCH_KM);
    let mut filled_km = Vec::with_capacity(PATCH_KM * PATCH_KM);
    let mut basin_km = Vec::with_capacity(PATCH_KM * PATCH_KM);
    for py in 0..PATCH_KM {
        let py = i32::try_from(py).unwrap_or(0);
        for px in 0..PATCH_KM {
            let px = i32::try_from(px).unwrap_or(0);
            let i = patch_index(&continent.grid, km0x + px, km0y + py);
            rainfall_km.push(continent.climate.rainfall[i]);
            regime_km.push(continent.climate.regime[i]);
            filled_km.push(continent.hydrology.filled[i]);
            basin_km.push(continent.hydrology.basin_surface[i]);
        }
    }
    let west_moisture = (0..PATCH_KM)
        .map(|j| {
            let j = i32::try_from(j).unwrap_or(0);
            let i = patch_index(&continent.grid, km0x, km0y + j);
            continent.climate.moisture[i]
        })
        .collect();

    let entering = entering_rivers(seed, continent, area);

    TileBundle {
        area,
        north,
        south,
        east,
        west,
        north_outside,
        west_outside,
        outside_corners,
        mean_height_mm,
        entering,
        rainfall_km,
        regime_km,
        filled_km,
        basin_km,
        wind: CompassOctant::West,
        west_moisture,
    }
}

/// Row-major index into a continent-grid-shaped array, clamped exactly
/// like [`ContinentGrid::get`]: out-of-range coordinates clamp to the
/// edge, which is always ocean.
fn patch_index(grid: &ContinentGrid, x: i32, y: i32) -> usize {
    let cx = x.clamp(0, grid.width() - 1);
    let cy = y.clamp(0, grid.height() - 1);
    usize::try_from(cy * grid.width() + cx).unwrap_or(0)
}

/// Patch side: the tile's 52 km plus the +1 bilinear row/column.
pub const PATCH_KM: usize = 53;

/// Strahler floor for an entering river (feature 03 §Q3): a
/// deterministic function of catchment so both sides of a seam agree
/// by construction. Ratio-4 floor-log anchored at the 3 km² channel
/// scale; tunable, re-derived at step 12.
#[must_use]
pub fn entering_order(catchment_km2: u32) -> u8 {
    if catchment_km2 < 3 {
        return 1;
    }
    let r = catchment_km2 / 3;
    let o = 1 + r.ilog2() / 2;
    u8::try_from(o.min(12)).unwrap_or(12)
}

/// Watercourses crossing into the tile (logic/01 step 10, feature 03
/// spec R4). Tiles are 51.2 km, so 1 km cells straddle tile lines;
/// crossings are judged against the exact 100 m boundary lines. Cell
/// centers sit at k×10+5 (odd), lines at multiples of 512 (even), so
/// strict sidedness is total. A corner double-crossing resolves to the
/// horizontal line (§Q3 tie rule). Merging uses a BTreeMap — ordered,
/// no hash-iteration in a sim path.
fn entering_rivers(seed: u64, continent: &Continent, area: AreaCoord) -> Vec<EnteringRiver> {
    let (w, h) = (continent.grid.width(), continent.grid.height());
    let n = i64::from(AREA_CELLS);
    let (x0, y0) = (i64::from(area.x) * n, i64::from(area.y) * n);
    let (x1, y1) = (x0 + n, y0 + n);
    let mut seeds: std::collections::BTreeMap<CellCoord, (u64, u64, u8)> =
        std::collections::BTreeMap::new();

    for ky in 0..h {
        for kx in 0..w {
            let i = usize::try_from(ky * w + kx).unwrap_or(0);
            let c_km2 = continent.hydrology.catchment_km2[i];
            if c_km2 < 3 {
                continue; // §Q3: below the artifact's channel scale
            }
            let Some(d) = continent.hydrology.downstream[i] else {
                continue;
            };
            let di = i64::from(d);
            let (dkx, dky) = (di % i64::from(w), di / i64::from(w));
            let (ocx, ocy) = (i64::from(kx) * 10 + 5, i64::from(ky) * 10 + 5);
            let (dcx, dcy) = (dkx * 10 + 5, dky * 10 + 5);
            if !(dcx > x0 && dcx < x1 && dcy > y0 && dcy < y1) {
                continue; // downstream center not strictly inside
            }

            // Which line is crossed inward; horizontal wins a corner.
            // (edge kind, window start on the boundary axis, window
            //  base, fixed local coordinate on the other axis)
            let (win0, base, fixed_x, fixed_y) = if ocy < y0 && dcy > y0 {
                (dkx * 10, x0, None, Some(0i64)) // north line
            } else if ocy > y1 && dcy < y1 {
                (dkx * 10, x0, None, Some(n - 1)) // south line
            } else if ocx < x0 && dcx > x0 {
                (dky * 10, y0, Some(0i64), None) // west line
            } else if ocx > x1 && dcx < x1 {
                (dky * 10, y0, Some(n - 1), None) // east line
            } else {
                continue; // no boundary crossed: an interior edge
            };

            // Entry window: the downstream km cell's 10-cell span on
            // the crossed line, clipped to the tile.
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
                let hgt = boundary_height(seed, &continent.grid, ax, ay);
                if hgt <= 0 {
                    continue; // sea cell cannot seed
                }
                if best.is_none_or(|(bh, _)| hgt < bh) {
                    best = Some((hgt, j)); // ties keep the smaller j
                }
            }
            let Some((_, j)) = best else {
                continue; // all-sea window: the river is already at sea
            };
            let (lx, ly) = (fixed_x.unwrap_or(j), fixed_y.unwrap_or(j));
            let Some(cell) = CellCoord::new(
                u16::try_from(lx).unwrap_or(0),
                u16::try_from(ly).unwrap_or(0),
            ) else {
                continue;
            };
            let entry = seeds.entry(cell).or_insert((0, 0, 0));
            entry.0 += u64::from(c_km2);
            entry.1 += continent.hydrology.discharge_l_s[i];
            entry.2 = entry.2.max(entering_order(c_km2));
        }
    }

    seeds
        .into_iter()
        .map(|(cell, (c, q, order))| EnteringRiver {
            cell,
            catchment_km2: u32::try_from(c).unwrap_or(u32::MAX),
            discharge: DischargeMilli::new(q),
            order,
        })
        .collect()
}

#[cfg(test)]
mod tests {
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
        let ctx = fixture_ctx();
        let area = AreaCoord::new(0, 1);
        let bundle = bundle_for(42, &ctx, area);
        let merged = independent_seed_parts(42, &ctx, area)
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
            "no physical merged seed in MICRO seed 42 tile (0,1)"
        );
    }

    #[test]
    fn a_merged_seed_keeps_the_max_part_order_not_the_summed_order() {
        // A distinguishing natural merge catches recomputing order from the
        // summed catchment, even when both rules happen to agree elsewhere.
        let seed = 362;
        let ctx = crate::continent::build_continent(seed, GenerateConfig::MICRO, 0);
        let area = AreaCoord::new(0, 2);
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
}
