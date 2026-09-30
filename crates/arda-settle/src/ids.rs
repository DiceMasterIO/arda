//! Serde adapters: every id and seed is a u64 serialised as a JSON string
//! (canonical conventions I5, I17), so JavaScript readers never lose digits.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serializer};

fn parse<'de, D: Deserializer<'de>>(s: &str) -> Result<u64, D::Error> {
    s.parse::<u64>().map_err(D::Error::custom)
}

/// `u64` as a decimal string.
pub mod string {
    use super::{parse, Deserialize, Deserializer, Serializer};

    /// Serialises.
    ///
    /// # Errors
    /// Serializer errors.
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn serialize<S: Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&v.to_string())
    }

    /// Deserialises.
    ///
    /// # Errors
    /// A string that is not a u64.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let s = String::deserialize(d)?;
        parse::<D>(&s)
    }
}

/// `Option<u64>` as a decimal string or null.
pub mod opt_string {
    use super::{parse, Deserialize, Deserializer, Serializer};

    /// Serialises.
    ///
    /// # Errors
    /// Serializer errors.
    #[allow(clippy::ref_option)]
    pub fn serialize<S: Serializer>(v: &Option<u64>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(v) => s.serialize_some(&v.to_string()),
            None => s.serialize_none(),
        }
    }

    /// Deserialises.
    ///
    /// # Errors
    /// A string that is not a u64.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|s| parse::<D>(&s))
            .transpose()
    }
}

/// `Vec<u64>` as decimal strings.
pub mod vec_string {
    use super::{parse, Deserialize, Deserializer, Serializer};
    use serde::ser::SerializeSeq;

    /// Serialises.
    ///
    /// # Errors
    /// Serializer errors.
    pub fn serialize<S: Serializer>(v: &[u64], s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(v.len()))?;
        for x in v {
            seq.serialize_element(&x.to_string())?;
        }
        seq.end()
    }

    /// Deserialises.
    ///
    /// # Errors
    /// A string that is not a u64.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u64>, D::Error> {
        Vec::<String>::deserialize(d)?
            .iter()
            .map(|s| parse::<D>(s))
            .collect()
    }
}
