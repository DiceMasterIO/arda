//! Errors for the ways crate.

use thiserror::Error;

/// Anything that can go wrong overlaying ways onto a layout.
#[derive(Debug, Error)]
pub enum WaysError {
    /// The window origin is not on the 5-ft square lattice.
    #[error("window origin ({x_m}, {y_m}) m is not a whole number of {square_m} m squares")]
    UnalignedOrigin {
        /// Origin, metres east.
        x_m: f64,
        /// Origin, metres south.
        y_m: f64,
        /// Square side in metres.
        square_m: f64,
    },
    /// An input exceeds a documented limit (refused, never trimmed).
    #[error("limit: {0}")]
    Limit(String),
    /// The layout's square buffer does not match its size.
    #[error("layout `{0}` has a square buffer that does not match width × height")]
    BadLayout(String),
    /// A sidecar could not be serialised.
    #[error("sidecar: {0}")]
    Json(#[from] serde_json::Error),
    /// A tactical-crate failure (rendering, library loading).
    #[error(transparent)]
    Tactical(#[from] arda_tactical::TacticalError),
    /// A file could not be written.
    #[error("{path}: {source}")]
    Io {
        /// The file involved.
        path: std::path::PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
}
