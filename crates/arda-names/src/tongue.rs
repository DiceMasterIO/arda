//! A serialisable recipe for one place's local speech: a culture's
//! language, the world's shared substrate, and the dialect at a position.
//!
//! `arda-settle` names each settlement in the dialect its culture region
//! speaks at the settlement's cell and writes the recipe beside the record,
//! so later stages (the NPC generator) name the settlement's people in the
//! same local speech without rebuilding settle's culture map (logic/15
//! §name-key; adapter A5).

use crate::dialect::DialectMap;
use crate::language::{Language, Options};
use crate::preset::Preset;
use serde::{Deserialize, Serialize};

/// Everything needed to rebuild a place's local language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Tongue {
    /// The culture's preset.
    pub preset: Preset,
    /// Seed of the culture's standard language.
    #[serde(with = "crate::ids")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub language_seed: u64,
    /// Seed of the world's shared substrate tongue.
    #[serde(with = "crate::ids")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub substrate_seed: u64,
    /// Seed of the dialect map of the culture region.
    #[serde(with = "crate::ids")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub dialect_seed: u64,
    /// Width of the dialect map, in the caller's units (cells).
    pub width: i64,
    /// Height of the dialect map.
    pub height: i64,
    /// Where the place lies on the map.
    pub x: i64,
    /// Where the place lies on the map.
    pub y: i64,
}

impl Tongue {
    /// The culture's standard language (before dialect changes).
    #[must_use]
    pub fn standard(&self) -> Language {
        let opts = Options {
            substrate_seed: Some(self.substrate_seed),
            blocklist: None,
        };
        Language::with_options(self.language_seed, self.preset, &opts)
    }

    /// The local language: the standard one with the sound changes whose
    /// isoglosses the place lies beyond.
    #[must_use]
    pub fn language(&self) -> Language {
        self.language_of(&self.standard())
    }

    /// The local form of `standard` (which must be [`Tongue::standard`]),
    /// for callers that already hold it.
    #[must_use]
    pub fn language_of(&self, standard: &Language) -> Language {
        DialectMap::new(standard, self.dialect_seed, self.width, self.height)
            .dialect_at(self.x, self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tongue_round_trips_and_rebuilds_the_same_language() {
        let t = Tongue {
            preset: Preset::Heartland,
            language_seed: u64::MAX,
            substrate_seed: 3,
            dialect_seed: 4,
            width: 1000,
            height: 2000,
            x: 400,
            y: 1500,
        };
        let j = serde_json::to_value(&t).unwrap();
        assert_eq!(j["language_seed"], "18446744073709551615");
        assert_eq!(j["preset"], "heartland");
        let back: Tongue = serde_json::from_value(j).unwrap();
        assert_eq!(back, t);
        assert_eq!(back.language(), t.language());
    }
}
