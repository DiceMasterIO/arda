//! Typed view of Arda's original content in `data/content/`.

use std::collections::BTreeMap;

use serde::Deserialize;

/// A personality trait on one axis.
#[derive(Debug, Clone, Deserialize)]
pub struct TraitEntry {
    pub axis: String,
    pub short: String,
    pub text: String,
}

/// An ideal with its alignment lean and coherence tags.
#[derive(Debug, Clone, Deserialize)]
pub struct IdealEntry {
    pub name: String,
    pub alignment: String,
    pub tags: Vec<String>,
    pub short: String,
    pub text: String,
}

/// A bond template.
#[derive(Debug, Clone, Deserialize)]
pub struct BondEntry {
    pub needs: String,
    pub text: String,
}

/// A flaw.
#[derive(Debug, Clone, Deserialize)]
pub struct FlawEntry {
    pub short: String,
    pub text: String,
}

/// `personality.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct PersonalityTables {
    pub traits: Vec<TraitEntry>,
    pub ideals: Vec<IdealEntry>,
    pub bonds: Vec<BondEntry>,
    pub flaws: Vec<FlawEntry>,
    pub mannerisms: Vec<String>,
    pub exclusions: BTreeMap<String, Vec<String>>,
    pub affinities: BTreeMap<String, Vec<String>>,
}

/// One occupation.
#[derive(Debug, Clone, Deserialize)]
pub struct JobData {
    pub title: String,
    pub title_female: Option<String>,
    pub category: String,
    pub rank: u8,
    pub stat_block: String,
    pub classes: BTreeMap<String, u32>,
    pub tools: Vec<String>,
    pub gear: Vec<String>,
    pub leader: bool,
}

/// How a building's residents form households.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HouseholdStyle {
    /// Families, with the odd lodger.
    Family,
    /// Unrelated single adults (garrison, clergy, camp).
    Singles,
    /// One family plus single retainers.
    FamilyWithStaff,
}

/// Staffing of a building function.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkplaceData {
    pub master: Option<String>,
    pub workers: Vec<String>,
    pub notable: bool,
    pub importance: u8,
    pub household: HouseholdStyle,
    #[serde(default)]
    pub family_job: Option<String>,
}

/// A workshop craft.
#[derive(Debug, Clone, Deserialize)]
pub struct CraftData {
    pub noun: String,
    pub tool: String,
}

/// A land job and its weights.
#[derive(Debug, Clone, Deserialize)]
pub struct LandJob {
    pub job: String,
    pub scale: String,
    pub base: u32,
    pub functions: BTreeMap<String, u32>,
    #[serde(default)]
    pub needs: Option<String>,
}

/// `occupations.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct Occupations {
    pub jobs: BTreeMap<String, JobData>,
    pub workplaces: BTreeMap<String, WorkplaceData>,
    pub crafts: BTreeMap<String, CraftData>,
    pub land_jobs: Vec<LandJob>,
}

/// A culture's ancestry weights (its tongue comes from `arda-names`).
#[derive(Debug, Clone, Deserialize)]
pub struct Culture {
    pub ancestry: BTreeMap<String, u32>,
}

/// `cultures.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct Cultures {
    pub default: String,
    pub cultures: BTreeMap<String, Culture>,
}

/// Per-tier parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct TierData {
    pub notables: [u32; 2],
    pub levels: [u8; 2],
    pub leader_levels: [u8; 2],
    pub rural_pm: u32,
    pub urban_pm: u32,
    pub officials: Vec<String>,
    pub promote: Vec<String>,
}

/// Rare high-level exceptions among notables.
#[derive(Debug, Clone, Deserialize)]
pub struct LevelException {
    pub chance_per_mille: u32,
    pub bonus_levels: u8,
    pub max_level: u8,
}

/// `tiers.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct Tiers {
    pub tiers: BTreeMap<String, TierData>,
    pub capital_officials: Vec<String>,
    pub exception: LevelException,
    pub household_sizes: Vec<[u32; 2]>,
}

/// A flavour background.
#[derive(Debug, Clone, Deserialize)]
pub struct BackgroundData {
    pub name: String,
    pub text: String,
    pub skills: Vec<String>,
}

/// `backgrounds.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct Backgrounds {
    pub backgrounds: BTreeMap<String, BackgroundData>,
}
