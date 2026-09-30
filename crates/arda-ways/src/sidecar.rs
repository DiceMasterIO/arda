//! The scene sidecar: per-square SRD semantics the layout's `Square` cannot
//! hold (it is `deny_unknown_fields`), in the vocabulary's sidecar
//! convention: same width and height as the layout, row-major (goal 48).
//!
//! [`Sidecar::to_rules`] is the thin adapter to the product's one rules
//! schema, `arda-scene`'s `RulesSidecar` format 2 (logic/12 §scene-sidecar,
//! I9): the six shared fields map one to one, the ways extras
//! (`feature`, `road_class`, `deck_elevation_ft`) go in each cell's `ext`
//! map and the edge rules in `edges`.

use crate::input::RoadClass;
use arda_tactical::layout::EdgeAxis;
use serde::{Deserialize, Serialize};

/// Sidecar schema version.
pub const SIDECAR_VERSION: u32 = 1;

/// What a square is, as far as ways are concerned.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    /// Nothing from this stage.
    #[default]
    None,
    /// Travelled road surface.
    Road,
    /// Wheel-rutted track surface.
    Ruts,
    /// Verge beside the surface.
    Verge,
    /// Side ditch (difficult terrain).
    Ditch,
    /// Shoulder graded between road and ground.
    Shoulder,
    /// Cut face above a benched road (difficult terrain).
    CutFace,
    /// River bank.
    Bank,
    /// Bridge deck over water.
    Bridge,
    /// Bridge abutment on land.
    Abutment,
    /// Ford through shallow water (difficult terrain).
    Ford,
    /// Dry gravel bar beside a ford.
    GravelBar,
    /// Ferry landing stage over water.
    Landing,
    /// Under the ferry's rope line.
    FerryRope,
    /// Inside a toll house or waystation.
    Building,
}

/// SRD cover a square grants (vocabulary I16).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cover {
    /// No cover.
    #[default]
    None,
    /// Half cover.
    Half,
    /// Three-quarters cover.
    ThreeQuarters,
    /// Total cover (`full` is accepted as an alias).
    #[serde(alias = "full")]
    Total,
}

/// Per-square rules. The first six fields mirror `arda-scene`'s
/// `RulesSidecar` names (vocabulary I9); the rest are ways-specific and
/// omitted when empty.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SquareRules {
    /// SRD difficult terrain: fords, ditches, cut faces and any undecked
    /// water shallower than [`crate::input::SWIM_FT`] (wading).
    pub difficult: bool,
    /// Water depth in feet (under a deck, the water below it). Five feet or
    /// more is swimming.
    pub water_depth_ft: u8,
    /// Cover granted by the square itself.
    pub cover: Cover,
    /// The square blocks line of sight.
    pub blocks_sight: bool,
    /// The square blocks movement.
    pub blocks_movement: bool,
    /// A walkable deck over the square's water (bridges, landing stages):
    /// creatures stand at `deck_elevation_ft`, not in the water below.
    pub deck: bool,
    /// Deck surface elevation in feet (absolute, 5-ft steps), when `deck`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deck_elevation_ft: Option<i16>,
    /// The square's role for this stage.
    #[serde(default, skip_serializing_if = "is_none_feature")]
    pub feature: Feature,
    /// Road class of the surface, verge or crossing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub road_class: Option<RoadClass>,
}

// serde's skip_serializing_if hands a reference.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_none_feature(f: &Feature) -> bool {
    *f == Feature::None
}

/// Why an edge exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeRole {
    /// Bridge parapet: blocks movement, not sight.
    Parapet,
    /// Low retaining wall at a bench's outer edge: blocks movement, not sight.
    Retaining,
    /// Building wall, door or window.
    Building,
}

/// Movement and sight rules for one wall segment of the layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeRule {
    /// Square column (as in `WallSegment`).
    pub x: u32,
    /// Square row.
    pub y: u32,
    /// Which edge.
    pub axis: EdgeAxis,
    /// Why it is there.
    pub role: EdgeRole,
    /// Blocks movement across the edge.
    pub blocks_movement: bool,
    /// Blocks line of sight across the edge.
    pub blocks_sight: bool,
}

/// The sidecar document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sidecar {
    /// [`SIDECAR_VERSION`].
    pub format_version: u32,
    /// Layout name.
    pub layout: String,
    /// Width in squares (equals the layout's).
    pub width: u32,
    /// Height in squares (equals the layout's).
    pub height: u32,
    /// Row-major rules, `width × height`.
    pub squares: Vec<SquareRules>,
    /// Rules for the wall segments this stage added.
    pub edges: Vec<EdgeRule>,
}

impl Sidecar {
    /// Rules of square `(x, y)`.
    #[must_use]
    pub fn at(&self, x: u32, y: u32) -> Option<&SquareRules> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.squares
            .get(y as usize * self.width as usize + x as usize)
    }

    /// Pretty JSON with a trailing newline.
    ///
    /// # Errors
    /// Serialisation failure.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self).map(|s| s + "\n")
    }
}

impl From<Cover> for arda_scene::CoverLevel {
    fn from(c: Cover) -> Self {
        match c {
            Cover::None => Self::None,
            Cover::Half => Self::Half,
            Cover::ThreeQuarters => Self::ThreeQuarters,
            Cover::Total => Self::Total,
        }
    }
}

impl From<EdgeRole> for arda_scene::EdgeRole {
    fn from(r: EdgeRole) -> Self {
        match r {
            EdgeRole::Parapet => Self::Parapet,
            EdgeRole::Retaining => Self::Retaining,
            EdgeRole::Building => Self::Building,
        }
    }
}

impl SquareRules {
    /// This square as a format-2 rules cell: only squares a way touches
    /// (`feature != none`) state anything, so the refine rules underneath
    /// stay in force elsewhere (logic/09 §reservations).
    #[must_use]
    pub fn to_cell(&self) -> arda_scene::RulesCell {
        let mut c = arda_scene::RulesCell::default();
        if self.feature == Feature::None {
            return c;
        }
        c.difficult = Some(self.difficult);
        c.water_depth_ft = Some(self.water_depth_ft);
        c.cover = Some(self.cover.into());
        c.blocks_sight = Some(self.blocks_sight);
        c.blocks_movement = Some(self.blocks_movement);
        c.deck = Some(self.deck);
        if let Ok(v) = serde_json::to_value(self.feature) {
            c.set_ext("feature", v);
        }
        if let Some(rc) = self.road_class.and_then(|r| serde_json::to_value(r).ok()) {
            c.set_ext("road_class", rc);
        }
        if let Some(d) = self.deck_elevation_ft {
            c.set_ext("deck_elevation_ft", serde_json::Value::from(d));
        }
        c
    }
}

impl Sidecar {
    /// The `arda-scene` `RulesSidecar` format 2 of this sidecar (adapter
    /// A8). Parapets and retaining walls block movement, not sight, and give
    /// half cover (logic/12 §scene-sidecar, "low wall").
    #[must_use]
    pub fn to_rules(&self) -> arda_scene::RulesSidecar {
        let mut r = arda_scene::RulesSidecar::empty(self.width, self.height);
        r.squares = self.squares.iter().map(SquareRules::to_cell).collect();
        r.edges = self
            .edges
            .iter()
            .map(|e| arda_scene::EdgeRule {
                x: e.x,
                y: e.y,
                axis: e.axis,
                role: e.role.into(),
                blocks_movement: e.blocks_movement,
                blocks_sight: e.blocks_sight,
                cover: match e.role {
                    EdgeRole::Building => arda_scene::CoverLevel::Total,
                    EdgeRole::Parapet | EdgeRole::Retaining => arda_scene::CoverLevel::Half,
                },
            })
            .collect();
        r
    }
}
