//! Area object lists (`02-models.md` Entities).
//!
//! Shapes follow the artifact's Water section: every river segment knows
//! which segment it feeds and how it ends; every lake records a surface
//! level, a depth, and the cell where its water leaves.

use crate::coords::CellCoord;
use crate::fixed::{DischargeMilli, HeightMm};

/// How a river segment stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Terminus {
    /// Flows into another segment at a confluence.
    #[default]
    Junction = 0,
    /// Reaches the sea.
    Sea = 1,
    /// Enters a lake.
    Lake = 2,
    /// Leaves the tile.
    OffTile = 3,
}

impl Terminus {
    /// Decodes a stored discriminant.
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Junction),
            1 => Some(Self::Sea),
            2 => Some(Self::Lake),
            3 => Some(Self::OffTile),
            _ => None,
        }
    }
}

/// One watercourse reach: source or junction, downstream to the next
/// junction, the sea, a lake, or the tile edge (artifact, Water).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiverSegment {
    /// Tile-local identifier, 1-based.
    pub id: u16,
    /// Strahler order.
    pub order: u8,
    /// Channel width in decimetres, from `4 * sqrt(discharge)`.
    pub width_dm: u16,
    /// Mean discharge.
    pub discharge: DischargeMilli,
    /// Segment this one flows into; `None` at sea, lake, or tile edge.
    pub feeds: Option<u16>,
    /// How this segment ends.
    pub ends: Terminus,
    /// Cells the channel runs through, upstream to downstream.
    pub course: Vec<CellCoord>,
}

/// A body of inland standing water (artifact, Relief and Water).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lake {
    /// Tile-local identifier, 1-based.
    pub id: u16,
    /// Water-surface elevation, the basin's spill level.
    pub surface: HeightMm,
    /// Greatest depth: surface minus the lowest submerged cell, millimetres.
    pub depth_mm: u32,
    /// The cell where the lake's water leaves; `None` when it spills
    /// off-tile.
    pub outlet: Option<CellCoord>,
    /// Cells covered by the lake.
    pub cells: Vec<CellCoord>,
}

/// Every object stored alongside one area's cells.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AreaObjects {
    /// Watercourse reaches.
    pub rivers: Vec<RiverSegment>,
    /// Lakes.
    pub lakes: Vec<Lake>,
}

impl AreaObjects {
    /// An area with no objects.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}
