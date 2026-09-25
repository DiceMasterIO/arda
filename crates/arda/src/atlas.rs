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
            let (bounds, window) =
                fine_window(reader, at).map_err(|source| arda_core::LoadError::Corrupt {
                    source: arda_core::FormatError::Terrain {
                        path: world
                            .dir()
                            .join(arda_core::FINE_TERRAIN_PATH)
                            .display()
                            .to_string(),
                        source,
                    },
                })?;
            Ok(AtlasTerrain::new_with_fine(
                cells, lakes, halo, at, bounds, window,
            )?)
        }
    }
}

// One source verification per export; one bounded owned window per rendered area.
pub(crate) const FINE_READER_BYTES: u64 = 1 << 20;
const MAX_WINDOW_SAMPLES: usize = 1325 * 1325;

fn fine_window<R: Read + Seek>(
    reader: &mut TerrainFileReader<R>,
    at: AreaCoord,
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
        let lo = (area_origin - 150_000_000 - step - start).div_euclid(step);
        let hi = (area_origin + 51_250_000_000 + step - start + step - 1).div_euclid(step);
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

#[cfg(test)]
mod tests;
