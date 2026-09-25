//! Generation stages: continent (`logic/01`), area (`logic/02`), block (`logic/03`).
//!
//! Every stage is a pure function over prior stages' outputs; the
//! orchestrator is the only caller that touches disk.

// `code-prefs.md` §Q1 bans unwrap/expect *outside* `#[cfg(test)]`.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod terrain_interpolation;

pub mod area;
pub mod block;
pub mod continent;
pub mod hydrology;
pub mod noise;
pub mod orchestrator;
pub mod spectral;
pub mod spectral_composition;

pub use orchestrator::{generate_world, generate_world_with_limits, GenError, HydrologyLimits};
