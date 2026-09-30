//! Shared foundations for Arda's product layers (integration plan A6, I21).
//!
//! One place for the things several crates must agree on byte for byte:
//!
//! - [`hash`]: the stable hash and subseed functions with a frozen
//!   derivation scheme (`logic/09` §hash; `logic/13` §npc-seed);
//! - [`ids`]: u64 id newtypes that travel as JSON strings
//!   (`vocabulary.md` "Ids (I5)"; `logic/16` §api-conventions);
//! - the canonical enums [`Biome`], [`RoadClass`], [`BuildingFunction`], [`Craft`],
//!   [`Cover`] and [`LandUse`] (`vocabulary.md` canonical conventions,
//!   `logic/08` §roads and §landuse, `logic/12` §scene-cover).
//!
//! The crate is serde-only and light: no I/O, no allocation-heavy work, and
//! its only other dependency is `blake3`.

#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod biome;
pub mod cover;
pub mod function;
pub mod hash;
pub mod ids;
pub mod land_use;
pub mod road;

pub use biome::Biome;
pub use cover::Cover;
pub use function::{BuildingFunction, Craft};
pub use hash::{digest, hash, mix, subseed, tag};
pub use ids::{BuildingId, NpcId, RealmId, SettlementId};
pub use land_use::LandUse;
pub use road::RoadClass;
