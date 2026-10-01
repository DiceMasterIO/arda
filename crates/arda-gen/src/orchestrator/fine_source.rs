//! Opt-in, whole-domain canonical fine source; not yet part of world publication.

use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
    path::Path,
};

use arda_core::{
    GenerateConfig, HeightMm, TerrainField, TerrainFileError, TerrainFileWriter, TerrainPoint,
};
use thiserror::Error;

use crate::{
    continent::{bundles::coarse_height, ContinentGrid},
    spectral::{calibration, filter_relief, white_noise, white_noise_world, SpectralError},
    spectral_composition::{compose_smooth_relief, forced_rim_taper_q32, FULL_RANGE_MM},
};

use super::prepared_domain::PreparedDomain;

/// Source lattice spacing inherited from the qualified 512²/20 km field.
pub const FINE_SPACING_UM: u32 = 39_062_500;
/// Fine-specific macro terrain with the qualified spectral gain. This is the raw
/// source version; the valley-carved canonical field is recipe 4.
pub const SOURCE_RECIPE_VERSION: u16 = 3;
const MACRO_SPACING_UM: i64 = 100_000_000;
const HEADER_BYTES: u128 = 88;
const REFERENCE_SIDE: usize = 512;
const REFERENCE_SPAN_MM: u32 = 3_000_000;
const BUFFER_BYTES: u128 = 65_536;
const OFFSETS: [(i32, i32); 8] = [
    (0, 50),
    (0, -50),
    (50, 0),
    (-50, 0),
    (35, 35),
    (35, -35),
    (-35, 35),
    (-35, -35),
];

/// Caller-admitted peak RAM and final terrain-file bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FineSourceLimits {
    /// Includes the caller-owned 1 km macro grid and temporary spectral arrays.
    pub max_ram_bytes: u128,
    /// Exact serialized terrain file ceiling.
    pub max_file_bytes: u128,
}

/// Geometry and conservative resource envelope, computed before any output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FineSourcePlan {
    /// Nominal request width, in micrometres.
    pub nominal_width_um: u64,
    /// Nominal request height, in micrometres.
    pub nominal_height_um: u64,
    /// Source coverage before rounding up to fine lattice points.
    pub covered_width_um: u64,
    /// Source coverage before rounding up to fine lattice points.
    pub covered_height_um: u64,
    /// Last stored fine X coordinate; may exceed the requested endpoint.
    pub sampled_last_x_um: u64,
    /// Last stored fine Y coordinate; may exceed the requested endpoint.
    pub sampled_last_y_um: u64,
    /// Closed fine lattice width.
    pub fine_width: u32,
    /// Closed fine lattice height.
    pub fine_height: u32,
    /// Single whole-source FFT width.
    pub fft_width: u32,
    /// Single whole-source FFT height.
    pub fft_height: u32,
    /// Conservative peak including caller-owned macro grid.
    pub admitted_peak_ram_bytes: u128,
    /// Header plus exact row-major i32 payload.
    pub file_bytes: u128,
}

/// Exact generation facts for a file that was successfully finalized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FineSourceReceipt {
    /// Source-composition recipe used for this persisted file.
    pub recipe_version: u16,
    /// Full 64-bit source seed.
    pub world_seed: u64,
    /// Deterministic validation attempt.
    pub attempt: u8,
    /// Admitted lattice and resource envelope.
    pub plan: FineSourcePlan,
    /// Physical sample interval in micrometres.
    pub spacing_um: u32,
    /// Native reference seed used only for fixed-gain calibration.
    pub calibration_seed: u32,
    /// Reference longest retained wavelength in fine samples.
    pub reference_period_samples: u32,
    /// Fixed reference relief span in millimetres.
    pub reference_span_mm: u32,
    /// Raw integer calibration denominator; serialized as decimal text.
    pub calibration_raw_range: String,
}

/// Rejected source geometry, resource request, or file operation.
#[derive(Debug, Error)]
pub enum FineSourceError {
    /// The request does not form a supported prepared world.
    #[error("invalid prepared world domain")]
    Domain,
    /// The supplied macro grid does not match the request's 1 km extent.
    #[error("macro grid dimensions differ from generation config")]
    MacroDimensions,
    /// A padded axis exceeds the fixed integer FFT's supported table.
    #[error("whole-domain spectral axis exceeds 32768 samples")]
    SpectralExtent,
    /// A checked count, byte size or absolute coordinate overflowed.
    #[error("fine source arithmetic overflow")]
    ArithmeticOverflow,
    /// A fallible allocation failed after the caller's budget was admitted.
    #[error("fine source allocation failed")]
    AllocationFailed,
    /// The conservative live memory or final file exceeds caller admission.
    #[error("fine source {resource} requires {required} bytes, admitted {limit} bytes")]
    ResourceLimit {
        /// The admitted resource.
        resource: &'static str,
        /// Required count.
        required: u128,
        /// Caller ceiling.
        limit: u128,
    },
    /// Integer source/filter/composition failed.
    #[error(transparent)]
    Spectral(#[from] SpectralError),
    /// File format or canonical sampling failed.
    #[error(transparent)]
    Terrain(#[from] TerrainFileError),
    /// In-memory two-row macro field was invalid.
    #[error(transparent)]
    Field(#[from] arda_core::TerrainFieldError),
    /// A canonical macro or gate query escaped its planned field.
    #[error("macro/gate query escaped admitted source coverage")]
    MacroCoverage,
    /// File creation or flushing failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

fn checked_add(a: u128, b: u128) -> Result<u128, FineSourceError> {
    a.checked_add(b).ok_or(FineSourceError::ArithmeticOverflow)
}
fn checked_mul(a: u128, b: u128) -> Result<u128, FineSourceError> {
    a.checked_mul(b).ok_or(FineSourceError::ArithmeticOverflow)
}
fn spectral_budget(limit: u128) -> u64 {
    u64::try_from(limit).unwrap_or(u64::MAX)
}
fn covered_cells(um: u64, spacing: u64) -> Result<u64, FineSourceError> {
    um.checked_add(spacing - 1)
        .map(|v| v / spacing + 1)
        .ok_or(FineSourceError::ArithmeticOverflow)
}

/// Admits one globally padded field and exact output before source allocation.
///
/// The macro grid is caller-owned but included in the conservative peak. The
/// estimate also includes live white+FFT+relief, column scratch, two old and
/// two new macro/gate rows during cache replacement, output row, a 64 KiB
/// writer buffer and 10 MiB for reference calibration/runtime slack.
pub fn admit(
    config: GenerateConfig,
    limits: FineSourceLimits,
) -> Result<FineSourcePlan, FineSourceError> {
    let domain = PreparedDomain::for_config(config).map_err(|_| FineSourceError::Domain)?;
    let size = config.size_km();
    let nominal_w = u64::from(size.width) * 1_000_000_000;
    let nominal_h = u64::from(size.height) * 1_000_000_000;
    let covered_w = nominal_w.max(u64::from(domain.width() - 1) * 100_000_000);
    let covered_h = nominal_h.max(u64::from(domain.height() - 1) * 100_000_000);
    let fine_w = covered_cells(covered_w, u64::from(FINE_SPACING_UM))?;
    let fine_h = covered_cells(covered_h, u64::from(FINE_SPACING_UM))?;
    let fft_w = fine_w
        .checked_next_power_of_two()
        .ok_or(FineSourceError::SpectralExtent)?;
    let fft_h = fine_h
        .checked_next_power_of_two()
        .ok_or(FineSourceError::SpectralExtent)?;
    if fft_w > 32_768 || fft_h > 32_768 {
        return Err(FineSourceError::SpectralExtent);
    }
    let fft_cells = checked_mul(u128::from(fft_w), u128::from(fft_h))?;
    let fine_cells = checked_mul(u128::from(fine_w), u128::from(fine_h))?;
    let macro_w = covered_cells((fine_w - 1) * u64::from(FINE_SPACING_UM), 100_000_000)?;
    let macro_grid = checked_mul(u128::from(size.width), u128::from(size.height))?;
    let peak = checked_add(
        checked_add(
            checked_mul(fft_cells, 28)?,
            checked_mul(u128::from(fft_h), 16)?,
        )?,
        checked_add(
            checked_add(
                checked_mul(macro_grid, 4)?,
                checked_mul(u128::from(macro_w), 32)?,
            )?,
            checked_add(
                checked_mul(u128::from(fine_w), 4)?,
                BUFFER_BYTES + 10 * 1024 * 1024,
            )?,
        )?,
    )?;
    let file_bytes = checked_add(HEADER_BYTES, checked_mul(fine_cells, 4)?)?;
    if peak > limits.max_ram_bytes {
        return Err(FineSourceError::ResourceLimit {
            resource: "RAM",
            required: peak,
            limit: limits.max_ram_bytes,
        });
    }
    if file_bytes > limits.max_file_bytes {
        return Err(FineSourceError::ResourceLimit {
            resource: "file",
            required: file_bytes,
            limit: limits.max_file_bytes,
        });
    }
    Ok(FineSourcePlan {
        nominal_width_um: nominal_w,
        nominal_height_um: nominal_h,
        covered_width_um: covered_w,
        covered_height_um: covered_h,
        sampled_last_x_um: (fine_w - 1) * u64::from(FINE_SPACING_UM),
        sampled_last_y_um: (fine_h - 1) * u64::from(FINE_SPACING_UM),
        fine_width: u32::try_from(fine_w).map_err(|_| FineSourceError::ArithmeticOverflow)?,
        fine_height: u32::try_from(fine_h).map_err(|_| FineSourceError::ArithmeticOverflow)?,
        fft_width: u32::try_from(fft_w).map_err(|_| FineSourceError::ArithmeticOverflow)?,
        fft_height: u32::try_from(fft_h).map_err(|_| FineSourceError::ArithmeticOverflow)?,
        admitted_peak_ram_bytes: peak,
        file_bytes,
    })
}

fn macro_pair(
    grid: &ContinentGrid,
    y: u32,
    width: u32,
) -> Result<(TerrainField, TerrainField), FineSourceError> {
    let count =
        usize::try_from(u64::from(width) * 2).map_err(|_| FineSourceError::ArithmeticOverflow)?;
    let mut macro_values = Vec::new();
    let mut range_values = Vec::new();
    macro_values
        .try_reserve_exact(count)
        .map_err(|_| FineSourceError::AllocationFailed)?;
    range_values
        .try_reserve_exact(count)
        .map_err(|_| FineSourceError::AllocationFailed)?;
    for row in [y, y + 1] {
        let yi = i32::try_from(row).map_err(|_| FineSourceError::ArithmeticOverflow)?;
        for x in 0..width {
            let xi = i32::try_from(x).map_err(|_| FineSourceError::ArithmeticOverflow)?;
            let base = HeightMm::new(coarse_height(grid, xi, yi));
            let neighbors =
                OFFSETS.map(|(dx, dy)| HeightMm::new(coarse_height(grid, xi + dx, yi + dy)));
            let mut lo = i64::from(base.raw());
            let mut hi = lo;
            for height in neighbors {
                let value = i64::from(height.raw());
                lo = lo.min(value);
                hi = hi.max(value);
            }
            macro_values.push(base);
            range_values.push(HeightMm::new(
                i32::try_from((hi - lo).min(i64::from(FULL_RANGE_MM)))
                    .map_err(|_| FineSourceError::ArithmeticOverflow)?,
            ));
        }
    }
    let origin = TerrainPoint {
        x_um: 0,
        y_um: i64::from(y) * MACRO_SPACING_UM,
    };
    Ok((
        TerrainField::new(origin, 100_000_000, width, 2, macro_values)?,
        TerrainField::new(origin, 100_000_000, width, 2, range_values)?,
    ))
}

/// Writes one recipe-3 raw source from a
/// [`generate_continent_attempt_fine`](crate::continent::generate_continent_attempt_fine)
/// macro grid without changing the normal world pipeline.
///
/// `destination` must not exist. Output is left provisional if row writing
/// fails; the caller owns temporary-name/atomic-publication policy.
pub fn generate(
    seed: u64,
    attempt: u8,
    config: GenerateConfig,
    macro_grid: &ContinentGrid,
    destination: &Path,
    limits: FineSourceLimits,
) -> Result<FineSourceReceipt, FineSourceError> {
    generate_with_recipe(
        seed,
        attempt,
        config,
        macro_grid,
        destination,
        limits,
        SOURCE_RECIPE_VERSION,
    )
}

/// Reproduces the delivered recipe-2 raw source from the caller's legacy
/// macro grid. Existing saved recipe-2 worlds remain readable regardless;
/// this entry keeps their generator available for exact replay.
pub fn generate_recipe2(
    seed: u64,
    attempt: u8,
    config: GenerateConfig,
    macro_grid: &ContinentGrid,
    destination: &Path,
    limits: FineSourceLimits,
) -> Result<FineSourceReceipt, FineSourceError> {
    generate_with_recipe(seed, attempt, config, macro_grid, destination, limits, 2)
}

fn generate_with_recipe(
    seed: u64,
    attempt: u8,
    config: GenerateConfig,
    macro_grid: &ContinentGrid,
    destination: &Path,
    limits: FineSourceLimits,
    recipe_version: u16,
) -> Result<FineSourceReceipt, FineSourceError> {
    let plan = admit(config, limits)?;
    let size = config.size_km();
    if macro_grid.width()
        != i32::try_from(size.width).map_err(|_| FineSourceError::ArithmeticOverflow)?
        || macro_grid.height()
            != i32::try_from(size.height).map_err(|_| FineSourceError::ArithmeticOverflow)?
    {
        return Err(FineSourceError::MacroDimensions);
    }
    let reference = white_noise(
        42,
        REFERENCE_SIDE,
        REFERENCE_SIDE,
        spectral_budget(limits.max_ram_bytes),
    )?;
    let scale = calibration(
        &reference,
        REFERENCE_SIDE,
        REFERENCE_SPAN_MM,
        spectral_budget(limits.max_ram_bytes),
    )?;
    drop(reference);
    let fw = usize::try_from(plan.fft_width).map_err(|_| FineSourceError::ArithmeticOverflow)?;
    let fh = usize::try_from(plan.fft_height).map_err(|_| FineSourceError::ArithmeticOverflow)?;
    let white = white_noise_world(seed, attempt, fw, fh, spectral_budget(limits.max_ram_bytes))?;
    let white_bytes =
        u64::try_from(white.len()).map_err(|_| FineSourceError::ArithmeticOverflow)? * 8;
    let scratch = spectral_budget(limits.max_ram_bytes)
        .checked_sub(white_bytes)
        .ok_or(FineSourceError::ArithmeticOverflow)?;
    let relief = filter_relief(&white, fw, fh, &scale, scratch)?;
    drop(white);

    let macro_w = u32::try_from(covered_cells(
        (u64::from(plan.fine_width) - 1) * u64::from(FINE_SPACING_UM),
        100_000_000,
    )?)
    .map_err(|_| FineSourceError::ArithmeticOverflow)?;
    let file = OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .open(destination)?;
    let buffered = BufWriter::with_capacity(
        usize::try_from(BUFFER_BYTES).map_err(|_| FineSourceError::ArithmeticOverflow)?,
        file,
    );
    let mut writer = TerrainFileWriter::new(
        buffered,
        TerrainPoint { x_um: 0, y_um: 0 },
        FINE_SPACING_UM,
        plan.fine_width,
        plan.fine_height,
    )?;
    let mut row = Vec::new();
    row.try_reserve_exact(
        usize::try_from(plan.fine_width).map_err(|_| FineSourceError::ArithmeticOverflow)?,
    )
    .map_err(|_| FineSourceError::ResourceLimit {
        resource: "RAM",
        required: plan.admitted_peak_ram_bytes,
        limit: limits.max_ram_bytes,
    })?;
    let mut cached_y = None;
    let mut pair = None;
    for y in 0..plan.fine_height {
        let py = i64::from(y) * i64::from(FINE_SPACING_UM);
        let my = u32::try_from(py / MACRO_SPACING_UM)
            .map_err(|_| FineSourceError::ArithmeticOverflow)?;
        if cached_y != Some(my) {
            pair = Some(macro_pair(macro_grid, my, macro_w)?);
            cached_y = Some(my);
        }
        let (macro_field, range_field) = pair.as_ref().ok_or(FineSourceError::MacroCoverage)?;
        row.clear();
        for x in 0..plan.fine_width {
            let point = TerrainPoint {
                x_um: i64::from(x) * i64::from(FINE_SPACING_UM),
                y_um: py,
            };
            let macro_height = macro_field
                .sample(point)
                .ok_or(FineSourceError::MacroCoverage)?;
            let range = range_field
                .sample(point)
                .ok_or(FineSourceError::MacroCoverage)?;
            let index = usize::try_from(u64::from(y) * u64::from(plan.fft_width) + u64::from(x))
                .map_err(|_| FineSourceError::ArithmeticOverflow)?;
            let fine_relief = relief
                .get(index)
                .copied()
                .ok_or(FineSourceError::ArithmeticOverflow)?;
            let taper = forced_rim_taper_q32(point, size.width, size.height);
            let range =
                u32::try_from(range.raw()).map_err(|_| FineSourceError::ArithmeticOverflow)?;
            row.push(compose_smooth_relief(
                macro_height,
                fine_relief,
                range,
                taper,
            )?);
        }
        writer.write_row(&row)?;
    }
    let mut buffered = writer.finish()?;
    buffered.flush()?;
    Ok(FineSourceReceipt {
        recipe_version,
        world_seed: seed,
        attempt,
        plan,
        spacing_um: FINE_SPACING_UM,
        calibration_seed: 42,
        reference_period_samples: scale.reference_period_samples(),
        reference_span_mm: scale.span_mm(),
        calibration_raw_range: scale.calibration_raw_range().to_string(),
    })
}

#[cfg(test)]
mod tests;
