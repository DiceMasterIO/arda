//! The per-cell field list (`02-models.md` Fields and types).
//!
//! Every field is the artifact's "what the finished map knows" list; sim
//! values are fixed-point so the layer is bit-identical across platforms.

use crate::fixed::{DischargeMilli, HeightMm, RainfallMm, TempCentiC};

/// Whether a cell is sea, dry land, or lake surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum TerrainKind {
    /// Below sea level and connected to the ocean.
    #[default]
    Sea = 0,
    /// Dry land.
    Land = 1,
    /// Inland standing water.
    Lake = 2,
}

impl TerrainKind {
    /// Decodes a stored discriminant.
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Sea),
            1 => Some(Self::Land),
            2 => Some(Self::Lake),
            _ => None,
        }
    }
}

/// Dominant ground cover (`logic/02` vegetation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Cover {
    /// Bare soil or sand.
    #[default]
    Bare = 0,
    /// Grassland.
    Grass = 1,
    /// Scrub and heath.
    Scrub = 2,
    /// Closed forest.
    Forest = 3,
    /// Marsh or fen.
    Marsh = 4,
    /// Exposed rock.
    Rock = 5,
    /// Permanent ice.
    Ice = 6,
}

impl Cover {
    /// Decodes a stored discriminant.
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Bare),
            1 => Some(Self::Grass),
            2 => Some(Self::Scrub),
            3 => Some(Self::Forest),
            4 => Some(Self::Marsh),
            5 => Some(Self::Rock),
            6 => Some(Self::Ice),
            _ => None,
        }
    }
}

/// Road class crossing a cell (`logic/02` roads).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum RoadClass {
    /// No road.
    #[default]
    None = 0,
    /// Footpath or cart track.
    Track = 1,
    /// Maintained road.
    Road = 2,
    /// Trunk corridor.
    Highway = 3,
}

impl RoadClass {
    /// Decodes a stored discriminant.
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::None),
            1 => Some(Self::Track),
            2 => Some(Self::Road),
            3 => Some(Self::Highway),
            _ => None,
        }
    }
}

/// Everything the finished map knows about one 100 m cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cell {
    /// Elevation.
    pub height: HeightMm,
    /// Sea, land, or lake.
    pub terrain: TerrainKind,
    /// Dominant ground cover.
    pub cover: Cover,
    /// Slope in thousandths of a degree.
    pub slope_milli_deg: u16,
    /// Downslope compass bearing, 0-359; 0 on flat ground.
    pub aspect_deg: u16,
    /// Mean annual temperature.
    pub temperature: TempCentiC,
    /// Mean annual rainfall.
    pub rainfall: RainfallMm,
    /// Soil moisture, 0-255.
    pub moisture: u8,
    /// Canopy closure, 0-255.
    pub forest_density: u8,
    /// Upstream cells draining through here.
    pub drainage_area_cells: u32,
    /// Mean discharge.
    pub discharge: DischargeMilli,
    /// Strahler order; 0 when no watercourse.
    pub watercourse_order: u8,
    /// Channel width in decimetres; 0 when no watercourse.
    pub watercourse_width_dm: u16,
    /// Height above the nearest downstream channel, decimetres.
    pub height_above_river_dm: u16,
    /// Standing-water tendency, 0-255.
    pub wetness: u8,
    /// Road class.
    pub road: RoadClass,
    /// Settlement that built on this cell; 0 when none.
    pub built_by: u16,
}
