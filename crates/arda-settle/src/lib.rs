//! Settlements, roads, realms and names for a generated arda world.
//!
//! Reads a stored world (`arda::World::load`), stitches its area cells into
//! one working grid, and derives, in order: per-cell suitability and site
//! tags, settlements placed by carrying capacity, land use, a road
//! hierarchy, realms and names. Output goes to `<world>/society/`; see the
//! README for the formats and the rules each stage follows.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod canvas;
pub mod central;
pub mod cost;
pub mod crossings;
pub mod culture;
pub mod error;
pub mod features;
pub mod field;
pub mod grid;
pub mod ids;
pub mod landuse;
pub mod load;
pub mod market;
pub mod model;
pub mod names;
pub mod naming;
pub mod num;
pub mod ore;
pub mod output;
pub mod pipeline;
pub mod place;
pub mod primacy;
pub mod profile;
pub mod realm_stats;
pub mod realms;
pub mod render;
pub mod rng;
pub mod roads;
pub mod route;
pub mod seats;
pub mod snap;
pub mod stats;
pub mod suitability;
pub mod synthetic;
pub mod tags;
pub mod trunk;

pub use error::SettleError;
pub use pipeline::{run, Society};
pub use place::PlaceParams;

use grid::Grid;
use std::path::{Path, PathBuf};

/// Directory under the world that holds this stage's files.
pub const SOCIETY_DIR: &str = "society";

/// Every society file as `(name, bytes)`, in write order.
///
/// # Errors
/// Serialisation errors.
pub fn encode(
    dir: &Path,
    g: &Grid,
    seed: u64,
    s: &Society,
) -> Result<Vec<(&'static str, Vec<u8>)>, SettleError> {
    let v = output::FORMAT_VERSION;
    let at = |name: &str| dir.join(name);
    let (w, h) = (g.width, g.height);
    Ok(vec![
        (
            "settlements.json",
            output::json_bytes(
                &at("settlements.json"),
                &output::SettlementsFile {
                    format_version: v,
                    seed,
                    width_cells: num::u32_of(g.width),
                    height_cells: num::u32_of(g.height),
                    settlements: s.settlements.clone(),
                },
            )?,
        ),
        (
            "roads.json",
            output::json_bytes(
                &at("roads.json"),
                &output::RoadsFile {
                    format_version: v,
                    roads: s.network.roads.clone(),
                    crossings: s.crossings.clone(),
                    passes: s.passes.clone(),
                },
            )?,
        ),
        (
            "realms.json",
            output::json_bytes(
                &at("realms.json"),
                &output::RealmsFile {
                    format_version: v,
                    realms: s.realms.realms.clone(),
                },
            )?,
        ),
        (
            "names.json",
            output::json_bytes(
                &at("names.json"),
                &output::NamesFile {
                    format_version: v,
                    rivers: s.rivers.clone(),
                    mountains: s.mountains.clone(),
                    regions: s.regions.clone(),
                },
            )?,
        ),
        (
            "landuse.bin",
            output::landuse_bytes(&at("landuse.bin"), w, h, &s.landuse.codes, &s.landuse.owner)?,
        ),
        (
            "realms.bin",
            output::realm_map_bytes(&at("realms.bin"), w, h, &s.realms.map)?,
        ),
        (
            "roads.bin",
            output::road_map_bytes(&at("roads.bin"), w, h, &s.network.raster)?,
        ),
        (
            "stats.json",
            output::json_bytes(&at("stats.json"), &s.stats)?,
        ),
    ])
}

/// Writes every society file into `dir` (created if missing).
///
/// Every file is encoded before the first is written, so a failure while
/// encoding (an allocation refused on a large world) leaves `dir` as it
/// was instead of a mix of new and old files (review round 2 #31).
///
/// # Errors
/// I/O and serialisation errors.
pub fn write(dir: &Path, g: &Grid, seed: u64, s: &Society) -> Result<(), SettleError> {
    let files = encode(dir, g, seed, s)?;
    std::fs::create_dir_all(dir).map_err(|e| SettleError::io(dir, e))?;
    for (name, bytes) in files {
        let path = dir.join(name);
        std::fs::write(&path, bytes).map_err(|e| SettleError::io(&path, e))?;
    }
    Ok(())
}

/// Loads the world at `world_dir`, runs every stage and writes
/// `<world_dir>/society/`. Returns the society and the directory written.
///
/// # Errors
/// Load, budget, allocation and I/O errors.
pub fn generate(world_dir: &Path, budget: u64) -> Result<(Society, PathBuf), SettleError> {
    let world = arda::World::load(world_dir)?;
    let g = load::load_grid(&world, budget)?;
    let params = PlaceParams {
        seed: world.seed(),
        density_per_km2: u32::from(world.manifest().config.mean_density_per_km2()),
    };
    let society = run(&g, params)?;
    let dir = world_dir.join(SOCIETY_DIR);
    write(&dir, &g, params.seed, &society)?;
    Ok((society, dir))
}
