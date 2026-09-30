//! Serde helpers for ids: settlement, realm and building ids and seeds are
//! `u64` serialised as JSON strings (vocabulary.md "Canonical conventions",
//! I5 and I17). Reading also accepts plain JSON numbers.

use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::ser::{SerializeSeq, Serializer};
use serde::Deserialize;
use std::fmt;

struct IdVisitor;

impl Visitor<'_> for IdVisitor {
    type Value = u64;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a u64 id as a string or number")
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

/// A `u64` read from a string or a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Id(u64);

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(IdVisitor).map(Id)
    }
}

/// `u64` ⇄ `"123"`.
pub mod str {
    use super::{Deserializer, Id, Serializer};
    use serde::Deserialize;

    /// Serialises as a string.
    ///
    /// # Errors
    /// Propagates serializer errors.
    pub fn serialize<S: Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&v.to_string())
    }

    /// Reads a string or a number.
    ///
    /// # Errors
    /// Fails on anything that is not a u64.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        Id::deserialize(d).map(|i| i.0)
    }
}

/// `Option<u64>` ⇄ `"123"` or `null`.
pub mod opt {
    use super::{Deserializer, Id, Serializer};
    use serde::Deserialize;

    /// Serialises as a string or null.
    ///
    /// # Errors
    /// Propagates serializer errors.
    #[allow(clippy::ref_option)]
    pub fn serialize<S: Serializer>(v: &Option<u64>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(x) => s.serialize_str(&x.to_string()),
            None => s.serialize_none(),
        }
    }

    /// Reads a string, a number or null.
    ///
    /// # Errors
    /// Fails on anything else.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
        Option::<Id>::deserialize(d).map(|o| o.map(|i| i.0))
    }
}

/// `Vec<u64>` ⇄ `["1", "2"]`.
pub mod vec {
    use super::{Deserializer, Id, SerializeSeq, Serializer};
    use serde::Deserialize;

    /// Serialises as an array of strings.
    ///
    /// # Errors
    /// Propagates serializer errors.
    pub fn serialize<S: Serializer>(v: &[u64], s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(v.len()))?;
        for x in v {
            seq.serialize_element(&x.to_string())?;
        }
        seq.end()
    }

    /// Reads an array of strings or numbers.
    ///
    /// # Errors
    /// Fails on anything else.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u64>, D::Error> {
        Vec::<Id>::deserialize(d).map(|v| v.into_iter().map(|i| i.0).collect())
    }
}

/// `[u64; 2]` ⇄ `["1", "2"]`.
pub mod pair {
    use super::{de, Deserializer, SeqAccess, Serializer, Visitor};
    use std::fmt;

    /// Serialises as two strings.
    ///
    /// # Errors
    /// Propagates serializer errors.
    pub fn serialize<S: Serializer>(v: &[u64; 2], s: S) -> Result<S::Ok, S::Error> {
        super::vec::serialize(v, s)
    }

    struct PairVisitor;

    impl<'de> Visitor<'de> for PairVisitor {
        type Value = [u64; 2];

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("two ids")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<[u64; 2], A::Error> {
            let a: super::Id = seq
                .next_element()?
                .ok_or_else(|| de::Error::invalid_length(0, &self))?;
            let b: super::Id = seq
                .next_element()?
                .ok_or_else(|| de::Error::invalid_length(1, &self))?;
            Ok([a.0, b.0])
        }
    }

    /// Reads two ids.
    ///
    /// # Errors
    /// Fails on anything else.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u64; 2], D::Error> {
        d.deserialize_seq(PairVisitor)
    }
}
