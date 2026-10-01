//! `world.json` — the manifest and completion stamp (`mockup/02`).
//!
//! Written last by the batch: its absence is what makes a partial world
//! unloadable (`04-data-flow.md`).

use super::shore::SHORE_PATH;
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

/// Identity of an optional canonical fine terrain layer.
///
/// Its path is fixed by [`FINE_TERRAIN_PATH`], so a manifest cannot redirect
/// the loader to an arbitrary file. The world seed is already in [`Manifest`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FineTerrainDescriptor {
    /// Version of the source recipe that generated the layer.
    pub recipe_version: u16,
    /// Deterministic continent/source validation attempt.
    pub attempt: u8,
}

/// The only relative path for a persisted canonical fine terrain layer.
pub const FINE_TERRAIN_PATH: &str = "terrain/fine.bin";
/// Highest fine-terrain recipe understood by this build. Recipe 2 is the
/// delivered baseline, recipe 3 is the fine-specific raw macro/spectral
/// source, recipe 4 is that source after canonical valley formation, and
/// recipe 5 is multi-resolution stream-power formation from the macro surface
/// as v0.1 shipped it, and recipe 6 (v0.2) adds tectonic margins, belt
/// relief, maturity, roughness, coast stages and stored water forms
/// (`logic/02` §fine-formation recipes). Every older recipe still loads and
/// renders as it did.
pub const FINE_TERRAIN_RECIPE_VERSION: u16 = 6;

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
    /// Present only for worlds that published a canonical fine terrain layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fine_terrain: Option<FineTerrainDescriptor>,
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
    Ok(effective_recipe(dir, manifest))
}

/// The recipe a world's terrain was actually formed by.
///
/// v0.2.0 formed its fine terrain with what is now recipe 6 but recorded
/// `recipe_version: 5`; only recipe 6 writes [`SHORE_PATH`]. A recipe-5
/// manifest beside a shore layer is therefore read as recipe 6, so
/// rendering and everything else recipe-dependent treat the terrain as it
/// was formed (`logic/02` §fine-formation recipes). The file on disk is
/// left as written.
fn effective_recipe(dir: &Path, mut manifest: Manifest) -> Manifest {
    if let Some(fine) = manifest.fine_terrain.as_mut() {
        if fine.recipe_version == 5 && dir.join(SHORE_PATH).is_file() {
            fine.recipe_version = 6;
        }
    }
    manifest
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
            fine_terrain: None,
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
    fn fine_descriptor_is_optional_without_changing_legacy_json() {
        let legacy = serde_json::to_value(sample()).unwrap();
        assert!(legacy.get("fine_terrain").is_none());
        let mut with_fine = sample();
        with_fine.fine_terrain = Some(FineTerrainDescriptor {
            recipe_version: FINE_TERRAIN_RECIPE_VERSION,
            attempt: 3,
        });
        let decoded: Manifest =
            serde_json::from_value(serde_json::to_value(&with_fine).unwrap()).unwrap();
        assert_eq!(decoded, with_fine);
        let decoded_legacy: Manifest = serde_json::from_value(legacy).unwrap();
        assert_eq!(decoded_legacy.fine_terrain, None);
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
    fn a_v0_2_0_recipe_5_world_with_a_shore_layer_reads_as_recipe_6() {
        // v0.2.0 recorded recipe 5 for recipe-6 terrain; the shore layer
        // (written only by recipe 6) tells the two apart.
        let dir = TempDir::new();
        let mut v020 = sample();
        v020.fine_terrain = Some(FineTerrainDescriptor {
            recipe_version: 5,
            attempt: 0,
        });
        write_manifest(dir.path(), &v020).unwrap();
        let read = |d: &Path| {
            read_manifest(d)
                .unwrap()
                .fine_terrain
                .map(|f| f.recipe_version)
        };
        assert_eq!(read(dir.path()), Some(5), "a true recipe-5 world");
        std::fs::create_dir_all(dir.path().join("terrain")).unwrap();
        std::fs::write(dir.path().join(SHORE_PATH), b"shore").unwrap();
        assert_eq!(read(dir.path()), Some(6), "v0.2.0 recipe-6 terrain");
        // The file on disk keeps what it recorded; other recipes are as
        // recorded whatever sits beside them.
        let raw: Manifest =
            serde_json::from_slice(&std::fs::read(dir.path().join(MANIFEST_NAME)).unwrap())
                .unwrap();
        assert_eq!(raw.fine_terrain.map(|f| f.recipe_version), Some(5));
        for recipe in [4, 6] {
            v020.fine_terrain = Some(FineTerrainDescriptor {
                recipe_version: recipe,
                attempt: 0,
            });
            write_manifest(dir.path(), &v020).unwrap();
            assert_eq!(read(dir.path()), Some(recipe));
        }
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
