//! Stitches a stored world's area tiles into one [`Grid`].

use crate::error::SettleError;
use crate::grid::Grid;
use crate::num::{iu, ui};
use arda::World;
use arda_core::{CellCoord, AREA_CELLS};
use std::collections::BTreeMap;

/// Smallest Strahler order counted as a "sizeable" river at a confluence.
const CONFLUENCE_ORDER: u8 = 2;

/// Reads every area of `world` into a grid, within `budget` bytes.
///
/// Areas are read uncached (`World::read_area`) and dropped as soon as they
/// are copied, so peak memory is the grid plus one tile.
///
/// # Errors
/// Budget, allocation and stored-layer errors.
pub fn load_grid(world: &World, budget: u64) -> Result<Grid, SettleError> {
    let side = usize::from(AREA_CELLS);
    let wide = usize::try_from(world.manifest().areas_wide)
        .map_err(|_| SettleError::Dimensions("negative area count"))?;
    let high = usize::try_from(world.manifest().areas_high)
        .map_err(|_| SettleError::Dimensions("negative area count"))?;
    let mut grid = Grid::sea(wide * side, high * side, budget)?;
    let mut confluences = Vec::new();
    for (ax, ay) in world.area_coords() {
        let area = world.read_area(ax, ay)?;
        let ox = iu(i64::from(ax)) * side;
        let oy = iu(i64::from(ay)) * side;
        for cy in 0..AREA_CELLS {
            for cx in 0..AREA_CELLS {
                let Some(at) = CellCoord::new(cx, cy) else {
                    continue;
                };
                let c = area.cells().get(at);
                let i = (oy + usize::from(cy)) * grid.width + ox + usize::from(cx);
                grid.height_mm[i] = c.height.raw();
                grid.terrain[i] = c.terrain;
                grid.cover[i] = c.cover;
                grid.slope_md[i] = c.slope_milli_deg;
                grid.aspect_deg[i] = c.aspect_deg;
                grid.temp_cc[i] = c.temperature.raw();
                grid.rain_mm[i] = c.rainfall.raw();
                grid.moisture[i] = c.moisture;
                grid.forest[i] = c.forest_density;
                grid.drainage[i] = c.drainage_area_cells;
                grid.order[i] = c.watercourse_order;
                grid.width_dm[i] = c.watercourse_width_dm;
                grid.har_dm[i] = c.height_above_river_dm;
            }
        }
        confluences.extend(
            area_confluences(area.rivers())
                .into_iter()
                .map(|at| (oy + usize::from(at.y())) * grid.width + ox + usize::from(at.x())),
        );
    }
    confluences.sort_unstable();
    confluences.dedup();
    grid.confluences = confluences;
    Ok(grid)
}

/// Head cells of segments fed by two or more sizeable tributaries.
fn area_confluences(rivers: &[arda::RiverSegment]) -> Vec<CellCoord> {
    let mut feeders: BTreeMap<u32, u32> = BTreeMap::new();
    for s in rivers {
        if let Some(target) = s.feeds {
            if s.order >= CONFLUENCE_ORDER {
                *feeders.entry(target).or_default() += 1;
            }
        }
    }
    rivers
        .iter()
        .filter(|s| feeders.get(&s.id).copied().unwrap_or(0) >= 2)
        .filter_map(|s| s.course.first().copied())
        .collect()
}

/// Metres of the grid's west–east and north–south extent.
#[must_use]
pub fn extent_m(grid: &Grid) -> (i64, i64) {
    (ui(grid.width) * 100, ui(grid.height) * 100)
}
