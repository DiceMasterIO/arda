//! Tactical wire types.
//!
//! [`TacticalLayoutDto`] and its parts mirror `arda_tactical::TacticalLayout`
//! field for field, so ts-rs can describe the layout JSON without the
//! tactical crate depending on ts-rs. A test round-trips every built-in
//! layout through the mirror to keep the two in step.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `GET /v1/tactical/layouts`: the built-in test layouts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, JsonSchema)]
pub struct TacticalLayouts {
    /// Seed of the served world (a decimal string); part of every render seed and cache key.
    pub world_seed: String,
    /// `library_version` of the loaded catalogue; part of every render cache key.
    pub library_version: String,
    /// Output pixels per square clients may request.
    pub ppsq_options: Vec<u32>,
    /// Default output pixels per square.
    pub default_ppsq: u32,
    /// Layouts in name order.
    pub layouts: Vec<TacticalLayoutSummary>,
}

/// One built-in layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, JsonSchema)]
pub struct TacticalLayoutSummary {
    /// Name used in `/v1/tactical/layout/{name}`.
    pub name: String,
    /// Width in 5-ft squares.
    pub width: u32,
    /// Height in 5-ft squares.
    pub height: u32,
    /// Tile pyramid geometry at the default ppsq.
    pub tiles: TacticalTilesDto,
}

/// Tile pyramid geometry of one render.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
pub struct TacticalTilesDto {
    /// Tile edge in pixels (512).
    pub tile_px: u32,
    /// Output pixels per square of the render the pyramid is cut from.
    pub ppsq: u32,
    /// Full-resolution render width, pixels.
    pub image_width_px: u32,
    /// Full-resolution render height, pixels.
    pub image_height_px: u32,
    /// Deepest zoom; it is the render at full resolution.
    pub max_zoom: u32,
    /// Tile file extension: `webp`.
    pub format: String,
}

/// `GET /v1/tactical/cell/{gx}/{gy}` and `GET /v1/tactical/window`: one
/// world-derived block or window (logic/16 §api-tactical).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
pub struct TacticalBlockDto {
    /// Envelope version of this body ([`TACTICAL_FORMAT`]).
    pub tactical_format: u32,
    /// `TacticalLayout` schema version ([`LAYOUT_SCHEMA`]).
    pub layout_schema: u32,
    /// Top-left square in world squares (64 per cell); equals `layout.origin`.
    #[ts(type = "[number, number]")]
    pub origin: [i64; 2],
    /// Width and height in squares.
    pub size: [u32; 2],
    /// Compositor seed of the images (a decimal string); the scene resolves
    /// asset queries with the same seed.
    pub render_seed: String,
    /// What the compositor draws.
    pub layout: TacticalLayoutDto,
    /// Per-square rules, `arda-scene` `RulesSidecar` format 2, or `null`.
    pub rules: Option<super::rules_dto::RulesSidecarDto>,
    /// Provenance: `source`, `relaxed`, `overlays`, `demo_at`, ….
    pub meta: std::collections::BTreeMap<String, String>,
    /// Scene data from the same layout (`arda-scene` `Scene`, format 1:
    /// movement, climb, cover, obscurement, walls, lights, regions, spawn
    /// hints), or `null` when not requested.
    #[ts(as = "Option<super::scene_dto::SceneDto>")]
    #[schemars(with = "Option<super::scene_dto::SceneDto>")]
    pub scene: Option<serde_json::Value>,
    /// Tile pyramid geometry at the requested ppsq.
    pub tiles: TacticalTilesDto,
}

/// Version of the tactical block envelope (logic/16 §api-versioning).
pub const TACTICAL_FORMAT: u32 = 1;
/// Version of the `TacticalLayout` schema; `origin` is additive (I10).
pub const LAYOUT_SCHEMA: u32 = 1;

/// Mirror of `arda_tactical::TacticalLayout`: a W × H battle map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TacticalLayoutDto {
    /// Layout name.
    pub name: String,
    /// Width in squares.
    pub width: u32,
    /// Height in squares.
    pub height: u32,
    /// Row-major squares, `width × height` of them.
    pub squares: Vec<SquareDto>,
    /// Wall, door, window and gate segments on square edges.
    #[serde(default)]
    pub walls: Vec<WallSegmentDto>,
    /// Props and vegetation.
    #[serde(default)]
    pub placements: Vec<PlacementDto>,
    /// Free-standing light sources.
    #[serde(default)]
    pub lights: Vec<LightSourceDto>,
    /// World square of square `(0, 0)` (I10); absent for layouts with no
    /// world position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "[number, number]", optional)]
    pub origin: Option<[i64; 2]>,
}

/// One 5-ft square.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SquareDto {
    /// Ground type key, matching a texture's `ground` field.
    pub ground: String,
    /// Elevation in feet.
    #[serde(default)]
    pub elevation_ft: i16,
    /// Water depth in feet; zero is dry land.
    #[serde(default)]
    pub water_depth_ft: u8,
    /// Climate dryness, `0` (lush) to `255` (parched steppe); omitted when
    /// zero. The compositor tints vegetated ground toward dry grass by it.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[ts(optional, as = "Option<u8>")]
    pub dryness: u8,
}

// serde's `skip_serializing_if` passes a reference.
#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_zero(v: &u8) -> bool {
    *v == 0
}

/// Which edge of a square a segment lies on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EdgeAxisDto {
    /// The north edge of square `(x, y)`.
    Horizontal,
    /// The west edge of square `(x, y)`.
    Vertical,
}

/// Wall-kit piece roles; layouts use the edge roles `run`, `door`, `window`, `gate`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WallRoleDto {
    /// Plain wall along one edge.
    Run,
    /// Doorway along one edge.
    Door,
    /// Wall with a window along one edge.
    Window,
    /// Wide gate along one edge.
    Gate,
    /// Joint with arms east and south.
    Corner,
    /// Joint with arms east, south and west.
    Tee,
    /// Joint with arms in all four directions.
    Cross,
    /// Joint with one arm, east.
    End,
    /// Optional joint on a straight run.
    Post,
}

/// Catalogue asset classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetClassDto {
    /// Tileable ground texture.
    Ground,
    /// Wall-kit piece.
    Wall,
    /// Furniture, goods, vehicles and structures.
    Prop,
    /// Trees, bushes, reeds and rocks.
    Vegetation,
    /// Tileable water texture.
    Water,
}

/// A wall-kit edge feature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WallSegmentDto {
    /// Square column.
    pub x: u32,
    /// Square row.
    pub y: u32,
    /// Which edge.
    pub axis: EdgeAxisDto,
    /// Edge role.
    pub kind: WallRoleDto,
    /// Wall kit name.
    pub kit: String,
    /// Free edge tags: `locked` and `secret` on doors. Absent when empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub tags: Option<Vec<String>>,
}

/// An asset chosen by id or by a deterministic tag query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetRefDto {
    /// A specific asset id.
    Id(String),
    /// Any asset of the class carrying every tag.
    Query {
        /// Restrict to a class.
        #[serde(default)]
        class: Option<AssetClassDto>,
        /// Required tags.
        tags: Vec<String>,
    },
}

/// A prop or vegetation placement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlacementDto {
    /// What to place.
    pub asset: AssetRefDto,
    /// Anchor position in squares (`3.5` is a square centre).
    pub x: f32,
    /// Anchor position in squares.
    pub y: f32,
    /// Clockwise rotation in degrees.
    #[serde(default)]
    pub rotation: u16,
    /// Horizontal mirror, applied before rotation.
    #[serde(default)]
    pub mirror: bool,
}

/// A light source not attached to an asset.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LightSourceDto {
    /// Position in squares.
    pub x: f32,
    /// Position in squares.
    pub y: f32,
    /// Bright-light radius in feet.
    pub radius_ft: u16,
    /// RGB colour.
    pub colour: [u8; 3],
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_tactical::{layouts, AssetClass, TacticalLayout, WallRole};
    use serde_json::{json, to_value, Value};

    fn through_mirror(v: &Value) -> Value {
        let m: TacticalLayoutDto = serde_json::from_value(v.clone()).unwrap();
        to_value(m).unwrap()
    }

    #[test]
    fn every_built_in_layout_round_trips_through_the_mirror() {
        for l in layouts::all() {
            let v = to_value(&l).unwrap();
            assert_eq!(through_mirror(&v), v, "{}", l.name);
        }
    }

    #[test]
    fn world_layouts_round_trip_through_the_mirror_with_origin() {
        let mut l = layouts::by_name("timber_house").unwrap();
        l.origin = Some([-64, 128]);
        let v = to_value(&l).unwrap();
        assert_eq!(through_mirror(&v), v);
        assert_eq!(v["origin"], json!([-64, 128]));
    }

    #[test]
    fn mirror_enums_cover_the_tactical_variants() {
        use AssetClass as C;
        use WallRole as R;
        for c in [C::Ground, C::Wall, C::Prop, C::Vegetation, C::Water] {
            let v = to_value(c).unwrap();
            assert_eq!(
                to_value(serde_json::from_value::<AssetClassDto>(v.clone()).unwrap()).unwrap(),
                v
            );
        }
        let roles = [
            R::Run,
            R::Door,
            R::Window,
            R::Gate,
            R::Corner,
            R::Tee,
            R::Cross,
            R::End,
            R::Post,
        ];
        for r in roles {
            let v = to_value(r).unwrap();
            assert_eq!(
                to_value(serde_json::from_value::<WallRoleDto>(v.clone()).unwrap()).unwrap(),
                v
            );
        }
        let q = json!({"name":"q","width":1,"height":1,"squares":[{"ground":"grass"}],
            "placements":[{"asset":{"query":{"class":"prop","tags":["crate"]}},"x":0.5,"y":0.5}]});
        let real = to_value(serde_json::from_value::<TacticalLayout>(q.clone()).unwrap()).unwrap();
        assert_eq!(through_mirror(&q), real);
    }
}
