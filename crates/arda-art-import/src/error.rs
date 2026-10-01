//! Errors for the importer. Problems with one raw image are not errors:
//! they become flags or rejections in the [`crate::report::Report`].

use std::path::PathBuf;
use thiserror::Error;

/// Anything that stops an import as a whole.
#[derive(Debug, Error)]
pub enum ImportError {
    /// A file or directory could not be read or written.
    #[error("{path}: {source}")]
    Io {
        /// The path involved.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// An image could not be decoded.
    #[error("image {path}: {message}")]
    Image {
        /// The image file.
        path: PathBuf,
        /// What went wrong.
        message: String,
    },
    /// The manifest is malformed.
    #[error("manifest {path}: {message}")]
    Manifest {
        /// The manifest file.
        path: PathBuf,
        /// The parser's message.
        message: String,
    },
    /// A tactical-library error (reading the base, writing PNGs).
    #[error(transparent)]
    Tactical(#[from] arda_tactical::TacticalError),
    /// JSON (de)serialisation failed.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// An option is out of range.
    #[error("option: {0}")]
    Options(String),
}

/// Shorthand for importer results.
pub type ImportResult<T> = Result<T, ImportError>;

/// Wraps an I/O error with its path.
pub fn io(path: &std::path::Path) -> impl FnOnce(std::io::Error) -> ImportError + '_ {
    move |source| ImportError::Io {
        path: path.to_path_buf(),
        source,
    }
}
