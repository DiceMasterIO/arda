//! Town layouts for arda (goals 36, 44, 45, 64).
//!
//! [`plan::generate`] turns a settlement record ([`site::TownSite`]) and its
//! terrain ([`site::TerrainInput`]) into a [`plan::TownPlan`] in world
//! metres: main streets along the entering roads, a market square at the
//! focal point, lanes, burgage plots, districts, a wall with gates and a
//! building on every plot. [`block::generate`] cuts any window of 5-ft
//! squares out of the plan as an `arda_tactical::TacticalLayout` with
//! walls, doors, rooms, furniture, dressing and lights, plus a sidecar that
//! keeps the plan's building ids.
//!
//! Everything is deterministic from the world seed and the site id.

#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod block;
pub mod error;
pub mod function;
pub mod geom;
pub mod ids;
pub mod num;
pub mod plan;
pub mod render;
pub mod rng;
pub mod samples;
pub mod site;

pub use error::TownError;
pub use function::BuildingFunction;
pub use plan::{generate, TownPlan};
pub use site::{TerrainInput, TownSite};
