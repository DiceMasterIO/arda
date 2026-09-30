//! Personal names through `arda-names` (I18, adapter A5; logic/15
//! §name-key).
//!
//! The settlement's people speak its local tongue: the language of its
//! culture with the dialect at its position, as `arda-settle` records it in
//! the settlement's `tongue`. Without a recorded tongue the culture's
//! standard language is built from the world seed. Ancestries with their
//! own tongue (dwarves, elves, halflings, gnomes, dragonborn, half-orcs,
//! tieflings) use their preset's language, one per world.

use crate::input::SettlementProfile;
use crate::npc::Sex;
use arda_names::{FamilyCtx, Language, Name, PersonName, Preset, Tongue};
use std::collections::BTreeMap;

/// Ancestries whose people are named in their own tongue; humans and
/// half-elves take the settlement's (keys are SRD race keys).
const OWN_TONGUE: [&str; 7] = [
    "dwarf",
    "elf",
    "halfling",
    "gnome",
    "dragonborn",
    "half-orc",
    "tiefling",
];

/// The languages one settlement's people are named in.
#[derive(Debug, Clone)]
pub(crate) struct Tongues {
    local: Language,
    ancestry: BTreeMap<&'static str, Language>,
}

fn preset_of(culture: &str) -> Preset {
    Preset::from_key(culture).unwrap_or(Preset::Heartland)
}

impl Tongues {
    /// The local tongue of `settlement` and one language per ancestry with
    /// its own tongue, all keyed by the world seed.
    pub(crate) fn new(seed: u64, settlement: &SettlementProfile) -> Self {
        let local = settlement.tongue.as_ref().map_or_else(
            || {
                let preset = preset_of(&settlement.culture);
                Tongue {
                    preset,
                    language_seed: arda_ids::hash::subseed(seed, "npc-language", &[0]),
                    substrate_seed: arda_ids::hash::subseed(seed, "npc-substrate", &[0]),
                    dialect_seed: 0,
                    width: 1,
                    height: 1,
                    x: 0,
                    y: 0,
                }
                .standard()
            },
            Tongue::language,
        );
        let ancestry = OWN_TONGUE
            .iter()
            .filter_map(|&key| {
                let preset = Preset::from_key(key).ok()?;
                let lang_seed = arda_ids::hash::subseed(seed, "npc-ancestry-language", &[0]);
                Some((key, Language::new(lang_seed, preset)))
            })
            .collect();
        Self { local, ancestry }
    }

    /// The language a person of `ancestry` is named in.
    pub(crate) fn of(&self, ancestry: &str) -> &Language {
        self.ancestry.get(ancestry).unwrap_or(&self.local)
    }
}

fn sex(s: Sex) -> arda_names::Sex {
    match s {
        Sex::Female => arda_names::Sex::Female,
        Sex::Male => arda_names::Sex::Male,
    }
}

/// A household's family name, drawn from the household's own key.
pub(crate) fn house_family(lang: &Language, key: u64) -> Option<Name> {
    arda_names::person_name(lang, arda_names::Sex::Male, &FamilyCtx::new(key)).family
}

/// A person's name: an inherited family name when they live in a family
/// household, else one of their own.
pub(crate) fn person(
    lang: &Language,
    s: Sex,
    key: u64,
    family: Option<Name>,
    adult: bool,
) -> PersonName {
    let ctx = FamilyCtx {
        id: key,
        father: None,
        family,
        trade: None,
        home: None,
        adult,
    };
    arda_names::person_name(lang, sex(s), &ctx)
}
