//! Errors for the refinement crate.

use thiserror::Error;

/// Everything that can stop a refinement.
#[derive(Debug, Error)]
pub enum RefineError {
    /// The world could not supply a layer.
    #[error("world layer unavailable: {0}")]
    Load(#[from] arda::LoadError),
    /// The fine terrain layer failed to read.
    #[error("fine terrain read failed: {0}")]
    Terrain(#[from] arda_core::TerrainFileError),
    /// A requested cell or window lies outside the world.
    #[error("{what} ({x}, {y}) lies outside the world")]
    OutOfWorld {
        /// What was requested.
        what: &'static str,
        /// Column.
        x: i64,
        /// Row.
        y: i64,
    },
    /// A window has no area or is too large.
    #[error("window {w}x{h} squares is empty or exceeds {max} squares per side")]
    Window {
        /// Width in squares.
        w: u32,
        /// Height in squares.
        h: u32,
        /// Maximum side.
        max: u32,
    },
    /// JSON serialisation failed.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// PNG encoding failed.
    #[error("png: {0}")]
    Png(#[from] png::EncodingError),
    /// File output failed.
    #[error("io on {path}: {source}")]
    Io {
        /// File path.
        path: String,
        /// Cause.
        #[source]
        source: std::io::Error,
    },
    /// An internal lock was poisoned by a panic elsewhere.
    #[error("internal cache lock poisoned")]
    Poisoned,
}
