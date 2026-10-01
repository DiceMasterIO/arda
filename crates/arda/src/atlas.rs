//! Bounded saved-neighbor loading for Atlas presentation.

use crate::{ExportError, World};
use arda_core::{AreaCells, AreaCoord, Lake, TerrainField, TerrainFileReader, TerrainPoint};
use arda_render::{AtlasFineWorldBounds, AtlasHalo, AtlasNeighbor, AtlasTerrain};
use std::{
    fs::File,
    io::{Read, Seek},
};

const DIRECTIONS: [AtlasNeighbor; 8] = [
    AtlasNeighbor::North,
    AtlasNeighbor::NorthEast,
    AtlasNeighbor::East,
    AtlasNeighbor::SouthEast,
    AtlasNeighbor::South,
    AtlasNeighbor::SouthWest,
    AtlasNeighbor::West,
    AtlasNeighbor::NorthWest,
];

fn offset(direction: &AtlasNeighbor) -> (i32, i32) {
    match direction {
        AtlasNeighbor::North => (0, -1),
        AtlasNeighbor::NorthEast => (1, -1),
        AtlasNeighbor::East => (1, 0),
        AtlasNeighbor::SouthEast => (1, 1),
        AtlasNeighbor::South => (0, 1),
        AtlasNeighbor::SouthWest => (-1, 1),
        AtlasNeighbor::West => (-1, 0),
        AtlasNeighbor::NorthWest => (-1, -1),
    }
}

/// Recipe-7 arid water codes (`arda_render::SALT_*`) of every coded cell of
/// the area at `(ax, ay)` and its eight neighbours, by global cell: saline
/// lake cells from the stored lake forms, salt crust and mudflat from the
/// stored playa runs (logic/04 §atlas-formed arid basins).
pub(crate) fn arid_cells(
    world: &World,
    ax: i32,
    ay: i32,
) -> Result<std::collections::HashMap<(i64, i64), u8>, ExportError> {
    let mut out = std::collections::HashMap::new();
    for y in ay - 1..=ay + 1 {
        for x in ax - 1..=ax + 1 {
            if !inside(world, x, y) {
                continue;
            }
            let objects = world.read_area_objects(x, y)?;
            let Some(water) = world.read_area_water(x, y)? else {
                continue;
            };
            let (ox, oy) = (i64::from(x) * 512, i64::from(y) * 512);
            for (lake, form) in objects.lakes.iter().zip(&water.lakes) {
                if form.saline {
                    for c in &lake.cells {
                        out.insert(
                            (ox + i64::from(c.x()), oy + i64::from(c.y())),
                            arda_render::SALT_SALINE_LAKE,
                        );
                    }
                }
            }
            for run in &water.pans {
                let code = match run.kind {
                    arda_core::water::PanKind::SaltCrust => arda_render::SALT_CRUST,
                    arda_core::water::PanKind::Mudflat => arda_render::SALT_MUDFLAT,
                };
                for dx in 0..run.len {
                    out.insert((ox + i64::from(run.x0 + dx), oy + i64::from(run.y)), code);
                }
            }
        }
    }
    Ok(out)
}

fn inside(world: &World, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && x < world.manifest().areas_wide && y < world.manifest().areas_high
}

pub(crate) fn atlas_terrain(
    world: &World,
    ax: i32,
    ay: i32,
    cells: &AreaCells,
    lakes: &[Lake],
    fine: Option<&mut TerrainFileReader<File>>,
    shore: Option<&arda_core::ShoreLayer>,
) -> Result<AtlasTerrain, ExportError> {
    let mut halo = AtlasHalo::new();
    for direction in DIRECTIONS {
        let (dx, dy) = offset(&direction);
        // World::read_area accepted the target and manifests contain at most 78 areas per axis.
        let nx = ax + dx;
        let ny = ay + dy;
        if inside(world, nx, ny) {
            let neighbor = world.read_area(nx, ny)?;
            halo.copy_neighbor_with_lakes(direction, neighbor.cells(), neighbor.lakes())?;
            drop(neighbor);
        } else {
            halo.mark_world_edge(direction)?;
        }
    }
    match fine {
        None => Ok(AtlasTerrain::new_with_lakes(cells, lakes, halo)?),
        Some(reader) => {
            let at = AreaCoord::new(ax, ay);
            let source_error = |source| arda_core::LoadError::Corrupt {
                source: arda_core::FormatError::Terrain {
                    path: world
                        .dir()
                        .join(arda_core::FINE_TERRAIN_PATH)
                        .display()
                        .to_string(),
                    source,
                },
            };
            let recipe = world
                .manifest()
                .fine_terrain
                .map_or(0, |descriptor| descriptor.recipe_version);
            let formed = recipe >= 5;
            let halo_um = if formed { FORMED_HALO_UM } else { 150_000_000 };
            let (bounds, window) = fine_window(reader, at, halo_um).map_err(&source_error)?;
            if formed {
                let sky = sky_context(reader, at).map_err(&source_error)?;
                // logic/04 §atlas-formed recipes: the world's recipe picks
                // the formed look, so recipe-5 worlds render as v0.1 drew them.
                let terrain = AtlasTerrain::new_with_fine_formed(
                    cells, lakes, halo, at, bounds, window, sky,
                )?
                .with_recipe(recipe);
                // logic/04 §atlas-formed shore: stored classes when present.
                let terrain = match shore {
                    Some(layer) => terrain.with_shore(layer, at),
                    None => terrain,
                };
                // logic/04 §atlas-formed arid basins (recipe 7): saline
                // lakes and salt pans.
                let terrain = if recipe >= 7 {
                    let cells = arid_cells(world, ax, ay)?;
                    terrain.with_salt(at, |gx, gy| {
                        cells
                            .get(&(gx, gy))
                            .copied()
                            .unwrap_or(arda_render::SALT_NONE)
                    })
                } else {
                    terrain
                };
                // Worlds with stored water forms carry their meanders in the
                // saved courses (logic/02 §world-water).
                let forms = world
                    .dir()
                    .join("areas")
                    .join(at.dir_name())
                    .join("water.bin")
                    .exists();
                Ok(if forms {
                    terrain.with_data_meanders()
                } else {
                    terrain
                })
            } else if world
                .manifest()
                .fine_terrain
                .is_some_and(|descriptor| descriptor.recipe_version >= 4)
            {
                let sky = sky_context(reader, at).map_err(&source_error)?;
                Ok(AtlasTerrain::new_with_fine_sky(
                    cells, lakes, halo, at, bounds, window, sky,
                )?)
            } else {
                Ok(AtlasTerrain::new_with_fine(
                    cells, lakes, halo, at, bounds, window,
                )?)
            }
        }
    }
}

// One source verification per export; one bounded owned window per rendered area.
pub(crate) const FINE_READER_BYTES: u64 = 1 << 20;
const MAX_WINDOW_SAMPLES: usize = 1400 * 1400;
/// Recipe-5 halo: 312.5 m mid-scale gradient plus pixel footprint and slack.
const FORMED_HALO_UM: i64 = 450_000_000;

fn fine_window<R: Read + Seek>(
    reader: &mut TerrainFileReader<R>,
    at: AreaCoord,
    halo_um: i64,
) -> Result<(AtlasFineWorldBounds, TerrainField), arda_core::TerrainFileError> {
    use arda_core::TerrainFileError;
    let invalid = || TerrainFileError::InvalidHeader("fine Atlas window geometry or memory limit");
    let step = i64::from(reader.spacing_um());
    if step != 39_062_500 || at.x < 0 || at.y < 0 {
        return Err(invalid());
    }
    let origin = reader.origin();
    let last = |start: i64, count: u32| {
        i64::try_from(i128::from(start) + i128::from(count - 1) * i128::from(step))
            .map_err(|_| invalid())
    };
    let bounds = AtlasFineWorldBounds {
        min: origin,
        max: TerrainPoint {
            x_um: last(origin.x_um, reader.width())?,
            y_um: last(origin.y_um, reader.height())?,
        },
    };
    let axis = |area: i32, start: i64, count: u32| {
        let area_origin = i64::from(area) * 51_200_000_000;
        // Include saved-neighbor support, the pixel footprint and one fine derivative step.
        let lo = (area_origin - halo_um - step - start).div_euclid(step);
        let hi =
            (area_origin + 51_100_000_000 + halo_um + step - start + step - 1).div_euclid(step);
        let lo = lo.max(0);
        let hi = hi.min(i64::from(count - 1));
        if hi <= lo {
            return Err(invalid());
        }
        Ok((lo, hi))
    };
    let (x0, x1) = axis(at.x, origin.x_um, reader.width())?;
    let (y0, y1) = axis(at.y, origin.y_um, reader.height())?;
    let width = u32::try_from(x1 - x0 + 1).map_err(|_| invalid())?;
    let height = u32::try_from(y1 - y0 + 1).map_err(|_| invalid())?;
    let count = usize::try_from(u64::from(width) * u64::from(height)).map_err(|_| invalid())?;
    if count > MAX_WINDOW_SAMPLES {
        return Err(invalid());
    }
    let mut heights = Vec::new();
    heights.try_reserve_exact(count).map_err(|_| invalid())?;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let point = TerrainPoint {
                x_um: origin.x_um + x * step,
                y_um: origin.y_um + y * step,
            };
            heights.push(reader.sample(point)?.ok_or_else(invalid)?);
        }
    }
    let window = TerrainField::new(
        TerrainPoint {
            x_um: origin.x_um + x0 * step,
            y_um: origin.y_um + y0 * step,
        },
        reader.spacing_um(),
        width,
        height,
        heights,
    )
    .map_err(|_| invalid())?;
    Ok((bounds, window))
}

/// A small globally aligned 1 km context for the recipe-4 sky-exposure term.
/// Only samples outside the canonical world's physical bounds are clamped;
/// area borders always read their true neighboring source elevations.
fn sky_context<R: Read + Seek>(
    reader: &mut TerrainFileReader<R>,
    at: AreaCoord,
) -> Result<TerrainField, arda_core::TerrainFileError> {
    use arda_core::TerrainFileError;
    const KM_UM: i128 = 1_000_000_000;
    const AREA_UM: i128 = 51_200_000_000;
    const HALF_CELL_UM: i128 = 50_000_000;
    const RADIUS_KM: i128 = 8;
    let invalid = || TerrainFileError::InvalidHeader("fine sky context geometry or memory limit");
    if reader.spacing_um() != 39_062_500 || at.x < 0 || at.y < 0 {
        return Err(invalid());
    }
    let axis = |area: i32| {
        let start = i128::from(area) * AREA_UM - HALF_CELL_UM;
        let end = i128::from(area) * AREA_UM + AREA_UM - HALF_CELL_UM;
        let low = start.div_euclid(KM_UM) - RADIUS_KM;
        let high = -(-end).div_euclid(KM_UM) + RADIUS_KM;
        if high - low + 1 > 72 {
            return Err(invalid());
        }
        Ok((low, high))
    };
    let (x0, x1) = axis(at.x)?;
    let (y0, y1) = axis(at.y)?;
    let width = u32::try_from(x1 - x0 + 1).map_err(|_| invalid())?;
    let height = u32::try_from(y1 - y0 + 1).map_err(|_| invalid())?;
    let mut heights = Vec::new();
    heights
        .try_reserve_exact(
            usize::try_from(u64::from(width) * u64::from(height)).map_err(|_| invalid())?,
        )
        .map_err(|_| invalid())?;
    let first = reader.origin();
    let last_x =
        i128::from(first.x_um) + i128::from(reader.width() - 1) * i128::from(reader.spacing_um());
    let last_y =
        i128::from(first.y_um) + i128::from(reader.height() - 1) * i128::from(reader.spacing_um());
    for y in y0..=y1 {
        for x in x0..=x1 {
            let point = TerrainPoint {
                x_um: i64::try_from((x * KM_UM).clamp(i128::from(first.x_um), last_x))
                    .map_err(|_| invalid())?,
                y_um: i64::try_from((y * KM_UM).clamp(i128::from(first.y_um), last_y))
                    .map_err(|_| invalid())?,
            };
            heights.push(reader.sample(point)?.ok_or_else(invalid)?);
        }
    }
    TerrainField::new(
        TerrainPoint {
            x_um: i64::try_from(x0 * KM_UM).map_err(|_| invalid())?,
            y_um: i64::try_from(y0 * KM_UM).map_err(|_| invalid())?,
        },
        u32::try_from(KM_UM).map_err(|_| invalid())?,
        width,
        height,
        heights,
    )
    .map_err(|_| invalid())
}

#[cfg(test)]
mod tests;

/// The Atlas shading context of one saved area, as the overview builds it:
/// saved cells with their eight-neighbour halo, the fine window and, for
/// recipe-5 worlds, the 1 km context and the stored shore classes
/// (`shore`, from [`World::shore`], loaded once by the caller). Close-zoom
/// relief tiles (arda-midzoom) shade through it so their look matches the
/// overview.
///
/// # Errors
/// The area is outside the world, or a stored layer is unreadable.
pub fn area_atlas_terrain(
    world: &World,
    ax: i32,
    ay: i32,
    shore: Option<&arda_core::ShoreLayer>,
) -> Result<AtlasTerrain, ExportError> {
    let area = world.read_area(ax, ay)?;
    let mut fine = world.fine_terrain(FINE_READER_BYTES)?;
    atlas_terrain(
        world,
        ax,
        ay,
        area.cells(),
        area.lakes(),
        fine.as_mut(),
        shore,
    )
}
