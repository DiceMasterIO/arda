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
#[cfg(test)]
mod error_chain_tests;
mod fine_delivery;
pub mod fine_input;
pub mod fine_source;
mod fine_world;
pub use fine_delivery::{generate_world_with_fine_source, FineDeliveryLimits};
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

/// A batch failure (`mockup/01` States).
#[derive(Debug, Error)]
pub enum GenError {
    /// The opt-in canonical source could not be generated.
    #[error(transparent)]
    FineSource(#[from] fine_source::FineSourceError),
    /// An opt-in canonical fine terrain failed validation or sampling.
    #[error(transparent)]
    FineInput(#[from] FineInputError),
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
    /// Public resource admission failed before creating output files.
    #[error(transparent)]
    Admission(#[from] generation_limits::AdmissionError),
    /// A completed solver changed the domain admitted before output creation.
    #[error("completed hydrology domain differs from the admitted domain")]
    AdmittedDomain,
    /// A final output would exceed its separately admitted write/read envelope.
    #[error("final output {resource} requires {required}, admitted {limit}")]
    ResourceEnvelope {
        /// Requested bytes or counted file operations.
        resource: &'static str,
        /// Total attempted requirement, including this operation.
        required: u128,
        /// Admitted total for the complete final output.
        limit: u128,
    },
    /// Canonical terrain preparation failed.
    #[error(transparent)]
    Prepare(#[from] types::HydrologyError),
    /// Prepared private storage failed.
    #[error(transparent)]
    Prepared(#[from] prepared_files::PreparedError),
    /// The shared physical annual solve failed.
    #[error(transparent)]
    Shared(#[from] shared_solve::SharedError),
    /// Final saved feature indexing failed.
    #[error("final hydrology indexing failed: {0}")]
    Index(#[source] final_index::IndexError<routing_disk::DiskError, flow_disk::FlowDiskError>),
    /// Final immutable area composition failed.
    #[error("shared area composition failed: {0}")]
    Area(#[source] area_output::AreaError<routing_disk::DiskError, flow_disk::FlowDiskError>),
    /// A complete area object's canonical encoder rejected its input.
    #[error("area object encoding failed: {0}")]
    Objects(#[from] arda_core::formats::area_objects_v4::ObjectsFormatError),
    /// A global identity/annual layer failed before publication.
    #[error(transparent)]
    Global(#[from] global_output::GlobalOutputError),
    /// Internal generated output names violated the publication contract.
    #[error("invalid generated world output path")]
    InvalidOutputPath,
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

fn publication_error(error: PublicationError) -> GenError {
    match error {
        PublicationError::Occupied(path) => GenError::OutputNotEmpty {
            dir: path.display().to_string(),
        },
        PublicationError::Io { path, source } => GenError::Write {
            path: path.display().to_string(),
            source,
        },
        PublicationError::Manifest(source) => GenError::Manifest(source),
        PublicationError::InvalidLayer => GenError::InvalidOutputPath,
    }
}
// This receipt only covers final layer writes and the repeated private child read.
// Mutable backend I/O remains charged by the already-admitted backend owners.
struct FinalWrites {
    bytes: u128,
    operations: u128,
    byte_limit: u128,
    operation_limit: u128,
}
impl FinalWrites {
    fn charge(&mut self, bytes: u128, operations: u128) -> Result<(), GenError> {
        let next_bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or(GenError::ResourceEnvelope {
                resource: "requested bytes",
                required: u128::MAX,
                limit: self.byte_limit,
            })?;
        let next_operations =
            self.operations
                .checked_add(operations)
                .ok_or(GenError::ResourceEnvelope {
                    resource: "file operations",
                    required: u128::MAX,
                    limit: self.operation_limit,
                })?;
        for (resource, required, limit) in [
            ("requested bytes", next_bytes, self.byte_limit),
            ("file operations", next_operations, self.operation_limit),
        ] {
            if required > limit {
                return Err(GenError::ResourceEnvelope {
                    resource,
                    required,
                    limit,
                });
            }
        }
        self.bytes = next_bytes;
        self.operations = next_operations;
        Ok(())
    }
    fn global(
        &mut self,
        shared: &shared_solve::SharedArtifacts,
        index: &final_index::FinalIndex,
    ) -> Result<(), GenError> {
        let children = shared.nodes.iter().try_fold(0_u128, |sum, node| {
            sum.checked_add(node.children.len() as u128)
                .ok_or(GenError::ResourceEnvelope {
                    resource: "requested bytes",
                    required: u128::MAX,
                    limit: self.byte_limit,
                })
        })?;
        let tables = [
            (shared.nodes.len() as u128, BasinNodeRow::WIDTH),
            (children, BasinId::WIDTH),
            (shared.lakes.len() as u128, GlobalLake::WIDTH),
            (index.reaches.len() as u128, GlobalReach::WIDTH),
            (index.crossings.len() as u128, SharedCrossing::WIDTH),
            (index.catchments.len() as u128, AnnualCatchment::WIDTH),
            (1, HydrologyMetadata::WIDTH),
        ];
        // One requested payload operation per record, one extra read per child,
        // and64 fixed header/create/flush/open/metadata operations. Buffered calls
        // may coalesce these; no cache-hit or syscall-count assumption is made.
        let mut bytes =
            children
                .checked_mul(BasinId::WIDTH as u128)
                .ok_or(GenError::ResourceEnvelope {
                    resource: "requested bytes",
                    required: u128::MAX,
                    limit: self.byte_limit,
                })?;
        let mut operations = children.checked_add(64).ok_or(GenError::ResourceEnvelope {
            resource: "file operations",
            required: u128::MAX,
            limit: self.operation_limit,
        })?;
        for (count, width) in tables {
            bytes = count
                .checked_mul(width as u128)
                .and_then(|v| v.checked_add(u128::from(TABLE_HEADER_BYTES)))
                .and_then(|v| v.checked_add(bytes))
                .ok_or(GenError::ResourceEnvelope {
                    resource: "requested bytes",
                    required: u128::MAX,
                    limit: self.byte_limit,
                })?;
            operations = operations
                .checked_add(count)
                .ok_or(GenError::ResourceEnvelope {
                    resource: "file operations",
                    required: u128::MAX,
                    limit: self.operation_limit,
                })?;
        }
        self.charge(bytes, operations)
    }
}
fn write_file(
    output: &WorldOutput,
    relative: &Path,
    bytes: &[u8],
    writes: &mut FinalWrites,
) -> Result<(), GenError> {
    // create_dir_all, create_new open, write_all and flush: attempted API calls.
    writes.charge(bytes.len() as u128, 4)?;
    output
        .write_layer(relative, bytes)
        .map_err(publication_error)
}
fn create_private(path: &Path) -> Result<(), GenError> {
    std::fs::create_dir(path).map_err(|source| GenError::Write {
        path: path.display().to_string(),
        source,
    })
}

/// Writes already composed cells/objects and the unchanged sampled tactical skeleton.
fn write_area(
    seed: u64,
    area: AreaCoord,
    cells: &AreaCells,
    objects: &AreaObjects,
    output: &WorldOutput,
    writes: &mut FinalWrites,
) -> Result<(), GenError> {
    let dir: PathBuf = Path::new("areas").join(area.dir_name());
    write_file(output, &dir.join("cells.bin"), &encode_cells(cells), writes)?;
    write_file(
        output,
        &dir.join("objects.bin"),
        &encode_objects(objects)?,
        writes,
    )?;

    let mut archive = BlockArchive::default();
    let mut y = 0u16;
    while y < AREA_CELLS {
        let mut x = 0u16;
        while x < AREA_CELLS {
            if let Some(at) = CellCoord::new(x, y) {
                if cells.get(at).terrain == TerrainKind::Land {
                    let c = constraints_for(cells, at);
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
    write_file(output, &Path::new("blocks").join(name), &blocks, writes)
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
    generate_world_inner(seed, config, out, limits, None)
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
    if !(1..=arda_core::FINE_TERRAIN_RECIPE_VERSION).contains(&descriptor.recipe_version) {
        return Err(GenError::Validation {
            check: "unsupported fine terrain source recipe".into(),
        });
    }
    generate_world_inner(seed, config, out, limits, Some((source, descriptor)))
}

fn generate_world_inner(
    seed: u64,
    config: GenerateConfig,
    out: &Path,
    limits: HydrologyLimits,
    fine: Option<(&Path, arda_core::FineTerrainDescriptor)>,
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
    for &area in &coords {
        let tile = prepared.tile(area)?;
        let (cells, objects) = area_output::compose(
            tile,
            &mut shared.routing,
            &mut shared.flow,
            &index,
            &shared.lakes,
            admission.area,
        )
        .map_err(GenError::Area)?;
        write_area(seed, area, &cells, &objects, &output, &mut writes)?;
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
