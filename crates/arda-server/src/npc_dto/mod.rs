//! Wire mirrors of the `arda-npc` types (I31, A12; `logic/16` §api-bindings).
//!
//! `arda-npc` stays free of ts-rs, so the server owns these mirrors. They
//! exist for the TypeScript bindings: handlers serialise the domain types
//! directly, and the tests prove that every mirror reads and writes the
//! domain JSON unchanged (`deny_unknown_fields` catches a field the mirror
//! lacks; equality of the re-serialised value catches the rest). Every id is
//! an `arda-ids` u64 and travels as a decimal string.

mod input;
mod sheet;

pub use input::{
    Biome, BuildingFunction, BuildingSpec, SettlementFunction, SettlementProfile, Tier, Tongue,
};
pub use sheet::{
    Ability, Attack, AttackKind, Background, Coins, Item, SaveBonus, Sheet, SheetKind, SkillBonus,
    SpellRef, Spellcasting,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// An inhabitant's id: the `arda-ids` u64 as a decimal string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
pub struct NpcId(pub String);

/// A household: its home building and index inside it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HouseholdId {
    /// Home building id.
    pub building: String,
    /// Index among the building's households.
    pub index: u32,
}

/// Biological sex.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Sex {
    /// Female.
    Female,
    /// Male.
    Male,
}

/// A person's name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Name {
    /// Given name.
    pub given: String,
    /// Family name.
    pub family: String,
    /// Optional nickname or epithet.
    pub byname: Option<String>,
}

/// Broad kind of work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobCategory {
    /// Fields and herds.
    Agriculture,
    /// Workshops and trades.
    Craft,
    /// Buying and selling.
    Trade,
    /// Inns, kitchens and households.
    Service,
    /// Clergy.
    Religion,
    /// Soldiers and watch.
    Military,
    /// Officials.
    Government,
    /// Boats and docks.
    Maritime,
    /// Mines and forests.
    Extraction,
    /// Books and teaching.
    Scholar,
    /// Unskilled work.
    Labour,
    /// Nobles.
    Nobility,
    /// Children and the retired.
    Dependent,
}

/// A person's occupation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Job {
    /// Occupation key.
    pub key: String,
    /// Display title.
    pub title: String,
    /// Broad category.
    pub category: JobCategory,
}

/// Standing in local society.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SocialRank {
    /// Children and dependants.
    Dependent,
    /// Labourers, hands and servants.
    Labourer,
    /// Journeymen, clerks and small traders.
    Tradesfolk,
    /// Masters, owners and minor officials.
    Master,
    /// Clergy leads, officers and magistrates.
    Gentry,
    /// Lords and ladies.
    Noble,
}

/// SRD 5.1 lifestyle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Lifestyle {
    /// Wretched.
    Wretched,
    /// Squalid.
    Squalid,
    /// Poor.
    Poor,
    /// Modest.
    Modest,
    /// Comfortable.
    Comfortable,
    /// Wealthy.
    Wealthy,
    /// Aristocratic.
    Aristocratic,
}

/// An ideal and its alignment lean.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Ideal {
    /// Short name.
    pub name: String,
    /// The ideal in the person's words.
    pub text: String,
    /// Alignment lean.
    pub alignment: String,
    /// Coherence tags.
    pub tags: Vec<String>,
}

/// Personality.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Personality {
    /// Two traits.
    pub traits: Vec<String>,
    /// One ideal.
    pub ideal: Ideal,
    /// One bond.
    pub bond: String,
    /// One flaw.
    pub flaw: String,
    /// A voice or mannerism hook.
    pub mannerism: String,
    /// One-line summary.
    pub summary: String,
}

/// How two people are related.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    /// Husband or wife.
    Spouse,
    /// Mother or father.
    Parent,
    /// Son or daughter.
    Child,
    /// Brother or sister.
    Sibling,
    /// Householder a lodger rents from.
    Landlord,
    /// Lodger.
    Lodger,
    /// Master of the workplace.
    Employer,
    /// Works under this person.
    Employee,
    /// Close friend.
    Friend,
    /// Rival or enemy.
    Rival,
}

/// One relationship: `other` is this person's `kind`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Relationship {
    /// What the other person is to this one.
    pub kind: RelationKind,
    /// The other person.
    pub other: NpcId,
}

/// One inhabitant with a full SRD 5.1 sheet (`logic/13` §npc-game-schema).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Npc {
    /// Stable id.
    pub id: NpcId,
    /// Name.
    pub name: Name,
    /// SRD race key.
    pub ancestry: String,
    /// SRD race display name.
    pub ancestry_name: String,
    /// SRD subrace.
    pub subrace: Option<String>,
    /// Age in years.
    pub age: u16,
    /// Sex.
    pub sex: Sex,
    /// Occupation.
    pub job: Job,
    /// Workplace building id.
    pub workplace_building: Option<String>,
    /// Home building id.
    pub home_building: String,
    /// Household.
    pub household: HouseholdId,
    /// Master of the workplace, for employees.
    pub employer: Option<NpcId>,
    /// Social standing.
    pub social_rank: SocialRank,
    /// Wealth, 0–255.
    pub wealth: u8,
    /// SRD lifestyle.
    pub lifestyle: Lifestyle,
    /// Personality.
    pub personality: Personality,
    /// Ties, sorted.
    pub relationships: Vec<Relationship>,
    /// SRD 5.1 character sheet.
    pub sheet: Sheet,
    /// Whether the person is a stored notable.
    pub notable: bool,
}

/// One family tie inside a household: `to` is `from`'s `kind`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HouseholdRelation {
    /// The person the relation belongs to.
    pub from: NpcId,
    /// What `to` is to `from`.
    pub kind: RelationKind,
    /// The other person.
    pub to: NpcId,
}

/// People sharing a home.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Household {
    /// Household id.
    pub id: HouseholdId,
    /// Home building id.
    pub home: String,
    /// Family name of the head.
    pub family_name: String,
    /// Members, head first.
    pub members: Vec<NpcId>,
    /// Family and lodger ties, both directions.
    pub relations: Vec<HouseholdRelation>,
}

/// Compact index entry for one inhabitant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RosterEntry {
    /// Person.
    pub id: NpcId,
    /// Index into `Population.job_keys`.
    pub job: u16,
    /// Workplace building id.
    pub workplace: Option<String>,
    /// Whether the person is a stored notable.
    pub notable: bool,
}

/// The population of one settlement; notables stored in full.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Population {
    /// Settlement id.
    pub settlement_id: String,
    /// Total inhabitants.
    pub population: u32,
    /// Every household.
    pub households: Vec<Household>,
    /// Stored notables.
    pub npcs: Vec<Npc>,
    /// Inhabitants who are not stored.
    pub commoner_count: u32,
    /// Job keys referenced by the roster.
    pub job_keys: Vec<String>,
    /// Category of each job key.
    pub job_categories: Vec<JobCategory>,
    /// One entry per inhabitant, in id order.
    pub roster: Vec<RosterEntry>,
}

/// Body of `POST /v1/npc/population`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PopulationRequest {
    /// World seed as a decimal string.
    pub world_seed: String,
    /// The settlement to populate.
    pub settlement: SettlementProfile,
    /// Its buildings.
    pub buildings: Vec<BuildingSpec>,
}

/// Body of `GET /v1/npc/demo`: the market-town example and its population.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NpcDemo {
    /// World seed as a decimal string.
    pub world_seed: String,
    /// The example settlement.
    pub settlement: SettlementProfile,
    /// Its buildings.
    pub buildings: Vec<BuildingSpec>,
    /// Its population.
    pub population: Population,
}

#[cfg(test)]
mod tests;
