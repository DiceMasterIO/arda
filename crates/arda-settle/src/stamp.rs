//! The generation stamp: which world and which code a `society/` directory
//! was settled from (review 3.3-4, §4 item 4).
//!
//! `arda settle` writes the stamp into `settlements.json`. Its `id` is a
//! digest of the stamp's own fields and of every byte the stage wrote, so
//! two settle runs that differ in any output differ in `id`. `arda society
//! build` records the `id` it read (`society.json` `settle_stamp`,
//! `notables.json` `settle_stamp`), and readers refuse files whose stamp
//! does not match the world or the settlement files beside them.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// `world_json` of a stamp written without a world (tests and fixtures that
/// call [`crate::write`] on a bare grid): the world check is skipped.
pub const DETACHED: &str = "detached";

/// The stamp in `settlements.json`.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    /// Digest of the fields below and of every file the stage wrote (with
    /// this stamp left out of `settlements.json`), 32 hex digits.
    pub id: String,
    /// World seed, a decimal string.
    pub seed: String,
    /// Version of the code that settled the world (`arda-settle`).
    pub arda_version: String,
    /// Version of the code that generated the world (`world.json`).
    pub world_arda_version: String,
    /// Fine-terrain recipe of the world, when it has fine terrain.
    pub recipe_version: Option<u32>,
    /// Digest of the world's `world.json`, 32 hex digits, or [`DETACHED`].
    pub world_json: String,
}

/// Where a settlement run's input came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// World seed.
    pub seed: u64,
    /// `arda_version` of `world.json`.
    pub world_arda_version: String,
    /// Fine-terrain recipe.
    pub recipe_version: Option<u32>,
    /// Digest of `world.json`, or [`DETACHED`].
    pub world_json: String,
}

impl Provenance {
    /// A run over a bare grid with no world on disk.
    #[must_use]
    pub fn detached(seed: u64) -> Self {
        Self {
            seed,
            world_arda_version: String::new(),
            recipe_version: None,
            world_json: DETACHED.to_string(),
        }
    }

    /// The provenance of the world stored at `world_dir`.
    ///
    /// # Errors
    /// I/O errors reading `world.json`.
    pub fn of_world(world: &arda::World, world_dir: &Path) -> std::io::Result<Self> {
        Ok(Self {
            seed: world.seed(),
            world_arda_version: world.manifest().arda_version.clone(),
            recipe_version: world
                .manifest()
                .fine_terrain
                .as_ref()
                .map(|f| u32::from(f.recipe_version)),
            world_json: world_digest(world_dir)?,
        })
    }

    /// The stamp of a run whose output bytes (stamp left out) are `outputs`.
    #[must_use]
    pub fn stamp(&self, outputs: &[&[u8]]) -> Stamp {
        let mut s = Stamp {
            id: String::new(),
            seed: self.seed.to_string(),
            arda_version: env!("CARGO_PKG_VERSION").to_string(),
            world_arda_version: self.world_arda_version.clone(),
            recipe_version: self.recipe_version,
            world_json: self.world_json.clone(),
        };
        let fields = serde_json::to_vec(&s).unwrap_or_default();
        let mut parts: Vec<&[u8]> = vec![&fields];
        parts.extend_from_slice(outputs);
        s.id = hex(&arda_ids::hash::digest(&parts));
        s
    }
}

/// Digest of `<world_dir>/world.json`, 32 hex digits.
///
/// # Errors
/// I/O errors.
pub fn world_digest(world_dir: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(world_dir.join("world.json"))?;
    Ok(hex(&arda_ids::hash::digest(&[&bytes])))
}

/// The first 16 bytes of a digest as 32 lowercase hex digits.
#[must_use]
pub fn hex(digest: &[u8; 32]) -> String {
    use std::fmt::Write;
    digest[..16]
        .iter()
        .fold(String::with_capacity(32), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}
