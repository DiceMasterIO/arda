//! Continent-tier records and objects (`02-models.md`, feature 02).

use crate::coords::KmCoord;
use crate::fixed::{DischargeMilli, HeightMm, RainfallMm, TempCentiC};

/// Climate regime of a 1 km cell (`logic/01` §Q6: band position and
/// elevation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ClimateRegime {
    /// The default mid-band regime.
    #[default]
    Temperate = 0,
    /// Below 42° latitude and under 1,000 m.
    Mediterranean = 1,
    /// Mean annual temperature below 3 °C.
    Boreal = 2,
    /// Below 23.5° latitude.
    Tropical = 3,
}

impl ClimateRegime {
    /// Decodes a stored discriminant.
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Temperate),
            1 => Some(Self::Mediterranean),
            2 => Some(Self::Boreal),
            3 => Some(Self::Tropical),
            _ => None,
        }
    }
}

/// One 1 km cell of the persisted continent grid (feature 02 §Q6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ContinentCell {
    /// Terrain elevation.
    pub height: HeightMm,
    /// Mean annual temperature.
    pub temperature: TempCentiC,
    /// Mean annual rainfall.
    pub rainfall: RainfallMm,
    /// Climate regime.
    pub regime: ClimateRegime,
    /// Downstream neighbour as an index into the fixed neighbour
    /// order; `None` for ocean cells and routing roots.
    pub downstream: Option<u8>,
    /// Drainage area above this cell; one cell is one km².
    pub catchment_km2: u32,
    /// Accumulated discharge (thousandths of a m³/s, i.e. L/s).
    pub discharge: DischargeMilli,
}

/// The whole persisted 1 km grid, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinentOverview {
    /// Grid width in 1 km cells.
    pub width: i32,
    /// Grid height in 1 km cells.
    pub height: i32,
    /// `width × height` cells, row-major.
    pub cells: Vec<ContinentCell>,
}

/// A continent-scale river (`logic/01` §Q7): catchment above the
/// feature-02 threshold, whole course at 1 km resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinentRiver {
    /// Continent-wide identifier, 1-based, deterministic. 1-based; 0 is
    /// reserved as the on-wire "no river" sentinel for `feeds`.
    pub id: u16,
    /// Catchment at the mouth, km².
    pub catchment_km2: u32,
    /// Discharge at the mouth.
    pub discharge: DischargeMilli,
    /// River this one joins; `None` when it reaches the sea. `Some(0)` is
    /// unrepresentable on the wire, since 0 encodes "no river".
    pub feeds: Option<u16>,
    /// Course cells, source to mouth.
    pub course: Vec<KmCoord>,
}

/// Every object stored in `continent/objects.bin`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContinentObjects {
    /// Continent-scale rivers.
    pub rivers: Vec<ContinentRiver>,
}

impl ContinentObjects {
    /// A continent with no objects yet.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regime_discriminants_round_trip() {
        for r in [
            ClimateRegime::Temperate,
            ClimateRegime::Mediterranean,
            ClimateRegime::Boreal,
            ClimateRegime::Tropical,
        ] {
            assert_eq!(ClimateRegime::from_u8(r as u8), Some(r));
        }
        assert_eq!(ClimateRegime::from_u8(4), None);
    }
}
