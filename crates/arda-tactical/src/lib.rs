//! Tactical battle maps for arda: the asset catalogue format, its validator,
//! a code-generated placeholder library, the layout input type and a
//! deterministic compositor with a lighting pass (goals 58–63).
//!
//! The catalogue format is documented in `crates/arda-tactical/README.md`.

#![deny(unsafe_code)]
// `code-prefs.md` §Q1 bans unwrap/expect *outside* `#[cfg(test)]`.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod catalog;
pub mod compose;
pub mod error;
pub mod layout;
pub mod layouts;
pub mod library;
pub mod noise;
pub mod placeholders;
pub mod raster;
pub mod validate;

#[cfg(test)]
mod validate_tests;

pub use catalog::{Asset, AssetClass, Catalog, Layer, WallRole};
pub use compose::world_tint::WorldTint;
pub use compose::{render, render_region, render_region_tinted, render_with, RenderOptions, Style};
pub use error::TacticalError;
pub use layout::TacticalLayout;
pub use library::Library;
pub use raster::Rgba;
pub use validate::{Issue, Rule, Thresholds};
