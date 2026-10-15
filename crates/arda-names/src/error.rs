//! Errors for the naming crate.

use thiserror::Error;

/// Something a caller asked for does not exist.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NamesError {
    /// No preset has this key.
    #[error("unknown naming preset `{0}`")]
    UnknownPreset(String),
    /// No meaning has this key.
    #[error("unknown meaning `{0}`")]
    UnknownMeaning(String),
}
