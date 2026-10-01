//! The saved-neighbour halo around one Atlas area: the eight adjacent areas'
//! border samples, or an explicit world edge, gathered before shading.

use super::{context_index, SourceSample, CONTEXT_SIDE};
use crate::atlas::invalid;
use crate::channels::validated_lake_depths;
use crate::RenderError;
use arda_core::{AreaCells, CellCoord, Lake};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NeighborState {
    Unset,
    Copied,
    WorldEdge,
}

/// One of the eight saved areas adjacent to the target area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtlasNeighbor {
    /// Area directly north.
    North,
    /// Area diagonally northeast.
    NorthEast,
    /// Area directly east.
    East,
    /// Area diagonally southeast.
    SouthEast,
    /// Area directly south.
    South,
    /// Area diagonally southwest.
    SouthWest,
    /// Area directly west.
    West,
    /// Area diagonally northwest.
    NorthWest,
}

impl AtlasNeighbor {
    pub(super) const fn index(self) -> usize {
        self as usize
    }
}

/// Nearest two saved-cell rows or columns from each adjacent area.
///
/// The context uses `-2..=513` local coordinates. A direction must be supplied
/// exactly once, either by copying its saved area or marking a world boundary.
#[derive(Debug)]
pub struct AtlasHalo {
    pub(super) context: Vec<Option<SourceSample>>,
    pub(super) states: [NeighborState; 8],
}

impl AtlasHalo {
    /// Creates an empty halo whose eight directions must be resolved before use.
    #[must_use]
    pub fn new() -> Self {
        Self {
            context: vec![None; CONTEXT_SIDE * CONTEXT_SIDE],
            states: [NeighborState::Unset; 8],
        }
    }

    /// Copies the nearest two saved-cell rows, columns, or corner from `cells`.
    ///
    /// `cells` must be exactly the named adjacent saved area; the caller must
    /// establish that identity from the world manifest.
    ///
    /// # Errors
    /// Returns [`RenderError::AtlasContext`] if this direction was already set.
    pub fn copy_neighbor(
        &mut self,
        direction: AtlasNeighbor,
        cells: &AreaCells,
    ) -> Result<(), RenderError> {
        self.copy_neighbor_with_lakes(direction, cells, &[])
    }

    /// Copies saved neighbor samples with validated lake depths.
    ///
    /// # Errors
    /// Returns invalid lake geometry or an already resolved direction.
    pub fn copy_neighbor_with_lakes(
        &mut self,
        direction: AtlasNeighbor,
        cells: &AreaCells,
        lakes: &[Lake],
    ) -> Result<(), RenderError> {
        if self.states[direction.index()] != NeighborState::Unset {
            return Err(invalid("neighbor direction already resolved"));
        }
        let (target_x, target_y, source_x, source_y, width, height) = match direction {
            AtlasNeighbor::North => (0, -2, 0, 510, 512, 2),
            AtlasNeighbor::NorthEast => (512, -2, 0, 510, 2, 2),
            AtlasNeighbor::East => (512, 0, 0, 0, 2, 512),
            AtlasNeighbor::SouthEast => (512, 512, 0, 0, 2, 2),
            AtlasNeighbor::South => (0, 512, 0, 0, 512, 2),
            AtlasNeighbor::SouthWest => (-2, 512, 510, 0, 2, 2),
            AtlasNeighbor::West => (-2, 0, 510, 0, 2, 512),
            AtlasNeighbor::NorthWest => (-2, -2, 510, 510, 2, 2),
        };
        let lake_depths = validated_lake_depths(cells, lakes)?;
        for dy in 0..height {
            for dx in 0..width {
                let source = CellCoord::new(source_x + dx, source_y + dy)
                    .ok_or_else(|| invalid("neighbor source coordinate outside area"))?;
                let target = context_index(
                    target_x
                        + i16::try_from(dx).map_err(|_| invalid("neighbor column conversion"))?,
                    target_y + i16::try_from(dy).map_err(|_| invalid("neighbor row conversion"))?,
                )?;
                let cell = cells.get(source);
                self.context[target] = Some(SourceSample {
                    height_mm: cell.height.raw(),
                    class: cell.terrain,
                    wetness: cell.wetness,
                    moisture: cell.moisture,
                    forest_density: cell.forest_density,
                    temperature_centi: cell.temperature.raw(),
                    channel_width_dm: cell.watercourse_width_dm,
                    lake_depth_mm: lake_depths
                        .get(usize::from(source.y()) * 512 + usize::from(source.x()))
                        .copied()
                        .flatten(),
                });
            }
        }
        self.states[direction.index()] = NeighborState::Copied;
        Ok(())
    }

    /// Marks a direction absent beyond the saved world's manifest bounds.
    ///
    /// This is valid only when manifest bounds prove that the named adjacent
    /// area does not exist. A missing or corrupt in-world area is an error.
    ///
    /// # Errors
    /// Returns [`RenderError::AtlasContext`] if this direction was already set.
    pub fn mark_world_edge(&mut self, direction: AtlasNeighbor) -> Result<(), RenderError> {
        if self.states[direction.index()] != NeighborState::Unset {
            return Err(invalid("neighbor direction already resolved"));
        }
        self.states[direction.index()] = NeighborState::WorldEdge;
        Ok(())
    }
}

impl Default for AtlasHalo {
    fn default() -> Self {
        Self::new()
    }
}
