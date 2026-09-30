//! Settlement products of a generated world (goals 45, 51–57; logic/10,
//! 13 and 14; integration adapters A2, A4, A5, A13, A14).
//!
//! `arda-settle` writes `<world>/society/` (settlements, roads, realms,
//! land use). This crate derives what a settlement's people and tactical
//! maps need from it, deterministically from the world seed:
//!
//! - [`terrain`]: a settlement's surroundings as `arda-town` terrain,
//!   sampled from the world's cells in the I1 frame;
//! - [`rivers`]: the refined rivers a plan is drawn on, the same pieces
//!   and water arda-refine draws;
//! - [`town`]: its town plan and the plan's buildings as `arda-npc`
//!   `BuildingSpec`s (A4), or buildings derived from the settlement's mix
//!   when no plan can be drawn;
//! - [`build`]: `arda society build`: plans every settlement, simulates
//!   `arda-society` with the plan buildings as explicit buildings (A14),
//!   and stores only the notables (goal 56) in `society/notables.json`;
//! - [`people`]: a settlement's population on demand, with society's role
//!   slots as its notables and commoners regenerated (goal 56).

#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod build;
pub mod files;
pub mod people;
pub mod rivers;
pub mod shared;
pub mod terrain;
pub mod town;

pub use build::{build, BuildReport};
pub use files::{NotablesFile, SocietyFiles, StoredNotable};
pub use people::World;

/// Why a settlement product could not be derived.
#[derive(Debug, thiserror::Error)]
pub enum PeopleError {
    /// A file could not be read or written.
    #[error("{path}: {source}")]
    Io {
        /// The file involved.
        path: String,
        /// OS error.
        #[source]
        source: std::io::Error,
    },
    /// A file is not valid JSON of the expected shape.
    #[error("{path}: {message}")]
    Format {
        /// The file involved.
        path: String,
        /// What is wrong.
        message: String,
    },
    /// No settlement has this id.
    #[error("unknown settlement {0}")]
    UnknownSettlement(u64),
    /// The world could not be loaded or read.
    #[error("world: {0}")]
    World(String),
    /// The society simulation refused its input.
    #[error(transparent)]
    Society(#[from] arda_society::SocietyError),
    /// The NPC generator refused its input.
    #[error(transparent)]
    Npc(#[from] arda_npc::NpcError),
    /// The settlement stage failed.
    #[error(transparent)]
    Settle(#[from] arda_settle::SettleError),
}

impl PeopleError {
    pub(crate) fn io(path: &std::path::Path, source: std::io::Error) -> Self {
        Self::Io {
            path: path.display().to_string(),
            source,
        }
    }

    pub(crate) fn format(path: &std::path::Path, message: impl std::fmt::Display) -> Self {
        Self::Format {
            path: path.display().to_string(),
            message: message.to_string(),
        }
    }
}
