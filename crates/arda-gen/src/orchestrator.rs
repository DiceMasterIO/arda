//! The batch orchestrator (`04-data-flow.md` lifecycle 1).
//!
//! Stages run sequentially per tier; areas fan out on the rayon pool. The
//! orchestrator is the only thing here that touches disk — the stages
//! themselves are pure.

use crate::area::generate_area;
use crate::block::{constraints_for, fill_block};
use crate::continent::bundles::bundle_for;
use crate::continent::generate_continent_attempt;
use arda_core::{
    encode_blocks, encode_cells, encode_objects, write_manifest, AreaCoord, BlockArchive,
    CellCoord, GenerateConfig, Manifest, TerrainKind, ValidationStats, AREA_CELLS, FORMAT_VERSION,
};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// A batch failure (`mockup/01` States).
#[derive(Debug, Error)]
pub enum GenError {
    /// The output directory already holds something.
    #[error("output directory {dir} is not empty; refusing to overwrite a world")]
    OutputNotEmpty {
        /// The directory that was targeted.
        dir: String,
    },
    /// A layer could not be written.
    #[error("failed writing {path}: {source}")]
    Write {
        /// The file being written.
        path: String,
        /// Underlying cause.
        #[source]
        source: std::io::Error,
    },
    /// A continent validation gate failed (`logic/01` step 9).
    #[error("continent validation failed: {check}")]
    Validation {
        /// The gate that rejected the continent.
        check: String,
    },
    /// A layer could not be encoded.
    #[error("failed encoding {path}: {source}")]
    Encode {
        /// The layer being encoded.
        path: String,
        /// Underlying cause.
        #[source]
        source: arda_core::FormatError,
    },
    /// The manifest could not be stamped.
    #[error("failed stamping the manifest: {0}")]
    Manifest(#[from] arda_core::LoadError),
}

/// Validation rerolls before the batch gives up (`logic/01` §Q9).
const CONTINENT_ATTEMPTS: u8 = 5;

/// Acceptable land fraction, per mille (`logic/01` step 9).
const LAND_FRACTION_GATE: std::ops::RangeInclusive<u16> = 250..=900;

/// Block sampling stride for the skeleton.
///
/// `ponytail:` sampled blocks; the full batch materialises one per land cell
/// (`mockup/02`). Build-order step 6 removes the stride.
const SKELETON_BLOCK_STRIDE: u16 = 64;

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), GenError> {
    let fail = |e: std::io::Error| GenError::Write {
        path: path.display().to_string(),
        source: e,
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(fail)?;
    }
    std::fs::write(path, bytes).map_err(fail)
}

/// Generates and writes one area tile, blocks included.
fn write_area(
    seed: u64,
    continent: &crate::continent::ContinentGrid,
    area: AreaCoord,
    out: &Path,
) -> Result<(), GenError> {
    let bundle = bundle_for(seed, continent, area);
    let (cells, objects) = generate_area(seed, continent, &bundle);

    let dir: PathBuf = out.join("areas").join(area.dir_name());
    write_file(&dir.join("cells.bin"), &encode_cells(&cells))?;
    write_file(&dir.join("objects.bin"), &encode_objects(&objects))?;

    let mut archive = BlockArchive::default();
    let mut y = 0u16;
    while y < AREA_CELLS {
        let mut x = 0u16;
        while x < AREA_CELLS {
            if let Some(at) = CellCoord::new(x, y) {
                if cells.get(at).terrain == TerrainKind::Land {
                    let c = constraints_for(&cells, at);
                    archive.insert(at, fill_block(seed, area, at, &c));
                }
            }
            x += SKELETON_BLOCK_STRIDE;
        }
        y += SKELETON_BLOCK_STRIDE;
    }

    let name = format!("{}.tiles.zst", area.dir_name());
    let blocks = encode_blocks(&archive).map_err(|e| GenError::Encode {
        path: format!("blocks/{name}"),
        source: e,
    })?;
    write_file(&out.join("blocks").join(name), &blocks)
}

/// Runs the whole batch and stamps the manifest.
///
/// # Errors
/// See [`GenError`].
pub fn generate_world(seed: u64, config: GenerateConfig, out: &Path) -> Result<Manifest, GenError> {
    let occupied = out
        .read_dir()
        .map(|mut d| d.next().is_some())
        .unwrap_or(false);
    if occupied {
        return Err(GenError::OutputNotEmpty {
            dir: out.display().to_string(),
        });
    }

    // Tier 1: continent, single-threaded, with the step-9 validation gate and
    // its deterministic reroll ladder (`logic/01` §Q9).
    let mut accepted: Option<(crate::continent::ContinentGrid, u16)> = None;
    let mut last_check = String::new();
    for attempt in 0..CONTINENT_ATTEMPTS {
        let candidate = generate_continent_attempt(seed, config, attempt);
        let land = candidate.land_fraction_permille();
        if LAND_FRACTION_GATE.contains(&land) {
            accepted = Some((candidate, land));
            break;
        }
        last_check = format!(
            "land fraction {land} per mille outside {}..={}",
            LAND_FRACTION_GATE.start(),
            LAND_FRACTION_GATE.end()
        );
    }
    let Some((continent, land)) = accepted else {
        return Err(GenError::Validation {
            check: format!("{last_check} after {CONTINENT_ATTEMPTS} rerolls"),
        });
    };

    // Tiers 2 and 3: areas fan out, each writing its own keyed outputs. The
    // work is order-free because every tile depends only on its bundle.
    let coords: Vec<AreaCoord> = config.area_coords().collect();
    coords
        .par_iter()
        .map(|&area| write_area(seed, &continent, area, out))
        .collect::<Result<Vec<()>, GenError>>()?;

    // Continent layer, then the manifest LAST — the completion stamp.
    write_file(&out.join("continent").join("overview.bin"), &[])?;

    let manifest = Manifest {
        format_version: FORMAT_VERSION,
        arda_version: env!("CARGO_PKG_VERSION").to_owned(),
        seed,
        config,
        areas_wide: config.areas_wide(),
        areas_high: config.areas_high(),
        stats: ValidationStats {
            land_fraction_permille: land,
            area_count: u32::try_from(coords.len()).unwrap_or(u32::MAX),
            settlement_count: 0,
            named_river_count: 0,
        },
    };
    write_manifest(out, &manifest)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continent::generate_continent_attempt;

    #[test]
    fn a_rejected_continent_rerolls_to_an_accepted_one() {
        // Seed 43 fails the step-9 gate at attempt 0 and must be rerolled
        // (`logic/01` §Q9), not rejected outright.
        let first = generate_continent_attempt(43, GenerateConfig::MICRO, 0);
        assert!(
            !LAND_FRACTION_GATE.contains(&first.land_fraction_permille()),
            "seed 43 attempt 0 was expected to fail the gate"
        );

        let accepted = (0..CONTINENT_ATTEMPTS)
            .map(|a| generate_continent_attempt(43, GenerateConfig::MICRO, a))
            .find(|c| LAND_FRACTION_GATE.contains(&c.land_fraction_permille()));
        assert!(
            accepted.is_some(),
            "no attempt within the ladder was accepted"
        );
    }

    #[test]
    fn the_reroll_sequence_is_deterministic() {
        // Same seed must produce the same reroll sequence (`logic/01` §Q9,
        // mockup Q9), so a world stays reproducible from its seed alone.
        for attempt in 0..CONTINENT_ATTEMPTS {
            assert_eq!(
                generate_continent_attempt(43, GenerateConfig::MICRO, attempt),
                generate_continent_attempt(43, GenerateConfig::MICRO, attempt)
            );
        }
    }

    #[test]
    fn attempts_differ_from_one_another() {
        let a = generate_continent_attempt(43, GenerateConfig::MICRO, 0);
        let b = generate_continent_attempt(43, GenerateConfig::MICRO, 1);
        assert_ne!(a, b, "a reroll must actually change the continent");
    }
}
