//! Bounded saved-neighbor loading for Atlas presentation.

use crate::{ExportError, World};
use arda_core::{AreaCells, Lake};
use arda_render::{AtlasHalo, AtlasNeighbor, AtlasTerrain};

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
    Ok(AtlasTerrain::new_with_lakes(cells, lakes, halo)?)
}
