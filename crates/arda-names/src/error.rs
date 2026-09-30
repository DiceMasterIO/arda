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
    /// Every name this language can give the place is taken in the scope.
    #[error("no unused {kind} name left in this scope (key {key})")]
    ScopeExhausted {
        /// Place kind, as its key.
        kind: String,
        /// The place's key.
        key: u64,
    },
}
