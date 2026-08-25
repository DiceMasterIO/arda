//! The tile vocabulary and its adjacency relation (`logic/03` §Q12).
//!
//! The skeleton ships 24 tiles; build-order step 6 grows this to the full
//! 200+ vocabulary. Tile ids are stable — a released id is never reused for
//! a different tile.

/// A tile identifier, two bytes on disk (`02-models.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TileId(u16);

impl TileId {
    /// Wraps a raw tile id.
    #[must_use]
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    /// The raw tile id.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }
}

/// Broad tile family; adjacency is decided between groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileGroup {
    /// Open water.
    Water,
    /// The wet/dry transition.
    Shore,
    /// Walkable ground.
    Ground,
    /// Plant cover standing on ground.
    Vegetation,
    /// Stone and cliff.
    Rock,
    /// Built surfaces.
    Structure,
}

/// One entry in the vocabulary.
#[derive(Debug, Clone, Copy)]
pub struct TileDef {
    /// Stable identifier.
    pub id: TileId,
    /// Stable snake_case name, used by the export legend (`logic/04`).
    pub name: &'static str,
    /// Adjacency family.
    pub group: TileGroup,
}

const fn def(id: u16, name: &'static str, group: TileGroup) -> TileDef {
    TileDef {
        id: TileId::new(id),
        name,
        group,
    }
}

/// The 24-tile skeleton vocabulary (`implementation.md` step 3).
pub const SKELETON_TILES: [TileDef; 24] = [
    def(0, "deep_water", TileGroup::Water),
    def(1, "open_water", TileGroup::Water),
    def(2, "shallow_water", TileGroup::Water),
    def(3, "reed_bed", TileGroup::Shore),
    def(4, "mudflat", TileGroup::Shore),
    def(5, "sand", TileGroup::Shore),
    def(6, "shingle", TileGroup::Shore),
    def(7, "wet_grass", TileGroup::Shore),
    def(8, "dirt", TileGroup::Ground),
    def(9, "grass", TileGroup::Ground),
    def(10, "tall_grass", TileGroup::Ground),
    def(11, "heath", TileGroup::Ground),
    def(12, "moss", TileGroup::Ground),
    def(13, "gravel", TileGroup::Ground),
    def(14, "fern", TileGroup::Vegetation),
    def(15, "bramble", TileGroup::Vegetation),
    def(16, "sapling", TileGroup::Vegetation),
    def(17, "tree_trunk", TileGroup::Vegetation),
    def(18, "deadfall", TileGroup::Vegetation),
    def(19, "boulder", TileGroup::Rock),
    def(20, "scree", TileGroup::Rock),
    def(21, "bedrock", TileGroup::Rock),
    def(22, "cliff_face", TileGroup::Rock),
    def(23, "packed_earth", TileGroup::Structure),
];

/// Looks up a tile by id.
#[must_use]
pub fn tile_def(id: TileId) -> Option<&'static TileDef> {
    SKELETON_TILES.iter().find(|t| t.id == id)
}

/// Every tile in one family.
pub fn tiles_in_group(group: TileGroup) -> impl Iterator<Item = &'static TileDef> {
    SKELETON_TILES.iter().filter(move |t| t.group == group)
}

/// Rank used to decide adjacency: families next to each other in the
/// wet-to-dry ordering may touch, families two steps apart may not.
const fn wetness_rank(group: TileGroup) -> i8 {
    match group {
        TileGroup::Water => 0,
        TileGroup::Shore => 1,
        TileGroup::Ground => 2,
        TileGroup::Vegetation | TileGroup::Structure => 3,
        TileGroup::Rock => 4,
    }
}

/// Whether two tiles may share an edge (`logic/03` §Q12).
///
/// Symmetric by construction: it compares family ranks, and deep water is
/// additionally pinned to water-only neighbours so an ocean square can never
/// abut dry ground.
#[must_use]
pub fn may_adjoin(a: TileId, b: TileId) -> bool {
    let (Some(da), Some(db)) = (tile_def(a), tile_def(b)) else {
        return false;
    };
    if da.name == "deep_water" || db.name == "deep_water" {
        return da.group == TileGroup::Water && db.group == TileGroup::Water;
    }
    (wetness_rank(da.group) - wetness_rank(db.group)).abs() <= 1
}
