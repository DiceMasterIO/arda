//! Shared types, the subseeded PRNG, and the only byte codec in the workspace.
//!
//! See `docs/capstone/01-architecture.md` for the crate boundary rules.

// `code-prefs.md` §Q1 bans unwrap/expect *outside* `#[cfg(test)]`.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod cell;
pub mod config;
pub mod continent;
pub mod coords;
pub mod error;
pub mod fixed;
pub mod formats;
pub mod hydrology;
pub mod objects;
pub mod rng;
pub mod terrain;
pub mod tiles;

pub use cell::{Cell, Cover, RoadClass, TerrainKind};
pub use config::{GenerateConfig, LatitudeBand, SizeKm};
pub use continent::{
    ClimateRegime, ContinentCell, ContinentObjects, ContinentOverview, ContinentRiver,
};
pub use coords::{
    AreaCoord, CellCoord, GlobalCell, KmCoord, SquareCoord, AREA_CELLS, BLOCK_SQUARES, CELL_SIZE_M,
    SQUARE_SIZE_MM,
};
pub use error::{ConfigError, FormatError, LoadError};
pub use fixed::{DischargeMilli, HeightMm, RainfallMm, TempCentiC};
pub use formats::blocks::{decode_blocks, encode_blocks, Block, BlockArchive, ZSTD_LEVEL};
pub use formats::cells::{decode_cells, encode_cells, AreaCells, CELL_BYTES};
pub use formats::manifest::{
    read_manifest, write_manifest, FineTerrainDescriptor, Manifest, ValidationStats,
    FINE_TERRAIN_PATH, FINE_TERRAIN_RECIPE_VERSION, MANIFEST_NAME,
};
pub use formats::objects::{
    decode_continent_objects, decode_objects, encode_continent_objects, encode_objects,
    CONTINENT_OBJECTS_MAGIC, OBJECTS_MAGIC,
};
pub use formats::overview::{
    decode_overview, encode_overview, NO_DOWNSTREAM, OVERVIEW_CELL_BYTES, OVERVIEW_MAGIC,
};
pub use formats::terrain::{TerrainFileError, TerrainFileReader, TerrainFileWriter};
pub use formats::FORMAT_VERSION;
pub use hydrology::{BasinId, Litres};
pub use objects::{AreaObjects, Lake, RiverSegment, Terminus};
pub use rng::{derive, rng, SeedKey, Stage, Tier};
pub use terrain::{TerrainField, TerrainFieldError, TerrainPoint};
pub use tiles::{may_adjoin, tile_def, tiles_in_group, TileDef, TileGroup, TileId, SKELETON_TILES};

#[cfg(test)]
mod foundation_tests;
