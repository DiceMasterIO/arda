//! Per-pixel overview sampling of one saved area: feature ownership over the
//! pixel's cell footprint and the Atlas ground beneath saved channels.

use super::{atlas_river_colour, atlas_river_size, Feature};
use crate::atlas::axis_kernel;
use crate::carto::{
    land_colour, river_band, river_band_colour, RiverBand, LAKE_FILL, LAKE_MIN_BLOCK_DEN,
    OVERVIEW_SEA,
};
use crate::{AtlasTerrain, RenderError};
use arda_core::{AreaCells, CellCoord, TerrainKind, AREA_CELLS};

pub(super) fn sample_atlas_channel_base_pixel(
    cells: &AreaCells,
    terrain: &AtlasTerrain,
    tile_width: u32,
    tile_height: u32,
    x: u32,
    y: u32,
) -> Result<([u8; 3], Feature), RenderError> {
    let (colour, feature) = sample_pixel(cells, Some(terrain), tile_width, tile_height, x, y)?;
    if matches!(feature, Feature::AtlasRiver(_, _)) {
        let ground = terrain.sample(
            axis_kernel(x, tile_width)?,
            axis_kernel(y, tile_height)?,
            TerrainKind::Land,
        )?;
        Ok((ground, Feature::AtlasLand))
    } else {
        Ok((colour, feature))
    }
}

pub(super) fn sample_pixel(
    cells: &AreaCells,
    terrain: Option<&AtlasTerrain>,
    tile_width: u32,
    tile_height: u32,
    x: u32,
    y: u32,
) -> Result<([u8; 3], Feature), RenderError> {
    // Half-open bounds retain the rightmost/bottom cells even when 512 is
    // not divisible by the output tile size. These bounds own feature masks.
    let side = u32::from(AREA_CELLS);
    let x0 = x * side / tile_width;
    let x1 = ((x + 1) * side / tile_width).max(x0 + 1);
    let y0 = y * side / tile_height;
    let y1 = ((y + 1) * side / tile_height).max(y0 + 1);
    // Lake coverage, rather than feature precedence, determines lake fill.
    let mut best = Feature::Sea;
    let mut best_width_dm = 0u32;
    let mut best_discharge = arda_core::DischargeMilli::new(0);
    let mut height_sum: i64 = 0;
    let mut land_count: i64 = 0;
    let mut lake_cells: u32 = 0;
    let mut block_cells: u32 = 0;
    for sy in y0..y1 {
        for sx in x0..x1 {
            let (Ok(sxu), Ok(syu)) = (u16::try_from(sx), u16::try_from(sy)) else {
                continue;
            };
            let Some(at) = CellCoord::new(sxu, syu) else {
                continue;
            };
            let cell = cells.get(at);
            block_cells += 1;
            match cell.terrain {
                TerrainKind::Lake => lake_cells += 1,
                TerrainKind::Sea => {}
                TerrainKind::Land => {
                    height_sum += i64::from(cell.height.raw());
                    land_count += 1;
                    let feature = match river_band(cell.discharge.raw()) {
                        Some(band) => Feature::River(band),
                        None => Feature::Land,
                    };
                    if feature > best {
                        best = feature;
                        best_width_dm = cell.watercourse_width_dm;
                        best_discharge = cell.discharge;
                    } else if feature == best {
                        best_width_dm = best_width_dm.max(cell.watercourse_width_dm);
                        best_discharge = best_discharge.max(cell.discharge);
                    }
                }
            }
        }
    }
    if lake_cells * LAKE_MIN_BLOCK_DEN >= block_cells {
        best = Feature::Lake;
    }
    // Saved worlds carry width. The fallback serves synthetic Atlas callers
    // whose channel cells set discharge but leave width at zero. Overflow in
    // physical width means the largest display symbol, not the smallest.
    if terrain.is_some() && matches!(best, Feature::River(_)) && best_width_dm == 0 {
        best_width_dm = arda_core::hydrology::channel_width_dm(best_discharge).unwrap_or(u32::MAX);
    }
    if let Some(terrain) = terrain {
        if best != Feature::Lake {
            let x_kernel = axis_kernel(x, tile_width)?;
            let y_kernel = axis_kernel(y, tile_height)?;
            if matches!(
                (x_kernel, y_kernel),
                (
                    crate::atlas::AxisKernel::Linear { .. },
                    crate::atlas::AxisKernel::Linear { .. }
                )
            ) {
                let owner = CellCoord::new(
                    u16::try_from(x0).map_err(|_| RenderError::ExactOverviewDimensions)?,
                    u16::try_from(y0).map_err(|_| RenderError::ExactOverviewDimensions)?,
                )
                .ok_or(RenderError::ExactOverviewDimensions)?;
                let saved = cells.get(owner).terrain;
                best = match terrain.contour_class(x_kernel, y_kernel, owner, saved)? {
                    TerrainKind::Sea => Feature::AtlasSea,
                    TerrainKind::Land => match best {
                        Feature::River(band) => {
                            Feature::AtlasRiver(band, atlas_river_size(best_width_dm, band))
                        }
                        _ => Feature::AtlasLand,
                    },
                    TerrainKind::Lake => Feature::Lake,
                };
            } else {
                best = match best {
                    Feature::Sea => Feature::AtlasSea,
                    Feature::Land => Feature::AtlasLand,
                    Feature::River(band) => {
                        Feature::AtlasRiver(band, atlas_river_size(best_width_dm, band))
                    }
                    other => other,
                };
            }
        }
    }
    let colour = match (best, terrain) {
        (Feature::Sea | Feature::AtlasSea, Some(terrain)) => terrain.sample(
            axis_kernel(x, tile_width)?,
            axis_kernel(y, tile_height)?,
            TerrainKind::Sea,
        )?,
        (Feature::Land | Feature::AtlasLand, Some(terrain)) => terrain.sample(
            axis_kernel(x, tile_width)?,
            axis_kernel(y, tile_height)?,
            TerrainKind::Land,
        )?,
        (Feature::Sea | Feature::AtlasSea, None) => OVERVIEW_SEA,
        // Include channel cells in the land mean to preserve the ground tint.
        (Feature::Land | Feature::AtlasLand, None) => {
            land_colour(i32::try_from(height_sum / land_count.max(1)).unwrap_or(0))
        }
        (Feature::River(band), _) => river_band_colour(band),
        (Feature::AtlasRiver(RiverBand::Light, _), _) => atlas_river_colour(RiverBand::Light),
        (Feature::AtlasRiver(_, _), Some(terrain)) => terrain.sample(
            axis_kernel(x, tile_width)?,
            axis_kernel(y, tile_height)?,
            TerrainKind::Land,
        )?,
        (Feature::AtlasRiver(_, _), None) => {
            land_colour(i32::try_from(height_sum / land_count.max(1)).unwrap_or(0))
        }
        (Feature::Lake, Some(terrain)) if terrain.has_lake_depths() => terrain.sample(
            axis_kernel(x, tile_width)?,
            axis_kernel(y, tile_height)?,
            TerrainKind::Lake,
        )?,
        (Feature::Lake, _) => LAKE_FILL,
    };
    Ok((colour, best))
}
