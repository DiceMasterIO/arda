//! Mirrors of the `arda-npc` input types.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// Settlement size class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// Hamlet.
    Hamlet,
    /// Village.
    Village,
    /// Town.
    Town,
    /// City.
    City,
}

/// What a settlement lives on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SettlementFunction {
    /// Arable fields.
    Farming,
    /// Herds and flocks.
    Pastoral,
    /// Fishing.
    Fishing,
    /// A harbour.
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

/// The closed settlement biome list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Biome {
    /// Open temperate lowland.
    Temperate,
    /// Warm temperate lowland.
    WarmTemperate,
    /// Broadleaf or mixed forest.
    TemperateForest,
    /// Cold conifer forest.
    BorealForest,
    /// Hills and mountains.
    Highland,
    /// Cold mountains.
    Alpine,
    /// Beside marsh or fen.
    Wetland,
    /// Dry grassland.
    Steppe,
    /// On the sea coast.
    Coastal,
}

/// One settlement to populate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct SettlementProfile {
    /// Settlement id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Size class.
    pub tier: Tier,
    /// Number of inhabitants.
    pub population: u32,
    /// Economic functions.
    pub functions: Vec<SettlementFunction>,
    /// Wealth, 0–255.
    pub wealth: u8,
    /// Culture key.
    pub culture: String,
    /// Realm id.
    pub realm_id: String,
    /// Biome.
    pub biome: Biome,
    /// On a sea coast.
    pub coastal: bool,
    /// On a river.
    pub riverine: bool,
    /// Optional ancestry weights replacing the culture's table.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub ancestry_mix: Option<BTreeMap<String, u32>>,
    /// The local speech the settlement was named in (`arda_names::Tongue`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub tongue: Option<Tongue>,
}

/// Recipe of a place's local speech (`arda_names::Tongue`): seeds are
/// decimal strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Tongue {
    /// Culture preset key (`heartland`, `sylvan`, …).
    pub preset: String,
    /// Seed of the standard language.
    pub language_seed: String,
    /// Seed of the shared substrate tongue.
    pub substrate_seed: String,
    /// Seed of the region's dialect map.
    pub dialect_seed: String,
    /// Dialect map width, cells.
    #[ts(type = "number")]
    pub width: i64,
    /// Dialect map height, cells.
    #[ts(type = "number")]
    pub height: i64,
    /// Place column, cells.
    #[ts(type = "number")]
    pub x: i64,
    /// Place row, cells.
    #[ts(type = "number")]
    pub y: i64,
}

/// What a building is for: a plain snake_case string (vocabulary I7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum BuildingFunction {
    /// A town house.
    House,
    /// A farmstead's dwelling.
    Farmhouse,
    /// A small cottage.
    Cottage,
    /// A noble manor.
    Manor,
    /// An inn.
    Inn,
    /// A tavern.
    Tavern,
    /// A bakery.
    Bakery,
    /// A brewery.
    Brewery,
    /// A mill.
    Mill,
    /// A forge.
    Smithy,
    /// A workshop; its craft is a `craft:<key>` tag.
    Workshop,
    /// A tannery.
    Tannery,
    /// An apothecary.
    Apothecary,
    /// A temple.
    Temple,
    /// A shrine.
    Shrine,
    /// A library.
    Library,
    /// A warehouse.
    Warehouse,
    /// An open market place.
    Market,
    /// A market stall.
    Stall,
    /// A dock.
    Dock,
    /// A boathouse.
    Boathouse,
    /// A keep.
    Keep,
    /// Barracks.
    Barracks,
    /// A guardhouse.
    Guardhouse,
    /// Stables.
    Stable,
    /// A barn.
    Barn,
    /// A street.
    Street,
    /// A farm yard.
    Farm,
    /// A market hall.
    MarketHall,
    /// A mine.
    Mine,
    /// A lumber camp.
    LumberCamp,
    /// A school.
    School,
}

/// One building of a settlement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct BuildingSpec {
    /// Building id, unique within the settlement.
    pub id: String,
    /// Owning settlement id.
    pub settlement_id: String,
    /// What the building is for.
    pub function: BuildingFunction,
    /// Free tags such as `craft:weaving`; omitted when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<String>>", optional)]
    pub tags: Vec<String>,
    /// Resident capacity.
    pub capacity: u16,
    /// Workplace slots, master included.
    pub workplace_slots: u16,
    /// Building wealth, 0–255.
    pub wealth: u8,
}
