//! Per-tile input bundles (`logic/01` step 10).
//!
//! A bundle is a pure function of coarse continent data plus the seed — never
//! of any area's fine output. Adjacent tiles are therefore mirror-consistent
//! by construction, and areas may generate in any order (`logic/01`
//! invariant).

use super::{Continent, ContinentGrid};
use crate::noise::fbm;
use arda_core::{AreaCoord, AREA_CELLS};

/// Area cells per continent kilometre: cells are 100 m, so ten.
const CELLS_PER_KM: i32 = 10;

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

/// Terrain elevation at an absolute cell, in millimetres.
///
/// This is the single source of area relief. Because it takes absolute
/// coordinates, two tiles sharing an edge call it with identical arguments
/// and get identical answers — the pinned-edge guarantee
/// (`implementation.md` "Pinned edges", `logic/02` amendment 3).
#[must_use]
pub fn boundary_height(seed: u64, continent: &ContinentGrid, abs_x: i32, abs_y: i32) -> i32 {
    let coarse = coarse_height(continent, abs_x, abs_y);

    // Area-scale detail, keyed by absolute position so it is edge-safe.
    // Amplitude scales with elevation, so detail never manufactures coastline
    // the continent stage did not put there (§Q8: area detail refines coarse
    // features but never relocates them).
    let amplitude = if coarse > 0 {
        (coarse / 12).clamp(2_000, 90_000)
    } else {
        1_500
    };
    let detail = i64::from(fbm(seed ^ 0x00A1_2EA5, abs_x, abs_y, 40, 5));

    #[allow(clippy::cast_possible_truncation)]
    {
        (i64::from(coarse) + ((detail * i64::from(amplitude)) >> 15)) as i32
    }
}

/// The smooth regional surface under a cell: a bilinear sample of the 1 km
/// continent grid, with no area-scale detail added.
///
/// This is the uplift pattern. The artifact raises land "fastest near the
/// mountain edge, slowly in the lowland" — a regional trend. Driving uplift
/// from the noise-refined relief instead amplifies every per-cell bump for
/// the whole run, and the bumps grow into dams that close off basins.
#[must_use]
pub fn coarse_height(continent: &ContinentGrid, abs_x: i32, abs_y: i32) -> i32 {
    // Bilinear sample of the 1 km continent grid at 100 m resolution.
    let km_x = abs_x.div_euclid(CELLS_PER_KM);
    let km_y = abs_y.div_euclid(CELLS_PER_KM);
    // Smoothstep weights, not linear.
    //
    // Straight bilinear leaves a crease along every 1 km cell edge, and those
    // creases are axis-aligned, so steepest descent follows them: the
    // diagonal share of flow directions fell to 14% and rivers were drawn as
    // straight combs. Smoothstep makes the interpolated surface C1 across
    // cell boundaries, so the gradient direction is free to point anywhere.
    let smooth = |v: i32| -> i64 {
        let t = i64::from(v) * 65536 / i64::from(CELLS_PER_KM);
        let t2 = (t * t) >> 16;
        let t3 = (t2 * t) >> 16;
        (3 * t2 - 2 * t3).clamp(0, 65536)
    };
    let fx = smooth(abs_x.rem_euclid(CELLS_PER_KM));
    let fy = smooth(abs_y.rem_euclid(CELLS_PER_KM));

    let at = |x: i32, y: i32| i64::from(continent.get(x, y).raw());
    let top = at(km_x, km_y) + (((at(km_x + 1, km_y) - at(km_x, km_y)) * fx) >> 16);
    let bottom = at(km_x, km_y + 1) + (((at(km_x + 1, km_y + 1) - at(km_x, km_y + 1)) * fx) >> 16);

    #[allow(clippy::cast_possible_truncation)]
    {
        (top + (((bottom - top) * fy) >> 16)) as i32
    }
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
    /// Mean elevation across the sampled edges, for cheap summaries.
    pub mean_height_mm: i32,
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

    let sum: i64 = north
        .iter()
        .chain(south.iter())
        .chain(east.iter())
        .chain(west.iter())
        .map(|&h| i64::from(h))
        .sum();
    #[allow(clippy::cast_possible_truncation)]
    let mean_height_mm = (sum / (4 * i64::from(n))) as i32;

    TileBundle {
        area,
        north,
        south,
        east,
        west,
        mean_height_mm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::GenerateConfig;

    fn fixture() -> Continent {
        crate::continent::build_continent(42, GenerateConfig::MICRO, 0)
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
}
