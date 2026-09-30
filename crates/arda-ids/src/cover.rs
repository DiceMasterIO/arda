//! Cover degrees (`vocabulary.md` I16; `logic/12` §scene-cover).

use serde::{Deserialize, Serialize};

/// SRD 5.1 cover. `full` is accepted on input as an alias of `total`;
/// output always writes `total`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Cover {
    /// No cover.
    #[default]
    None,
    /// Half cover (+2 AC and Dexterity saves).
    Half,
    /// Three-quarters cover (+5 AC and Dexterity saves).
    ThreeQuarters,
    /// Total cover: cannot be targeted directly.
    #[serde(alias = "full")]
    Total,
}

impl Cover {
    /// The serde key.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Half => "half",
            Self::ThreeQuarters => "three_quarters",
            Self::Total => "total",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_keys_and_full_alias() {
        for c in [Cover::None, Cover::Half, Cover::ThreeQuarters, Cover::Total] {
            assert_eq!(
                serde_json::to_string(&c).unwrap(),
                format!("\"{}\"", c.key())
            );
        }
        assert_eq!(
            serde_json::from_str::<Cover>("\"full\"").unwrap(),
            Cover::Total
        );
    }
}
