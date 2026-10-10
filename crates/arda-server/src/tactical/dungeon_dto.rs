//! Wire mirror of `arda_dungeon::Dungeon`, the `GET /v1/tactical/dungeon`
//! body (logic/18 §dgn-serve, logic/16 §api-bindings), so ts-rs and
//! schemars describe it without the dungeon crate depending on either. The
//! handler keeps serialising the domain `Dungeon`; the tests prove the
//! mirror reads and writes that JSON unchanged.
//!
//! The layout and rules reuse their own mirrors ([`TacticalLayoutDto`],
//! [`RulesSidecarDto`]); the seed is a decimal string (I5, I17).

use super::dto::{EdgeAxisDto, TacticalLayoutDto};
use super::rules_dto::RulesSidecarDto;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `GET /v1/tactical/dungeon`: a generated level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DungeonDto {
    /// The parameters it was made from.
    pub params: DungeonParamsDto,
    /// The battle map; `POST /render` takes it as it is.
    pub layout: TacticalLayoutDto,
    /// Per-square rules: rock impassable and opaque, scree difficult.
    pub rules: RulesSidecarDto,
    /// Rooms (one `cavern` spanning the map for caves).
    pub rooms: Vec<DungeonRoomDto>,
    /// Every door, including locked and secret ones.
    pub doors: Vec<DungeonDoorDto>,
    /// Stairs and mouths.
    pub exits: Vec<DungeonExitDto>,
}

/// Generation parameters (`arda_dungeon::Params`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DungeonParamsDto {
    /// Seed, a decimal string.
    pub seed: String,
    /// Dungeon or cave.
    pub kind: DungeonKindDto,
    /// Width in squares (16–160).
    pub width: u32,
    /// Height in squares (16–160).
    pub height: u32,
}

/// What was generated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DungeonKindDto {
    /// Built rooms and corridors.
    Dungeon,
    /// Natural caverns.
    Cave,
}

/// The purpose of a room, which decided its dressing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoomPurposeDto {
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DungeonRoomDto {
    /// Index in `rooms`.
    pub id: u32,
    /// What it is for.
    pub purpose: RoomPurposeDto,
    /// Left column.
    pub x: u32,
    /// Top row.
    pub y: u32,
    /// Width in squares.
    pub w: u32,
    /// Height in squares.
    pub h: u32,
}

/// A door on a square edge, in layout edge terms (as `WallSegmentDto`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DungeonDoorDto {
    /// Square column.
    pub x: u32,
    /// Square row.
    pub y: u32,
    /// Which edge.
    pub axis: EdgeAxisDto,
    /// Needs a key or a check.
    pub locked: bool,
    /// Looks like wall until found.
    pub secret: bool,
}

/// A way in or out of the level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DungeonExitKindDto {
    /// Stairs up (`prop.stairs`).
    StairsUp,
    /// Stairs down (`prop.stairs_down`).
    StairsDown,
    /// An opening on the map edge (a cave mouth).
    Mouth,
}

/// An exit: its kind and the floor square a token arrives on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DungeonExitDto {
    /// Kind.
    pub kind: DungeonExitKindDto,
    /// Column.
    pub x: u32,
    /// Row.
    pub y: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_dungeon::{generate, Dungeon, Kind, Params};
    use serde_json::{to_value, Value};

    fn through_mirror(v: &Value) -> Value {
        let m: DungeonDto = serde_json::from_value(v.clone()).unwrap();
        to_value(m).unwrap()
    }

    #[test]
    fn dungeons_and_caves_round_trip_through_the_mirror() {
        let mut purposes = std::collections::BTreeSet::new();
        let mut exits = std::collections::BTreeSet::new();
        for kind in [Kind::Dungeon, Kind::Cave] {
            for seed in [7, 42, u64::MAX - 7] {
                let p = Params {
                    seed,
                    kind,
                    width: 48,
                    height: 36,
                };
                let d = generate(&p).unwrap();
                let mut v = to_value(&d).unwrap();
                // `RulesSidecarDto.edges` is always written; the domain
                // omits an empty list (and the schema does not require it).
                if v["rules"].get("edges").is_none() {
                    v["rules"]["edges"] = Value::Array(Vec::new());
                }
                let m = through_mirror(&v);
                for k in ["params", "layout", "rules", "rooms", "doors", "exits"] {
                    assert_eq!(m[k], v[k], "{kind:?} {seed}: {k}");
                }
                assert_eq!(m, v, "{kind:?} {seed}");
                assert_eq!(v["params"]["seed"], Value::from(seed.to_string()));
                let back: Dungeon = serde_json::from_value(through_mirror(&v)).unwrap();
                assert_eq!(back, d);
                for r in &d.rooms {
                    purposes.insert(format!("{:?}", r.purpose));
                }
                for e in &d.exits {
                    exits.insert(format!("{:?}", e.kind));
                }
            }
        }
        assert!(purposes.len() >= 4, "{purposes:?}");
        assert_eq!(exits.len(), 3, "{exits:?}");
    }
}
