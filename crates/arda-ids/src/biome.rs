//! The closed settlement biome list (`logic/08` settlement record, I22).
//!
//! Mirrors `arda-settle`'s `model::Biome` value for value, so a settlement
//! record's `biome` deserialises into `arda-npc`'s profile unchanged.

use serde::{Deserialize, Serialize};

/// The biome a settlement reports, serialised snake_case.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Biome {
    /// Open temperate lowland: farmland and grass (the default).
    #[default]
    Temperate,
    /// Warm temperate lowland, 15 °C mean or more.
    WarmTemperate,
    /// Broadleaf or mixed forest.
    TemperateForest,
    /// Cold conifer forest, under 6 °C mean.
    BorealForest,
    /// Hills and mountains, 5 °C mean or more.
    Highland,
    /// Mountains under 5 °C mean.
    Alpine,
    /// Beside marsh or fen.
    Wetland,
    /// Dry grassland.
    Steppe,
    /// On the sea coast.
    Coastal,
}

impl Biome {
    /// Every biome, in declaration order.
    pub const ALL: [Self; 9] = [
        Self::Temperate,
        Self::WarmTemperate,
        Self::TemperateForest,
        Self::BorealForest,
        Self::Highland,
        Self::Alpine,
        Self::Wetland,
        Self::Steppe,
        Self::Coastal,
    ];

    /// The serde key.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Temperate => "temperate",
            Self::WarmTemperate => "warm_temperate",
            Self::TemperateForest => "temperate_forest",
            Self::BorealForest => "boreal_forest",
            Self::Highland => "highland",
            Self::Alpine => "alpine",
            Self::Wetland => "wetland",
            Self::Steppe => "steppe",
            Self::Coastal => "coastal",
        }
    }

    /// The namespaced catalogue tag, `biome:<key>` (vocabulary I8).
    #[must_use]
    pub fn tag(self) -> String {
        format!("biome:{}", self.key())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_match_serde_and_the_settle_list() {
        for b in Biome::ALL {
            assert_eq!(
                serde_json::to_string(&b).unwrap(),
                format!("\"{}\"", b.key())
            );
        }
        assert_eq!(Biome::default(), Biome::Temperate);
        assert!(serde_json::from_str::<Biome>("\"boreal forest\"").is_err());
    }
}
