//! Errors for the town crate.

use std::path::PathBuf;
use thiserror::Error;

/// Anything that can go wrong planning or rendering a town.
#[derive(Debug, Error)]
pub enum TownError {
    /// The site record is unusable (for example, zero population).
    #[error("site {site}: {message}")]
    Site {
        /// Site name.
        site: String,
        /// What is wrong.
        message: String,
    },
    /// The planner could not satisfy a hard rule.
    #[error("plan {site}: {message}")]
    Plan {
        /// Site name.
        site: String,
        /// What failed.
        message: String,
    },
    /// A tactical window is empty or too large.
    #[error("window {0}")]
    Window(String),
    /// A site name was not recognised.
    #[error("unknown site `{0}`")]
    UnknownSite(String),
    /// A file could not be written.
    #[error("{path}: {source}")]
    Io {
        /// The file involved.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// JSON (de)serialisation failed.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// The tactical crate rejected a layout or library.
    #[error(transparent)]
    Tactical(#[from] arda_tactical::TacticalError),
}
