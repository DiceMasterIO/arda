//! The public facade for arda. Consumers depend on this crate only.
//!
//! Mirrors `mockup/04`'s transcript: `World` (manifest) to `Area` (cells and
//! objects) to `Cell` / `Block`.

// `code-prefs.md` §Q1 bans unwrap/expect *outside* `#[cfg(test)]`.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::path::{Path, PathBuf};

pub use arda_core::{
    AreaCells, AreaObjects, Block, Cell, Cover, GenerateConfig, Lake, LatitudeBand, LoadError,
    Manifest, RiverSegment, RoadClass, SizeKm, TerrainKind, TileId, ValidationStats,
};
pub use arda_gen::{GenError, HydrologyLimits};
pub use arda_render::AreaImageScale;
mod world;
pub use world::{Area, World};

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
    if format == ExportFormat::Json && scale == AreaImageScale::Detail {
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
