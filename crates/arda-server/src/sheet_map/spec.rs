//! The mapping file format (`--sheet-mapping`, goal 69): what a maintainer
//! writes. Unknown keys are refused everywhere, so a typo fails at startup.

use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// The only `format` a mapping file may declare.
pub const FORMAT: &str = "arda-sheet-mapping";
/// The only `version` this server reads.
pub const VERSION: u32 = 1;

/// A whole mapping file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappingFile {
    /// Must be [`FORMAT`].
    pub format: String,
    /// Must be [`VERSION`].
    pub version: u32,
    /// Short name, sent back in the `X-Arda-Sheet-Mapping` header
    /// (`[A-Za-z0-9._-]`, 1–64 characters).
    pub name: String,
    /// Free text for humans.
    #[serde(default)]
    pub description: String,
    /// What the output starts from: `copy` (the Arda NPC) or `empty` (`{}`).
    pub base: Base,
    /// Named lookup tables for `table` and `keys` conversions.
    #[serde(default)]
    pub tables: BTreeMap<String, BTreeMap<String, Value>>,
    /// Rules, applied in order; a later rule overwrites an earlier one.
    #[serde(default)]
    pub rules: Vec<Rule>,
    /// Output pointers removed after the rules (missing ones are ignored).
    #[serde(default)]
    pub drop: Vec<String>,
}

/// The starting output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Base {
    /// A deep copy of the Arda NPC: rules rename, add and reshape, `drop`
    /// removes.
    Copy,
    /// `{}`: only what the rules write.
    Empty,
}

/// One rule: write one output location.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    /// JSON Pointer (RFC 6901) into the output. Missing objects are created;
    /// a numeric token creates an array (its index must be at most the
    /// length), `-` appends.
    pub to: String,
    /// JSON Pointer into the source NPC (`""` is the whole source).
    #[serde(default)]
    pub from: Option<String>,
    /// A constant value.
    #[serde(default, rename = "const")]
    pub constant: Option<Value>,
    /// A string with `{/pointer}` placeholders into the source.
    #[serde(default)]
    pub template: Option<String>,
    /// Sub-rules applied to the `from` value: once to an object, or to every
    /// element of an array (giving an array). Their pointers are relative.
    #[serde(default)]
    pub fields: Option<Vec<Rule>>,
    /// With `fields` over an array: keep only elements whose pointers equal
    /// these values.
    #[serde(default, rename = "where")]
    pub filter: Option<BTreeMap<String, Value>>,
    /// Append the array to an array already at `to` instead of replacing it.
    #[serde(default)]
    pub append: bool,
    /// Conversions applied in order to the value before it is written.
    #[serde(default)]
    pub convert: Vec<Op>,
    /// A missing or `null` `from` skips the rule (or writes `default`)
    /// instead of failing.
    #[serde(default)]
    pub optional: bool,
    /// With `optional`: the value written when `from` is missing or `null`.
    #[serde(default)]
    pub default: Option<Value>,
}

/// What a `table` or `keys` conversion does with a value the table lacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unknown {
    /// Fail the request (and the startup check).
    #[default]
    Error,
    /// Pass the value through unchanged.
    Keep,
    /// Write `null`.
    Null,
}

/// Letter case and word-separator conversions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Case {
    /// `plate armor`.
    Lower,
    /// `PLATE ARMOR`.
    Upper,
    /// `Plate Armor`: the first letter of every word upper-cased.
    Title,
    /// `plate-armor`: lower case, every run of other characters one `-`.
    Kebab,
    /// `plate_armor`: as kebab, with `_`.
    Snake,
}

/// A value conversion. `null` passes through every conversion unchanged.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Op {
    /// Enum conversion: a string (or number, by its decimal text) looked up
    /// in `tables[table]`.
    Table {
        /// Table name.
        table: String,
        /// Values the table lacks.
        #[serde(default)]
        unknown: Unknown,
    },
    /// Renames the keys of an object through `tables[table]` (the table's
    /// values must be strings).
    Keys {
        /// Table name.
        table: String,
        /// Keys the table lacks.
        #[serde(default)]
        unknown: Unknown,
    },
    /// Unit conversion of a number: `value × factor + offset`, then rounded
    /// to `round` decimals (0 gives an integer) when given.
    Scale {
        /// Multiplier.
        factor: f64,
        /// Added after multiplying.
        #[serde(default)]
        offset: f64,
        /// Decimals to keep.
        #[serde(default)]
        round: Option<u8>,
    },
    /// Formats a scalar into `pattern`, replacing each `{}`.
    Format {
        /// Pattern with `{}` placeholders.
        pattern: String,
    },
    /// Changes the case of a string.
    Case {
        /// Target case.
        to: Case,
    },
    /// Replaces every occurrence of `find` in a string.
    Replace {
        /// Text to find (non-empty).
        find: String,
        /// Replacement.
        with: String,
    },
    /// Joins an array of scalars into one string.
    Join {
        /// Separator.
        sep: String,
    },
    /// Splits a string into an array of strings (empty input gives `[]`).
    Split {
        /// Separator (non-empty).
        sep: String,
    },
    /// A scalar as its JSON text (strings unchanged).
    ToString,
    /// A numeric string as a number (numbers unchanged).
    ToNumber,
    /// Applies `convert` to every element of an array.
    Each {
        /// Conversions per element.
        convert: Vec<Op>,
    },
}
