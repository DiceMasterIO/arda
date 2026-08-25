//! Area object lists (`02-models.md` Entities).
//!
//! The skeleton carries rivers and lakes; settlements, roads, crossings, and
//! passes join at build-order step 5 as additional sections.

use crate::coords::CellCoord;
use crate::fixed::{DischargeMilli, HeightMm};

/// One watercourse reach inside an area tile (`logic/02` water).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiverSegment {
    /// Tile-local identifier, 1-based.
    pub id: u16,
    /// Strahler order.
    pub order: u8,
    /// Channel width in decimetres.
    pub width_dm: u16,
    /// Mean discharge.
    pub discharge: DischargeMilli,
    /// Cells the channel runs through, upstream to downstream.
    pub course: Vec<CellCoord>,
}

/// A body of inland standing water.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lake {
    /// Tile-local identifier, 1-based.
    pub id: u16,
    /// Water-surface elevation.
    pub surface: HeightMm,
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
