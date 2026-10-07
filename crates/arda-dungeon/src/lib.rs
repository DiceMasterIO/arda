//! Dungeons and caves for arda: deterministic underground battle maps.
//!
//! [`generate`] turns a seed, a [`Kind`] and a size into a [`Dungeon`]: an
//! ordinary [`TacticalLayout`] (squares, edge walls with doors, props and
//! lights, so the existing compositor and `arda-scene` handle it), a
//! [`RulesSidecar`] that makes the solid rock impassable and opaque and
//! marks loose scree as difficult terrain, and a description of the rooms,
//! doors and stairs for game logic.
//!
//! - [`Kind::Dungeon`]: built halls. Binary space partition rooms, joined by
//!   a minimum spanning tree of corridors plus a few loops; doors where a
//!   corridor enters a room (some `locked`, loop doors sometimes `secret`);
//!   rooms dressed by purpose (crypt, barracks, storage, shrine, prison
//!   with cells, treasure, hall); stairs up and down; torches and braziers.
//! - [`Kind::Cave`]: natural caverns. Cellular-automaton rock, every pocket
//!   joined to the main cavern by tunnels, a mouth on the map edge, pools
//!   and a stream, scree and rubble, stalagmites, rocks and mushrooms, and
//!   a descent.
//!
//! Everything is a pure function of [`Params`]: the same seed always gives
//! the same layout, byte for byte. See `crates/arda-dungeon/README.md`.

#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod built;
mod cave;
pub mod check;
mod dress;
mod grid;
mod rng;

use arda_scene::RulesSidecar;
use arda_tactical::layout::EdgeAxis;
use arda_tactical::TacticalLayout;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Smallest map side in squares.
pub const MIN_SIDE: u32 = 16;
/// Largest map side in squares.
pub const MAX_SIDE: u32 = 160;

/// Wall kit of built dungeons.
pub const DUNGEON_KIT: &str = "stone";
/// Wall kit of natural caves.
pub const CAVE_KIT: &str = "cave";
/// Ground key of the solid rock around passages.
pub const ROCK_GROUND: &str = "bedrock";

/// What to generate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Built rooms and corridors.
    Dungeon,
    /// Natural caverns.
    Cave,
}

impl Kind {
    /// Parses `dungeon` or `cave`.
    ///
    /// # Errors
    /// Any other word.
    pub fn parse(s: &str) -> Result<Self, DungeonError> {
        match s {
            "dungeon" => Ok(Self::Dungeon),
            "cave" => Ok(Self::Cave),
            other => Err(DungeonError::Params(format!(
                "kind must be `dungeon` or `cave`, not `{other}`"
            ))),
        }
    }

    /// The lower-case name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Dungeon => "dungeon",
            Self::Cave => "cave",
        }
    }
}

/// Generation parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Params {
    /// Seed.
    pub seed: u64,
    /// Dungeon or cave.
    pub kind: Kind,
    /// Width in squares.
    pub width: u32,
    /// Height in squares.
    pub height: u32,
}

impl Params {
    /// Checks the size bounds.
    ///
    /// # Errors
    /// A side outside [`MIN_SIDE`]..=[`MAX_SIDE`].
    pub fn check(&self) -> Result<(), DungeonError> {
        for (name, v) in [("width", self.width), ("height", self.height)] {
            if !(MIN_SIDE..=MAX_SIDE).contains(&v) {
                return Err(DungeonError::Params(format!(
                    "{name} {v} is outside {MIN_SIDE}..={MAX_SIDE}"
                )));
            }
        }
        Ok(())
    }
}

/// The purpose of a built room, which decides its dressing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    /// Where the stairs up arrive.
    Entrance,
    /// Tombs, coffins and bones.
    Crypt,
    /// Beds, racks and a table.
    Barracks,
    /// Barrels, crates and sacks.
    Storage,
    /// An altar, candles and statues.
    Shrine,
    /// Locked cells and cages.
    Prison,
    /// Treasure chests behind a locked door.
    Treasure,
    /// A plain hall with a table and a brazier.
    Hall,
    /// A natural cavern (caves only).
    Cavern,
}

/// A room: its purpose and its floor rectangle in squares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Room {
    /// Index in [`Dungeon::rooms`].
    pub id: usize,
    /// What it is for.
    pub purpose: Purpose,
    /// Left column.
    pub x: u32,
    /// Top row.
    pub y: u32,
    /// Width in squares.
    pub w: u32,
    /// Height in squares.
    pub h: u32,
}

/// A door on a square edge, in layout edge terms (as `WallSegment`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Door {
    /// Square column.
    pub x: u32,
    /// Square row.
    pub y: u32,
    /// Which edge.
    pub axis: EdgeAxis,
    /// Needs a key or a check.
    pub locked: bool,
    /// Looks like wall until found.
    pub secret: bool,
}

/// A way in or out of the level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitKind {
    /// Stairs up (`prop.stairs`).
    StairsUp,
    /// Stairs down (`prop.stairs_down`).
    StairsDown,
    /// An opening on the map edge (a cave mouth).
    Mouth,
}

/// An exit: its kind and the square a token arrives on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exit {
    /// Kind.
    pub kind: ExitKind,
    /// Column.
    pub x: u32,
    /// Row.
    pub y: u32,
}

/// A generated level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dungeon {
    /// The parameters it was made from.
    pub params: Params,
    /// The battle map.
    pub layout: TacticalLayout,
    /// Per-square rules: rock impassable and opaque, scree difficult.
    pub rules: RulesSidecar,
    /// Rooms (one `cavern` spanning the map for caves).
    pub rooms: Vec<Room>,
    /// Every door, including locked and secret ones.
    pub doors: Vec<Door>,
    /// Stairs and mouths.
    pub exits: Vec<Exit>,
}

/// Generation failures.
#[derive(Debug, Error)]
pub enum DungeonError {
    /// Out-of-range parameters.
    #[error("dungeon parameters: {0}")]
    Params(String),
    /// The generator could not meet its own invariants (a bug).
    #[error("dungeon generation: {0}")]
    Generation(String),
}

/// Generates a level.
///
/// # Errors
/// Out-of-range parameters.
pub fn generate(params: &Params) -> Result<Dungeon, DungeonError> {
    params.check()?;
    match params.kind {
        Kind::Dungeon => built::generate(params),
        Kind::Cave => cave::generate(params),
    }
}

/// A stable seed for the site at world cell `(gx, gy)` of a world with seed
/// `world_seed`, so a world's dungeon entrances always open onto the same
/// level. `salt` tells apart several sites in one cell (0 for the first).
#[must_use]
pub fn site_seed(world_seed: u64, gx: i64, gy: i64, salt: u64) -> u64 {
    arda_tactical::noise::hash2(
        arda_tactical::noise::hash_str(world_seed ^ salt, "arda-dungeon site"),
        gx,
        gy,
    )
}
