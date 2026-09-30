//! Land-use codes (`logic/08` §landuse).

use serde::{Deserialize, Serialize};

/// The canonical land-use class stored in `society/landuse.bin` as a u8.
/// Codes 0–7 are logic/08 §landuse; `arda-settle` also stores meadow (8),
/// fallow (9) and farmstead (10), which the fields layer draws directly
/// (adapter A17). Wild ground is `None`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum LandUse {
    /// Untouched; prior cover stands.
    #[default]
    None = 0,
    /// Built-up footprint.
    Built = 1,
    /// Ploughed fields (`arable` is accepted on input).
    #[serde(alias = "arable")]
    Field = 2,
    /// Pasture.
    Pasture = 3,
    /// Orchard (counts as farmland).
    Orchard = 4,
    /// Managed woodland kept on slopes.
    Woodland = 5,
    /// A water-mill site.
    Mill = 6,
    /// A mine or quarry head (`mine_quarry` is accepted on input).
    #[serde(alias = "mine_quarry")]
    Mine = 7,
    /// Hay meadow.
    Meadow = 8,
    /// Resting ploughland.
    Fallow = 9,
    /// A farmstead: farmhouse, barn and yard.
    Farmstead = 10,
}

impl LandUse {
    /// Every class, in code order.
    pub const ALL: [Self; 11] = [
        Self::None,
        Self::Built,
        Self::Field,
        Self::Pasture,
        Self::Orchard,
        Self::Woodland,
        Self::Mill,
        Self::Mine,
        Self::Meadow,
        Self::Fallow,
        Self::Farmstead,
    ];

    /// The stored code.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Decodes a stored code.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::None),
            1 => Some(Self::Built),
            2 => Some(Self::Field),
            3 => Some(Self::Pasture),
            4 => Some(Self::Orchard),
            5 => Some(Self::Woodland),
            6 => Some(Self::Mill),
            7 => Some(Self::Mine),
            8 => Some(Self::Meadow),
            9 => Some(Self::Fallow),
            10 => Some(Self::Farmstead),
            _ => None,
        }
    }

    /// The serde key.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Built => "built",
            Self::Field => "field",
            Self::Pasture => "pasture",
            Self::Orchard => "orchard",
            Self::Woodland => "woodland",
            Self::Mill => "mill",
            Self::Mine => "mine",
            Self::Meadow => "meadow",
            Self::Fallow => "fallow",
            Self::Farmstead => "farmstead",
        }
    }

    /// Whether the class is ploughed farmland (fields, fallow, orchards).
    #[must_use]
    pub const fn is_farmland(self) -> bool {
        matches!(self, Self::Field | Self::Orchard | Self::Fallow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip() {
        for (i, class) in LandUse::ALL.iter().enumerate() {
            assert_eq!(usize::from(class.code()), i);
            assert_eq!(LandUse::from_code(class.code()), Some(*class));
        }
        assert_eq!(LandUse::from_code(11), None);
        assert_eq!(
            serde_json::to_string(&LandUse::Pasture).unwrap(),
            "\"pasture\""
        );
    }
}
