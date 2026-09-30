//! `u64` keys, ids and seeds as JSON strings (vocabulary I5, I17): numbers
//! above 2^53 are rounded by JavaScript clients. Reading also accepts plain
//! JSON numbers.

use serde::de::{self, Deserializer, Visitor};
use serde::Serializer;
use std::fmt;

struct IdVisitor;

impl Visitor<'_> for IdVisitor {
    type Value = u64;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a u64 as a decimal string or a number")
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

/// Serialises as a decimal string.
///
/// # Errors
/// Propagates serializer errors.
#[allow(clippy::trivially_copy_pass_by_ref)] // serde's `with` signature
pub fn serialize<S: Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&v.to_string())
}

/// Reads a decimal string or a number.
///
/// # Errors
/// Anything that is not a u64.
pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    d.deserialize_any(IdVisitor)
}
