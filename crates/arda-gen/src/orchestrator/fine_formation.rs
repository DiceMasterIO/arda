//! Recipe-5 and recipe-6 canonical fine terrain: stream-power formation
//! written to the canonical fine terrain file (logic/02 §fine-formation).

use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
    path::Path,
};

use arda_core::{GenerateConfig, HeightMm, TerrainFileWriter, TerrainPoint};

use crate::{
    continent::ContinentGrid,
    formation::{form_world, water::WaterFeatures, FormationError, Recipe, FINE_SPACING_UM},
};

use super::{fine_source, GenError};

/// Recipe version recorded for formed terrain by default.
pub const RECIPE_VERSION: u16 = Recipe::DEFAULT.version();

/// Forms and writes one `recipe` candidate. `destination` must not exist.
/// Returns the rivers and lakes formation shaped (logic/02 §world-water),
/// which only recipe 6 records; only recipe 6 stages a shore layer.
///
/// # Errors
/// Geometry/file admission, formation RAM admission, allocation or I/O.
pub fn generate(
    seed: u64,
    attempt: u8,
    config: GenerateConfig,
    macro_grid: &ContinentGrid,
    destination: &Path,
    limits: fine_source::FineSourceLimits,
    recipe: Recipe,
) -> Result<Option<WaterFeatures>, GenError> {
    // Geometry and file-size admission are shared with the spectral source;
    // its FFT RAM estimate does not apply, so RAM is admitted by `form`.
    let geometry_limits = fine_source::FineSourceLimits {
        max_ram_bytes: u128::MAX,
        max_file_bytes: limits.max_file_bytes,
    };
    let plan = fine_source::admit(config, geometry_limits)?;
    let formed = form_world(
        seed,
        attempt,
        macro_grid,
        (plan.fine_width, plan.fine_height),
        limits.max_ram_bytes,
        recipe,
    )?;
    let (lattice, shore, water) = (formed.lattice, formed.shore, formed.water);
    let io = |source: std::io::Error| GenError::Write {
        path: destination.display().to_string(),
        source,
    };
    // logic/02 §fine-formation shore: the shore layer travels beside the
    // candidate and is published with it (`terrain/shore.bin`).
    if let Some(shore) = shore {
        let sidecar = super::fine_world::shore_sidecar(destination);
        std::fs::write(&sidecar, shore.encode()).map_err(|source| GenError::Write {
            path: sidecar.display().to_string(),
            source,
        })?;
    }
    let file = OpenOptions::new()
        .write(true)
        .read(true)
        .create_new(true)
        .open(destination)
        .map_err(io)?;
    let mut writer = TerrainFileWriter::new(
        BufWriter::with_capacity(1 << 16, file),
        TerrainPoint { x_um: 0, y_um: 0 },
        u32::try_from(FINE_SPACING_UM).map_err(|_| FormationError::ArithmeticOverflow)?,
        plan.fine_width,
        plan.fine_height,
    )
    .map_err(fine_source::FineSourceError::from)?;
    let mut row = Vec::new();
    row.try_reserve_exact(lattice.width)
        .map_err(|_| FormationError::AllocationFailed)?;
    for y in 0..lattice.height {
        row.clear();
        row.extend(
            lattice.z[y * lattice.width..(y + 1) * lattice.width]
                .iter()
                .map(|&z| HeightMm::new(z)),
        );
        writer
            .write_row(&row)
            .map_err(fine_source::FineSourceError::from)?;
    }
    let mut buffered = writer
        .finish()
        .map_err(fine_source::FineSourceError::from)?;
    buffered.flush().map_err(io)?;
    Ok(water)
}
