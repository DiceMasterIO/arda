//! Reads the settlement stage's real `<world>/society/` files into a
//! [`WorldSettlements`] (the settle → society adapter).
//!
//! `arda-settle` writes three documents: `settlements.json`
//! (`{format_version, seed, width_cells, height_cells, settlements}`),
//! `roads.json` (`{format_version, roads, crossings, passes}`) and
//! `realms.json` (`{format_version, realms}`). Their records deserialise
//! into this crate's input types unchanged (extra fields are ignored), so
//! the adapter only unwraps the envelopes and checks the format versions.

use crate::error::SocietyError;
use crate::input::{Realm, Road, Settlement, WorldSettlements};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::path::Path;

/// The settlement stage's file format this reader understands.
pub const SETTLE_FORMAT: u32 = 1;
/// Largest society file read, bytes (a full world's roads are ~100 MB).
pub const MAX_FILE_BYTES: u64 = 1 << 30;

#[derive(Deserialize)]
struct SettlementsFile {
    format_version: u32,
    settlements: Vec<Settlement>,
}

#[derive(Deserialize)]
struct RoadsFile {
    format_version: u32,
    roads: Vec<Road>,
}

#[derive(Deserialize)]
struct RealmsFile {
    format_version: u32,
    realms: Vec<Realm>,
}

fn read<T: DeserializeOwned>(path: &Path) -> Result<T, SocietyError> {
    let io = |source| SocietyError::Io {
        path: path.display().to_string(),
        source,
    };
    let len = std::fs::metadata(path).map_err(io)?.len();
    if len > MAX_FILE_BYTES {
        return Err(SocietyError::Input(format!(
            "{} is {len} bytes; the limit is {MAX_FILE_BYTES}",
            path.display()
        )));
    }
    let text = std::fs::read_to_string(path).map_err(io)?;
    serde_json::from_str(&text).map_err(|e| SocietyError::Input(format!("{}: {e}", path.display())))
}

fn check(path: &Path, version: u32) -> Result<(), SocietyError> {
    if version == SETTLE_FORMAT {
        Ok(())
    } else {
        Err(SocietyError::Input(format!(
            "{}: format_version {version}, expected {SETTLE_FORMAT}",
            path.display()
        )))
    }
}

impl WorldSettlements {
    /// Reads `dir` (a world's `society/` directory as `arda-settle` writes
    /// it). Settlements with no explicit buildings get theirs derived from
    /// the building mix.
    ///
    /// # Errors
    /// [`SocietyError::Io`] for unreadable files, [`SocietyError::Input`]
    /// for malformed JSON, oversized files or an unknown format version.
    pub fn read_dir(dir: &Path) -> Result<Self, SocietyError> {
        let (sp, rp, mp) = (
            dir.join("settlements.json"),
            dir.join("roads.json"),
            dir.join("realms.json"),
        );
        let s: SettlementsFile = read(&sp)?;
        check(&sp, s.format_version)?;
        let r: RoadsFile = read(&rp)?;
        check(&rp, r.format_version)?;
        let m: RealmsFile = read(&mp)?;
        check(&mp, m.format_version)?;
        Ok(Self {
            format_version: s.format_version,
            settlements: s.settlements,
            roads: r.roads,
            realms: m.realms,
            buildings: Vec::new(),
        })
    }
}
