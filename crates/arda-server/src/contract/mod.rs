//! The versioned world contract: what any 100 m global cell means in plain units.
//!
//! Tactical generation and the game read the world only through these DTOs,
//! so every field names its unit and its source. Bump [`CONTRACT_VERSION`]
//! whenever a field changes meaning, unit or presence.
//!
//! Geometry (vocabulary I1, contract 2): global cell `(gx, gy)` covers
//! `[gx·100, gx·100 + 100) × [gy·100, gy·100 + 100)` metres from the modeled
//! world's north-west corner (x east, y south); its centre is
//! `(gx·100 + 50, gy·100 + 50)`. A point belongs to the cell `floor(x/100)`.
//! The world's stored cell values are sampled by the generator at the
//! cell's north-west corner (`fine_input::prepared_heights`); the contract
//! reports them as the cell's values and takes fine heights over the whole
//! footprint, centred on the centre.

pub mod coast;
pub mod convert;
pub mod snow;

use serde::Serialize;
use ts_rs::TS;

/// Version of the cell contract. Clients must reject a version they do not know.
pub const CONTRACT_VERSION: u32 = 2;

/// Cell edge, metres.
pub const CELL_M: f64 = 100.0;

/// Sea, dry land, or lake surface (source: `Cell::terrain`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum TerrainKindDto {
    /// Below sea level and connected to the ocean.
    Sea,
    /// Dry land.
    Land,
    /// Inland standing water.
    Lake,
}

/// Dominant ground cover (source: `Cell::cover`, `logic/02` vegetation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum CoverDto {
    /// Bare soil or sand.
    Bare,
    /// Grassland.
    Grass,
    /// Scrub and heath.
    Scrub,
    /// Closed forest.
    Forest,
    /// Marsh or fen.
    Marsh,
    /// Exposed rock.
    Rock,
    /// Permanent ice.
    Ice,
}

/// Road class crossing the cell (source: `Cell::road`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum RoadDto {
    /// No road.
    None,
    /// Footpath or cart track.
    Track,
    /// Maintained road.
    Road,
    /// Trunk corridor.
    Highway,
}

/// Derived coast facts.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
pub struct CoastSample {
    /// A land cell with at least one sea cell among its 8 neighbours.
    pub is_coast: bool,
    /// Euclidean centre-to-centre distance to the nearest coast cell, metres;
    /// 0 on the coast, `null` when none lies within [`coast::MAX_DISTANCE_M`].
    pub distance_m: Option<f64>,
}

/// Derived snow proxy. Not simulated: a documented rule of thumb on saved climate.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
pub struct SnowSample {
    /// Perennial snow cover of the cell, 0–1 (see [`snow`]).
    pub fraction: f64,
    /// The same rule at the footprint's highest fine-terrain point; `null` without fine terrain.
    pub peak_fraction: Option<f64>,
    /// `fraction >= 0.5`, or permanent-ice cover.
    pub perennial: bool,
    /// Altitude, metres, where the lapse-adjusted mean annual temperature crosses
    /// the snow midpoint (−5.5 °C). Ground above it is snowy under this proxy.
    pub snowline_m: f64,
}

/// The watercourse reach that owns this cell, when a saved river course passes through it.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct RiverMembership {
    /// Tile-local segment id (`RiverSegment::id`), unique within the area.
    pub segment_id: u32,
    /// Stable global reach id, decimal string (u64 does not fit a JS number).
    pub global_reach_id: String,
    /// Strahler order of the segment.
    pub order: u8,
    /// Segment channel width, metres.
    pub width_m: f64,
    /// Segment mean discharge, m³/s.
    pub discharge_m3s: f64,
}

/// The lake covering this cell, when a saved lake lists it.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct LakeMembership {
    /// Tile-local lake id (`Lake::id`), unique within the area.
    pub lake_id: u32,
    /// Stable global basin id, decimal string.
    pub global_basin_id: String,
    /// Representative water surface, metres above sea level.
    pub surface_m: f64,
    /// Water depth in this cell, metres: surface minus cell height, never negative.
    pub depth_m: f64,
    /// Greatest local depth of the lake fragment, metres.
    pub max_depth_m: f64,
}

/// Canonical fine terrain (`terrain/fine.bin`, 39.0625 m lattice) over the cell.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
pub struct FineHeights {
    /// Bilinear height at the cell centre, metres.
    pub centre_m: f64,
    /// Exact minimum of the bilinear surface over the footprint, metres.
    pub min_m: f64,
    /// Exact maximum of the bilinear surface over the footprint, metres.
    pub max_m: f64,
}

/// Everything the world knows about one 100 m global cell, in plain units.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct CellSample {
    /// [`CONTRACT_VERSION`] this sample was produced under.
    pub contract_version: u32,
    /// Global cell column.
    pub gx: u32,
    /// Global cell row.
    pub gy: u32,
    /// Area tile column containing the cell.
    pub ax: i32,
    /// Area tile row containing the cell.
    pub ay: i32,
    /// Column within the area, 0–511.
    pub cx: u16,
    /// Row within the area, 0–511.
    pub cy: u16,
    /// West edge of the cell, metres east of the world origin (`gx·100`).
    pub x_m: f64,
    /// North edge of the cell, metres south of the world origin (`gy·100`).
    pub y_m: f64,
    /// Cell centre, metres east of the world origin (`gx·100 + 50`).
    pub centre_x_m: f64,
    /// Cell centre, metres south of the world origin (`gy·100 + 50`).
    pub centre_y_m: f64,
    /// Elevation above sea level, metres (`Cell::height`, mm).
    pub height_m: f64,
    /// Sea, land or lake (`Cell::terrain`).
    pub terrain: TerrainKindDto,
    /// Dominant cover (`Cell::cover`).
    pub cover: CoverDto,
    /// Slope, degrees (`Cell::slope_milli_deg`).
    pub slope_deg: f64,
    /// Downslope compass bearing, degrees 0–359; 0 on flat ground (`Cell::aspect_deg`).
    pub aspect_deg: f64,
    /// Mean annual temperature, °C (`Cell::temperature`, centi-°C).
    pub temperature_c: f64,
    /// Mean annual rainfall, mm (`Cell::rainfall`).
    pub rainfall_mm: f64,
    /// Soil moisture, 0–1 (`Cell::moisture` / 255).
    pub moisture: f64,
    /// Standing-water tendency, 0–1 (`Cell::wetness` / 255).
    pub wetness: f64,
    /// Canopy closure, 0–1 (`Cell::forest_density` / 255).
    pub forest_density: f64,
    /// Upstream drainage area, km² (`Cell::drainage_area_cells` × 0.01 km²).
    pub drainage_area_km2: f64,
    /// Mean discharge, m³/s (`Cell::discharge`, L/s).
    pub discharge_m3s: f64,
    /// Strahler order; 0 when no watercourse (`Cell::watercourse_order`).
    pub watercourse_order: u8,
    /// Channel width, metres; 0 when no watercourse (`Cell::watercourse_width_dm`).
    pub watercourse_width_m: f64,
    /// Height above the nearest downstream channel, metres (`Cell::height_above_river_dm`).
    pub height_above_river_m: f64,
    /// Road class (`Cell::road`).
    pub road: RoadDto,
    /// Settlement that built on this cell; `null` when none (`Cell::built_by`).
    pub built_by: Option<u16>,
    /// Derived coast facts.
    pub coast: CoastSample,
    /// Derived snow proxy.
    pub snow: SnowSample,
    /// Saved river course through this cell, if any.
    pub river: Option<RiverMembership>,
    /// Saved lake covering this cell, if any.
    pub lake: Option<LakeMembership>,
    /// Fine-terrain heights; `null` for worlds without `terrain/fine.bin`.
    pub fine: Option<FineHeights>,
}

/// Where a [`PointSample`] height came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum HeightSource {
    /// Bilinear on the canonical fine terrain.
    Fine,
    /// Bilinear on the four surrounding 100 m cell centres (worlds without fine terrain).
    Cells,
}

/// A height query at arbitrary world metres, plus the nearest cell's sample.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct PointSample {
    /// [`CONTRACT_VERSION`].
    pub contract_version: u32,
    /// Query position east of the world origin, metres.
    pub x_m: f64,
    /// Query position south of the world origin, metres.
    pub y_m: f64,
    /// Interpolated elevation, metres.
    pub height_m: f64,
    /// Which surface was interpolated.
    pub height_source: HeightSource,
    /// Sample of the cell containing the point.
    pub cell: CellSample,
}
