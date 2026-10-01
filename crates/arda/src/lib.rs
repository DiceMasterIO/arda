//! The public facade for arda. Consumers depend on this crate only.
//!
//! Mirrors `mockup/04`'s transcript: `World` (manifest) to `Area` (cells and
//! objects) to `Cell` / `Block`.

// `code-prefs.md` §Q1 bans unwrap/expect *outside* `#[cfg(test)]`.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::path::{Path, PathBuf};

pub use arda_core::{
    AreaCells, AreaObjects, Block, Cell, Cover, GenerateConfig, Island, IslandCause, Lake,
    LatitudeBand, LoadError, Manifest, RiverSegment, RoadClass, ShoreClass, ShoreLayer, SizeKm,
    TerrainKind, TileId, ValidationStats, FINE_TERRAIN_RECIPE_VERSION,
};
pub use arda_gen::orchestrator::{FineDeliveryLimits, FineRecipe};
pub use arda_gen::{GenError, HydrologyLimits};
pub use arda_render::{AreaImageScale, ImageQuality};
mod atlas;
mod export_quality;
pub use export_quality::{
    export_area_with_quality, export_area_with_quality_and_style, export_overview_with_quality,
    export_overview_with_quality_and_style,
};
mod world;
pub use atlas::area_atlas_terrain;
pub use world::{Area, World};

/// Cartographic presentation applied to PNG exports.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MapStyle {
    /// Existing categorical cartography.
    #[default]
    Classic,
    /// Natural atlas terrain color and deterministic relief.
    Atlas,
}

/// Runs the batch, writing a world directory (`mockup/01`).
///
/// # Errors
/// See [`GenError`].
pub fn generate(seed: u64, config: GenerateConfig, out: &Path) -> Result<Manifest, GenError> {
    arda_gen::generate_world(seed, config, out)
}

/// Runs offline world generation with explicit resource limits.
/// Limits admit or reject work without changing the generated physical state.
///
/// # Errors
/// See [`GenError`]. Insufficient admission leaves the output path untouched.
pub fn generate_with_limits(
    seed: u64,
    config: GenerateConfig,
    out: &Path,
    limits: HydrologyLimits,
) -> Result<Manifest, GenError> {
    arda_gen::generate_world_with_limits(seed, config, out, limits)
}

/// Generates a complete world from the first validated canonical fine
/// source of the default recipe ([`arda_core::FINE_TERRAIN_RECIPE_VERSION`])
/// in the fixed deterministic attempt ladder. Legacy generation remains
/// available through [`generate`].
///
/// # Errors
/// Returns source/world admission, generation, validation, or publication errors.
pub fn generate_from_fine_source(
    seed: u64,
    config: GenerateConfig,
    out: &Path,
    limits: FineDeliveryLimits,
) -> Result<Manifest, GenError> {
    arda_gen::orchestrator::generate_world_with_fine_source(seed, config, out, limits)
}

/// [`generate_from_fine_source`] with an explicit fine-terrain recipe:
/// 4 (spectral valleys), 5 (v0.1 formation, replayed exactly) or 6 (the
/// default). The manifest records it, and loading and rendering follow it.
///
/// # Errors
/// [`GenError::Validation`] for a recipe this build cannot generate, else
/// as [`generate_from_fine_source`].
pub fn generate_from_fine_recipe(
    seed: u64,
    config: GenerateConfig,
    out: &Path,
    limits: FineDeliveryLimits,
    recipe_version: u16,
) -> Result<Manifest, GenError> {
    let recipe = FineRecipe::from_version(recipe_version).ok_or_else(|| GenError::Validation {
        check: format!("unsupported fine terrain recipe {recipe_version} (expected 4, 5 or 6)"),
    })?;
    arda_gen::orchestrator::generate_world_with_fine_recipe(seed, config, out, limits, recipe)
}

/// Output format for `export` (`mockup/03`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Cartographic PNG.
    Png,
    /// Versioned JSON.
    Json,
}

/// An export failure at the facade boundary.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// The world could not supply the requested tile.
    #[error(transparent)]
    Load(#[from] LoadError),
    /// The renderer refused.
    #[error(transparent)]
    Render(#[from] arda_render::RenderError),
    /// Detailed raster scale is meaningful only for area PNGs.
    #[error("detailed image scale is valid only for area PNG exports")]
    InvalidImageScale,
    /// The artifact could not be written.
    #[error("failed writing {path}: {source}")]
    Write {
        /// The file being written.
        path: String,
        /// Underlying cause.
        #[source]
        source: std::io::Error,
    },
}

/// Exports one area tile to `out`, returning the file written.
///
/// # Errors
/// See [`ExportError`].
pub fn export_area(
    world: &World,
    ax: i32,
    ay: i32,
    out: &Path,
    format: ExportFormat,
) -> Result<PathBuf, ExportError> {
    export_area_with_scale(world, ax, ay, out, format, AreaImageScale::Preview)
}

/// Exports an area at an explicit image scale without retaining it in memory.
///
/// # Errors
/// Detailed scale with JSON is refused. Other failures propagate through
/// [`ExportError`], including stored-layer and physical geometry errors.
pub fn export_area_with_scale(
    world: &World,
    ax: i32,
    ay: i32,
    out: &Path,
    format: ExportFormat,
    scale: AreaImageScale,
) -> Result<PathBuf, ExportError> {
    if format == ExportFormat::Json && scale != AreaImageScale::Preview {
        return Err(ExportError::InvalidImageScale);
    }
    let area = world.read_area(ax, ay)?;
    let coordinate = |v| {
        u32::try_from(v).map_err(|_| arda_render::RenderError::ChannelGeometry {
            reason: "area origin has a negative global coordinate",
        })
    };
    let origin = arda_core::GlobalCell {
        x: coordinate(ax)? * 512,
        y: coordinate(ay)? * 512,
    };
    let name = format!("area_{ax:02}_{ay:02}");
    let (path, bytes) = match format {
        ExportFormat::Png => (
            out.join(if scale == AreaImageScale::Detail {
                format!("{name}_detail.png")
            } else {
                format!("{name}.png")
            }),
            arda_render::render_area_png(area.cells(), area.objects(), origin, scale)?,
        ),
        ExportFormat::Json => (
            out.join(format!("{name}.json")),
            arda_render::area_json(world.manifest(), ax, ay, area.cells(), area.objects())?
                .into_bytes(),
        ),
    };
    std::fs::write(&path, bytes).map_err(|e| ExportError::Write {
        path: path.display().to_string(),
        source: e,
    })?;
    Ok(path)
}

/// Default pixels per area tile in an overview render.
pub const OVERVIEW_PX_PER_AREA: u32 = 48;

/// Renders every loaded area into one overview image at `<out>/overview.png`.
///
/// # Errors
/// Propagates render and write failures.
pub fn export_overview(world: &World, out: &Path, px: u32) -> Result<PathBuf, ExportError> {
    let mut raster = arda_render::OverviewRaster::new(
        world.manifest().areas_wide,
        world.manifest().areas_high,
        px,
    )?;
    for (x, y) in world.area_coords() {
        let area = world.read_area(x, y)?;
        raster.push(arda_core::AreaCoord::new(x, y), area.cells())?;
    }
    let bytes = raster.finish()?;
    let path = out.join("overview.png");
    std::fs::write(&path, bytes).map_err(|e| ExportError::Write {
        path: path.display().to_string(),
        source: e,
    })?;
    Ok(path)
}

/// Exports one tactical block — the wave-function-collapse tile layer — as
/// a symbolic PNG or its JSON with the tile legend.
///
/// # Errors
/// [`ExportError::Load`] when no block was materialised for that cell.
pub fn export_block(
    world: &World,
    ax: i32,
    ay: i32,
    cx: u16,
    cy: u16,
    out: &Path,
    format: ExportFormat,
) -> Result<PathBuf, ExportError> {
    let block = world.block(ax, ay, cx, cy)?;
    let name = format!("block_{ax:02}_{ay:02}_{cx:03}_{cy:03}");
    let (path, bytes) = match format {
        ExportFormat::Png => (
            out.join(format!("{name}.png")),
            arda_render::render_block_png(block)?,
        ),
        ExportFormat::Json => (
            out.join(format!("{name}.json")),
            arda_render::block_json(world.manifest(), block).into_bytes(),
        ),
    };
    std::fs::write(&path, bytes).map_err(|e| ExportError::Write {
        path: path.display().to_string(),
        source: e,
    })?;
    Ok(path)
}
