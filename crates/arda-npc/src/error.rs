//! Errors of the NPC generator.

use thiserror::Error;

use crate::input::{BuildingId, SettlementId};
use crate::npc::NpcId;

/// Everything that can refuse a population run.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NpcError {
    /// A bundled data file failed to parse or refers to a missing entry.
    #[error("bundled data is invalid: {0}")]
    Data(String),
    /// A building belongs to another settlement.
    #[error("building {building:?} belongs to settlement {found:?}, not {expected:?}")]
    ForeignBuilding {
        /// Offending building.
        building: BuildingId,
        /// Settlement being generated.
        expected: SettlementId,
        /// Settlement named by the building.
        found: SettlementId,
    },
    /// Two buildings share an id.
    #[error("building id {0:?} appears twice")]
    DuplicateBuilding(BuildingId),
    /// The buildings cannot house the requested population. Limits refuse the
    /// run and never silently trim it.
    #[error("population {population} exceeds housing capacity {capacity}")]
    NotEnoughHousing {
        /// Requested population.
        population: u32,
        /// Total resident capacity of the buildings.
        capacity: u32,
    },
    /// The id does not name an inhabitant of this settlement.
    #[error("no inhabitant {0:?} in this settlement")]
    UnknownNpc(NpcId),
    /// A request names a key the bundled tables do not have
    /// (`generate_one`: a job, office, ancestry or culture).
    #[error("unknown {what} {key:?}; valid: {valid}")]
    UnknownKey {
        /// What kind of key: `job`, `office`, `ancestry` or `culture`.
        what: &'static str,
        /// The key as given.
        key: String,
        /// The valid keys, comma-separated.
        valid: String,
    },
    /// A request asks for something its own inputs rule out, such as a
    /// rank the job does not have in the tier (`generate_one`).
    #[error("{0}")]
    Unreachable(String),
    /// Every bounded attempt produced nobody who matches every filter
    /// (`generate_one`).
    #[error("{0}")]
    Unsatisfiable(String),
}
