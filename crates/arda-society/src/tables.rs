//! Embedded JSON data tables (constraint: data in JSON, not in code). All
//! text material is original to Arda; stat-block and class names are SRD 5.1.

use crate::error::SocietyError;
use crate::input::Tier;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::BTreeMap;

/// `[good key, amount]`.
#[derive(Debug, Clone, Deserialize)]
pub struct Yield(pub String, pub u32);

/// One trade good.
#[derive(Debug, Clone, Deserialize)]
pub struct Good {
    /// Stable key.
    pub key: String,
    /// Readable name.
    pub label: String,
    /// Value of one load, silver pieces.
    pub value_sp: u32,
    /// How far a load is worth carrying, km.
    pub range_km: u32,
    /// Loads consumed per 100 people a year.
    pub demand_per_100: u32,
    /// Only towns and cities want it.
    #[serde(default)]
    pub urban_only: bool,
    /// A staple even small places need (drives village shortage hooks).
    #[serde(default)]
    pub staple: bool,
    /// Demand scales with wealth (128 = ×1).
    #[serde(default)]
    pub wealth_scaled: bool,
}

/// A processing input: `per` loads of `good` per 10 loads of output.
#[derive(Debug, Clone, Deserialize)]
pub struct Input {
    /// Input good.
    pub good: String,
    /// Loads of input per 10 loads of output.
    pub per: u32,
}

/// Biome effect on land yields.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct BiomeMod {
    /// Percent multipliers per good (100 = unchanged).
    #[serde(default)]
    pub mult_pct: BTreeMap<String, u32>,
    /// Extra yields per 100 farming workers.
    #[serde(default)]
    pub extra: Vec<Yield>,
}

/// `goods.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct GoodsTable {
    /// All goods.
    pub goods: Vec<Good>,
    /// Percent of people working the land, per tier key.
    pub land_share_pct: BTreeMap<String, u32>,
    /// Land yields per 100 workers, per function key.
    pub land: BTreeMap<String, Vec<Yield>>,
    /// Share of the land workforce each land function draws.
    pub function_weight: BTreeMap<String, u32>,
    /// Yields per building, per building-function key.
    pub buildings: BTreeMap<String, Vec<Yield>>,
    /// Yields per workshop, per craft key.
    pub crafts: BTreeMap<String, Vec<Yield>>,
    /// Craft cycle for workshops derived from a building mix.
    pub workshop_crafts: Vec<String>,
    /// Processing inputs per output good.
    pub inputs: BTreeMap<String, Input>,
    /// Biome modifiers by biome name.
    pub biomes: BTreeMap<String, BiomeMod>,
    /// Yields per 100 people from site tags.
    pub sites: BTreeMap<String, Vec<Yield>>,
}

/// A government type.
#[derive(Debug, Clone, Deserialize)]
pub struct Government {
    /// Readable name.
    pub label: String,
    /// Whether rule passes by blood.
    pub hereditary: bool,
    /// Levy on trade, percent.
    pub tax_pct: u32,
    /// Ruler titles `[male, female]` per realm rank.
    pub titles: BTreeMap<String, [String; 2]>,
    /// Realm style per rank, with `{name}`.
    pub style: BTreeMap<String, String>,
    /// Vassal title `[male, female]`.
    pub vassal: [String; 2],
    /// One-line description.
    pub blurb: String,
    /// Suggested stat build for the ruler of a town or city seat.
    #[serde(default)]
    pub ruler: Option<Suggestion>,
}

/// `politics.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct PoliticsTable {
    /// Government types by key.
    pub governments: BTreeMap<String, Government>,
    /// Special head titles `[male, female]` (`village`, `hamlet`, `fortress`, `border`).
    pub heads: BTreeMap<String, [String; 2]>,
    /// Vassal grievance templates per key.
    pub grievances: BTreeMap<String, String>,
    /// Relation reason templates per key.
    pub reasons: BTreeMap<String, String>,
}

/// One kind of faction.
#[derive(Debug, Clone, Deserialize)]
pub struct FactionKind {
    /// Stable key.
    pub key: String,
    /// Readable kind.
    pub label: String,
    /// Present when the settlement has any of these buildings.
    #[serde(default)]
    pub buildings_any: Vec<String>,
    /// Present when the settlement has any of these functions.
    #[serde(default)]
    pub functions_any: Vec<String>,
    /// Smallest tier where it forms.
    pub min_tier: Tier,
    /// Name templates.
    pub names: Vec<String>,
    /// Goal templates.
    pub goals: Vec<String>,
    /// Goal templates for villages and hamlets, when they differ.
    #[serde(default)]
    pub goals_rural: Vec<String>,
    /// Role kind of its leader.
    pub leader: String,
    /// Role kind of its leader in a hamlet, where one figure speaks for
    /// everyone (`head` = the settlement's elder); `leader` when absent.
    #[serde(default)]
    pub leader_hamlet: Option<String>,
    /// Base influence.
    pub power: u32,
    /// Preferred seat buildings, first match wins.
    pub seat: Vec<String>,
}

/// `[kind a, kind b, affinity]`, affinity −2 (hostile) to +2 (allied).
#[derive(Debug, Clone, Deserialize)]
pub struct Affinity(pub String, pub String, pub i32);

/// `factions.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct FactionTable {
    /// Faction kinds.
    pub kinds: Vec<FactionKind>,
    /// Pairwise base affinities.
    pub affinity: Vec<Affinity>,
    /// Reason templates per stance key.
    pub reasons: BTreeMap<String, Vec<String>>,
    /// Original patron names for temples.
    pub patrons: Vec<String>,
    /// Reasons for a specific kind pair `"a|b"` (keys sorted), used for
    /// rival and hostile stances.
    #[serde(default)]
    pub pair_reasons: BTreeMap<String, Vec<String>>,
}

/// SRD 5.1 stat block or class suggestion.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Suggestion {
    /// SRD NPC stat block name.
    #[serde(default)]
    pub block: Option<String>,
    /// SRD class and level.
    #[serde(default)]
    pub class: Option<(String, u8)>,
}

/// One notable role kind.
#[derive(Debug, Clone, Deserialize)]
pub struct RoleDef {
    /// Title template.
    pub title: String,
    /// Preferred buildings, first match wins.
    pub buildings: Vec<String>,
    /// Stat suggestion per tier key.
    pub by_tier: BTreeMap<String, Suggestion>,
    /// What the role does.
    pub duties: String,
}

/// `roles.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct RoleTable {
    /// Role kinds.
    pub roles: BTreeMap<String, RoleDef>,
}

/// `history.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct HistoryTable {
    /// Era name for dates.
    pub era: String,
    /// Era abbreviation.
    pub era_abbrev: String,
    /// Event titles per event kind.
    pub titles: BTreeMap<String, Vec<String>>,
    /// Event text per event kind.
    pub events: BTreeMap<String, Vec<String>>,
    /// History-hook text per hook kind.
    pub lore: BTreeMap<String, Vec<String>>,
    /// War name patterns.
    pub war_names: Vec<String>,
    /// Plague names.
    pub plague_names: Vec<String>,
    /// How a reign ends, per cause key.
    pub reign_ends: BTreeMap<String, String>,
    /// Ruler epithets.
    pub epithets: Vec<String>,
    /// Ruin kinds per former tier.
    pub ruin_kinds: BTreeMap<String, Vec<String>>,
    /// Site phrase per site tag, in priority order of `site_order`.
    pub sites: BTreeMap<String, String>,
    /// Site tags in the order they are preferred for prose.
    pub site_order: Vec<String>,
    /// Phrase when no tag applies.
    pub site_default: String,
}

/// One campaign-hook template set: `[title, text]` pairs, so a title always
/// matches its text.
#[derive(Debug, Clone, Deserialize)]
pub struct HookDef {
    /// `[title, text]` template pairs.
    pub variants: Vec<[String; 2]>,
}

/// `hooks.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct HookTable {
    /// Hook kinds.
    pub hooks: BTreeMap<String, HookDef>,
    /// Bandit hideouts when no ruin lies on the road.
    pub hideouts: Vec<String>,
}

/// Syllables for one culture.
#[derive(Debug, Clone, Deserialize)]
pub struct CultureNames {
    /// Opening consonant clusters.
    pub onsets: Vec<String>,
    /// Vowels.
    pub nuclei: Vec<String>,
    /// Closing consonants.
    pub codas: Vec<String>,
    /// Male given-name endings.
    pub given_m: Vec<String>,
    /// Female given-name endings.
    pub given_f: Vec<String>,
    /// House-name endings.
    pub house: Vec<String>,
    /// Place-name endings.
    pub place: Vec<String>,
}

/// `names.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct NameTable {
    /// Fallback culture key.
    pub default: String,
    /// Syllables per culture key.
    pub cultures: BTreeMap<String, CultureNames>,
}

/// All tables.
#[derive(Debug, Clone)]
pub struct Tables {
    /// Goods and production.
    pub goods: GoodsTable,
    /// Governments and titles.
    pub politics: PoliticsTable,
    /// Factions.
    pub factions: FactionTable,
    /// Notable roles.
    pub roles: RoleTable,
    /// History text.
    pub history: HistoryTable,
    /// Campaign hooks.
    pub hooks: HookTable,
    /// Names.
    pub names: NameTable,
}

fn parse<T: DeserializeOwned>(table: &'static str, text: &str) -> Result<T, SocietyError> {
    serde_json::from_str(text).map_err(|source| SocietyError::Table { table, source })
}

impl Tables {
    /// Parses and cross-checks the embedded tables.
    ///
    /// # Errors
    /// [`SocietyError::Table`] or [`SocietyError::TableContent`] when a
    /// table is malformed or references something undefined.
    pub fn load() -> Result<Self, SocietyError> {
        let t = Self {
            goods: parse("goods.json", include_str!("../data/goods.json"))?,
            politics: parse("politics.json", include_str!("../data/politics.json"))?,
            factions: parse("factions.json", include_str!("../data/factions.json"))?,
            roles: parse("roles.json", include_str!("../data/roles.json"))?,
            history: parse("history.json", include_str!("../data/history.json"))?,
            hooks: parse("hooks.json", include_str!("../data/hooks.json"))?,
            names: parse("names.json", include_str!("../data/names.json"))?,
        };
        t.check()?;
        Ok(t)
    }

    fn check(&self) -> Result<(), SocietyError> {
        let bad =
            |table: &'static str, detail: String| Err(SocietyError::TableContent { table, detail });
        let g = &self.goods;
        let known = |k: &str| g.goods.iter().any(|x| x.key == k);
        let yields = g
            .land
            .values()
            .chain(g.buildings.values())
            .chain(g.crafts.values())
            .chain(g.sites.values())
            .chain(g.biomes.values().map(|b| &b.extra))
            .flatten();
        for y in yields {
            if !known(&y.0) {
                return bad("goods.json", format!("unknown good {}", y.0));
            }
        }
        for (out, inp) in &g.inputs {
            if !known(out) || !known(&inp.good) {
                return bad("goods.json", format!("unknown input pair {out}"));
            }
        }
        for k in &self.factions.kinds {
            if k.leader != "head" && !self.roles.roles.contains_key(&k.leader) {
                return bad("factions.json", format!("unknown leader role {}", k.leader));
            }
        }
        if !self.names.cultures.contains_key(&self.names.default) {
            return bad("names.json", "default culture missing".to_string());
        }
        if !self.politics.governments.contains_key("monarchy") {
            return bad("politics.json", "missing government monarchy".to_string());
        }
        Ok(())
    }

    /// The good with `key`.
    #[must_use]
    pub fn good(&self, key: &str) -> Option<&Good> {
        self.goods.goods.iter().find(|g| g.key == key)
    }

    /// Readable label of good `key` (the key itself when unknown).
    #[must_use]
    pub fn good_label<'a>(&'a self, key: &'a str) -> &'a str {
        self.good(key).map_or(key, |g| g.label.as_str())
    }
}
