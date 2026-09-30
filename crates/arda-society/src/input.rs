//! Input records. They mirror `arda-settle`'s `society/settlements.json`,
//! `roads.json` and `realms.json` field for field (goal 04 step 8), so a world
//! written by that stage deserialises here unchanged. Enum values match
//! `arda-npc`'s `Tier` and `SettlementFunction`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Settlement size class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// A handful of farms.
    Hamlet,
    /// A village of a few hundred.
    Village,
    /// A town, 1,000–8,000 people.
    Town,
    /// A city, 8,000 people or more.
    City,
}

impl Tier {
    /// Lower-case key, as used in the data tables.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Hamlet => "hamlet",
            Self::Village => "village",
            Self::Town => "town",
            Self::City => "city",
        }
    }

    /// Whether this is a town or a city.
    #[must_use]
    pub const fn is_urban(self) -> bool {
        matches!(self, Self::Town | Self::City)
    }
}

/// What a settlement lives on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Function {
    /// Arable fields.
    Farming,
    /// Herds and flocks.
    Pastoral,
    /// Inshore or river fishing.
    Fishing,
    /// A harbour with sea or river trade.
    Port,
    /// A regular market.
    Market,
    /// Mines or quarries.
    Mining,
    /// Forestry and charcoal.
    Logging,
    /// Workshops and guilds.
    Crafting,
    /// A garrison or castle.
    Fortress,
    /// A religious house.
    Abbey,
    /// A ford, bridge or ferry.
    Crossing,
    /// The seat of a realm.
    Capital,
}

impl Function {
    /// Lower-case key, as used in the data tables.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Farming => "farming",
            Self::Pastoral => "pastoral",
            Self::Fishing => "fishing",
            Self::Port => "port",
            Self::Market => "market",
            Self::Mining => "mining",
            Self::Logging => "logging",
            Self::Crafting => "crafting",
            Self::Fortress => "fortress",
            Self::Abbey => "abbey",
            Self::Crossing => "crossing",
            Self::Capital => "capital",
        }
    }
}

/// One settlement record (`settlements.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settlement {
    /// Stable identifier.
    #[serde(with = "crate::ids::str")]
    pub id: u64,
    /// Display name.
    pub name: String,
    /// Size class.
    pub tier: Tier,
    /// Inhabitants.
    pub population: u32,
    /// Economic functions.
    pub functions: Vec<Function>,
    /// Overall wealth, 0–255.
    pub wealth: u8,
    /// Culture key.
    pub culture: String,
    /// Owning realm.
    #[serde(with = "crate::ids::str")]
    pub realm_id: u64,
    /// Biome key, snake_case as arda-settle writes it (`temperate_forest`);
    /// it must be a key of `data/goods.json` `biomes`.
    pub biome: String,
    /// On a sea coast.
    pub coastal: bool,
    /// On a river.
    pub riverine: bool,
    /// Centre, metres east of the world's west edge.
    pub x_m: i64,
    /// Centre, metres south of the world's north edge.
    pub y_m: i64,
    /// Centre cell column.
    #[serde(default)]
    pub cell_x: u32,
    /// Centre cell row.
    #[serde(default)]
    pub cell_y: u32,
    /// Ground height at the centre, metres.
    #[serde(default)]
    pub height_m: i32,
    /// Rank among towns and cities (1 = largest); 0 otherwise.
    #[serde(default)]
    pub rank: u32,
    /// Site tags (`ford`, `harbour`, `ore`, `river`, …).
    #[serde(default)]
    pub site_tags: Vec<String>,
    /// One-line site history hook from the settlement stage.
    #[serde(default)]
    pub history: String,
    /// Estimated building mix: building-function key → count.
    #[serde(default)]
    pub buildings: BTreeMap<String, u32>,
}

impl Settlement {
    /// Whether the settlement has `f`.
    #[must_use]
    pub fn has(&self, f: Function) -> bool {
        self.functions.contains(&f)
    }

    /// Whether the settlement carries site tag `tag`.
    #[must_use]
    pub fn tagged(&self, tag: &str) -> bool {
        self.site_tags.iter().any(|t| t == tag)
    }
}

/// Road class (vocabulary.md I3): the world's stored codes with footpath
/// appended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoadClass {
    /// No road (code 0).
    None,
    /// Track to a hamlet (code 1).
    Track,
    /// Road to a village (code 2).
    Road,
    /// Trunk road between towns (code 3).
    Highway,
    /// Footpath between neighbours (code 4).
    Footpath,
}

impl RoadClass {
    /// Stored code: none 0, track 1, road 2, highway 3, footpath 4.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Track => 1,
            Self::Road => 2,
            Self::Highway => 3,
            Self::Footpath => 4,
        }
    }

    /// Readable name.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "way",
            Self::Footpath => "footpath",
            Self::Track => "track",
            Self::Road => "road",
            Self::Highway => "highway",
        }
    }
}

/// One road object (`roads.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Road {
    /// Stable id (a u64 as a JSON string, I5; numbers are accepted).
    #[serde(with = "crate::ids::str")]
    pub id: u64,
    /// Class.
    pub class: RoadClass,
    /// Settlement the route starts at.
    #[serde(with = "crate::ids::str")]
    pub from: u64,
    /// Settlement the route ends at, if any.
    #[serde(default, with = "crate::ids::opt")]
    pub to: Option<u64>,
    /// Map edge the route leaves through, if any.
    #[serde(default)]
    pub to_edge: Option<String>,
    /// Route length, metres.
    pub length_m: u64,
    /// Straight-line distance between the ends, metres.
    #[serde(default)]
    pub straight_m: u64,
    /// Length of the new cells this road added, metres.
    #[serde(default)]
    pub new_m: u64,
    /// New stretches as polylines of `[x_m, y_m]` points.
    #[serde(default)]
    pub segments: Vec<Vec<[i64; 2]>>,
}

/// One realm (`realms.json`, `logic/06` "State transitions").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Realm {
    /// Stable id (settlements' `realm_id`).
    #[serde(with = "crate::ids::str")]
    pub id: u64,
    /// Realm name.
    pub name: String,
    /// Seat settlement id.
    #[serde(with = "crate::ids::str")]
    pub seat: u64,
    /// Member settlement ids (optional; derived from `realm_id` when empty).
    /// `arda-settle` writes them as `settlements`; when present they must
    /// agree with the members' `realm_id` (review round 2, #27).
    #[serde(default, alias = "settlements", with = "crate::ids::vec")]
    pub members: Vec<u64>,
}

/// An explicit building (vocabulary.md I5–I7): `function` is a plain
/// snake_case key; a workshop's craft is the free tag `craft:<name>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingSpec {
    /// Building id, unique within the settlement.
    #[serde(with = "crate::ids::str")]
    pub id: u64,
    /// Owning settlement.
    #[serde(with = "crate::ids::str")]
    pub settlement_id: u64,
    /// Building-function key (`inn`, `keep`, `workshop`, …).
    pub function: String,
    /// Free tags, e.g. `craft:weaver`.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Everything society is built from.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WorldSettlements {
    /// Format version of the settlement stage's files.
    #[serde(default)]
    pub format_version: u32,
    /// All settlements.
    pub settlements: Vec<Settlement>,
    /// All roads.
    #[serde(default)]
    pub roads: Vec<Road>,
    /// All realms.
    #[serde(default)]
    pub realms: Vec<Realm>,
    /// Explicit buildings; settlements without any get buildings derived
    /// from their mix (see `buildings.rs`).
    #[serde(default)]
    pub buildings: Vec<BuildingSpec>,
}
