//! The layout schema: `arda-tactical`'s `TacticalLayout` itself (adapter
//! A7 of the integration plan). The crate used to emit a serde mirror of it;
//! it now builds the real type, so the JSON is the consumer's by
//! construction. Per-square SRD rules go in `arda-scene`'s `RulesSidecar`
//! (see [`crate::rules`]).

pub use arda_tactical::catalog::AssetClass;
pub use arda_tactical::layout::{
    AssetRef, EdgeAxis, LightSource, Placement, Square, TacticalLayout, WallSegment,
};
