//! World-derived tactical blocks (goals 42, 46, 48 and 67; logic/09, 12
//! and 16 §api-tactical).
//!
//! A [`Pipeline`] refines any window of world squares with `arda-refine`,
//! caches the refined 64 × 64 blocks, and composes the optional
//! [`Overlays`] onto the result in the fixed order ways, then fields, then
//! town, by the reservation precedence of logic/09 §reservations. The
//! output, [`Composed`], is one `TacticalLayout` (with its world `origin`,
//! I10) and one `arda-scene` `RulesSidecar` format 2 (I9), so the painted
//! image and the scene data come from the same layout (goal 48).
//!
//! [`society::SocietyOverlays`] fills the [`Overlays`] seam from the
//! world's society data (adapter A9): settle's roads and crossings through
//! `arda-ways`, its land use through `arda-fields` and each settlement's
//! town plan through `arda-town`. [`demo::DemoOverlays`] keeps the
//! synthetic samples the three crates ship, anchored at a chosen cell.

#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod demo;
pub mod overlays;
pub mod pipeline;
pub mod society;

pub use overlays::{OverlayCtx, OverlayLayer, Overlays, Owner};
pub use pipeline::{Composed, Pipeline, SQUARES_PER_CELL};

/// Why a block could not be composed.
#[derive(Debug, thiserror::Error)]
pub enum BlocksError {
    /// The window is empty, too large or outside the world.
    #[error("window {gsx0},{gsy0} {w}x{h}: {reason}")]
    Window {
        /// First square column.
        gsx0: i64,
        /// First square row.
        gsy0: i64,
        /// Width in squares.
        w: u32,
        /// Height in squares.
        h: u32,
        /// Why it was refused.
        reason: String,
    },
    /// The cell holds no land square (open sea, logic/09 Branches).
    #[error("cell {gx},{gy} is open sea: no land square in the block")]
    OpenSea {
        /// Cell column.
        gx: i64,
        /// Cell row.
        gy: i64,
    },
    /// Refinement failed.
    #[error(transparent)]
    Refine(#[from] arda_refine::RefineError),
    /// An overlay failed.
    #[error("{layer} overlay: {message}")]
    Overlay {
        /// `ways`, `fields` or `town`.
        layer: &'static str,
        /// What went wrong.
        message: String,
    },
    /// A sidecar did not fit its layout.
    #[error(transparent)]
    Scene(#[from] arda_scene::SceneError),
    /// A cache lock was poisoned by a panicking thread.
    #[error("a block cache lock was poisoned")]
    Poisoned,
}
