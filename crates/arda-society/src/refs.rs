//! Typed references between society entities. Every hook, event and role
//! points at the world through these, and a test resolves every one.

use serde::{Deserialize, Serialize};

/// A reference to one entity of the input world or of the society.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EntityRef {
    /// An input settlement.
    Settlement {
        /// Settlement id.
        #[serde(with = "crate::ids::str")]
        #[cfg_attr(feature = "schema", schemars(with = "String"))]
        id: u64,
    },
    /// An input realm.
    Realm {
        /// Realm id.
        #[serde(with = "crate::ids::str")]
        #[cfg_attr(feature = "schema", schemars(with = "String"))]
        id: u64,
    },
    /// An input road.
    Road {
        /// Road id.
        #[serde(with = "crate::ids::str")]
        #[cfg_attr(feature = "schema", schemars(with = "String"))]
        id: u64,
    },
    /// A building of a settlement.
    Building {
        /// Settlement id.
        #[serde(with = "crate::ids::str")]
        #[cfg_attr(feature = "schema", schemars(with = "String"))]
        settlement: u64,
        /// Building id within the settlement.
        #[serde(with = "crate::ids::str")]
        #[cfg_attr(feature = "schema", schemars(with = "String"))]
        id: u64,
    },
    /// A faction.
    Faction {
        /// Faction id.
        id: String,
    },
    /// A notable NPC slot.
    Role {
        /// Role id.
        id: String,
    },
    /// A ruin of an abandoned settlement.
    Ruin {
        /// Ruin id.
        id: u32,
    },
    /// A timeline event.
    Event {
        /// Event id.
        id: u32,
    },
    /// A ruling house.
    Dynasty {
        /// Dynasty id.
        id: String,
    },
    /// A trade good.
    Good {
        /// Good key.
        key: String,
    },
}

impl EntityRef {
    /// Shorthand for a settlement reference.
    #[must_use]
    pub const fn settlement(id: u64) -> Self {
        Self::Settlement { id }
    }

    /// Shorthand for a realm reference.
    #[must_use]
    pub const fn realm(id: u64) -> Self {
        Self::Realm { id }
    }

    /// Shorthand for a road reference.
    #[must_use]
    pub const fn road(id: u64) -> Self {
        Self::Road { id }
    }

    /// Shorthand for a role reference.
    #[must_use]
    pub fn role(id: &str) -> Self {
        Self::Role { id: id.to_string() }
    }

    /// Shorthand for a faction reference.
    #[must_use]
    pub fn faction(id: &str) -> Self {
        Self::Faction { id: id.to_string() }
    }

    /// Shorthand for a good reference.
    #[must_use]
    pub fn good(key: &str) -> Self {
        Self::Good {
            key: key.to_string(),
        }
    }
}
