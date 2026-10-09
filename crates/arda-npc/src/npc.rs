//! The generated person and the small types around it.

use serde::{Deserialize, Serialize};

use crate::input::BuildingId;
use crate::sheet::Sheet;

/// Stable identity of an inhabitant: the `arda-ids` u64 derived from the
/// settlement, the home building and the person's index among its residents
/// ([`NpcId::from_parts`]). The same id always names the same person for a
/// given world seed and inputs, and it travels as a JSON string.
pub use arda_ids::NpcId;

/// Identity of a household: its home building and index inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HouseholdId {
    /// Home building.
    pub building: BuildingId,
    /// Index among the building's households.
    pub index: u32,
}

/// Biological sex, used for names and gendered titles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sex {
    /// Female.
    Female,
    /// Male.
    Male,
}

/// A person's name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Name {
    /// Given name.
    pub given: String,
    /// Family name, shared within a family household.
    pub family: String,
    /// Optional nickname or epithet.
    pub byname: Option<String>,
}

impl Name {
    /// "Given Family" or "Given 'Byname' Family".
    #[must_use]
    pub fn full(&self) -> String {
        match &self.byname {
            Some(b) => format!("{} \"{}\" {}", self.given, b, self.family),
            None => format!("{} {}", self.given, self.family),
        }
    }
}

/// Broad kind of work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

impl JobCategory {
    /// Parses a category key from the occupation table.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "agriculture" => Self::Agriculture,
            "craft" => Self::Craft,
            "trade" => Self::Trade,
            "service" => Self::Service,
            "religion" => Self::Religion,
            "military" => Self::Military,
            "government" => Self::Government,
            "maritime" => Self::Maritime,
            "extraction" => Self::Extraction,
            "scholar" => Self::Scholar,
            "labour" => Self::Labour,
            "nobility" => Self::Nobility,
            "dependent" => Self::Dependent,
            _ => return None,
        })
    }
}

/// A person's occupation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Job {
    /// Occupation key in `occupations.json`, e.g. `smith`.
    pub key: String,
    /// Display title, e.g. "Master Smith" or "Master Weaver".
    pub title: String,
    /// Broad category.
    pub category: JobCategory,
}

/// Standing in local society.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

impl SocialRank {
    /// Rank from the occupation table's 0–5 scale.
    #[must_use]
    pub fn from_level(level: u8) -> Self {
        match level {
            0 => Self::Dependent,
            1 => Self::Labourer,
            2 => Self::Tradesfolk,
            3 => Self::Master,
            4 => Self::Gentry,
            _ => Self::Noble,
        }
    }
}

/// SRD 5.1 lifestyle, derived from wealth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lifestyle {
    /// Wretched lifestyle.
    Wretched,
    /// Squalid lifestyle.
    Squalid,
    /// Poor lifestyle.
    Poor,
    /// Modest lifestyle.
    Modest,
    /// Comfortable lifestyle.
    Comfortable,
    /// Wealthy lifestyle.
    Wealthy,
    /// Aristocratic lifestyle.
    Aristocratic,
}

impl Lifestyle {
    /// Lifestyle of a 0–255 wealth score.
    #[must_use]
    pub fn from_wealth(wealth: u8) -> Self {
        match wealth {
            0..=19 => Self::Wretched,
            20..=49 => Self::Squalid,
            50..=89 => Self::Poor,
            90..=139 => Self::Modest,
            140..=179 => Self::Comfortable,
            180..=219 => Self::Wealthy,
            _ => Self::Aristocratic,
        }
    }

    /// The inclusive wealth range of the lifestyle (`from_wealth`).
    #[must_use]
    pub fn wealth_range(self) -> (u8, u8) {
        match self {
            Self::Wretched => (0, 19),
            Self::Squalid => (20, 49),
            Self::Poor => (50, 89),
            Self::Modest => (90, 139),
            Self::Comfortable => (140, 179),
            Self::Wealthy => (180, 219),
            Self::Aristocratic => (220, 255),
        }
    }
}

/// An ideal and the alignment it leans toward.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ideal {
    /// Short name, e.g. "Duty".
    pub name: String,
    /// The ideal in the person's words.
    pub text: String,
    /// Alignment lean, e.g. "lawful good".
    pub alignment: String,
    /// Coherence tags such as `faith` or `wealth`.
    pub tags: Vec<String>,
}

/// Personality from Arda's original tables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Personality {
    /// Two traits on different axes.
    pub traits: Vec<String>,
    /// One ideal, compatible with the job.
    pub ideal: Ideal,
    /// One bond, filled in with real people and places.
    pub bond: String,
    /// One flaw.
    pub flaw: String,
    /// A voice or mannerism hook.
    pub mannerism: String,
    /// One-line summary.
    pub summary: String,
}

/// How two people are related. Every kind has an inverse, and the
/// generator guarantees both directions exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
    /// Lodger in this person's household.
    Lodger,
    /// Master of this person's workplace.
    Employer,
    /// Works under this person.
    Employee,
    /// Close friend.
    Friend,
    /// Rival or enemy.
    Rival,
}

impl RelationKind {
    /// The same relation seen from the other person.
    #[must_use]
    pub fn inverse(self) -> Self {
        match self {
            Self::Spouse => Self::Spouse,
            Self::Parent => Self::Child,
            Self::Child => Self::Parent,
            Self::Sibling => Self::Sibling,
            Self::Landlord => Self::Lodger,
            Self::Lodger => Self::Landlord,
            Self::Employer => Self::Employee,
            Self::Employee => Self::Employer,
            Self::Friend => Self::Friend,
            Self::Rival => Self::Rival,
        }
    }
}

/// One relationship: `other` is this person's `kind`
/// (e.g. `kind: Parent` means `other` is the parent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Relationship {
    /// What the other person is to this one.
    pub kind: RelationKind,
    /// The other person.
    pub other: NpcId,
}

/// One inhabitant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Npc {
    /// Stable id.
    pub id: NpcId,
    /// Name.
    pub name: Name,
    /// SRD race key, e.g. `half-elf`.
    pub ancestry: String,
    /// SRD race display name.
    pub ancestry_name: String,
    /// SRD subrace, if the race has one.
    pub subrace: Option<String>,
    /// Age in years.
    pub age: u16,
    /// Sex.
    pub sex: Sex,
    /// Occupation.
    pub job: Job,
    /// Where the person works, if at a building.
    pub workplace_building: Option<BuildingId>,
    /// Where the person lives.
    pub home_building: BuildingId,
    /// Household.
    pub household: HouseholdId,
    /// The master of the workplace, for employees.
    pub employer: Option<NpcId>,
    /// Social standing.
    pub social_rank: SocialRank,
    /// Wealth, 0–255.
    pub wealth: u8,
    /// SRD lifestyle matching the wealth.
    pub lifestyle: Lifestyle,
    /// Personality.
    pub personality: Personality,
    /// Family, work, friend and rival ties, sorted.
    pub relationships: Vec<Relationship>,
    /// SRD 5.1 character sheet.
    pub sheet: Sheet,
    /// Whether the person is a stored notable.
    pub notable: bool,
}
