//! The SRD rules sidecar: per-square terrain rules and per-edge wall rules
//! for a window, with the field and compound records behind them.
//!
//! `Square` in the layout denies unknown fields, so rules live here (same
//! width and height, row-major), as `vocabulary.md` requires.

use crate::compounds::CompoundKind;
use crate::fields::{Crop, FieldKind};
use arda_tactical::catalog::WallRole;
use arda_tactical::layout::EdgeAxis;
use serde::Serialize;

/// The sidecar schema version.
pub const SIDECAR_VERSION: u32 = 1;

/// SRD cover levels in the canonical spelling (`vocabulary.md`, I16).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SrdCover {
    /// No cover.
    #[default]
    None,
    /// Half cover (+2 AC).
    Half,
    /// Three-quarters cover (+5 AC).
    ThreeQuarters,
    /// Total cover.
    Total,
}

impl SrdCover {
    /// The catalogue's cover level (`total` is the catalogue's `full`).
    #[must_use]
    pub fn catalog(self) -> arda_tactical::catalog::Cover {
        use arda_tactical::catalog::Cover;
        match self {
            Self::None => Cover::None,
            Self::Half => Cover::Half,
            Self::ThreeQuarters => Cover::ThreeQuarters,
            Self::Total => Cover::Full,
        }
    }
}

/// SRD rules and records for one window.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Sidecar {
    /// Schema version, [`SIDECAR_VERSION`].
    pub format_version: u32,
    /// Layout name.
    pub name: String,
    /// Generation seed, as a string (I17).
    pub seed: String,
    /// Width in squares.
    pub width: u32,
    /// Height in squares.
    pub height: u32,
    /// Global square of the window's north-west corner.
    pub origin_square: [i64; 2],
    /// Metres per square.
    pub square_m: f64,
    /// Row-major square rules.
    pub squares: Vec<SquareRules>,
    /// Wall, fence, hedge and gate rules, in layout edge coordinates.
    pub edges: Vec<EdgeRules>,
    /// Fields touching the window.
    pub fields: Vec<FieldRecord>,
    /// Compounds touching the window.
    pub compounds: Vec<CompoundRecord>,
}

/// Rules of one square, with the `RulesSidecar` field names (I9).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SquareRules {
    /// Canonical ground key (before any library fallback).
    pub ground: String,
    /// Difficult terrain (ploughland, scrub, scree, mud, shallow water,
    /// bushes).
    pub difficult: bool,
    /// Water depth in feet; under 5 is shallow (wade), 5 or more deep (swim).
    pub water_depth_ft: u8,
    /// Cover from what stands on the square.
    pub cover: SrdCover,
    /// Something on the square blocks sight.
    pub blocks_sight: bool,
    /// Something on the square blocks movement.
    pub blocks_movement: bool,
    /// A walkable deck over water (bridges, docks); none in the countryside.
    pub deck: bool,
    /// Crop, where farmed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crop: Option<Crop>,
    /// Furrow direction (unit vector, y south), where ploughed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub furrow: Option<[f32; 2]>,
    /// Field id (a u64 as a string), where in a field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

/// Rules of one edge segment.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EdgeRules {
    /// Square column (layout coordinates).
    pub x: u32,
    /// Square row (layout coordinates).
    pub y: u32,
    /// Which edge.
    pub axis: EdgeAxis,
    /// Role.
    pub kind: WallRole,
    /// Kit.
    pub kit: String,
    /// Blocks line of sight.
    pub blocks_sight: bool,
    /// Blocks movement.
    pub blocks_movement: bool,
    /// Cover to a creature behind it.
    pub cover: SrdCover,
    /// Crossing it costs climbing movement (low walls, hurdles).
    pub climb: bool,
    /// A door or gate that can be opened.
    pub openable: bool,
}

/// A field record.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FieldRecord {
    /// Stable id (a u64 as a string).
    pub id: String,
    /// Use.
    pub kind: FieldKind,
    /// Crop.
    pub crop: Crop,
    /// Boundary kit (empty for open land).
    pub kit: String,
    /// Area in hectares.
    pub hectares: f64,
    /// Whether it is enclosed.
    pub enclosed: bool,
    /// Number of gate edges.
    pub gates: usize,
    /// Whether it lies wholly inside the window.
    pub complete: bool,
}

/// A compound record.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompoundRecord {
    /// Kind.
    pub kind: CompoundKind,
    /// Land-use cell.
    pub cell: [i64; 2],
    /// Whether a lane joins it to a road.
    pub lane: bool,
}

impl From<SrdCover> for arda_scene::CoverLevel {
    fn from(c: SrdCover) -> Self {
        match c {
            SrdCover::None => Self::None,
            SrdCover::Half => Self::Half,
            SrdCover::ThreeQuarters => Self::ThreeQuarters,
            SrdCover::Total => Self::Total,
        }
    }
}

impl Sidecar {
    /// The `arda-scene` `RulesSidecar` format 2 of this sidecar (adapter A8,
    /// I9). Only squares in `owned` (row-major; the squares this layer
    /// reserves, logic/09 §reservations) state rules; `crop`, `furrow` and
    /// `field` ride in each cell's `ext`. Edges become `boundary` rules, or
    /// `gate` for openable ones.
    #[must_use]
    pub fn to_rules(&self, owned: &[bool]) -> arda_scene::RulesSidecar {
        let mut r = arda_scene::RulesSidecar::empty(self.width, self.height);
        for ((cell, sq), own) in r.squares.iter_mut().zip(&self.squares).zip(owned) {
            if !own {
                continue;
            }
            cell.difficult = Some(sq.difficult);
            cell.water_depth_ft = Some(sq.water_depth_ft);
            cell.cover = Some(sq.cover.into());
            cell.blocks_sight = Some(sq.blocks_sight);
            cell.blocks_movement = Some(sq.blocks_movement);
            cell.deck = Some(sq.deck);
            if let Some(v) = sq.crop.and_then(|c| serde_json::to_value(c).ok()) {
                cell.set_ext("crop", v);
            }
            if let Some(f) = sq.furrow {
                cell.set_ext("furrow", serde_json::json!(f));
            }
            if let Some(f) = &sq.field {
                cell.set_ext("field", serde_json::Value::from(f.clone()));
            }
        }
        r.edges = self
            .edges
            .iter()
            .map(|e| arda_scene::EdgeRule {
                x: e.x,
                y: e.y,
                axis: e.axis,
                role: if e.openable {
                    arda_scene::EdgeRole::Gate
                } else {
                    arda_scene::EdgeRole::Boundary
                },
                blocks_movement: e.blocks_movement,
                blocks_sight: e.blocks_sight,
                cover: e.cover.into(),
            })
            .collect();
        r
    }
}

/// The rules of an edge by kit and role (the SRD reading of each kit).
#[must_use]
pub fn edge_rules(kit: &str, role: WallRole) -> (bool, bool, SrdCover, bool, bool) {
    // (blocks_sight, blocks_movement, cover, climb, openable)
    let field_kit = matches!(kit, "hedge" | "drystone" | "wattle");
    match (kit, role) {
        (_, WallRole::Gate) if field_kit => (false, false, SrdCover::Half, false, true),
        ("hedge", _) => (true, true, SrdCover::ThreeQuarters, false, false),
        ("drystone" | "wattle", _) => (false, false, SrdCover::Half, true, false),
        (_, WallRole::Window) => (false, true, SrdCover::ThreeQuarters, false, false),
        (_, WallRole::Door | WallRole::Gate) => (true, true, SrdCover::Total, false, true),
        _ => (true, true, SrdCover::Total, false, false),
    }
}

/// The rules of what stands on a square, by placed asset id:
/// `(cover, blocks_sight, blocks_movement, difficult)`.
#[must_use]
pub fn placement_rules(id: &str) -> (SrdCover, bool, bool, bool) {
    match id {
        _ if id.starts_with("veg.tree_") => (SrdCover::Half, false, false, false),
        "veg.boulder" | "veg.rock_large" => (SrdCover::ThreeQuarters, false, true, false),
        "veg.bush" | "veg.bush_flowering" | "veg.fern" | "veg.heather" | "veg.fallen_log" => {
            (SrdCover::Half, false, false, true)
        }
        "veg.stones" | "veg.rock_small" | "veg.scree_patch" | "veg.stump" => {
            (SrdCover::None, false, false, true)
        }
        "prop.waterwheel" | "prop.crane" => (SrdCover::Total, true, true, false),
        "prop.hay_bale" | "prop.trough" | "prop.cart" | "prop.haycart" | "prop.woodpile"
        | "prop.crate" | "prop.barrel" | "prop.sacks" | "prop.well" | "prop.millstone"
        | "prop.hearth" | "prop.table" | "prop.bed" | "prop.chest" | "prop.cupboard" => {
            (SrdCover::Half, false, true, false)
        }
        _ => (SrdCover::None, false, false, false),
    }
}

/// Whether a ground key is difficult terrain.
#[must_use]
pub fn difficult_ground(ground: &str) -> bool {
    matches!(
        ground,
        "farmland" | "scrub" | "scree" | "mud" | "marsh" | "reed_bed" | "rock" | "cliff"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hedges_block_except_at_gates_and_walls_give_half_cover() {
        let hedge = edge_rules("hedge", WallRole::Run);
        assert!(hedge.0 && hedge.1);
        let gate = edge_rules("hedge", WallRole::Gate);
        assert!(!gate.0 && !gate.1);
        let wall = edge_rules("drystone", WallRole::Run);
        assert_eq!(wall.2, SrdCover::Half);
        assert!(!wall.0);
        let door = edge_rules("timber", WallRole::Door);
        assert!(door.1 && door.4);
    }
}
