//! The crate's error type.

/// Everything that can stop the settlement stage.
#[derive(Debug, thiserror::Error)]
pub enum SettleError {
    /// The stored world could not be read.
    #[error(transparent)]
    Load(#[from] arda::LoadError),
    /// The overview render failed.
    #[error(transparent)]
    Export(#[from] arda::ExportError),
    /// The working rasters would exceed the memory budget (goal-prompt §8).
    #[error("working rasters need {needed} bytes, over the {budget}-byte budget")]
    Budget {
        /// Bytes the run would need.
        needed: u64,
        /// The admitted ceiling.
        budget: u64,
    },
    /// An allocation was refused before any output was written.
    #[error("could not reserve {bytes} bytes for {what}")]
    Reserve {
        /// What was being allocated.
        what: &'static str,
        /// Requested size.
        bytes: u64,
    },
    /// The world's dimensions do not fit the working grid.
    #[error("world dimensions are out of range: {0}")]
    Dimensions(&'static str),
    /// A file could not be read or written.
    #[error("{path}: {source}")]
    Io {
        /// The file involved.
        path: String,
        /// Underlying cause.
        #[source]
        source: std::io::Error,
    },
    /// JSON (de)serialisation failed.
    #[error("{path}: {source}")]
    Json {
        /// The file involved.
        path: String,
        /// Underlying cause.
        #[source]
        source: serde_json::Error,
    },
    /// A PNG could not be decoded or encoded.
    #[error("png: {0}")]
    Png(String),
    /// A name scope ran out of unique names (limits refuse, never repeat).
    #[error("names: {0}")]
    Names(#[from] arda_names::NamesError),
    /// A stored society file is malformed.
    #[error("{path}: {reason}")]
    Format {
        /// The file involved.
        path: String,
        /// What is wrong.
        reason: &'static str,
    },
}

impl SettleError {
    /// Wraps an I/O error with its path.
    pub(crate) fn io(path: &std::path::Path, source: std::io::Error) -> Self {
        Self::Io {
            path: path.display().to_string(),
            source,
        }
    }
}
