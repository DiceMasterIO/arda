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

/// Full per-square rules as the refiner computes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SquareRules {
    /// Difficult terrain.
    pub difficult: bool,
    /// Water depth, feet.
    pub water_depth_ft: u8,
    /// Cover.
    pub cover: CoverRule,
    /// Heavily obscured by canopy.
    pub blocks_sight: bool,
    /// Lightly obscured: a thinner canopy or reeds.
    pub lightly_obscured: bool,
    /// Impassable.
    pub blocks_movement: bool,
    /// Relaxed-fill square needing review.
    pub review: bool,
}

impl SquareRules {
    /// The sidecar cell: every field stated explicitly.
    #[must_use]
    pub fn cell(&self) -> RulesCell {
        RulesCell {
            difficult: Some(self.difficult),
            water_depth_ft: Some(self.water_depth_ft),
            cover: Some(self.cover),
            blocks_sight: Some(self.blocks_sight),
            lightly_obscured: Some(self.lightly_obscured),
            blocks_movement: Some(self.blocks_movement),
            deck: None,
            ext: None,
        }
    }
}
