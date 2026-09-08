//! Unpersisted local diagnostic objects for the retained tile-only routing probes.
//!
//! These types have no global authority and cannot be encoded as format-4 area
//! objects. Published worlds use the shared solver and core AreaObjects instead.
use arda_core::{CellCoord, DischargeMilli, HeightMm, Terminus};

/// Local-only channel fragment used by old physical/routing regression probes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalRiverSegment {
    /// One-based local identifier.
    pub id: u32,
    /// Local Strahler order.
    pub order: u8,
    /// Local physical width in decimetres.
    pub width_dm: u32,
    /// Local diagnostic discharge.
    pub discharge: DischargeMilli,
    /// Local downstream segment, if any.
    pub feeds: Option<u32>,
    /// Local stopping condition.
    pub ends: Terminus,
    /// Ordered local channel cells.
    pub course: Vec<CellCoord>,
}

/// Local-only geometric depression diagnostic, without annual support or global identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalLake {
    /// One-based local identifier.
    pub id: u32,
    /// Local geometric surface in millimetres.
    pub surface: HeightMm,
    /// Local maximum depth in millimetres.
    pub depth_mm: u32,
    /// Local potential outward cell.
    pub outlet: Option<CellCoord>,
    /// Local geometric wet candidates.
    pub cells: Vec<CellCoord>,
}

/// Results of tile-only diagnostic composition; no persistence encoder accepts this type.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LocalAreaObjects {
    /// Local channel diagnostics.
    pub rivers: Vec<LocalRiverSegment>,
    /// Local depression diagnostics.
    pub lakes: Vec<LocalLake>,
}
