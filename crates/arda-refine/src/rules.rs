//! The SRD 5.1 rules sidecar (goal 48) and the refiner's own metadata.
//!
//! `Square` denies unknown fields, so per-square rules travel in
//! `arda-scene`'s `RulesSidecar`, format 2 (logic/09 §rules-sidecar, I9):
//! same width, height and row-major order as the layout. The world origin
//! is the layout's own `origin` (I10); everything else the refiner knows
//! (review flags, WFC records, placement tags) goes in [`MapMeta`].

use serde::{Deserialize, Serialize};

pub use arda_scene::SIDECAR_FORMAT_VERSION;
/// Metadata format tag. Version 2 drops `origin`, which the layout carries.
pub const META_FORMAT: &str = "arda-refine.meta/2";

/// SRD degrees of cover: `arda-scene`'s `CoverLevel` (`total`, alias
/// `full`, convention I16).
pub type CoverRule = arda_scene::CoverLevel;

pub use arda_scene::{RulesCell, RulesSidecar};

/// Metadata for one placement, parallel to the layout's placements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacementMeta {
    /// Semantic tag such as `tree:broadleaf:large`.
    pub tag: String,
    /// Cover the object grants.
    pub cover: CoverRule,
    /// Whether it makes its square difficult terrain.
    pub difficult: bool,
}

/// How one block was generated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockMeta {
    /// Global cell.
    pub cell: [i64; 2],
    /// WFC attempts used.
    pub attempts: u8,
    /// Local WFC repairs.
    pub repairs: u32,
    /// Whether the relaxed fill was used (goal 47: review this block).
    pub relaxed: bool,
}

/// The refiner's metadata beside a layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapMeta {
    /// Always [`META_FORMAT`].
    pub format: String,
    /// Matches the layout name.
    pub name: String,
    /// Width in squares.
    pub width: u32,
    /// Height in squares.
    pub height: u32,
    /// Whether any block needs review.
    pub relaxed: bool,
    /// Row-major indices of squares whose tile came from a relaxed fill.
    pub review: Vec<u32>,
    /// Per-block generation records.
    pub blocks: Vec<BlockMeta>,
    /// Placement metadata, parallel to the layout's placements.
    pub placements: Vec<PlacementMeta>,
}

/// Per-square rules as the refiner knows them: the ground and water it
/// laid, the canopy its trees close overhead and the SRD cover of its
/// trunks and rocks. Blocking and the difficulty of undergrowth are the
/// catalogue's to say: the scene folds them in from the placements
/// (logic/12 §scene-sidecar, the rules contract), so refine never states
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SquareRules {
    /// Difficult ground (scree, mud, marsh, shallow water).
    pub difficult: bool,
    /// Water depth, feet.
    pub water_depth_ft: u8,
    /// Cover of the trunk or rock on the square (SRD: a thick trunk gives
    /// three-quarters); merged with the catalogue's by taking the stronger.
    pub cover: CoverRule,
    /// Heavily obscured by canopy.
    pub blocks_sight: bool,
    /// Lightly obscured: a thinner canopy or reeds.
    pub lightly_obscured: bool,
    /// Relaxed-fill square needing review.
    pub review: bool,
}

impl SquareRules {
    /// The sidecar cell under the rules contract: water depth always (the
    /// refiner laid it), each flag and cover only when they hold, and no
    /// blocking, which comes from the catalogue.
    #[must_use]
    pub fn cell(&self) -> RulesCell {
        RulesCell {
            difficult: RulesCell::flag(self.difficult),
            water_depth_ft: Some(self.water_depth_ft),
            cover: RulesCell::stated_cover(self.cover),
            blocks_sight: RulesCell::flag(self.blocks_sight),
            lightly_obscured: RulesCell::flag(self.lightly_obscured),
            blocks_movement: None,
            deck: None,
            deck_elevation_ft: None,
            ext: None,
        }
    }
}
