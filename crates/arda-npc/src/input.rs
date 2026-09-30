//! Input descriptions of a settlement and its buildings.
//!
//! Ids, the biome, building functions and crafts are the shared `arda-ids`
//! types (integration plan A6), so a settlement record written by
//! `arda-settle` deserialises straight into [`SettlementProfile`] (extra
//! record fields are ignored) and every id travels as a JSON string.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use arda_ids::{Biome, BuildingFunction, BuildingId, Craft, RealmId, SettlementId};

/// Settlement size class (`logic/06` step 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// A handful of farms, roughly 20–100 people.
    Hamlet,
    /// A village, roughly 100–1,000 people.
    Village,
    /// A town, roughly 1,000–8,000 people.
    Town,
    /// A city, 8,000 people or more.
    City,
}

impl Tier {
    /// The key used in `data/content/tiers.json`.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Hamlet => "hamlet",
            Self::Village => "village",
            Self::Town => "town",
            Self::City => "city",
        }
    }
}

/// What a settlement lives on. A settlement usually has several.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettlementFunction {
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

impl SettlementFunction {
    /// The key used in `data/content/occupations.json`.
    #[must_use]
    pub fn key(self) -> &'static str {
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

/// One settlement to populate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementProfile {
    /// Settlement identifier; part of every seed key.
    pub id: SettlementId,
    /// Display name, used in personality bonds.
    pub name: String,
    /// Size class.
    pub tier: Tier,
    /// Number of inhabitants to generate. Must fit the buildings' capacity.
    pub population: u32,
    /// Economic functions; drive the land-job mix.
    pub functions: Vec<SettlementFunction>,
    /// Overall wealth, 0 (destitute) to 255 (opulent).
    pub wealth: u8,
    /// Culture key for names and the ancestry mix (see `data/content/cultures.json`).
    pub culture: String,
    /// Realm the settlement belongs to.
    pub realm_id: RealmId,
    /// Biome, from the closed settlement list; informational.
    pub biome: Biome,
    /// On a sea coast.
    pub coastal: bool,
    /// On a river.
    pub riverine: bool,
    /// Optional per-settlement ancestry weights (ancestry key → weight) that
    /// replace the culture's table.
    #[serde(default)]
    pub ancestry_mix: Option<BTreeMap<String, u32>>,
    /// The local speech `arda-settle` named the settlement in; its people
    /// are named in it too (A5). Without it the culture's standard
    /// language is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tongue: Option<arda_names::Tongue>,
}

/// A notable slot fixed by the society layer (`arda-society` `NpcRole`,
/// logic/14 §soc-offices; adapter A14): a role in a building, and the name
/// and sex history gave the holder, if any. The generator binds each slot
/// to one inhabitant, who becomes a stored notable with that name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotableSlot {
    /// Stable role id (`r<settlement>.<kind>`).
    pub id: String,
    /// Role kind (`ruler`, `lord`, `reeve`, `captain`, …).
    pub kind: String,
    /// Title the holder is addressed by; replaces the job title.
    pub title: String,
    /// Workplace building, if the role has one.
    #[serde(default)]
    pub building: Option<BuildingId>,
    /// Given name fixed by history.
    #[serde(default)]
    pub given_name: Option<String>,
    /// Family name fixed by history; the holder's household shares it.
    #[serde(default)]
    pub family_name: Option<String>,
    /// Sex fixed by history.
    #[serde(default)]
    pub female: Option<bool>,
}

impl SettlementProfile {
    /// Whether the settlement has `function`.
    #[must_use]
    pub fn has(&self, function: SettlementFunction) -> bool {
        self.functions.contains(&function)
    }
}

/// One building of a settlement.
///
/// `function` is a plain snake_case string (vocabulary I7). A workshop's
/// craft travels as a free `craft:<key>` tag in `tags`; a workshop without
/// one practises a craft drawn from its own seed key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingSpec {
    /// Building identifier, unique within the settlement.
    pub id: BuildingId,
    /// Owning settlement; must equal the profile's id.
    pub settlement_id: SettlementId,
    /// What the building is for.
    pub function: BuildingFunction,
    /// Free tags such as `craft:weaving`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// How many people can live here.
    pub capacity: u16,
    /// How many people work here, master included.
    pub workplace_slots: u16,
    /// Building wealth, 0–255.
    pub wealth: u8,
}

impl BuildingSpec {
    /// The craft named by the first valid `craft:<key>` tag.
    #[must_use]
    pub fn craft_tag(&self) -> Option<Craft> {
        self.tags.iter().find_map(|tag| {
            let key = tag.strip_prefix("craft:")?;
            Craft::ALL.into_iter().find(|c| c.key() == key)
        })
    }

    /// Tags for a workshop practising `craft`.
    #[must_use]
    pub fn craft_tags(craft: Craft) -> Vec<String> {
        vec![craft.tag()]
    }
}

/// A readable name such as "market hall" or "weaving workshop".
#[must_use]
pub fn label(function: BuildingFunction, craft: Option<Craft>) -> String {
    match (function, craft) {
        (BuildingFunction::Workshop, Some(craft)) => format!("{} workshop", craft.key()),
        (other, _) => other.key().replace('_', " "),
    }
}
