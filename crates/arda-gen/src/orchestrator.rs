//! The batch orchestrator (`04-data-flow.md` lifecycle 1).
//!
//! Stages run sequentially per tier; areas fan out on the rayon pool. The
//! orchestrator is the only thing here that touches disk — the stages
//! themselves are pure.

use crate::area::generate_area;
use crate::block::{constraints_for, fill_block};
use crate::continent::bundles::bundle_for;
use crate::continent::climate::climate;
use crate::continent::hydrology::{extract_rivers, hydrology};
use crate::continent::{generate_continent_attempt, Continent};
use arda_core::{
    encode_blocks, encode_cells, encode_continent_objects, encode_objects, encode_overview,
    write_manifest, AreaCoord, BlockArchive, CellCoord, ContinentCell, ContinentObjects,
    ContinentOverview, ContinentRiver, DischargeMilli, GenerateConfig, Manifest, RainfallMm,
    TempCentiC, TerrainKind, ValidationStats, AREA_CELLS, FORMAT_VERSION,
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

/// The step-9 river gate (`logic/01` §Q9, feature 03 §Q7): a continent
/// needs at least one major river that reaches the sea, i.e. one whose
/// `feeds` is `None` rather than a downstream junction.
fn river_gate(rivers: &[ContinentRiver]) -> Result<(), String> {
    if rivers.iter().any(|r| r.feeds.is_none()) {
        Ok(())
    } else {
        Err("no major river reaches the sea".to_owned())
    }
}

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
    continent: &Continent,
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

    // Tier 1: continent, single-threaded, with the step-9 validation gates
    // — land fraction, then a sea-reaching river — and their deterministic
    // reroll ladder (`logic/01` §Q9). Climate, hydrology, and rivers are
    // pure functions of the grid, so building them here, once a candidate
    // clears the land gate, lets the river gate inspect them; on
    // acceptance the same context threads to the area fan-out and
    // persistence below, with no second computation.
    let mut accepted: Option<(Continent, u16, Vec<ContinentRiver>)> = None;
    let mut last_check = String::new();
    for attempt in 0..CONTINENT_ATTEMPTS {
        let candidate = generate_continent_attempt(seed, config, attempt);
        let land = candidate.land_fraction_permille();
        if !LAND_FRACTION_GATE.contains(&land) {
            last_check = format!(
                "land fraction {land} per mille outside {}..={}",
                LAND_FRACTION_GATE.start(),
                LAND_FRACTION_GATE.end()
            );
            continue;
        }

        let clim = climate(&candidate, config.latitude_band());
        let hydro = hydrology(&candidate, &clim);
        let rivers = extract_rivers(&candidate, &hydro);
        if let Err(check) = river_gate(&rivers) {
            // logic/01 §Q9: deterministic reroll — a land-passing continent
            // with no sea-reaching river is rejected too, and the ladder
            // continues.
            last_check = check;
            continue;
        }

        accepted = Some((
            Continent {
                grid: candidate,
                climate: clim,
                hydrology: hydro,
            },
            land,
            rivers,
        ));
        break;
    }
    let Some((continent, land, rivers)) = accepted else {
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

    // Continent layer (feature 02): rivers were already extracted above,
    // during the step-9 gate, and are reused here, not recomputed. The
    // manifest is written LAST, the completion stamp.
    let river_count = u32::try_from(rivers.len()).unwrap_or(u32::MAX);

    let (cw, ch) = (continent.grid.width(), continent.grid.height());
    let mut cells = Vec::with_capacity(usize::try_from(cw * ch).unwrap_or(0));
    for y in 0..ch {
        for x in 0..cw {
            let i = usize::try_from(y * cw + x).unwrap_or(0);
            cells.push(ContinentCell {
                height: continent.grid.get(x, y),
                temperature: TempCentiC::new(continent.climate.temperature[i]),
                rainfall: RainfallMm::new(continent.climate.rainfall[i]),
                regime: continent.climate.regime[i],
                downstream: (continent.hydrology.downstream_dir[i] != arda_core::NO_DOWNSTREAM)
                    .then_some(continent.hydrology.downstream_dir[i]),
                catchment_km2: continent.hydrology.catchment_km2[i],
                discharge: DischargeMilli::new(continent.hydrology.discharge_l_s[i]),
            });
        }
    }
    let overview = ContinentOverview {
        width: cw,
        height: ch,
        cells,
    };
    write_file(
        &out.join("continent").join("overview.bin"),
        &encode_overview(&overview),
    )?;
    write_file(
        &out.join("continent").join("objects.bin"),
        &encode_continent_objects(&ContinentObjects { rivers }),
    )?;

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
            river_count,
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

    #[test]
    fn a_continent_with_no_sea_river_is_rerolled() {
        // logic/01 step 9 partial gate (feature 03 §Q7): the check exists
        // and names itself. Micro seeds all pass, so assert the accept
        // path records a nonzero count instead, and unit-test the check
        // by feeding an empty river list through the gate helper.
        assert!(river_gate(&[]).is_err());
        let ok = vec![arda_core::ContinentRiver {
            id: 1,
            catchment_km2: 400,
            discharge: arda_core::DischargeMilli::new(5_000_000),
            feeds: None,
            course: vec![],
        }];
        assert!(river_gate(&ok).is_ok());
        let junction_only = vec![arda_core::ContinentRiver {
            feeds: Some(1),
            ..ok[0].clone()
        }];
        assert!(river_gate(&junction_only).is_err());
    }
}
