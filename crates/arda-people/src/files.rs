//! The files of `<world>/society/`: what `arda-settle` writes, and what
//! `arda society build` adds (`society.json`, `notables.json`).

use crate::PeopleError;
use arda_npc::Npc;
use arda_settle::model::Settlement;
use arda_settle::output::{RoadsFile, SettlementsFile};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Version of `notables.json`.
pub const NOTABLES_FORMAT: u32 = 1;
/// The society simulation's output, written by `arda society build`.
pub const SOCIETY_FILE: &str = "society.json";
/// The stored notables, written by `arda society build`.
pub const NOTABLES_FILE: &str = "notables.json";

/// One settlement's stored notables (goal 56: only notables are stored;
/// commoners are regenerated).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredNotable {
    /// The settlement.
    pub settlement_id: arda_ids::SettlementId,
    /// Where its buildings came from: `plan` (arda-town) or `mix` (the
    /// settlement's estimated building mix).
    pub buildings: String,
    /// The notables, in id order.
    pub npcs: Vec<Npc>,
}

/// `notables.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotablesFile {
    /// Format version.
    pub format_version: u32,
    /// World seed, a decimal string.
    pub seed: String,
    /// Per settlement, in settlement order.
    pub settlements: Vec<StoredNotable>,
}

/// Everything read from `<world>/society/`.
#[derive(Debug, Clone)]
pub struct SocietyFiles {
    /// The directory.
    pub dir: PathBuf,
    /// `settlements.json`.
    pub settlements: SettlementsFile,
    /// The same records as raw JSON, for the crates that read their own
    /// view of a record (`arda-town` `TownSite`, `arda-npc`
    /// `SettlementProfile`; adapter A2).
    pub records: Vec<serde_json::Value>,
    /// `roads.json`.
    pub roads: RoadsFile,
    /// Record index by settlement id.
    pub index: BTreeMap<u64, usize>,
}

fn read_value(path: &Path) -> Result<serde_json::Value, PeopleError> {
    let bytes = std::fs::read(path).map_err(|e| PeopleError::io(path, e))?;
    serde_json::from_slice(&bytes).map_err(|e| PeopleError::format(path, e))
}

/// Reads a JSON document of `dir`.
///
/// # Errors
/// I/O or format errors.
pub fn read<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, PeopleError> {
    let bytes = std::fs::read(path).map_err(|e| PeopleError::io(path, e))?;
    serde_json::from_slice(&bytes).map_err(|e| PeopleError::format(path, e))
}

/// Writes pretty JSON with a trailing newline, byte-identical for equal
/// values.
///
/// # Errors
/// Serialisation or I/O errors.
pub fn write<T: Serialize>(path: &Path, value: &T) -> Result<(), PeopleError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| PeopleError::format(path, e))?;
    bytes.push(b'\n');
    std::fs::write(path, bytes).map_err(|e| PeopleError::io(path, e))
}

impl SocietyFiles {
    /// Reads the settlement stage's files in `dir` (a world's `society/`).
    ///
    /// # Errors
    /// I/O or format errors.
    pub fn read(dir: &Path) -> Result<Self, PeopleError> {
        let spath = dir.join("settlements.json");
        let settlements: SettlementsFile = read(&spath)?;
        let raw = read_value(&spath)?;
        let records = raw
            .get("settlements")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .ok_or_else(|| PeopleError::format(&spath, "no settlements array"))?;
        let roads: RoadsFile = read(&dir.join("roads.json"))?;
        let index = settlements
            .settlements
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.get(), i))
            .collect();
        Ok(Self {
            dir: dir.to_path_buf(),
            settlements,
            records,
            roads,
            index,
        })
    }

    /// The settlement with `id`.
    ///
    /// # Errors
    /// [`PeopleError::UnknownSettlement`].
    pub fn settlement(&self, id: u64) -> Result<(&Settlement, &serde_json::Value), PeopleError> {
        let i = *self
            .index
            .get(&id)
            .ok_or(PeopleError::UnknownSettlement(id))?;
        match (self.settlements.settlements.get(i), self.records.get(i)) {
            (Some(s), Some(r)) => Ok((s, r)),
            _ => Err(PeopleError::UnknownSettlement(id)),
        }
    }
}
