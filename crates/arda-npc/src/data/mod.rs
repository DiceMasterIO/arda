//! Bundled JSON data, parsed once and shared.

pub mod content;
pub mod srd;

use std::sync::OnceLock;

use serde::de::DeserializeOwned;

use crate::error::NpcError;
use content::{Backgrounds, Cultures, JobData, Occupations, PersonalityTables, Tiers};
use srd::Srd;

/// All bundled data.
#[derive(Debug, Clone)]
pub struct Data {
    pub srd: Srd,
    pub personality: PersonalityTables,
    pub occupations: Occupations,
    pub cultures: Cultures,
    pub tiers: Tiers,
    pub backgrounds: Backgrounds,
}

static DATA: OnceLock<Result<Data, String>> = OnceLock::new();

fn parse<T: DeserializeOwned>(file: &str, text: &str) -> Result<T, String> {
    serde_json::from_str(text).map_err(|e| format!("{file}: {e}"))
}

impl Data {
    /// The shared data, parsed on first use.
    ///
    /// # Errors
    /// [`NpcError::Data`] when a bundled file does not parse or is inconsistent.
    pub fn get() -> Result<&'static Self, NpcError> {
        DATA.get_or_init(Self::load)
            .as_ref()
            .map_err(|e| NpcError::Data(e.clone()))
    }

    fn load() -> Result<Self, String> {
        let data = Self {
            srd: Srd {
                races: parse("races.json", include_str!("../../data/srd/races.json"))?,
                classes: parse("classes.json", include_str!("../../data/srd/classes.json"))?,
                slots: parse(
                    "spellcasting.json",
                    include_str!("../../data/srd/spellcasting.json"),
                )?,
                spells: parse("spells.json", include_str!("../../data/srd/spells.json"))?,
                stat_blocks: parse(
                    "stat_blocks.json",
                    include_str!("../../data/srd/stat_blocks.json"),
                )?,
                equipment: parse(
                    "equipment.json",
                    include_str!("../../data/srd/equipment.json"),
                )?,
                languages: parse(
                    "languages.json",
                    include_str!("../../data/srd/languages.json"),
                )?,
            },
            personality: parse(
                "personality.json",
                include_str!("../../data/content/personality.json"),
            )?,
            occupations: parse(
                "occupations.json",
                include_str!("../../data/content/occupations.json"),
            )?,
            cultures: parse(
                "cultures.json",
                include_str!("../../data/content/cultures.json"),
            )?,
            tiers: parse("tiers.json", include_str!("../../data/content/tiers.json"))?,
            backgrounds: parse(
                "backgrounds.json",
                include_str!("../../data/content/backgrounds.json"),
            )?,
        };
        data.validate()?;
        Ok(data)
    }

    /// Cross-file references must resolve.
    fn validate(&self) -> Result<(), String> {
        for (key, job) in &self.occupations.jobs {
            if self.srd.stat_block(&job.stat_block).is_none() {
                return Err(format!("job {key}: unknown stat block {}", job.stat_block));
            }
            if let Some(class) = job.classes.keys().find(|c| self.srd.class(c).is_none()) {
                return Err(format!("job {key}: unknown class {class}"));
            }
            if let Some(item) = job
                .tools
                .iter()
                .chain(&job.gear)
                .find(|i| !self.srd.is_item(i))
            {
                return Err(format!("job {key}: {item} is not SRD equipment"));
            }
            if !self.backgrounds.backgrounds.contains_key(&job.category) {
                return Err(format!("job {key}: no background for {}", job.category));
            }
        }
        for (key, place) in &self.occupations.workplaces {
            for job in place
                .master
                .iter()
                .chain(&place.workers)
                .chain(&place.family_job)
            {
                self.job(job)
                    .map_err(|_| format!("workplace {key}: unknown job {job}"))?;
            }
        }
        for land in &self.occupations.land_jobs {
            self.job(&land.job)
                .map_err(|_| format!("land job {}", land.job))?;
        }
        for tier in self.tiers.tiers.values() {
            let promote = tier.promote.iter().filter(|j| *j != content::KEEP_LAND_JOB);
            for job in tier.officials.iter().chain(promote) {
                self.job(job).map_err(|_| format!("tier job {job}"))?;
            }
        }
        let ranks = self
            .occupations
            .jobs
            .iter()
            .map(|(k, j)| (format!("job {k}"), j.rank, &j.rank_by_tier))
            .chain(
                self.occupations
                    .offices
                    .iter()
                    .map(|(k, o)| (format!("office {k}"), o.rank, &o.rank_by_tier)),
            );
        for (what, rank, by_tier) in ranks {
            if let Some(t) = by_tier.keys().find(|t| !self.tiers.tiers.contains_key(*t)) {
                return Err(format!("{what}: unknown tier {t}"));
            }
            if by_tier.values().chain([&rank]).any(|&r| r > 5) {
                return Err(format!("{what}: rank above 5"));
            }
        }
        for (key, office) in &self.occupations.offices {
            for job in &office.covers {
                self.job(job)
                    .map_err(|_| format!("office {key}: unknown job {job}"))?;
            }
        }
        if let Some((rank, _)) = self
            .occupations
            .rank_lifestyles
            .iter()
            .find(|(_, [lo, hi])| lo > hi)
        {
            return Err(format!("rank {rank:?}: lifestyle band is inverted"));
        }
        for culture in self.cultures.cultures.values() {
            if let Some(a) = culture.ancestry.keys().find(|a| self.srd.race(a).is_none()) {
                return Err(format!("unknown ancestry {a}"));
            }
        }
        for class in &self.srd.classes {
            for armor in class.loadout.armor.iter().flatten() {
                let category = self.srd.armor(armor).map(|a| a.category.to_lowercase());
                if !category.is_some_and(|c| class.armor_training.contains(&c)) {
                    return Err(format!("{}: not trained for {armor}", class.key));
                }
            }
        }
        for block in &self.srd.stat_blocks {
            let spells = block.spellcasting.iter().flat_map(|s| &s.spells);
            if let Some((name, _)) = spells.clone().find(|(n, _)| self.srd.spell(n).is_none()) {
                return Err(format!("{}: unknown spell {name}", block.name));
            }
        }
        for craft in self.occupations.crafts.values() {
            if !self.srd.is_item(&craft.tool) {
                return Err(format!("craft tool {} is not SRD equipment", craft.tool));
            }
        }
        Ok(())
    }

    /// Job by key.
    ///
    /// # Errors
    /// [`NpcError::Data`] for an unknown key.
    pub fn job(&self, key: &str) -> Result<&JobData, NpcError> {
        self.occupations
            .jobs
            .get(key)
            .ok_or_else(|| NpcError::Data(format!("unknown job {key}")))
    }
}
