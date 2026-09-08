//! Area object lists (`02-models.md` Entities).
//!
//! Shapes follow the artifact's Water section: every river segment knows
//! which segment it feeds and how it ends; every lake records a surface
//! level, a depth, and the cell where its water leaves.

use crate::coords::CellCoord;
use crate::fixed::{DischargeMilli, HeightMm};
use crate::hydrology::{AreaHydrologyContext, BasinId, ChannelEdge, ReachId};

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
    /// Enters a physical basin with no positive-depth representative lake.
    Basin = 4,
    /// Splits into multiple actual outgoing reaches at a named physical junction.
    Divergence = 5,
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
            4 => Some(Self::Basin),
            5 => Some(Self::Divergence),
            _ => None,
        }
    }
}

/// One watercourse reach: source or junction, downstream to the next
/// junction, the sea, a lake, or the tile edge (artifact, Water).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiverSegment {
    /// Stable identity of the global reach containing this fragment.
    pub global_id: ReachId,
    /// Tile-local identifier, 1-based.
    pub id: u32,
    /// Strahler order.
    pub order: u8,
    /// Channel width in decimetres, from `4 * sqrt(discharge)`.
    pub width_dm: u32,
    /// Mean discharge.
    pub discharge: DischargeMilli,
    /// Single local downstream segment at a confluence; None for every other terminus, including divergence.
    pub feeds: Option<u32>,
    /// How this segment ends.
    pub ends: Terminus,
    /// Cells the channel runs through, upstream to downstream.
    pub course: Vec<CellCoord>,
}

/// A body of inland standing water (artifact, Relief and Water).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lake {
    /// Stable identity of the connected lake containing this fragment.
    pub global_id: BasinId,
    /// Tile-local identifier, 1-based.
    pub id: u32,
    /// Representative annual water surface in whole millimetres.
    /// The matching global lake record is the level authority.
    pub surface: HeightMm,
    /// Greatest local positive depth in whole millimetres.
    pub depth_mm: u32,
    /// Local source cell of supported annual surface outflow.
    /// `None` when annual outflow is zero or the shared outlet belongs to another area.
    /// The global record separately preserves the potential spill.
    pub outlet: Option<CellCoord>,
    /// Cells covered by the lake.
    pub cells: Vec<CellCoord>,
}

/// Every object stored alongside one area's cells.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AreaObjects {
    /// Saved physical channel geometry, including the required neighboring halo.
    pub channel_edges: Vec<ChannelEdge>,
    /// Bounded copies of the global records needed to interpret this area.
    pub global: AreaHydrologyContext,
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
