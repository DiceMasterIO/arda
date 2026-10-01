//! The batch orchestrator (`04-data-flow.md` lifecycle 1).
//!
//! One modeled rectangle evolves before sequential preparation slices feed the
//! shared annual solve and immutable area composition. Publication boundaries
//! do not constrain terrain evolution; slicing order never changes its physical
//! state. Manifest publication is the final successful operation.

pub(crate) mod annual_source;
pub(crate) mod child_links;
#[cfg(test)]
mod entrypoint_tests;
mod error;
#[cfg(test)]
mod error_chain_tests;
mod final_writes;
mod fine_delivery;
pub mod fine_formation;
pub mod fine_input;
mod fine_materials;
pub mod fine_source;
mod fine_valleys;
mod fine_world;
pub use fine_delivery::{
    generate_world_with_fine_recipe, generate_world_with_fine_source, FineDeliveryLimits,
    FineRecipe,
};
use fine_input::FineInputError;
pub(crate) mod flow_disk;
mod generation_limits;
#[cfg(test)]
mod generation_limits_tests;
pub(crate) mod global_output;
pub(crate) mod prepared_files;
pub(crate) mod publication;
pub(crate) mod routing_disk;
pub(crate) mod shared_solve;
mod water_forms;

use crate::area::prepare::{prepare_area_terrain, SharedTerrain};
use crate::block::{constraints_for, fill_block};
use crate::continent::bundles::bundle_for;
use crate::continent::climate::climate;
use crate::continent::hydrology::{extract_rivers, hydrology};
use crate::continent::{generate_continent_attempt, Continent};
use crate::hydrology::fine_flow::FlowStore;
use crate::hydrology::routing::RoutingStore;
pub use crate::hydrology::types::HydrologyLimits;
use crate::hydrology::{area_output, final_index, prepared_codec, prepared_domain, routing, types};
use arda_core::formats::hydrology::{BasinNodeRow, FixedRecord, TABLE_HEADER_BYTES};
use arda_core::hydrology::{
    AnnualCatchment, BasinId, GlobalLake, GlobalReach, HydrologyMetadata, SharedCrossing,
};
use arda_core::{
    encode_blocks, encode_cells, encode_continent_objects, encode_objects, encode_overview,
    AreaCells, AreaCoord, AreaObjects, BlockArchive, CellCoord, ContinentCell, ContinentObjects,
    ContinentOverview, ContinentRiver, DischargeMilli, GenerateConfig, Manifest, RainfallMm,
    TempCentiC, TerrainKind, ValidationStats, AREA_CELLS, FORMAT_VERSION,
};
use publication::{PublicationError, WorldOutput};
use std::path::{Path, PathBuf};
use thiserror::Error;

use error::publication_error;
pub use error::GenError;
use final_writes::{create_private, write_area, write_file, FinalWrites};

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

/// Runs the whole batch and stamps the manifest.
///
/// # Errors
/// See [`GenError`].
pub fn generate_world(seed: u64, config: GenerateConfig, out: &Path) -> Result<Manifest, GenError> {
    generate_world_with_limits(seed, config, out, HydrologyLimits::default())
}

/// Runs the same deterministic batch with explicit offline-generation resource limits.
/// Limits only admit work or return an error; they never change the physical result.
///
/// # Errors
/// See [`GenError`]. Admission happens before output creation; later failures leave
/// an incomplete output without its manifest, requiring the existing clean rerun.
pub fn generate_world_with_limits(
    seed: u64,
    config: GenerateConfig,
    out: &Path,
    limits: HydrologyLimits,
) -> Result<Manifest, GenError> {
    generate_world_inner(seed, config, out, limits, None, None)
}

/// Generate a complete world from one finalized canonical fine terrain.
///
/// This opt-in source path preserves its raw heights: it applies neither the
/// legacy detail source nor the legacy 160-step evolution. Climate, continent
/// drainage and prepared area heights all sample the supplied field. The caller
/// must supply the file produced by `fine_source::generate` for this seed,
/// configuration, recipe version and attempt, declared in `descriptor`. The
/// file checksum binds geometry and heights; it does not verify that provenance.
/// A rejected field is an error, never a reroll to
/// a different terrain source. The accepted field is copied into the world.
///
/// # Errors
/// Source coverage/checksum, resource, continent validation or hydrology failures
/// leave no completed manifest. Occupied output directories are never replaced.
pub fn generate_world_from_fine_terrain(
    seed: u64,
    descriptor: arda_core::FineTerrainDescriptor,
    config: GenerateConfig,
    source: &Path,
    out: &Path,
    limits: HydrologyLimits,
) -> Result<Manifest, GenError> {
    if !(1..=arda_core::FINE_TERRAIN_LATEST_RECIPE_VERSION).contains(&descriptor.recipe_version) {
        return Err(GenError::Validation {
            check: "unsupported fine terrain source recipe".into(),
        });
    }
    generate_world_inner(seed, config, out, limits, Some((source, descriptor)), None)
}

/// [`generate_world_from_fine_terrain`] for a field formed in this run,
/// also publishing each area's river and lake forms from what formation
/// shaped (`areas/<ax>_<ay>/water.bin`, logic/02 §world-water).
///
/// # Errors
/// As [`generate_world_from_fine_terrain`].
pub fn generate_world_from_formed_terrain(
    seed: u64,
    descriptor: arda_core::FineTerrainDescriptor,
    config: GenerateConfig,
    source: &Path,
    out: &Path,
    limits: HydrologyLimits,
    water: &crate::formation::water::WaterFeatures,
) -> Result<Manifest, GenError> {
    if !(1..=arda_core::FINE_TERRAIN_LATEST_RECIPE_VERSION).contains(&descriptor.recipe_version) {
        return Err(GenError::Validation {
            check: "unsupported fine terrain source recipe".into(),
        });
    }
    generate_world_inner(
        seed,
        config,
        out,
        limits,
        Some((source, descriptor)),
        Some(water),
    )
}

fn generate_world_inner(
    seed: u64,
    config: GenerateConfig,
    out: &Path,
    limits: HydrologyLimits,
    fine: Option<(&Path, arda_core::FineTerrainDescriptor)>,
    water: Option<&crate::formation::water::WaterFeatures>,
) -> Result<Manifest, GenError> {
    let fine_admission = fine
        .map(|(path, _)| fine_world::admit(path, config, limits))
        .transpose()?;
    let limits = fine_admission.as_ref().map_or(limits, |a| a.remaining);
    let admission = generation_limits::admit(config, &WorldOutput::scratch_path(out), limits)?;
    let mut writes = FinalWrites {
        bytes: 0,
        operations: 0,
        byte_limit: admission.reservations.final_io_bytes,
        operation_limit: u128::from(admission.reservations.final_io_operations),
    };
    // The fixed-size manifest and transaction names have a conservative1MiB
    // payload allowance;16 requested API calls cover reservation and publication.
    // This is admitted before creating output and retained through the final commit.
    writes.charge(1 << 20, 16)?;
    let output = WorldOutput::begin(out).map_err(publication_error)?;
    let mut fine_reader = match (fine, &fine_admission) {
        (Some((path, _)), Some(admission)) => Some(fine_world::copy_and_open(
            path, &output, out, config, admission,
        )?),
        _ => None,
    };

    // Tier 1: continent, single-threaded, with the step-9 validation gates
    // — land fraction, then a sea-reaching river — and their deterministic
    // reroll ladder (`logic/01` §Q9). Climate, hydrology, and rivers are
    // pure functions of the grid, so building them here, once a candidate
    // clears the land gate, lets the river gate inspect them; on
    // acceptance the same context threads to the area fan-out and
    // persistence below, with no second computation.
    let mut accepted: Option<(Continent, u16, Vec<ContinentRiver>)> = None;
    let mut last_check = String::new();
    let attempts = if fine.is_some() {
        1
    } else {
        CONTINENT_ATTEMPTS
    };
    for attempt in 0..attempts {
        let candidate = if let Some(reader) = &mut fine_reader {
            fine_input::continent(
                reader,
                config,
                admission.reservations.continent_preparation_bytes,
            )?
        } else {
            generate_continent_attempt(seed, config, attempt)
        };
        let land = candidate.land_fraction_permille();
        if !LAND_FRACTION_GATE.contains(&land) {
            last_check = format!(
                "land fraction {land} per mille outside {}..={}",
                LAND_FRACTION_GATE.start(),
                LAND_FRACTION_GATE.end()
            );
            continue;
        }

        let recipe = fine.map_or(0, |(_, d)| d.recipe_version);
        let clim = if recipe >= 7 {
            // logic/01 §Q6 subtropical highs (recipe 7): the dry Hadley belt.
            crate::continent::aridity::climate_recipe7(&candidate, config.latitude_band())
        } else if recipe >= 5 {
            crate::continent::climate::climate_smoothed_rain(&candidate, config.latitude_band())
        } else {
            climate(&candidate, config.latitude_band())
        };
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
            check: format!("{last_check} after {attempts} candidate(s)"),
        });
    };

    // Prepare every modeled cell, including cropped physical fringe tiles. Final
    // workers consume the completed shared state and never read another area's output.
    let prepared_dir = output.scratch().join("prepared");
    let solve_dir = output.scratch().join("solve");
    create_private(&prepared_dir)?;
    create_private(&solve_dir)?;
    let terrain = if let Some(reader) = &mut fine_reader {
        SharedTerrain::from_heights(
            admission.domain,
            fine_input::prepared_heights(
                reader,
                config,
                admission.reservations.continent_preparation_bytes,
            )?,
        )?
    } else {
        SharedTerrain::build(seed, &continent, admission.domain)?
    };
    drop(fine_reader);
    let mut writer =
        prepared_files::PreparedWriter::new(&prepared_dir, admission.domain, admission.prepared)?;
    for entry in admission.domain.entries() {
        let bundle = bundle_for(seed, &continent, entry.area);
        let prepared = prepare_area_terrain(&terrain, &continent, &bundle, entry.valid)?;
        writer.write(&prepared)?;
    }
    drop(terrain);
    let mut prepared = writer.finish()?;
    let mut shared = shared_solve::solve(
        &mut prepared,
        &continent.grid,
        &continent.climate,
        config,
        &solve_dir,
        admission.shared,
    )?;
    if shared.routing.extent() != admission.extent || shared.flow.extent() != admission.extent {
        return Err(GenError::AdmittedDomain);
    }
    let domain = arda_core::hydrology::HydrologyDomain {
        width_cells: admission.domain.width(),
        height_cells: admission.domain.height(),
        exported_areas_wide: u32::try_from(config.areas_wide())
            .map_err(|_| GenError::InvalidOutputPath)?,
        exported_areas_high: u32::try_from(config.areas_high())
            .map_err(|_| GenError::InvalidOutputPath)?,
    };
    let index = final_index::build(
        &mut shared.routing,
        &mut shared.flow,
        domain,
        &shared.nodes,
        &shared.solution.leaf_net,
        admission.index,
    )
    .map_err(GenError::Index)?;
    let coords: Vec<AreaCoord> = config.area_coords().collect();
    // logic/02 §world-water publication: forms are written after every
    // area, once lake origins found in any area are known.
    let mut forms = Vec::new();
    let mut origins = std::collections::BTreeMap::new();
    for &area in &coords {
        let tile = prepared.tile(area)?;
        let (mut cells, objects) = area_output::compose(
            tile,
            &mut shared.routing,
            &mut shared.flow,
            &index,
            &shared.lakes,
            admission.area,
        )
        .map_err(GenError::Area)?;
        if fine.is_some_and(|(_, descriptor)| descriptor.recipe_version >= 4) {
            fine_materials::apply(&mut cells, seed, area);
        }
        write_area(seed, area, &cells, &objects, &output, &mut writes)?;
        if let Some(features) = water {
            let w = water_forms::area_water(
                area,
                &cells,
                &objects,
                features,
                &shared.lakes,
                &mut origins,
            );
            let ids: Vec<_> = objects.lakes.iter().map(|l| l.global_id).collect();
            forms.push((area, ids, w));
        }
    }
    for (area, ids, mut w) in forms {
        water_forms::resolve_origins(&mut w, &ids, &origins);
        let path = Path::new("areas").join(area.dir_name()).join("water.bin");
        // Recipe 7 adds saline flags and playas (layout version 2); recipe 6
        // keeps writing version 1 byte for byte.
        let bytes = if fine.is_some_and(|(_, d)| d.recipe_version >= 7) {
            arda_core::encode_water_v2(&w)
        } else {
            arda_core::encode_water(&w)
        };
        write_file(&output, &path, &bytes, &mut writes)?;
    }
    writes.global(&shared, &index)?;
    global_output::write(&output, &mut shared, &index)?;
    // Completed flow/receiver caches and all file readers must close before scratch
    // cleanup and the final manifest rename, including on Windows hosts.
    shared
        .routing
        .flush()
        .map_err(shared_solve::SharedError::Routing)?;
    shared
        .flow
        .flush()
        .map_err(shared_solve::SharedError::FlowStore)?;
    drop(shared);
    drop(prepared);
    drop(index);

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
        &output,
        &Path::new("continent").join("overview.bin"),
        &encode_overview(&overview),
        &mut writes,
    )?;
    write_file(
        &output,
        &Path::new("continent").join("objects.bin"),
        &encode_continent_objects(&ContinentObjects { rivers }),
        &mut writes,
    )?;

    let manifest = Manifest {
        fine_terrain: fine.map(|(_, descriptor)| descriptor),
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
    output.commit(&manifest).map_err(publication_error)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests;
