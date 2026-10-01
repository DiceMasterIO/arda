//! Settlement records. Field names and enum values mirror `arda-npc`'s
//! `SettlementProfile`, `Tier` and `SettlementFunction`, so a record
//! deserialises into a profile 1:1 (the extra fields are ignored there).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Settlement size class (`arda-npc` `Tier`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// A handful of farms, 12–80 people.
    Hamlet,
    /// A village of a few hundred.
    Village,
    /// A town, 1,000–8,000 people.
    Town,
    /// A city, 8,000 people or more.
    City,
}

impl Tier {
    /// Tier for a population (towns and cities come from the rank-size list).
    #[must_use]
    pub const fn of_town(population: u32) -> Self {
        if population >= CITY_MIN {
            Self::City
        } else {
            Self::Town
        }
    }

    /// Whether this is a town or a city.
    #[must_use]
    pub const fn is_urban(self) -> bool {
        matches!(self, Self::Town | Self::City)
    }
}

/// Smallest city population (`arda-npc` tier ranges).
pub const CITY_MIN: u32 = 8000;
/// Smallest town population.
pub const TOWN_MIN: u32 = 1000;

/// What a settlement lives on (`arda-npc` `SettlementFunction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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

/// The closed biome list a settlement reports (serialised snake_case),
/// shared through `arda-ids` (I22).
pub use arda_ids::Biome;
/// Building functions of the building mix, shared through `arda-ids` (I6).
pub use arda_ids::BuildingFunction;
pub use arda_ids::{RealmId, SettlementId};

/// One settlement, as written to `society/settlements.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Settlement {
    /// Stable identifier, 1-based in placement order (a JSON string, I5).
    pub id: SettlementId,
    /// Display name.
    pub name: String,
    /// Size class.
    pub tier: Tier,
    /// Inhabitants.
    pub population: u32,
    /// Economic functions, sorted.
    pub functions: Vec<Function>,
    /// Overall wealth, 0–255.
    pub wealth: u8,
    /// Culture key (`arda-npc` `data/content/cultures.json`).
    pub culture: String,
    /// Owning realm, 1-based (a JSON string, I5).
    pub realm_id: RealmId,
    /// Biome, from a closed list.
    pub biome: Biome,
    /// On a sea coast.
    pub coastal: bool,
    /// On a river.
    pub riverine: bool,
    /// English gloss of the name ("Oakford"); `name` is the native form.
    pub name_gloss: String,
    /// Centre, metres east of the world's west edge.
    pub x_m: i64,
    /// Centre, metres south of the world's north edge.
    pub y_m: i64,
    /// Centre cell column.
    pub cell_x: u32,
    /// Centre cell row.
    pub cell_y: u32,
    /// Ground height at the centre, metres.
    pub height_m: i32,
    /// Rank among towns and cities (1 = largest); 0 for villages and hamlets.
    pub rank: u32,
    /// Descriptive site tags (see the README).
    pub site_tags: Vec<String>,
    /// A one-line origin hook for the referee.
    pub history: String,
    /// Estimated building mix: building function → count.
    pub buildings: BTreeMap<BuildingFunction, u32>,
    /// The local speech the name was drawn in, for naming the settlement's
    /// people in the same dialect (A5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tongue: Option<arda_names::Tongue>,
    /// Tag bits of the centre cell (not serialised).
    #[serde(skip)]
    pub tag_bits: u32,
    /// Placement score (not serialised).
    #[serde(skip)]
    pub score: u32,
}

impl Settlement {
    /// Row-major grid index of the centre.
    #[must_use]
    pub fn index(&self, width: usize) -> usize {
        usize::try_from(self.cell_y).unwrap_or(0) * width
            + usize::try_from(self.cell_x).unwrap_or(0)
    }

    /// Whether the settlement has `f`.
    #[must_use]
    pub fn has(&self, f: Function) -> bool {
        self.functions.contains(&f)
    }

    /// A bare settlement at a cell, for unit tests.
    #[cfg(test)]
    #[must_use]
    pub fn bare(id: u64, cell_x: u32, cell_y: u32, tier: Tier, population: u32) -> Self {
        Self {
            id: SettlementId(id),
            name: String::new(),
            tier,
            population,
            functions: Vec::new(),
            wealth: 0,
            culture: String::new(),
            realm_id: RealmId(0),
            biome: Biome::default(),
            coastal: false,
            riverine: false,
            name_gloss: String::new(),
            x_m: i64::from(cell_x) * 100,
            y_m: i64::from(cell_y) * 100,
            cell_x,
            cell_y,
            height_m: 0,
            rank: 0,
            site_tags: Vec::new(),
            history: String::new(),
            buildings: BTreeMap::new(),
            tongue: None,
            tag_bits: 0,
            score: 0,
        }
    }
}
