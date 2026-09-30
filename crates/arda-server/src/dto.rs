//! Response bodies other than the cell contract.

use crate::contract::convert::{discharge_m3s, height_m, mm_to_m};
use crate::contract::CONTRACT_VERSION;
use crate::query::WorldQuery;
use arda_core::{CellCoord, Lake, RiverSegment, Terminus, AREA_CELLS};
use serde::Serialize;
use ts_rs::TS;

/// Version tag of the HTTP API; every route lives under `/v1`.
pub const API_VERSION: &str = "v1";

/// Continent extent in kilometres (manifest `config.size_km`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
pub struct SizeKmDto {
    /// East–west extent.
    pub width: u32,
    /// North–south extent.
    pub height: u32,
}

/// Latitude belt the continent sits in, degrees north.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
pub struct LatitudeDto {
    /// Southern edge.
    pub south_deg: i16,
    /// Northern edge.
    pub north_deg: i16,
}

/// Canonical fine terrain descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
pub struct FineTerrainDto {
    /// Recipe that produced `terrain/fine.bin`.
    pub recipe_version: u16,
    /// Lattice spacing, metres (39.0625).
    pub spacing_m: f64,
}

/// Slippy overview tile pyramid geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
pub struct TilePyramidDto {
    /// Tile edge, pixels.
    pub tile_px: u32,
    /// Deepest zoom; zoom 0 is the whole world in one tile.
    pub max_zoom: u32,
    /// Edge of the square pyramid at `max_zoom`, pixels (`tile_px << max_zoom`).
    pub base_px: u32,
    /// Overview image width inside the base square, pixels.
    pub image_width_px: u32,
    /// Overview image height inside the base square, pixels.
    pub image_height_px: u32,
    /// Deepest zoom of `/v1/tiles/relief` (levels `max_zoom + 1..=` this);
    /// equal to `max_zoom` when the world has no relief levels.
    pub relief_max_zoom: u32,
}

/// `/v1/world`: identity, size and grids of the served world.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct WorldInfo {
    /// Cell contract version.
    pub contract_version: u32,
    /// HTTP API version.
    pub api_version: String,
    /// World seed as a decimal string (u64 does not fit a JS number).
    pub seed: String,
    /// Generator version that wrote the world.
    pub arda_version: String,
    /// Stored format major.
    pub format_version: u32,
    /// Nominal continent extent.
    pub size_km: SizeKmDto,
    /// Latitude belt.
    pub latitude: LatitudeDto,
    /// Area tiles per row.
    pub areas_wide: i32,
    /// Area tile rows.
    pub areas_high: i32,
    /// Cells along an area edge (512).
    pub area_cells: u32,
    /// Cell spacing, metres (100).
    pub cell_size_m: u32,
    /// Global cell columns (`areas_wide × 512`).
    pub cells_wide: u32,
    /// Global cell rows (`areas_high × 512`).
    pub cells_high: u32,
    /// Fine terrain, when the world has it.
    pub fine_terrain: Option<FineTerrainDto>,
    /// Overview tile pyramid.
    pub tiles: TilePyramidDto,
}

impl WorldInfo {
    /// Describes `query`'s world with the given tile pyramid.
    #[must_use]
    pub fn new(query: &WorldQuery, tiles: TilePyramidDto) -> Self {
        let m = query.world().manifest();
        let size = m.config.size_km();
        let band = m.config.latitude_band();
        let (cells_wide, cells_high) = query.cells();
        Self {
            contract_version: CONTRACT_VERSION,
            api_version: API_VERSION.to_owned(),
            seed: m.seed.to_string(),
            arda_version: m.arda_version.clone(),
            format_version: m.format_version,
            size_km: SizeKmDto {
                width: size.width,
                height: size.height,
            },
            latitude: LatitudeDto {
                south_deg: band.south_deg,
                north_deg: band.north_deg,
            },
            areas_wide: m.areas_wide,
            areas_high: m.areas_high,
            area_cells: u32::from(AREA_CELLS),
            cell_size_m: 100,
            cells_wide,
            cells_high,
            fine_terrain: m.fine_terrain.map(|f| FineTerrainDto {
                recipe_version: f.recipe_version,
                spacing_m: 39.0625,
            }),
            tiles,
        }
    }
}

/// `/v1/health`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct Health {
    /// Always `"ok"` when the service answers.
    pub status: String,
    /// Cell contract version.
    pub contract_version: u32,
    /// Seed of the served world, decimal string.
    pub seed: String,
}

/// How a river segment ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum TerminusDto {
    /// Flows into another segment at a confluence.
    Junction,
    /// Reaches the sea.
    Sea,
    /// Enters a lake.
    Lake,
    /// Leaves the tile.
    OffTile,
    /// Enters a basin without a positive-depth lake.
    Basin,
    /// Splits into several outgoing reaches.
    Divergence,
}

const fn terminus(t: Terminus) -> TerminusDto {
    match t {
        Terminus::Junction => TerminusDto::Junction,
        Terminus::Sea => TerminusDto::Sea,
        Terminus::Lake => TerminusDto::Lake,
        Terminus::OffTile => TerminusDto::OffTile,
        Terminus::Basin => TerminusDto::Basin,
        Terminus::Divergence => TerminusDto::Divergence,
    }
}

/// One saved watercourse reach fragment inside an area.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct RiverDto {
    /// Tile-local id, 1-based.
    pub id: u32,
    /// Stable global reach id, decimal string.
    pub global_id: String,
    /// Strahler order.
    pub order: u8,
    /// Channel width, metres.
    pub width_m: f64,
    /// Mean discharge, m³/s.
    pub discharge_m3s: f64,
    /// Local downstream segment at a confluence, if any.
    pub feeds: Option<u32>,
    /// How the segment ends.
    pub ends: TerminusDto,
    /// Global cells `[gx, gy]`, upstream to downstream.
    pub course: Vec<[u32; 2]>,
}

/// One saved lake fragment inside an area.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct LakeDto {
    /// Tile-local id, 1-based.
    pub id: u32,
    /// Stable global basin id, decimal string.
    pub global_id: String,
    /// Representative water surface, metres.
    pub surface_m: f64,
    /// Greatest local depth, metres.
    pub max_depth_m: f64,
    /// Global cell of supported surface outflow, if in this area.
    pub outlet: Option<[u32; 2]>,
    /// Covered global cells `[gx, gy]`.
    pub cells: Vec<[u32; 2]>,
}

/// `/v1/area/{ax}/{ay}/rivers`.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct AreaRivers {
    /// Cell contract version.
    pub contract_version: u32,
    /// Area column.
    pub ax: i32,
    /// Area row.
    pub ay: i32,
    /// Segments in stored order.
    pub rivers: Vec<RiverDto>,
}

/// `/v1/area/{ax}/{ay}/lakes`.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct AreaLakes {
    /// Cell contract version.
    pub contract_version: u32,
    /// Area column.
    pub ax: i32,
    /// Area row.
    pub ay: i32,
    /// Lakes in stored order.
    pub lakes: Vec<LakeDto>,
}

/// Area origin in global cells.
#[derive(Debug, Clone, Copy)]
pub struct Origin {
    /// Global column of local x = 0.
    pub gx: u32,
    /// Global row of local y = 0.
    pub gy: u32,
}

impl Origin {
    fn global(self, at: CellCoord) -> [u32; 2] {
        [self.gx + u32::from(at.x()), self.gy + u32::from(at.y())]
    }
}

/// Saved segment to its wire form.
#[must_use]
pub fn river(segment: &RiverSegment, origin: Origin) -> RiverDto {
    RiverDto {
        id: segment.id,
        global_id: segment.global_id.0.to_string(),
        order: segment.order,
        width_m: f64::from(segment.width_dm) / 10.0,
        discharge_m3s: discharge_m3s(segment.discharge.raw()),
        feeds: segment.feeds,
        ends: terminus(segment.ends),
        course: segment.course.iter().map(|&c| origin.global(c)).collect(),
    }
}

/// Saved lake to its wire form.
#[must_use]
pub fn lake(lake: &Lake, origin: Origin) -> LakeDto {
    LakeDto {
        id: lake.id,
        global_id: lake.global_id.0.to_string(),
        surface_m: height_m(lake.surface),
        max_depth_m: mm_to_m(i64::from(lake.depth_mm)),
        outlet: lake.outlet.map(|c| origin.global(c)),
        cells: lake.cells.iter().map(|&c| origin.global(c)).collect(),
    }
}
