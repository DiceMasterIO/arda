//! Errors raised by the society simulation.

use thiserror::Error;

/// Everything that can stop [`crate::simulate_society`] or the JSON writer.
#[derive(Debug, Error)]
pub enum SocietyError {
    /// An embedded data table failed to parse.
    #[error("data table {table}: {source}")]
    Table {
        /// Table file name.
        table: &'static str,
        /// Parser error.
        source: serde_json::Error,
    },
    /// A data table parsed but is missing something the simulation needs.
    #[error("data table {table}: {detail}")]
    TableContent {
        /// Table file name.
        table: &'static str,
        /// What is missing.
        detail: String,
    },
    /// The input world is inconsistent (duplicate ids, dangling references).
    #[error("invalid world input: {0}")]
    Input(String),
    /// Serialising the society failed.
    #[error("serialise society: {0}")]
    Serialise(#[from] serde_json::Error),
    /// Writing output failed.
    #[error("write {path}: {source}")]
    Io {
        /// Target path.
        path: String,
        /// OS error.
        source: std::io::Error,
    },
}
