//! Errors for the fields crate.

use thiserror::Error;

/// Anything that can go wrong building a countryside window.
#[derive(Debug, Error)]
pub enum FieldsError {
    /// The requested window is empty or too large.
    #[error("window {width}x{height} squares is outside 1..={max} on a side")]
    WindowSize {
        /// Requested width in squares.
        width: u32,
        /// Requested height in squares.
        height: u32,
        /// Largest side accepted.
        max: u32,
    },
    /// The window origin is not a finite coordinate.
    #[error("window origin ({0}, {1}) m is not finite")]
    Origin(f64, f64),
    /// A sidecar or report could not be serialised.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// The tactical crate refused a layout or library.
    #[error(transparent)]
    Tactical(#[from] arda_tactical::TacticalError),
}
