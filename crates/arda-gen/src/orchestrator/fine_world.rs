//! Admission and copy of an opt-in canonical terrain into a world transaction.
use super::{publication_error, FineInputError, GenError, HydrologyLimits, WorldOutput};
use crate::hydrology::{prepared_domain::PreparedDomain, HydrologyError};
use arda_core::{GenerateConfig, TerrainFileReader};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const READER_BYTES: u64 = 1 << 20;
const RAM_BYTES: u64 = READER_BYTES + 65_536;

/// The shore layer staged beside a formed fine candidate (logic/02
/// §fine-formation shore classes).
pub(super) fn shore_sidecar(source: &Path) -> PathBuf {
    source.with_extension("shore")
}

/// Largest shore sidecar accepted beside a fine file of `fine_bytes`.
const fn shore_allowance(fine_bytes: u64) -> u64 {
    fine_bytes / 8 + (1 << 20)
}

pub(super) struct Admission {
    bytes: u64,
    pub remaining: HydrologyLimits,
}

fn reserve(resource: &'static str, available: u128, required: u128) -> Result<u128, GenError> {
    available.checked_sub(required).ok_or_else(|| {
        HydrologyError::ResourceLimit {
            stage: resource,
            requested: required,
            limit: available,
        }
        .into()
    })
}

fn io(path: &Path, source: std::io::Error) -> GenError {
    GenError::Write {
        path: path.display().to_string(),
        source,
    }
}

pub(super) fn admit(
    path: &Path,
    config: GenerateConfig,
    limits: HydrologyLimits,
) -> Result<Admission, GenError> {
    let bytes = std::fs::metadata(path).map_err(|e| io(path, e))?.len();
    admit_bytes(bytes, config, limits)
}

pub(super) fn admit_bytes(
    bytes: u64,
    config: GenerateConfig,
    mut limits: HydrologyLimits,
) -> Result<Admission, GenError> {
    let domain = PreparedDomain::for_config(config).map_err(FineInputError::from)?;
    // Bound each query as a cold two-row read. This deliberately overcounts
    // the reader's row reuse and includes checksum reads, copy reads/writes,
    // and a second complete output checksum verification.
    let samples = u128::from(domain.width()) * u128::from(domain.height())
        + u128::from(config.size_km().width) * u128::from(config.size_km().height);
    // An optional shore sidecar is read and written once each.
    let io_bytes = u128::from(bytes) * 4
        + u128::from(shore_allowance(bytes)) * 2
        + samples * u128::from(READER_BYTES);
    let operations = samples * 8 + u128::from(bytes.div_ceil(65_536)) * 8 + 64;
    limits.ram_bytes = u64::try_from(reserve(
        "fine reader RAM",
        u128::from(limits.ram_bytes),
        u128::from(RAM_BYTES),
    )?)
    .map_err(|_| GenError::InvalidOutputPath)?;
    limits.global_io_bytes = reserve("fine terrain I/O bytes", limits.global_io_bytes, io_bytes)?;
    limits.global_io_operations = u64::try_from(reserve(
        "fine terrain I/O operations",
        u128::from(limits.global_io_operations),
        operations,
    )?)
    .map_err(|_| GenError::InvalidOutputPath)?;
    limits.global_event_operations = u64::try_from(reserve(
        "fine terrain sampling work",
        u128::from(limits.global_event_operations),
        samples * 64 + u128::from(bytes),
    )?)
    .map_err(|_| GenError::InvalidOutputPath)?;
    Ok(Admission {
        bytes,
        remaining: limits,
    })
}

pub(super) fn copy_and_open(
    source: &Path,
    output: &WorldOutput,
    out: &Path,
    config: GenerateConfig,
    admission: &Admission,
) -> Result<TerrainFileReader<File>, GenError> {
    let relative = Path::new(arda_core::formats::manifest::FINE_TERRAIN_PATH);
    let destination = out.join(relative);
    let mut input = File::open(source).map_err(|e| io(source, e))?;
    let mut target = output.create_layer(relative).map_err(publication_error)?;
    let mut buffer = [0u8; 65_536];
    let mut remaining = admission.bytes;
    while remaining > 0 {
        let count = usize::try_from(remaining.min(buffer.len() as u64))
            .map_err(|_| GenError::InvalidOutputPath)?;
        input
            .read_exact(&mut buffer[..count])
            .map_err(|e| io(source, e))?;
        target
            .write_all(&buffer[..count])
            .map_err(|e| io(&destination, e))?;
        remaining -= count as u64;
    }
    if input.read(&mut buffer[..1]).map_err(|e| io(source, e))? != 0 {
        return Err(GenError::Validation {
            check: "fine terrain file grew after admission".into(),
        });
    }
    target.flush().map_err(|e| io(&destination, e))?;
    drop(target);
    copy_shore(source, output, out, admission.bytes)?;
    let reader = TerrainFileReader::open(
        File::open(&destination).map_err(|e| io(&destination, e))?,
        READER_BYTES,
    )
    .map_err(FineInputError::from)?;
    if reader.origin() != (arda_core::TerrainPoint { x_um: 0, y_um: 0 })
        || reader.spacing_um() != super::fine_source::FINE_SPACING_UM
    {
        return Err(GenError::Validation {
            check: "canonical fine recipe requires origin zero and 39.0625 m spacing".into(),
        });
    }
    super::fine_input::validate_coverage(&reader, config)?;
    Ok(reader)
}

/// Streams an optional shore sidecar into `terrain/shore.bin`. The loader
/// verifies its checksum; here only its size is bounded.
fn copy_shore(
    source: &Path,
    output: &WorldOutput,
    out: &Path,
    fine_bytes: u64,
) -> Result<(), GenError> {
    let sidecar = shore_sidecar(source);
    let Ok(meta) = std::fs::metadata(&sidecar) else {
        return Ok(());
    };
    if meta.len() > shore_allowance(fine_bytes) {
        return Err(GenError::Validation {
            check: "shore layer exceeds its admitted size".into(),
        });
    }
    let relative = Path::new(arda_core::SHORE_PATH);
    let destination = out.join(relative);
    let mut input = File::open(&sidecar).map_err(|e| io(&sidecar, e))?;
    let mut target = output.create_layer(relative).map_err(publication_error)?;
    let copied = std::io::copy(&mut (&mut input).take(meta.len()), &mut target)
        .map_err(|e| io(&destination, e))?;
    if copied != meta.len() {
        return Err(GenError::Validation {
            check: "shore layer changed size during copy".into(),
        });
    }
    target.flush().map_err(|e| io(&destination, e))
}
