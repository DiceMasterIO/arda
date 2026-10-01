//! Dry floors of arid endorheic basins (recipe 7, logic/02 §world-water
//! arid basins): the playa runs stored in `water.bin` layout version 2.

use crate::water::AreaWater;

/// Surface of a dry arid-basin floor (recipe 7, logic/02 §world-water arid
/// basins): the playa around a terminal lake, or the whole floor of a
/// basin whose lake has dried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
#[repr(u8)]
pub enum PanKind {
    /// White evaporite crust on the low centre of the playa.
    #[default]
    SaltCrust = 0,
    /// Pale clay and silt flats on the playa margin.
    Mudflat = 1,
}

impl PanKind {
    /// Decodes a stored discriminant.
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::SaltCrust),
            1 => Some(Self::Mudflat),
            _ => None,
        }
    }
}

/// A run of playa cells along one area row: cells `x0..x0 + len` of row `y`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PanRun {
    /// Row.
    pub y: u16,
    /// First column.
    pub x0: u16,
    /// Number of cells.
    pub len: u16,
    /// Surface.
    pub kind: PanKind,
}

impl AreaWater {
    /// The playa surface of local cell `(x, y)`, if any. Lake cells hold
    /// water, not a pan, whatever lies beneath.
    #[must_use]
    pub fn pan_at(&self, x: u16, y: u16) -> Option<PanKind> {
        let i = self.pans.partition_point(|r| (r.y, r.x0) <= (y, x));
        let r = self.pans[..i].last()?;
        (r.y == y && x < r.x0.saturating_add(r.len)).then_some(r.kind)
    }
}
