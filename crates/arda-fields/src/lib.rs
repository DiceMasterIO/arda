//! Tactical land use around settlements (goals 39 and 43): the countryside
//! between villages at 5-ft scale.
//!
//! [`fields_window`] turns a 100 m land-use raster, a terrain callback, the
//! culture, wealth, settlements and roads into a [`TacticalLayout`]:
//! open-field strips near villages, enclosed fields behind hedgerows or
//! drystone walls with gates, pasture with livestock, hay meadows, orchards,
//! woodland with a scrub fringe, farmsteads with lanes to the road,
//! watermills with leats, and mines and quarries with spoil heaps.
//! [`generate`] also returns the SRD rules sidecar.
//!
//! Everything is computed on one global square lattice from the seed, so
//! windows agree exactly along shared edges. See `README.md`.

#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod boundary;
pub mod compounds;
pub mod degrade;
pub mod dress;
pub mod error;
pub mod fields;
pub mod fringe;
pub mod geom;
pub mod input;
pub mod linear;
pub mod overlay;
pub mod partition;
pub mod plan;
pub mod sidecar;
pub mod strips;
pub mod supplement;
pub mod synthetic;
pub mod window;

pub use arda_tactical::TacticalLayout;
pub use error::FieldsError;
pub use input::{
    FieldInputs, LandUse, LandUseGrid, LandUseMap, Region, RiverLine, Road, RoadClass, Settlement,
    Terrain, TerrainSample, Tier,
};
pub use window::{fields_window, generate, FieldsWindow};
