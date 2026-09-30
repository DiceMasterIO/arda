//! 64-bit integers as JSON strings (JS numbers lose precision past 2^53).
//!
//! Per the canonical conventions (I5, I17), seeds and ids serialise as
//! strings. Input also accepts a plain number, for hand-written JSON.

use serde::de::{self, Visitor};
use serde::{Deserializer, Serializer};

/// Serialises a `u64` as a decimal string.
///
/// # Errors
/// Serializer failure.
pub fn serialize<S: Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&v.to_string())
}

/// Deserialises a `u64` from a decimal string or a number.
///
/// # Errors
/// Anything but a non-negative integer that fits in 64 bits.
pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    struct V;
    impl Visitor<'_> for V {
        type Value = u64;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a u64 as a decimal string")
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<u64, E> {
            v.parse().map_err(E::custom)
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<u64, E> {
            Ok(v)
        }
    }
    d.deserialize_any(V)
}

#[cfg(test)]
mod tests {
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct T(#[serde(with = "super")] u64);

    #[test]
    fn u64_round_trips_as_a_string() {
        let json = serde_json::to_string(&T(u64::MAX)).unwrap();
        assert_eq!(json, "\"18446744073709551615\"");
        assert_eq!(serde_json::from_str::<T>(&json).unwrap(), T(u64::MAX));
        assert_eq!(serde_json::from_str::<T>("7").unwrap(), T(7));
        assert!(serde_json::from_str::<T>("\"-1\"").is_err());
    }
}
