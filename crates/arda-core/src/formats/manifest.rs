//! `world.json` — the manifest and completion stamp (`mockup/02`).
//!
//! Written last by the batch: its absence is what makes a partial world
//! unloadable (`04-data-flow.md`).

use super::FORMAT_VERSION;
use crate::config::GenerateConfig;
use crate::error::LoadError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Plausibility statistics printed by `generate` and stored for `load`
/// (`logic/01` step 9, `mockup/01` validation footer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationStats {
    /// Land cells per thousand continent cells.
    pub land_fraction_permille: u16,
    /// Area tiles written.
    pub area_count: u32,
    /// Settlements placed across the continent.
    pub settlement_count: u32,
    /// Named rivers reaching the sea.
    pub named_river_count: u32,
    /// Continent river objects (unnamed until step 8).
    #[serde(default)]
    pub river_count: u32,
}

/// Everything needed to identify, verify, or regenerate a world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// World-format major (`logic/05` version gate).
    pub format_version: u32,
    /// The arda build that wrote this world.
    pub arda_version: String,
    /// The sole source of nondeterminism (`mockup/01` `--seed`).
    pub seed: u64,
    /// Config echo — enough to regenerate.
    pub config: GenerateConfig,
    /// Area tiles across.
    pub areas_wide: i32,
    /// Area tiles down.
    pub areas_high: i32,
    /// Validation statistics.
    pub stats: ValidationStats,
}

/// Manifest file name inside a world directory.
pub const MANIFEST_NAME: &str = "world.json";

/// Writes `world.json` into `dir`.
///
/// Serialises a struct with a fixed field order, so repeated writes of equal
/// manifests are byte-identical (`logic/04`).
///
/// # Errors
/// Returns [`LoadError::ManifestUnreadable`] when the directory cannot be
/// written.
pub fn write_manifest(dir: &Path, manifest: &Manifest) -> Result<(), LoadError> {
    let unreadable = |reason: String| LoadError::ManifestUnreadable {
        dir: dir.display().to_string(),
        reason,
    };
    let bytes = serde_json::to_vec_pretty(manifest).map_err(|e| unreadable(e.to_string()))?;
    std::fs::write(dir.join(MANIFEST_NAME), bytes).map_err(|e| unreadable(e.to_string()))
}

/// Reads and version-gates `world.json` from `dir`.
///
/// # Errors
/// - [`LoadError::ManifestMissing`] when the file is absent (partial world).
/// - [`LoadError::ManifestUnreadable`] when it will not parse.
/// - [`LoadError::VersionSkew`] when the format major does not exactly match
///   this build's.
pub fn read_manifest(dir: &Path) -> Result<Manifest, LoadError> {
    let path = dir.join(MANIFEST_NAME);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(LoadError::ManifestMissing {
                dir: dir.display().to_string(),
            })
        }
        Err(e) => {
            return Err(LoadError::ManifestUnreadable {
                dir: dir.display().to_string(),
                reason: e.to_string(),
            })
        }
    };

    let manifest: Manifest =
        serde_json::from_slice(&bytes).map_err(|e| LoadError::ManifestUnreadable {
            dir: dir.display().to_string(),
            reason: e.to_string(),
        })?;

    // Exact major match: older and newer worlds are both refused with
    // the regenerate remedy (feature 02 §Q6, `logic/05`).
    if manifest.format_version != FORMAT_VERSION {
        return Err(LoadError::VersionSkew {
            found: manifest.format_version,
            supported: FORMAT_VERSION,
        });
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GenerateConfig;
    use crate::formats::TempDir;

    fn sample() -> Manifest {
        Manifest {
            format_version: FORMAT_VERSION,
            arda_version: "0.1.0".to_owned(),
            seed: 42,
            config: GenerateConfig::MICRO,
            areas_wide: 2,
            areas_high: 4,
            stats: ValidationStats {
                land_fraction_permille: 612,
                area_count: 8,
                settlement_count: 0,
                named_river_count: 0,
                river_count: 3,
            },
        }
    }

    #[test]
    fn manifest_round_trips_through_disk() {
        let dir = TempDir::new();
        write_manifest(dir.path(), &sample()).unwrap();
        let read = read_manifest(dir.path()).unwrap();
        assert_eq!(read, sample());
    }

    #[test]
    fn manifest_bytes_are_identical_across_writes() {
        // logic/04: export byte-identity. The manifest must not reorder keys.
        let a = TempDir::new();
        let b = TempDir::new();
        write_manifest(a.path(), &sample()).unwrap();
        write_manifest(b.path(), &sample()).unwrap();
        assert_eq!(
            std::fs::read(a.path().join("world.json")).unwrap(),
            std::fs::read(b.path().join("world.json")).unwrap()
        );
    }

    #[test]
    fn a_manifest_without_the_river_count_key_parses_as_zero() {
        // `#[serde(default)]` unhappy path: a same-major world written
        // before this feature lacks `river_count` on disk and must still
        // load (`logic/01` step 9), defaulting the count to 0.
        let dir = TempDir::new();
        let mut value = serde_json::to_value(sample()).unwrap();
        let removed = value["stats"]
            .as_object_mut()
            .unwrap()
            .remove("river_count");
        assert!(
            removed.is_some(),
            "sample() should have had the key to remove"
        );
        assert!(!value["stats"]
            .as_object()
            .unwrap()
            .contains_key("river_count"));
        std::fs::write(
            dir.path().join("world.json"),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();

        let read = read_manifest(dir.path()).unwrap();
        assert_eq!(read.stats.river_count, 0);
    }

    #[test]
    fn missing_manifest_names_the_directory() {
        // mockup/02 States: loaders refuse a partial world.
        let dir = TempDir::new();
        let err = read_manifest(dir.path()).unwrap_err();
        assert!(matches!(err, LoadError::ManifestMissing { .. }));
        assert!(err.to_string().contains("did not finish"));
    }

    #[test]
    fn newer_format_major_is_refused_with_the_regenerate_remedy() {
        // logic/05: version skew carries both versions.
        let dir = TempDir::new();
        let mut future = sample();
        future.format_version = FORMAT_VERSION + 1;
        write_manifest(dir.path(), &future).unwrap();

        let err = read_manifest(dir.path()).unwrap_err();
        match err {
            LoadError::VersionSkew { found, supported } => {
                assert_eq!(found, FORMAT_VERSION + 1);
                assert_eq!(supported, FORMAT_VERSION);
            }
            other => panic!("expected VersionSkew, got {other:?}"),
        }
        assert!(read_manifest(dir.path())
            .unwrap_err()
            .to_string()
            .contains("regenerate"));
    }

    #[test]
    fn older_format_major_is_refused_with_the_regenerate_remedy() {
        // Feature 02 §Q6 / spec R6: a format-(N-1) world must not load.
        let dir = TempDir::new();
        let mut old = sample();
        old.format_version = FORMAT_VERSION - 1;
        write_manifest(dir.path(), &old).unwrap();
        match read_manifest(dir.path()).unwrap_err() {
            LoadError::VersionSkew { found, supported } => {
                assert_eq!(found, FORMAT_VERSION - 1);
                assert_eq!(supported, FORMAT_VERSION);
            }
            other => panic!("expected VersionSkew, got {other:?}"),
        }
    }

    #[test]
    fn unparseable_manifest_is_refused() {
        let dir = TempDir::new();
        std::fs::write(dir.path().join("world.json"), b"{ not json").unwrap();
        let err = read_manifest(dir.path()).unwrap_err();
        assert!(matches!(err, LoadError::ManifestUnreadable { .. }));
    }
}
