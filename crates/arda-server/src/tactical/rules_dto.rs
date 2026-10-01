//! Wire mirror of `arda-scene`'s `RulesSidecar` format 2 (I9; logic/12
//! §scene-sidecar), so ts-rs can describe it without the scene crate
//! depending on ts-rs (logic/16 §api-bindings). A test round-trips a
//! sidecar through the mirror to keep the two in step.

use super::dto::EdgeAxisDto;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// SRD 5.1 degrees of cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CoverLevelDto {
    /// No cover.
    None,
    /// Half cover: +2 AC and Dexterity saves.
    Half,
    /// Three-quarters cover: +5.
    ThreeQuarters,
    /// Total cover: cannot be targeted directly.
    Total,
}

/// Why a rules edge exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EdgeRoleDto {
    /// Bridge parapet: blocks movement, not sight.
    Parapet,
    /// Low retaining wall.
    Retaining,
    /// A building's wall, door or window.
    Building,
    /// Hedge, drystone wall or fence around a field.
    Boundary,
    /// A gate in a field boundary.
    Gate,
}

/// One square's rules; an absent field means "no opinion".
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RulesCellDto {
    /// Difficult terrain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub difficult: Option<bool>,
    /// Water depth in feet: under 5 wades, 5 or more swims (I15).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub water_depth_ft: Option<u8>,
    /// Cover the square's contents grant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub cover: Option<CoverLevelDto>,
    /// Heavily obscured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub blocks_sight: Option<bool>,
    /// Lightly obscured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub lightly_obscured: Option<bool>,
    /// Impassable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub blocks_movement: Option<bool>,
    /// A walkable deck (bridge, jetty) over the square.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub deck: Option<bool>,
    /// Layer extras: `feature`, `road_class`, `deck_elevation_ft` (ways),
    /// `crop`, `field`, `furrow` (fields), `building` (town).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "Record<string, unknown>")]
    pub ext: Option<BTreeMap<String, serde_json::Value>>,
}

/// Movement and sight rules of one unit edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeRuleDto {
    /// Square column.
    pub x: u32,
    /// Square row.
    pub y: u32,
    /// Which edge of the square.
    pub axis: EdgeAxisDto,
    /// Why it is there.
    pub role: EdgeRoleDto,
    /// Blocks movement across the edge.
    pub blocks_movement: bool,
    /// Blocks sight across the edge.
    pub blocks_sight: bool,
    /// Cover it grants.
    #[serde(default = "no_cover")]
    pub cover: CoverLevelDto,
}

const fn no_cover() -> CoverLevelDto {
    CoverLevelDto::None
}

/// The per-square rules sidecar, format 2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RulesSidecarDto {
    /// Always 2.
    pub format_version: u32,
    /// Width in squares (the layout's).
    pub width: u32,
    /// Height in squares.
    pub height: u32,
    /// Row-major cells.
    pub squares: Vec<RulesCellDto>,
    /// Low edge features that are not wall-kit walls (always present on
    /// the wire; the domain JSON omits an empty list).
    #[serde(default)]
    pub edges: Vec<EdgeRuleDto>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_scene::{CoverLevel, EdgeRole, EdgeRule, RulesSidecar};
    use arda_tactical::layout::EdgeAxis;
    use serde_json::to_value;

    #[test]
    fn a_format_2_sidecar_round_trips_through_the_mirror() {
        let mut r = RulesSidecar::empty(3, 1);
        r.squares[0].difficult = Some(true);
        r.squares[0].cover = Some(CoverLevel::ThreeQuarters);
        r.squares[1].deck = Some(true);
        r.squares[1].water_depth_ft = Some(7);
        r.squares[1].set_ext("feature", serde_json::json!("bridge"));
        r.squares[2].lightly_obscured = Some(true);
        r.squares[2].blocks_sight = Some(false);
        r.squares[2].blocks_movement = Some(false);
        for role in [
            EdgeRole::Parapet,
            EdgeRole::Retaining,
            EdgeRole::Building,
            EdgeRole::Boundary,
            EdgeRole::Gate,
        ] {
            r.edges.push(EdgeRule {
                x: 1,
                y: 0,
                axis: EdgeAxis::Vertical,
                role,
                blocks_movement: true,
                blocks_sight: false,
                cover: CoverLevel::Half,
            });
        }
        let v = to_value(&r).unwrap();
        let m: RulesSidecarDto = serde_json::from_value(v.clone()).unwrap();
        assert_eq!(to_value(m).unwrap(), v);
    }
}
