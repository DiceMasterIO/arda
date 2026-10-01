//! Road classes (`vocabulary.md` I3/I4; `logic/08` §roads).

use serde::{Deserialize, Serialize};

/// The canonical road class. Codes match the world's stored `RoadClass`
/// (`arda-core`), with `footpath` appended as code 4.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum RoadClass {
    /// No road.
    #[default]
    None = 0,
    /// Cart track.
    Track = 1,
    /// Maintained road.
    Road = 2,
    /// Trunk highway.
    Highway = 3,
    /// Footpath.
    Footpath = 4,
}

impl RoadClass {
    /// Every class, in code order.
    pub const ALL: [Self; 5] = [
        Self::None,
        Self::Track,
        Self::Road,
        Self::Highway,
        Self::Footpath,
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
            1 => Some(Self::Track),
            2 => Some(Self::Road),
            3 => Some(Self::Highway),
            4 => Some(Self::Footpath),
            _ => None,
        }
    }

    /// The serde key.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Track => "track",
            Self::Road => "road",
            Self::Highway => "highway",
            Self::Footpath => "footpath",
        }
    }

    /// Importance, lowest first: none 0, footpath 1, track 2, road 3,
    /// highway 4. Compare classes by this, never by the code.
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Footpath => 1,
            Self::Track => 2,
            Self::Road => 3,
            Self::Highway => 4,
        }
    }

    /// The real road classes, most important first.
    pub const ROADS: [Self; 4] = [Self::Highway, Self::Road, Self::Track, Self::Footpath];

    /// The more important of a stored code and this class, as a code.
    #[must_use]
    pub const fn max_code(self, code: u8) -> u8 {
        match Self::from_code(code) {
            Some(old) if old.rank() >= self.rank() => code,
            _ => self.code(),
        }
    }

    /// Carriageway width in 5-ft squares (`vocabulary.md` I4).
    #[must_use]
    pub const fn width_squares(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Footpath => 1,
            Self::Track => 2,
            Self::Road => 4,
            Self::Highway => 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_keys_and_widths_are_canonical() {
        for class in RoadClass::ALL {
            assert_eq!(RoadClass::from_code(class.code()), Some(class));
            let json = serde_json::to_string(&class).unwrap();
            assert_eq!(json, format!("\"{}\"", class.key()));
        }
        assert_eq!(RoadClass::from_code(5), None);
        let codes: Vec<u8> = RoadClass::ALL.iter().map(|c| c.code()).collect();
        assert_eq!(codes, [0, 1, 2, 3, 4]);
        let widths: Vec<u8> = RoadClass::ALL.iter().map(|c| c.width_squares()).collect();
        assert_eq!(widths, [0, 2, 4, 5, 1]);
        let ranks: Vec<u8> = RoadClass::ROADS.iter().map(|c| c.rank()).collect();
        assert_eq!(ranks, [4, 3, 2, 1]);
        assert_eq!(RoadClass::Footpath.max_code(RoadClass::Highway.code()), 3);
        assert_eq!(RoadClass::Road.max_code(RoadClass::Footpath.code()), 2);
        assert_eq!(RoadClass::Track.max_code(0), 1);
    }
}
