//! Wire mirror of `arda-scene`'s `Scene` format 1 (adapter A12; logic/12
//! §scene-format, logic/16 §api-bindings), so ts-rs and schemars describe the
//! scene JSON without the scene crate depending on either. Handlers keep
//! serialising the domain `arda_scene::Scene`; the tests prove the mirror
//! reads and writes that JSON unchanged.
//!
//! Per-square layers are run-length encoded as `[count, value]` pairs
//! ([`Runs`]), squares are `[x, y]` pairs ([`SqDto`]) and the seed is a
//! decimal string (I5, I17).

use super::rules_dto::CoverLevelDto;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A row-major per-square layer as runs `[[count, value], …]`, expanding to
/// `width × height` values (`arda_scene::Grid`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
pub struct Runs<T>(pub Vec<(u32, T)>);

/// A square `[x, y]` (column, row) in the scene's own squares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
pub struct SqDto(pub u32, pub u32);

/// How a creature moves through a square (SRD 5.1 "Movement").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MovementDto {
    /// Ordinary ground.
    Normal,
    /// Difficult terrain: every foot costs 1 extra foot.
    Difficult,
    /// Shallow water (1–4 ft): wading, as difficult terrain.
    Wade,
    /// Water 5 ft deep or more: swimming.
    Swim,
    /// Solid obstacle; cannot be entered.
    Impassable,
}

/// SRD 5.1 vision obscurement of a square.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObscurementDto {
    /// Clear.
    Clear,
    /// Lightly obscured.
    Light,
    /// Heavily obscured or opaque.
    Heavy,
}

/// The type of a wall-edge feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WallKindDto {
    /// Solid wall.
    Wall,
    /// Door; has an open state.
    Door,
    /// Window: blocks movement, not sight.
    Window,
    /// Gate; has an open state.
    Gate,
    /// Secret door; has an open state.
    Secret,
}

/// A wall polyline on grid vertices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneWallDto {
    /// Feature type.
    pub kind: WallKindDto,
    /// Open state of doors, gates and secret doors; absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub open: Option<bool>,
    /// Grid vertices `[x, y]`, at least two.
    pub points: Vec<[u32; 2]>,
    /// Blocks line of sight in its current state.
    pub blocks_sight: bool,
    /// Blocks movement in its current state.
    pub blocks_movement: bool,
    /// Blocks light in its current state.
    pub blocks_light: bool,
    /// Cover it grants when it does not block sight.
    pub cover: CoverLevelDto,
    /// Wall kit name.
    pub kit: String,
}

/// Where a vision blocker comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BlockerKindDto {
    /// A tree canopy.
    Canopy,
    /// A prop that blocks sight.
    Prop,
}

/// A polygon that obscures vision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VisionBlockerDto {
    /// Source class.
    pub kind: BlockerKindDto,
    /// Asset id.
    pub asset: String,
    /// `light` for foliage, `heavy` for opaque.
    pub obscurement: ObscurementDto,
    /// Closed polygon in squares, clockwise in y-down coordinates.
    pub polygon: Vec<[f32; 2]>,
}

/// A light source with SRD bright and dim radii.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneLightDto {
    /// Position in squares.
    pub x: f32,
    /// Position in squares.
    pub y: f32,
    /// Bright-light radius in feet.
    pub bright_ft: u16,
    /// Outer edge of dim light in feet.
    pub dim_ft: u16,
    /// RGB colour.
    pub colour: [u8; 3],
    /// Emitting asset id; absent for a free layout light.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub asset: Option<String>,
}

/// What a region marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RegionKindDto {
    /// Difficult terrain on land.
    Difficult,
    /// Shallow water (wading).
    ShallowWater,
    /// Deep water (swimming).
    DeepWater,
}

/// Merged squares of one kind as closed rings on grid vertices (even-odd fill).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RegionDto {
    /// What the region marks.
    pub kind: RegionKindDto,
    /// Closed rings of `[x, y]` vertices (first point not repeated).
    pub rings: Vec<Vec<[u32; 2]>>,
}

/// A map edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
pub enum EdgeDto {
    /// North (y = 0).
    N,
    /// East.
    E,
    /// South.
    S,
    /// West (x = 0).
    W,
}

/// A run of enterable squares along one map edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeExitDto {
    /// Which edge.
    pub edge: EdgeDto,
    /// First square of the run.
    pub from: SqDto,
    /// Last square of the run (inclusive).
    pub to: SqDto,
}

/// Squares beside a door or gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EntranceDto {
    /// Index into `walls`.
    pub wall: u32,
    /// Enterable squares on either side of it.
    pub squares: Vec<SqDto>,
}

/// Where tokens may be put.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpawnHintsDto {
    /// Open normal-movement squares, row-major.
    pub open: Vec<SqDto>,
    /// Squares by doors and gates.
    pub entrances: Vec<EntranceDto>,
    /// Enterable runs along the map edges, N, E, S, W order.
    pub exits: Vec<EdgeExitDto>,
}

/// Where an NPC's token stands (scene token slot).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TokenSlotDto {
    /// NPC id, a decimal string.
    pub npc_id: String,
    /// Token centre in squares (`3.5` is a square centre).
    pub x: f32,
    /// Token centre in squares.
    pub y: f32,
    /// Facing in degrees clockwise from north, `0..360`.
    pub facing: u16,
}

/// `arda-scene` `Scene` format 1: everything the game needs to run a map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneDto {
    /// Schema version (1).
    pub format_version: u32,
    /// Layout name.
    pub name: String,
    /// Width in squares.
    pub width: u32,
    /// Height in squares.
    pub height: u32,
    /// Seed the asset queries were resolved with, a decimal string.
    pub seed: String,
    /// Library name.
    pub library: String,
    /// Library art version.
    pub library_version: String,
    /// Per-square movement.
    pub movement: Runs<MovementDto>,
    /// Per-square climb mask: bit `d` (0 N, 1 NE, … 7 NW) set means the step
    /// to that neighbour crosses 10 ft or more and needs climbing.
    pub climb: Runs<u8>,
    /// Per-square cover granted by what stands in the square.
    pub cover: Runs<CoverLevelDto>,
    /// Per-square obscurement.
    pub obscured: Runs<ObscurementDto>,
    /// Per-square elevation in feet.
    pub elevation_ft: Runs<i16>,
    /// Per-square water depth in feet.
    pub water_depth_ft: Runs<u8>,
    /// Wall, door, window, gate and secret-door polylines.
    pub walls: Vec<SceneWallDto>,
    /// Canopy and prop polygons that obscure vision.
    pub vision_blockers: Vec<VisionBlockerDto>,
    /// Light sources.
    pub lights: Vec<SceneLightDto>,
    /// Difficult-terrain and water regions.
    pub regions: Vec<RegionDto>,
    /// Token placement hints.
    pub spawn_hints: SpawnHintsDto,
    /// NPC token slots.
    pub tokens: Vec<TokenSlotDto>,
    /// World square of square `(0, 0)` (I17): present in
    /// `TacticalBlockDto.scene`, absent in `TacticalScene.scene`, whose
    /// envelope carries it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "[number, number]")]
    pub origin_gs: Option<[i64; 2]>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_scene::{build_scene, Scene};
    use arda_tactical::{layouts, Library};
    use serde_json::{to_value, Value};

    fn through_mirror(v: &Value) -> Value {
        let m: SceneDto = serde_json::from_value(v.clone()).unwrap();
        to_value(m).unwrap()
    }

    fn library() -> Library {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/tactical/placeholder");
        Library::load(&dir).unwrap()
    }

    #[test]
    fn every_built_in_scene_round_trips_through_the_mirror() {
        let lib = library();
        let mut kinds = std::collections::BTreeSet::new();
        for l in layouts::all() {
            let scene = build_scene(&l, &lib, u64::MAX - 7, None).unwrap();
            let v = to_value(&scene).unwrap();
            assert_eq!(through_mirror(&v), v, "{}", l.name);
            assert_eq!(v["seed"], Value::from((u64::MAX - 7).to_string()));
            for w in scene.walls {
                kinds.insert(format!("{:?}", w.kind));
            }
        }
        assert!(kinds.len() >= 2, "{kinds:?}");
    }

    #[test]
    fn mirror_reads_every_optional_part_the_domain_writes() {
        let lib = library();
        let l = layouts::by_name("riverside").unwrap();
        let mut scene = build_scene(&l, &lib, 9, None).unwrap();
        scene.tokens.push(arda_scene::TokenSlot {
            npc_id: "18446744073709551615".into(),
            x: 3.5,
            y: 4.5,
            facing: 90,
        });
        let v = to_value(&scene).unwrap();
        assert_eq!(through_mirror(&v), v);
        let back: Scene = serde_json::from_value(through_mirror(&v)).unwrap();
        assert_eq!(back, scene);
        assert!(v["movement"][0][0].is_u64(), "{}", v["movement"]);
        let mut block_scene = v.clone();
        block_scene["origin_gs"] = serde_json::json!([-64, 128]);
        assert_eq!(through_mirror(&block_scene), block_scene);
    }
}
