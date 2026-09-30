//! Errors for the tactical crate.

use std::path::PathBuf;
use thiserror::Error;

/// Anything that can go wrong loading, validating or rendering tactical maps.
#[derive(Debug, Error)]
pub enum TacticalError {
    /// A file could not be read or written.
    #[error("{path}: {source}")]
    Io {
        /// The file involved.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// An image could not be decoded or encoded.
    #[error("image {path}: {message}")]
    Image {
        /// The image file.
        path: PathBuf,
        /// What went wrong.
        message: String,
    },
    /// The catalogue JSON is malformed or breaks the schema.
    #[error("catalogue {path}: {source}")]
    Catalog {
        /// The catalogue file.
        path: PathBuf,
        /// The serde error.
        source: serde_json::Error,
    },
    /// A layout JSON could not be parsed or written.
    #[error("layout: {0}")]
    LayoutJson(serde_json::Error),
    /// The library failed validation; each line names the asset and the rule.
    #[error("library failed validation with {} issue(s):\n{}", .0.len(), join_issues(.0))]
    Invalid(Vec<crate::validate::Issue>),
    /// A layout is inconsistent with itself or with the catalogue.
    #[error("layout {layout}: {message}")]
    Layout {
        /// Layout name.
        layout: String,
        /// What is wrong.
        message: String,
    },
    /// A render option is out of range.
    #[error("render option: {0}")]
    Options(String),
    /// The render would exceed the memory ceiling (goal-prompt §8).
    #[error("resource limit: {0}")]
    ResourceLimit(String),
}

fn join_issues(issues: &[crate::validate::Issue]) -> String {
    issues
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}
