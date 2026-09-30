//! JSON-safe 64-bit integers: written as strings, read from strings or
//! numbers (vocabulary.md "Canonical conventions", I5 and I17).

use serde::de::{self, Visitor};
use serde::{Deserializer, Serializer};
use std::fmt;

/// Serialises a `u64` as a decimal string.
///
/// # Errors
/// Serializer failure.
pub fn serialize<S: Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&v.to_string())
}

struct U64Visitor;

impl Visitor<'_> for U64Visitor {
    type Value = u64;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a u64 as a string or number")
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<u64, E> {
        Ok(v)
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<u64, E> {
        u64::try_from(v).map_err(E::custom)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<u64, E> {
        v.parse().map_err(E::custom)
    }
}

/// Reads a `u64` from a string or a number.
///
/// # Errors
/// Anything that is not a non-negative integer.
pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    d.deserialize_any(U64Visitor)
}
