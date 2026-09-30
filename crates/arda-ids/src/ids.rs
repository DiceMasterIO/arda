//! u64 id newtypes (`vocabulary.md` "Ids (I5)"; `logic/16` §api-conventions).
//!
//! Every id is a plain `u64` inside, and travels on the wire as a decimal
//! JSON **string** (`"12"`), because JavaScript numbers lose precision above
//! 2^53. Deserialisation also accepts a non-negative JSON integer, so files
//! written before the switch still load; serialisation always writes the
//! string form.

use std::fmt;
use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

macro_rules! u64_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        pub struct $name(pub u64);

        impl $name {
            /// The raw value.
            #[must_use]
            pub const fn get(self) -> u64 {
                self.0
            }
        }

        impl From<u64> for $name {
            fn from(v: u64) -> Self {
                Self(v)
            }
        }

        impl From<$name> for u64 {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = std::num::ParseIntError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                s.parse().map(Self)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.collect_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                d.deserialize_any(U64Visitor).map(Self)
            }
        }
    };
}

u64_id!(
    /// A settlement, unique within a world.
    SettlementId
);
u64_id!(
    /// A realm, unique within a world.
    RealmId
);
u64_id!(
    /// A building, unique within its settlement.
    BuildingId
);
u64_id!(
    /// A person, unique within a world. See [`NpcId::from_parts`].
    NpcId
);

impl NpcId {
    /// The id of the `index`-th resident of `building` in `settlement`:
    /// [`crate::subseed`] with the settlement id as seed, domain `"npc-id"`
    /// and arguments `[building, index]`. Settlement ids are unique within a
    /// world, so the id is too (up to a 2^-64 collision, which generators
    /// must refuse rather than resolve).
    #[must_use]
    pub fn from_parts(settlement: SettlementId, building: BuildingId, index: u32) -> Self {
        Self(crate::subseed(
            settlement.0,
            "npc-id",
            &[building.0, u64::from(index)],
        ))
    }
}

struct U64Visitor;

impl Visitor<'_> for U64Visitor {
    type Value = u64;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a u64 id as a decimal string or a non-negative integer")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<u64, E> {
        v.parse()
            .map_err(|_| E::invalid_value(de::Unexpected::Str(v), &self))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<u64, E> {
        Ok(v)
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<u64, E> {
        u64::try_from(v).map_err(|_| E::invalid_value(de::Unexpected::Signed(v), &self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_serialise_as_strings_and_read_both_forms() {
        let id = SettlementId(u64::MAX);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"18446744073709551615\"");
        assert_eq!(serde_json::from_str::<SettlementId>(&json).unwrap(), id);
        assert_eq!(
            serde_json::from_str::<BuildingId>("12").unwrap(),
            BuildingId(12)
        );
        assert!(serde_json::from_str::<RealmId>("-1").is_err());
        assert!(serde_json::from_str::<RealmId>("\"x1\"").is_err());
        assert!(serde_json::from_str::<RealmId>("1.5").is_err());
    }

    #[test]
    fn display_and_parse_round_trip() {
        let id: NpcId = "9007199254740993".parse().unwrap();
        assert_eq!(id.to_string(), "9007199254740993");
        assert_eq!(u64::from(id), 9_007_199_254_740_993);
    }

    #[test]
    fn npc_ids_from_parts_are_stable_and_distinct() {
        let a = NpcId::from_parts(SettlementId(2), BuildingId(3), 4);
        assert_eq!(a, NpcId::from_parts(SettlementId(2), BuildingId(3), 4));
        assert_ne!(a, NpcId::from_parts(SettlementId(2), BuildingId(3), 5));
        assert_ne!(a, NpcId::from_parts(SettlementId(3), BuildingId(2), 4));
    }
}
