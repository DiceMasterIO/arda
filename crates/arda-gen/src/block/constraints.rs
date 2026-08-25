//! Cell-and-neighbours to WFC constraints (`logic/03`).

use arda_core::{tiles_in_group, AreaCells, CellCoord, TerrainKind, TileGroup, TileId, AREA_CELLS};

/// What a block may be built from, derived from its cell and the eight
/// neighbours around it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockConstraints {
    /// Tiles permitted in this block.
    pub allowed: Vec<TileId>,
    /// How wet the surroundings are, 0-255; drives the water-to-land mix.
    pub wet_fraction: u8,
}

/// Builds the constraint set for one cell.
#[must_use]
pub fn constraints_for(cells: &AreaCells, at: CellCoord) -> BlockConstraints {
    let here = cells.get(at);
    let n = i32::from(AREA_CELLS);
    let (cx, cy) = (i32::from(at.x()), i32::from(at.y()));

    let mut wet_neighbours = 0u32;
    let mut counted = 0u32;
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let (nx, ny) = (cx + dx, cy + dy);
            if nx < 0 || ny < 0 || nx >= n || ny >= n {
                continue;
            }
            let (Ok(unx), Ok(uny)) = (u16::try_from(nx), u16::try_from(ny)) else {
                continue;
            };
            let Some(nb) = CellCoord::new(unx, uny) else {
                continue;
            };
            counted += 1;
            if cells.get(nb).terrain != TerrainKind::Land {
                wet_neighbours += 1;
            }
        }
    }
    let wet_fraction = if counted == 0 {
        0u8
    } else {
        u8::try_from(wet_neighbours * 255 / counted).unwrap_or(255)
    };

    let groups: &[TileGroup] = match here.terrain {
        // Open water: water and shore only, so a sea block cannot grow trees.
        TerrainKind::Sea | TerrainKind::Lake => &[TileGroup::Water, TileGroup::Shore],
        TerrainKind::Land if here.watercourse_order > 0 => {
            &[TileGroup::Shore, TileGroup::Ground, TileGroup::Vegetation]
        }
        TerrainKind::Land if wet_fraction > 0 => &[
            TileGroup::Shore,
            TileGroup::Ground,
            TileGroup::Vegetation,
            TileGroup::Rock,
        ],
        TerrainKind::Land => &[
            TileGroup::Ground,
            TileGroup::Vegetation,
            TileGroup::Rock,
            TileGroup::Structure,
        ],
    };

    let mut allowed: Vec<TileId> = groups
        .iter()
        .flat_map(|&g| tiles_in_group(g))
        .map(|t| t.id)
        .collect();
    allowed.sort_unstable();
    allowed.dedup();

    BlockConstraints {
        allowed,
        wet_fraction,
    }
}
