//! Registered samples of one persisted physical terrain for downstream stages.
//!
//! The 1 km and 100 m lattices share origin (0,0): every climate/drainage node
//! is the exact same query as the corresponding hundred-metre node. No extra
//! relief, clamping, erosion or datum shift is applied during this handoff.
use crate::continent::{ContinentGrid, ContinentGridError};
use crate::hydrology::prepared_domain::{DomainError, PreparedDomain};
use arda_core::{GenerateConfig, TerrainFileError, TerrainFileReader, TerrainPoint};
use std::io::{Read, Seek};

/// A fine terrain cannot supply a requested downstream raster.
#[derive(Debug, thiserror::Error)]
pub enum FineInputError {
    /// File validation or sampling failed.
    #[error(transparent)]
    File(#[from] TerrainFileError),
    /// Invalid modeled domain.
    #[error(transparent)]
    Domain(#[from] DomainError),
    /// Invalid continent shape.
    #[error(transparent)]
    Grid(#[from] ContinentGridError),
    /// Arithmetic or allocation is not representable.
    #[error("fine terrain input raster allocation failed")]
    Allocation,
    /// No edge extrapolation is allowed.
    #[error("fine terrain does not cover the requested or modeled domain")]
    Coverage,
    /// Output payload exceeds the caller's explicit remaining RAM allowance.
    #[error("fine terrain input needs {required} bytes, limit {limit}")]
    Budget {
        /// Required output raster payload.
        required: u64,
        /// Caller allowance, excluding an already-owned reader.
        limit: u64,
    },
}

/// Validate coverage of both the requested closed extent and prepared fringe.
///
/// # Errors
/// Rejects invalid configuration or incomplete coverage before allocating a raster.
pub fn validate_coverage<R: Read + Seek>(
    source: &TerrainFileReader<R>,
    config: GenerateConfig,
) -> Result<PreparedDomain, FineInputError> {
    let domain = PreparedDomain::for_config(config)?;
    let size = config.size_km();
    let required_x =
        (i64::from(size.width) * 1_000_000_000).max(i64::from(domain.width() - 1) * 100_000_000);
    let required_y =
        (i64::from(size.height) * 1_000_000_000).max(i64::from(domain.height() - 1) * 100_000_000);
    let origin = source.origin();
    let end_x =
        i128::from(origin.x_um) + i128::from(source.width() - 1) * i128::from(source.spacing_um());
    let end_y =
        i128::from(origin.y_um) + i128::from(source.height() - 1) * i128::from(source.spacing_um());
    if origin.x_um > 0
        || origin.y_um > 0
        || end_x < i128::from(required_x)
        || end_y < i128::from(required_y)
    {
        return Err(FineInputError::Coverage);
    }
    Ok(domain)
}

fn raster<R: Read + Seek>(
    source: &mut TerrainFileReader<R>,
    width: u32,
    height: u32,
    spacing_um: i64,
    max_output_bytes: u64,
) -> Result<Vec<i32>, FineInputError> {
    let count = u64::from(width) * u64::from(height);
    let required = count.checked_mul(4).ok_or(FineInputError::Allocation)?;
    if required > max_output_bytes {
        return Err(FineInputError::Budget {
            required,
            limit: max_output_bytes,
        });
    }
    let count = usize::try_from(count).map_err(|_| FineInputError::Allocation)?;
    let mut result = Vec::new();
    result
        .try_reserve_exact(count)
        .map_err(|_| FineInputError::Allocation)?;
    for y in 0..height {
        for x in 0..width {
            result.push(
                source
                    .sample(TerrainPoint {
                        x_um: i64::from(x) * spacing_um,
                        y_um: i64::from(y) * spacing_um,
                    })?
                    .ok_or(FineInputError::Coverage)?
                    .raw(),
            );
        }
    }
    Ok(result)
}

/// Sample the requested 1 km climate/drainage grid from persisted terrain.
///
/// `max_output_bytes` admits the returned grid only; the reader is already owned.
/// # Errors
/// Reports coverage, resource, allocation and file errors without a fallback surface.
pub fn continent<R: Read + Seek>(
    source: &mut TerrainFileReader<R>,
    config: GenerateConfig,
    max_output_bytes: u64,
) -> Result<ContinentGrid, FineInputError> {
    validate_coverage(source, config)?;
    let size = config.size_km();
    let heights = raster(
        source,
        size.width,
        size.height,
        1_000_000_000,
        max_output_bytes,
    )?;
    Ok(ContinentGrid::from_heights(
        i32::try_from(size.width).map_err(|_| FineInputError::Allocation)?,
        i32::try_from(size.height).map_err(|_| FineInputError::Allocation)?,
        heights,
    )?)
}

/// Sample the complete 100 m prepared bed, including unexported fringe cells.
///
/// No evolution is applied here. If a subsequent formation step changes these
/// heights, its output must become the authority before climate or routing.
/// # Errors
/// Reports coverage, resource, allocation and file errors without a fallback surface.
pub fn prepared_heights<R: Read + Seek>(
    source: &mut TerrainFileReader<R>,
    config: GenerateConfig,
    max_output_bytes: u64,
) -> Result<Vec<i32>, FineInputError> {
    let domain = validate_coverage(source, config)?;
    raster(
        source,
        domain.width(),
        domain.height(),
        100_000_000,
        max_output_bytes,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::{HeightMm, TerrainFileWriter};
    use std::io::Cursor;

    fn source(width: u32, height: u32) -> TerrainFileReader<Cursor<Vec<u8>>> {
        let mut writer = TerrainFileWriter::new(
            Cursor::new(Vec::new()),
            TerrainPoint { x_um: 0, y_um: 0 },
            1_000_000_000,
            width,
            height,
        )
        .unwrap();
        for y in 0..height {
            let row: Vec<_> = (0..width)
                .map(|x| HeightMm::new(i32::try_from(1000 * x + 2000 * y).unwrap() - 50_000))
                .collect();
            writer.write_row(&row).unwrap();
        }
        TerrainFileReader::open(writer.finish().unwrap(), 1 << 20).unwrap()
    }

    #[test]
    fn shared_registered_nodes_agree_and_fringe_is_covered() {
        let config = GenerateConfig::MICRO;
        // MICRO's last prepared sample lies at 102.3,204.7 km, beyond its
        // nominal 102,204 km rectangle. The file must include those points.
        let mut source = source(104, 206);
        let domain = validate_coverage(&source, config).unwrap();
        let grid = continent(&mut source, config, 102 * 204 * 4).unwrap();
        let bed = prepared_heights(&mut source, config, 1024 * 2048 * 4).unwrap();
        for y in 0..grid.height() {
            for x in 0..grid.width() {
                let i = usize::try_from(y * 10).unwrap() * domain.width() as usize
                    + usize::try_from(x * 10).unwrap();
                assert_eq!(grid.get(x, y).raw(), bed[i]);
            }
        }
        assert_eq!(*bed.last().unwrap(), 461_700);
        assert!(matches!(
            continent(&mut source, config, 1),
            Err(FineInputError::Budget { .. })
        ));
    }

    #[test]
    fn nominal_extent_alone_does_not_cover_prepared_fringe() {
        let source = source(103, 205);
        assert!(matches!(
            validate_coverage(&source, GenerateConfig::MICRO),
            Err(FineInputError::Coverage)
        ));
    }
}
