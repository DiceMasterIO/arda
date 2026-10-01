//! Generic deterministic wave-function collapse for arda (goals 42, 44, 47).
//!
//! A [`Problem`] is a grid of cells, each with a [`TileSet`] of options,
//! [`Rules`] saying which tiles may meet across each side and **edge kind**
//! (open floor, wall, door, border), integer weights, anchors and limits.
//! [`solve`] collapses it deterministically from `(seed, key, attempt)` with
//! bounded attempts; a [`Failure`] tells the caller to use its relaxed fill
//! and mark it for review (goal 47). Towns (`arda-town`) solve interiors,
//! streets and yards with it; `arda-refine` shares its hashing and work
//! lists ([`hash`], [`frontier`]) for its corner-lattice terrain solver.

#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod check;
pub mod frontier;
pub mod hash;
pub mod rules;
pub mod set;
pub mod solve;

pub use check::violations;
pub use rules::{Dir, Rules};
pub use set::{TileSet, MAX_TILES};
pub use solve::{solve, Anchor, Failure, Limit, Problem, Solution, Weight, MAX_ATTEMPTS};

/// Errors building a WFC vocabulary.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WfcError {
    /// Too many tiles or no edge kinds.
    #[error("a WFC vocabulary needs 1..=256 tiles and at least one edge kind, not {tiles} tiles and {kinds} kinds")]
    Vocabulary {
        /// Tiles asked for.
        tiles: usize,
        /// Edge kinds asked for.
        kinds: usize,
    },
}
