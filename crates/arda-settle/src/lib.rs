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
pub mod model;
pub mod names;
pub mod naming;
pub mod num;
pub mod ore;
pub mod output;
pub mod pipeline;
pub mod place;
pub mod profile;
pub mod realms;
pub mod render;
pub mod rng;
pub mod roads;
pub mod route;
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

/// Writes every society file into `dir` (created if missing).
///
/// # Errors
/// I/O and serialisation errors.
pub fn write(dir: &Path, g: &Grid, seed: u64, s: &Society) -> Result<(), SettleError> {
    std::fs::create_dir_all(dir).map_err(|e| SettleError::io(dir, e))?;
    let v = output::FORMAT_VERSION;
    output::write_json(
        &dir.join("settlements.json"),
        &output::SettlementsFile {
            format_version: v,
            seed,
            width_cells: num::u32_of(g.width),
            height_cells: num::u32_of(g.height),
            settlements: s.settlements.clone(),
        },
    )?;
    output::write_json(
        &dir.join("roads.json"),
        &output::RoadsFile {
            format_version: v,
            roads: s.network.roads.clone(),
            crossings: s.crossings.clone(),
            passes: s.passes.clone(),
        },
    )?;
    output::write_json(
        &dir.join("realms.json"),
        &output::RealmsFile {
            format_version: v,
            realms: s.realms.realms.clone(),
        },
    )?;
    output::write_json(
        &dir.join("names.json"),
        &output::NamesFile {
            format_version: v,
            rivers: s.rivers.clone(),
            mountains: s.mountains.clone(),
            regions: s.regions.clone(),
        },
    )?;
    output::write_landuse(
        &dir.join("landuse.bin"),
        g.width,
        g.height,
        &s.landuse.codes,
        &s.landuse.owner,
    )?;
    output::write_realm_map(&dir.join("realms.bin"), g.width, g.height, &s.realms.map)?;
    output::write_stats(&dir.join("stats.json"), &s.stats)
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
