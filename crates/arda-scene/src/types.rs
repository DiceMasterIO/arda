//! The scene DTO: everything the game needs to run a tactical map.
//!
//! Coordinates are in 5-ft squares, origin at the map's top-left corner, y
//! growing south, matching `TacticalLayout`. Grid vertices are integers; a
//! square `(x, y)` spans `x..x+1 × y..y+1`. The JSON schema is documented in
//! `crates/arda-scene/README.md`; change both together.

use crate::rle::Grid;
use serde::{Deserialize, Serialize};

/// The scene schema version this crate writes.
pub const SCENE_FORMAT_VERSION: u32 = 1;

/// A square, as `[x, y]` in JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Sq(pub u32, pub u32);

impl Sq {
    /// Column.
    #[must_use]
    pub fn x(self) -> u32 {
        self.0
    }

    /// Row.
    #[must_use]
    pub fn y(self) -> u32 {
        self.1
    }
}

/// How a creature moves through a square (SRD 5.1, "Movement").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Movement {
    /// Ordinary ground.
    #[default]
    Normal,
    /// Difficult terrain: every foot costs 1 extra foot.
    Difficult,
    /// Shallow water (1–4 ft): wading, treated as difficult terrain.
    Wade,
    /// Water at least 5 ft deep: swimming, every foot costs 1 extra foot.
    Swim,
    /// Solid obstacle; cannot be entered.
    Impassable,
}

impl Movement {
    /// Whether moving through the square costs 1 extra foot per foot.
    #[must_use]
    pub fn is_costly(self) -> bool {
        matches!(self, Self::Difficult | Self::Wade | Self::Swim)
    }
}

/// SRD 5.1 degrees of cover, ordered from least to most protective.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CoverLevel {
    /// No cover.
    #[default]
    None,
    /// Half cover: +2 to AC and Dexterity saving throws.
    Half,
    /// Three-quarters cover: +5 to AC and Dexterity saving throws.
    ThreeQuarters,
    /// Total cover: cannot be targeted directly.
    #[serde(alias = "full")]
    Total,
}

impl From<arda_tactical::catalog::Cover> for CoverLevel {
    fn from(c: arda_tactical::catalog::Cover) -> Self {
        use arda_tactical::catalog::Cover;
        match c {
            Cover::None => Self::None,
            Cover::Half => Self::Half,
            Cover::ThreeQuarters => Self::ThreeQuarters,
            Cover::Full => Self::Total,
        }
    }
}

/// SRD 5.1 vision obscurement of a square.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Obscurement {
    /// Clear.
    #[default]
    Clear,
    /// Lightly obscured (moderate foliage): disadvantage on sight Perception.
    Light,
    /// Heavily obscured or opaque: blocks vision entirely.
    Heavy,
}

/// The type of a wall-edge feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WallKind {
    /// Solid wall.
    Wall,
    /// Door; has an open state.
    Door,
    /// Window: blocks movement, not sight.
    Window,
    /// Gate; has an open state.
    Gate,
    /// Secret door; has an open state, and looks like wall until found.
    Secret,
}

impl WallKind {
    /// Doors, gates and secret doors open and close.
    #[must_use]
    pub fn opens(self) -> bool {
        matches!(self, Self::Door | Self::Gate | Self::Secret)
    }
}

/// A wall polyline on grid lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wall {
    /// Feature type.
    pub kind: WallKind,
    /// `Some(open)` for doors, gates and secret doors; `None` otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<bool>,
    /// Grid vertices `[x, y]`, at least two; consecutive points are collinear
    /// runs merged from unit edges.
    pub points: Vec<[u32; 2]>,
    /// Blocks line of sight in its current state.
    pub blocks_sight: bool,
    /// Blocks movement in its current state.
    pub blocks_movement: bool,
    /// Blocks light in its current state.
    pub blocks_light: bool,
    /// Cover it grants to a creature behind it when it does not block sight.
    #[serde(default)]
    pub cover: CoverLevel,
    /// Wall kit name.
    pub kit: String,
    /// A locked door, gate or secret door (layout edge tag `locked`): it
    /// needs a key or a check to open. Omitted when false.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub locked: bool,
}

/// Where a vision blocker comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerKind {
    /// A tree canopy (canopy-layer asset).
    Canopy,
    /// A prop that blocks sight.
    Prop,
}

/// A polygon that obscures vision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisionBlocker {
    /// Source class.
    pub kind: BlockerKind,
    /// Asset id.
    pub asset: String,
    /// How much it obscures: `light` for foliage, `heavy` for opaque.
    pub obscurement: Obscurement,
    /// Closed polygon in squares, clockwise in y-down coordinates.
    pub polygon: Vec<[f32; 2]>,
}

/// A light source with SRD bright and dim radii.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneLight {
    /// Position in squares.
    pub x: f32,
    /// Position in squares.
    pub y: f32,
    /// Bright-light radius in feet.
    pub bright_ft: u16,
    /// Outer edge of dim light in feet: dim light extends an additional
    /// `bright_ft` beyond the bright radius (the SRD torch pattern).
    pub dim_ft: u16,
    /// RGB colour.
    pub colour: [u8; 3],
    /// Emitting asset id, or `None` for a free layout light.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset: Option<String>,
}

/// What a region marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionKind {
    /// Difficult terrain on land.
    Difficult,
    /// Shallow water (wading).
    ShallowWater,
    /// Deep water (swimming).
    DeepWater,
}

/// Merged squares of one kind, as closed rings on grid vertices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Region {
    /// What the region marks.
    pub kind: RegionKind,
    /// Closed rings (first point not repeated). Fill with the even-odd rule:
    /// outer rings run clockwise in y-down coordinates, holes anticlockwise.
    pub rings: Vec<Vec<[u32; 2]>>,
}

/// A map edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Edge {
    /// North (y = 0).
    N,
    /// East (x = width - 1).
    E,
    /// South (y = height - 1).
    S,
    /// West (x = 0).
    W,
}

/// A run of enterable squares along one map edge, for travel to the
/// neighbouring map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeExit {
    /// Which edge.
    pub edge: Edge,
    /// First square of the run.
    pub from: Sq,
    /// Last square of the run (inclusive).
    pub to: Sq,
}

/// A square beside a door or gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entrance {
    /// Index into [`Scene::walls`].
    pub wall: usize,
    /// Enterable squares on either side of it.
    pub squares: Vec<Sq>,
}

/// Where tokens may be put.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpawnHints {
    /// Normal-movement squares with no cover whose 8 neighbours are the same:
    /// room for a token and some elbow room, row-major.
    pub open: Vec<Sq>,
    /// Squares by doors and gates.
    pub entrances: Vec<Entrance>,
    /// Enterable runs along the map edges, N, E, S, W order.
    pub exits: Vec<EdgeExit>,
}

/// Game-facing scene data for one tactical layout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    /// Schema version, [`SCENE_FORMAT_VERSION`].
    pub format_version: u32,
    /// Layout name.
    pub name: String,
    /// Width in squares.
    pub width: u32,
    /// Height in squares.
    pub height: u32,
    /// Seed the layout's asset queries were resolved with; a JSON string.
    #[serde(with = "crate::serde_str")]
    pub seed: u64,
    /// Library name.
    pub library: String,
    /// Library art version; with the seed, the scene's identity.
    pub library_version: String,
    /// Per-square movement.
    pub movement: Grid<Movement>,
    /// Per-square climb mask: bit `d` set means stepping to the neighbour in
    /// direction `d` (0 N, 1 NE, 2 E, 3 SE, 4 S, 5 SW, 6 W, 7 NW) crosses an
    /// elevation step of 10 ft or more and needs climbing.
    pub climb: Grid<u8>,
    /// Per-square cover granted by what stands in the square.
    pub cover: Grid<CoverLevel>,
    /// Per-square obscurement.
    pub obscured: Grid<Obscurement>,
    /// Per-square elevation in feet.
    pub elevation_ft: Grid<i16>,
    /// Per-square water depth in feet (after the sidecar).
    pub water_depth_ft: Grid<u8>,
    /// Wall, door, window, gate and secret-door polylines.
    pub walls: Vec<Wall>,
    /// Canopy and prop polygons that obscure vision.
    pub vision_blockers: Vec<VisionBlocker>,
    /// Light sources.
    pub lights: Vec<SceneLight>,
    /// Difficult-terrain and water regions.
    pub regions: Vec<Region>,
    /// Token placement hints.
    pub spawn_hints: SpawnHints,
    /// NPC token slots (goal 48); empty until the NPC stage fills them.
    #[serde(default)]
    pub tokens: Vec<TokenSlot>,
}

/// Where an NPC's token stands on the map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenSlot {
    /// NPC id: a u64 written as a decimal string (canonical convention I5).
    pub npc_id: String,
    /// Token centre in squares (`3.5` is a square centre).
    pub x: f32,
    /// Token centre in squares.
    pub y: f32,
    /// Facing in degrees clockwise from north, `0..360`.
    #[serde(default)]
    pub facing: u16,
}

impl Scene {
    /// Row-major index of a square, if inside.
    #[must_use]
    pub fn square_index(&self, s: Sq) -> Option<usize> {
        (s.0 < self.width && s.1 < self.height)
            .then(|| s.1 as usize * self.width as usize + s.0 as usize)
    }

    /// Movement of a square; `Impassable` outside the map.
    #[must_use]
    pub fn movement_at(&self, s: Sq) -> Movement {
        self.square_index(s)
            .and_then(|i| self.movement.get(i).copied())
            .unwrap_or(Movement::Impassable)
    }

    /// Cover of a square; `None` outside the map.
    #[must_use]
    pub fn cover_at(&self, s: Sq) -> CoverLevel {
        self.square_index(s)
            .and_then(|i| self.cover.get(i).copied())
            .unwrap_or_default()
    }
}
