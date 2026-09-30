//! Culture presets: the phonological and naming "flavour" a seed varies.
//!
//! Rule: a preset fixes the character of a language (which sounds, which
//! syllable shapes, which naming customs); the seed then drops and reweights
//! sounds and picks grammar options, so two seeds of one preset are related
//! but distinct languages.

mod ancestry;
mod human;

use crate::error::NamesError;
use serde::{Deserialize, Serialize};

/// Where the main stress falls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stress {
    /// First syllable.
    Initial,
    /// Second to last syllable.
    Penult,
    /// Last syllable.
    Final,
}

/// How family names are formed; weights live in [`Spec::family`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FamilyStyle {
    /// Son or daughter of the father ("Kelmarsen").
    Patronymic,
    /// From a trade ("Smith").
    Occupational,
    /// From the home place ("of Oakford").
    Toponymic,
    /// A descriptive two-element house name ("Stonehand").
    Compound,
    /// Kin of a named ancestor ("kin of Brann").
    Clan,
}

/// All family styles in [`Spec::family`] weight order.
pub const FAMILY_STYLES: [FamilyStyle; 5] = [
    FamilyStyle::Patronymic,
    FamilyStyle::Occupational,
    FamilyStyle::Toponymic,
    FamilyStyle::Compound,
    FamilyStyle::Clan,
];

/// Static description of a preset. Phonemes are table codes (see
/// [`crate::phoneme::TABLE`]) with optional integer weights (`"t4"`);
/// clusters and affixes join codes with `+`; alternatives split on `|`.
#[derive(Debug, Clone, Copy)]
pub struct Spec {
    /// Display name.
    pub name: &'static str,
    /// Consonants with weights.
    pub consonants: &'static str,
    /// Vowels with weights.
    pub vowels: &'static str,
    /// Diphthongs, e.g. `"a+i e+i"`.
    pub diphthongs: &'static str,
    /// Allowed onset clusters.
    pub clusters: &'static str,
    /// Single codas with weights.
    pub codas: &'static str,
    /// Allowed coda clusters.
    pub coda_clusters: &'static str,
    /// Consonants never found word-initially.
    pub no_initial: &'static str,
    /// Per mille of words that begin with a consonant.
    pub initial_onset: u32,
    /// Per mille of onsets that are clusters.
    pub cluster_rate: u32,
    /// Per mille of non-final syllables that are closed.
    pub medial_coda: u32,
    /// Per mille of words that end in a consonant.
    pub final_coda: u32,
    /// Per mille of nuclei that are diphthongs.
    pub diph_rate: u32,
    /// Vowels may meet across a syllable boundary.
    pub hiatus: bool,
    /// Front/back vowel harmony within words.
    pub harmony: bool,
    /// Double consonants allowed between vowels.
    pub geminates: bool,
    /// Most consonants in a row inside a word (2 or 3).
    pub max_medial: u8,
    /// Weights for roots of 1, 2 and 3 syllables.
    pub roots: [u32; 3],
    /// Stress rule.
    pub stress: Stress,
    /// Per mille chance a seed makes the language head-first ("Ford-of-Oaks").
    pub head_first: u32,
    /// Per mille chance a head-first language uses a genitive linker.
    pub linker: u32,
    /// Epenthetic vowel code for breaking clusters.
    pub epenthetic: &'static str,
    /// Spelling overrides, `"k=c x=ch"`.
    pub ortho: &'static str,
    /// Word-final spelling overrides.
    pub final_spell: &'static str,
    /// Spelling of /k/ before front vowels (empty: unchanged).
    pub k_front: &'static str,
    /// Feminine given-name endings.
    pub fem: &'static str,
    /// Masculine given-name endings.
    pub masc: &'static str,
    /// Weights of [`FAMILY_STYLES`].
    pub family: [u32; 5],
    /// Per mille of adults with a byname.
    pub byname: u32,
    /// Per mille of consonants a seed may drop from the inventory.
    pub drop: u32,
}

/// Defaults the presets override.
pub(crate) const BASE: Spec = Spec {
    name: "",
    consonants: "",
    vowels: "a5 e4 i4 o3 u3",
    diphthongs: "",
    clusters: "",
    codas: "n4 r3 l3 s2",
    coda_clusters: "",
    no_initial: "ng '",
    initial_onset: 850,
    cluster_rate: 150,
    medial_coda: 450,
    final_coda: 600,
    diph_rate: 60,
    hiatus: false,
    harmony: false,
    geminates: false,
    max_medial: 2,
    roots: [4, 5, 1],
    stress: Stress::Initial,
    head_first: 100,
    linker: 300,
    epenthetic: "e",
    ortho: "",
    final_spell: "",
    k_front: "",
    fem: "a",
    masc: "",
    family: [1, 1, 1, 1, 0],
    byname: 150,
    drop: 70,
};

/// A culture or ancestry preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    /// Mellow lowland farming folk.
    Heartland,
    /// Clipped, consonant-heavy hill folk.
    Highland,
    /// Soft, breathy forest folk.
    Sylvan,
    /// Harsh northern seafarers.
    Coastal,
    /// Flowing, open-syllabled warm-country folk.
    Southern,
    /// Steppe marchfolk with vowel harmony.
    Borderland,
    /// Dwarvish: heavy, closed, back-vowelled.
    Dwarvish,
    /// Elvish: sibilant, vowel-rich, end-stressed.
    Elvish,
    /// Halfling: bouncy, homely, doubled consonants.
    Halfling,
    /// Gnomish: quick, long, buzzing.
    Gnomish,
    /// Orcish: guttural with glottal breaks.
    Orcish,
    /// Draconic: hissing, rolling, long vowels.
    Draconic,
    /// Infernal: dark, sonorous, buzzing.
    Infernal,
    /// The pre-settlement substrate tongue river names come from.
    Ancient,
}

impl Preset {
    /// Every preset except [`Preset::Ancient`].
    pub const ALL: [Preset; 13] = [
        Preset::Heartland,
        Preset::Highland,
        Preset::Sylvan,
        Preset::Coastal,
        Preset::Southern,
        Preset::Borderland,
        Preset::Dwarvish,
        Preset::Elvish,
        Preset::Halfling,
        Preset::Gnomish,
        Preset::Orcish,
        Preset::Draconic,
        Preset::Infernal,
    ];

    /// The static description.
    #[must_use]
    pub fn spec(self) -> &'static Spec {
        match self {
            Preset::Heartland => &human::HEARTLAND,
            Preset::Highland => &human::HIGHLAND,
            Preset::Sylvan => &human::SYLVAN,
            Preset::Coastal => &human::COASTAL,
            Preset::Southern => &human::SOUTHERN,
            Preset::Borderland => &human::BORDERLAND,
            Preset::Dwarvish => &ancestry::DWARVISH,
            Preset::Elvish => &ancestry::ELVISH,
            Preset::Halfling => &ancestry::HALFLING,
            Preset::Gnomish => &ancestry::GNOMISH,
            Preset::Orcish => &ancestry::ORCISH,
            Preset::Draconic => &ancestry::DRACONIC,
            Preset::Infernal => &ancestry::INFERNAL,
            Preset::Ancient => &human::ANCIENT,
        }
    }

    /// Stable key.
    #[must_use]
    pub fn key(self) -> &'static str {
        self.spec().name
    }

    /// Looks a preset up by key. Accepts `arda-settle` culture keys
    /// (`heartland`, `coastal`, ...) and SRD ancestry keys (`dwarf`,
    /// `half-orc`, `dragonborn`, `tiefling`, ...).
    ///
    /// # Errors
    /// [`NamesError::UnknownPreset`] for anything else.
    pub fn from_key(key: &str) -> Result<Self, NamesError> {
        let k = key.trim().to_ascii_lowercase().replace(['_', ' '], "-");
        let p = match k.as_str() {
            "dwarf" | "hill-dwarf" | "mountain-dwarf" => Preset::Dwarvish,
            "elf" | "high-elf" | "wood-elf" | "dark-elf" | "drow" => Preset::Elvish,
            "halfling" | "lightfoot" | "stout" => Preset::Halfling,
            "gnome" | "rock-gnome" | "forest-gnome" => Preset::Gnomish,
            "orc" | "half-orc" => Preset::Orcish,
            "dragonborn" | "dragon" => Preset::Draconic,
            "tiefling" | "fiend" => Preset::Infernal,
            other => Self::ALL
                .iter()
                .chain(std::iter::once(&Preset::Ancient))
                .copied()
                .find(|p| p.key() == other)
                .ok_or_else(|| NamesError::UnknownPreset(key.to_string()))?,
        };
        Ok(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phoneme::Ph;

    fn codes(s: &str) -> impl Iterator<Item = &str> {
        s.split([' ', '|', '+'])
            .map(|t| t.trim_end_matches(|c: char| c.is_ascii_digit()))
            .filter(|t| !t.is_empty())
    }

    #[test]
    fn every_preset_code_parses() {
        for p in Preset::ALL.iter().chain(std::iter::once(&Preset::Ancient)) {
            let s = p.spec();
            let fields = [
                s.consonants,
                s.vowels,
                s.diphthongs,
                s.clusters,
                s.codas,
                s.coda_clusters,
                s.no_initial,
                s.epenthetic,
                s.fem,
                s.masc,
            ];
            for f in fields {
                for code in codes(f) {
                    assert!(Ph::from_code(code).is_some(), "{}: bad code {code}", s.name);
                }
            }
            for pair in s
                .ortho
                .split_whitespace()
                .chain(s.final_spell.split_whitespace())
            {
                let code = pair.split('=').next().unwrap_or("");
                assert!(
                    Ph::from_code(code).is_some(),
                    "{}: bad ortho {pair}",
                    s.name
                );
            }
            assert_eq!(Preset::from_key(p.key()).ok(), Some(*p));
        }
        assert_eq!(Preset::from_key("Half-Orc").ok(), Some(Preset::Orcish));
        assert!(Preset::from_key("martian").is_err());
    }
}
