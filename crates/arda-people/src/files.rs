//! The files of `<world>/society/`: what `arda-settle` writes, and what
//! `arda society build` adds (`society.json`, `notables.json`).

use crate::PeopleError;
use arda_npc::Npc;
use arda_settle::model::Settlement;
use arda_settle::output::{RealmsFile, RoadsFile, SettlementsFile};
use arda_settle::realms::Realm;
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
    /// `realms.json`; `None` when the file is absent.
    pub realms: Option<RealmsFile>,
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
        let rpath = dir.join("realms.json");
        let realms = if rpath.is_file() {
            Some(read::<RealmsFile>(&rpath)?)
        } else {
            None
        };
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
            realms,
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

impl SocietyFiles {
    /// The realm with `id`, when `realms.json` was read and lists it.
    #[must_use]
    pub fn realm(&self, id: u64) -> Option<&Realm> {
        self.realms.as_ref()?.realms.iter().find(|r| r.id == id)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("arda-people-files-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            d.join("settlements.json"),
            r#"{"format_version":1,"seed":"42","width_cells":1,"height_cells":1,"settlements":[]}"#,
        )
        .unwrap();
        std::fs::write(
            d.join("roads.json"),
            r#"{"format_version":1,"roads":[],"crossings":[],"passes":[]}"#,
        )
        .unwrap();
        d
    }

    #[test]
    fn missing_realms_file_is_none() {
        let d = dir("none");
        let f = SocietyFiles::read(&d).unwrap();
        assert!(f.realms.is_none());
        assert!(f.realm(1).is_none());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn realms_file_is_read_and_looked_up() {
        let d = dir("some");
        std::fs::write(
            d.join("realms.json"),
            r#"{"format_version":1,"realms":[{"id":"2","name":"Stonemark","name_gloss":"Stonemark","seat":"7","culture":"c","settlements":["7"],"population":10,"land_cells":5,"neighbours":[],"borders":[]}]}"#,
        )
        .unwrap();
        let f = SocietyFiles::read(&d).unwrap();
        assert_eq!(f.realm(2).map(|r| r.name.as_str()), Some("Stonemark"));
        assert!(f.realm(1).is_none());
        std::fs::write(d.join("realms.json"), "{").unwrap();
        assert!(SocietyFiles::read(&d).is_err());
        std::fs::remove_dir_all(&d).unwrap();
    }
}
