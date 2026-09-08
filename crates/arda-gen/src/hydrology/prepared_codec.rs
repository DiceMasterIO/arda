//! Checked private prepared tiles: physical height, annual rain and temperature reference.

use super::types::{PreparedExtent, PreparedTerrain};
use arda_core::{AreaCoord, CellCoord, HeightMm, RainfallMm, AREA_CELLS};

const MAGIC: &[u8; 8] = b"ARDAPRP1";
/// Explicit stored row width, independent of Rust struct alignment.
pub const PREPARED_CELL_BYTES: usize = 10;
/// Fixed private header, including cropped extent and domain-side flags.
pub const PREPARED_HEADER_BYTES: usize = 32;

/// One valid cropped prepared cell; no solved marine/lake/river state is encoded here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedCell {
    /// Immutable physical terrain in millimetres.
    pub height: HeightMm,
    /// Canonical annual precipitation before classification masks.
    pub annual_rain: RainfallMm,
    /// Unclamped lapse-removed reference, finalized after fine marine connectivity.
    pub temperature_base_centi: i32,
}

/// A cropped private tile, distinct from the full 512² preparation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTile {
    area: AreaCoord,
    valid: PreparedExtent,
    cells: Vec<PreparedCell>,
}
impl PreparedTile {
    /// Canonical area coordinate from the caller's sorted preparation index.
    #[must_use]
    pub fn area(&self) -> AreaCoord {
        self.area
    }
    /// Actual modeled extent and outer-domain side flags.
    #[must_use]
    pub fn valid(&self) -> PreparedExtent {
        self.valid
    }
    /// Cropped row-major cells, excluding unmodeled padding.
    #[must_use]
    pub fn cells(&self) -> &[PreparedCell] {
        &self.cells
    }
    /// Reads a modeled local cell; unmodeled right/bottom padding is absent.
    #[must_use]
    pub fn get(&self, at: CellCoord) -> Option<&PreparedCell> {
        if at.x() >= self.valid.width || at.y() >= self.valid.height {
            return None;
        }
        self.cells
            .get(usize::from(at.y()) * usize::from(self.valid.width) + usize::from(at.x()))
    }
}

/// Invalid private input or bytes; callers attach the actual scratch path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PreparedFormatError {
    /// Area/extent falls outside the supported modeled coordinate range.
    #[error("invalid prepared tile extent or coordinate")]
    Extent,
    /// Pure preparation did not retain all three full canonical arrays.
    #[error("prepared terrain arrays must contain exactly 512 by 512 cells")]
    ArrayShape,
    /// The exact byte count differs from the admitted cropped payload.
    #[error("prepared tile byte count disagrees with its indexed extent")]
    Length,
    /// Magic, identity, extent, row width, flags, count or reserved bytes disagree.
    #[error("prepared tile header disagrees with its canonical index")]
    Header,
}

/// Exact encoded size; validate before allocating or writing a private tile.
///
/// # Errors
/// Rejects empty/oversized cropped dimensions.
pub fn encoded_len(valid: PreparedExtent) -> Result<usize, PreparedFormatError> {
    if valid.width == 0
        || valid.height == 0
        || valid.width > AREA_CELLS
        || valid.height > AREA_CELLS
    {
        return Err(PreparedFormatError::Extent);
    }
    Ok(PREPARED_HEADER_BYTES
        + usize::from(valid.width) * usize::from(valid.height) * PREPARED_CELL_BYTES)
}

fn header(area: AreaCoord, valid: PreparedExtent) -> Result<[u8; 32], PreparedFormatError> {
    encoded_len(valid)?;
    let x = u32::try_from(area.x).map_err(|_| PreparedFormatError::Extent)?;
    let y = u32::try_from(area.y).map_err(|_| PreparedFormatError::Extent)?;
    let end_x = x
        .checked_mul(u32::from(AREA_CELLS))
        .and_then(|n| n.checked_add(u32::from(valid.width)))
        .ok_or(PreparedFormatError::Extent)?;
    let end_y = y
        .checked_mul(u32::from(AREA_CELLS))
        .and_then(|n| n.checked_add(u32::from(valid.height)))
        .ok_or(PreparedFormatError::Extent)?;
    if end_x > 40_000
        || end_y > 40_000
        || valid.boundary.north != (y == 0)
        || valid.boundary.west != (x == 0)
    {
        return Err(PreparedFormatError::Extent);
    }
    let mut bytes = [0; 32];
    bytes[..8].copy_from_slice(MAGIC);
    bytes[8..12].copy_from_slice(&area.x.to_le_bytes());
    bytes[12..16].copy_from_slice(&area.y.to_le_bytes());
    bytes[16..18].copy_from_slice(&valid.width.to_le_bytes());
    bytes[18..20].copy_from_slice(&valid.height.to_le_bytes());
    bytes[20] = u8::from(valid.boundary.north)
        | (u8::from(valid.boundary.east) << 1)
        | (u8::from(valid.boundary.south) << 2)
        | (u8::from(valid.boundary.west) << 3);
    bytes[24..26].copy_from_slice(&10_u16.to_le_bytes());
    bytes[28..32]
        .copy_from_slice(&(u32::from(valid.width) * u32::from(valid.height)).to_le_bytes());
    Ok(bytes)
}

/// Encodes only modeled rows/columns from full canonical preparation arrays.
///
/// # Errors
/// Rejects invalid extents and noncanonical full-array lengths before output allocation.
pub fn encode_prepared(prepared: &PreparedTerrain) -> Result<Vec<u8>, PreparedFormatError> {
    let head = header(prepared.area, prepared.valid)?;
    let count = usize::from(AREA_CELLS) * usize::from(AREA_CELLS);
    if prepared.heights.len() != count
        || prepared.annual_rain.len() != count
        || prepared.temperature_base_centi.len() != count
    {
        return Err(PreparedFormatError::ArrayShape);
    }
    let mut bytes = Vec::with_capacity(encoded_len(prepared.valid)?);
    bytes.extend_from_slice(&head);
    for y in 0..prepared.valid.height {
        for x in 0..prepared.valid.width {
            let at = usize::from(y) * usize::from(AREA_CELLS) + usize::from(x);
            bytes.extend_from_slice(&prepared.heights[at].raw().to_le_bytes());
            bytes.extend_from_slice(&prepared.annual_rain[at].raw().to_le_bytes());
            bytes.extend_from_slice(&prepared.temperature_base_centi[at].to_le_bytes());
        }
    }
    Ok(bytes)
}

/// Decodes the exact tile named by a canonical preparation index entry.
///
/// # Errors
/// Rejects invalid indexed layout, any byte-count mismatch and any header mismatch
/// before a count-driven allocation. Does not invent values outside the cropped extent.
pub fn decode_prepared(
    area: AreaCoord,
    valid: PreparedExtent,
    bytes: &[u8],
) -> Result<PreparedTile, PreparedFormatError> {
    let expected_header = header(area, valid)?;
    if bytes.len() != encoded_len(valid)? {
        return Err(PreparedFormatError::Length);
    }
    if bytes[..PREPARED_HEADER_BYTES] != expected_header {
        return Err(PreparedFormatError::Header);
    }
    let mut cells = Vec::with_capacity(usize::from(valid.width) * usize::from(valid.height));
    for row in bytes[PREPARED_HEADER_BYTES..]
        .as_chunks::<PREPARED_CELL_BYTES>()
        .0
    {
        cells.push(PreparedCell {
            height: HeightMm::new(i32::from_le_bytes([row[0], row[1], row[2], row[3]])),
            annual_rain: RainfallMm::new(u16::from_le_bytes([row[4], row[5]])),
            temperature_base_centi: i32::from_le_bytes([row[6], row[7], row[8], row[9]]),
        });
    }
    Ok(PreparedTile { area, valid, cells })
}

#[cfg(test)]
mod tests {
    use super::super::types::MarineBoundary;
    use super::*;
    fn terrain(width: u16, height: u16) -> PreparedTerrain {
        let count = usize::from(AREA_CELLS) * usize::from(AREA_CELLS);
        PreparedTerrain {
            area: AreaCoord::new(0, 0),
            valid: PreparedExtent {
                width,
                height,
                boundary: MarineBoundary {
                    north: true,
                    west: true,
                    east: true,
                    south: true,
                },
            },
            heights: (0..count)
                .map(|i| HeightMm::new(i32::try_from(i).unwrap() - 1000))
                .collect(),
            annual_rain: vec![RainfallMm::new(u16::MAX); count],
            temperature_base_centi: (0..count)
                .map(|i| 40_000 - i32::try_from(i).unwrap())
                .collect(),
        }
    }
    #[test]
    fn cropped_rows_keep_i32_temperature_reference_and_all_cell_precipitation() {
        let input = terrain(2, 3);
        let bytes = encode_prepared(&input).unwrap();
        assert_eq!(bytes.len(), 32 + 6 * 10);
        let tile = decode_prepared(input.area, input.valid, &bytes).unwrap();
        assert_eq!(tile.area(), input.area);
        assert_eq!(tile.valid(), input.valid);
        assert_eq!(tile.cells().len(), 6);
        for y in 0..3 {
            for x in 0..2 {
                let at = CellCoord::new(x, y).unwrap();
                let cell = tile.get(at).unwrap();
                assert_eq!(cell.height, input.heights[at.index()]);
                assert_eq!(cell.annual_rain.raw(), u16::MAX);
                assert_eq!(
                    cell.temperature_base_centi,
                    input.temperature_base_centi[at.index()]
                );
            }
        }
        assert!(tile.get(CellCoord::new(2, 0).unwrap()).is_none());
        assert!(tile.get(CellCoord::new(0, 3).unwrap()).is_none());
    }
    #[test]
    fn full_tile_extremes_and_repeat_bytes_are_exact() {
        let mut input = terrain(512, 512);
        let last = input.heights.len() - 1;
        input.heights[0] = HeightMm::new(i32::MIN);
        input.heights[last] = HeightMm::new(i32::MAX);
        input.temperature_base_centi[0] = i32::MAX;
        input.temperature_base_centi[last] = i32::MIN;
        let bytes = encode_prepared(&input).unwrap();
        assert_eq!(bytes.len(), 32 + 262_144 * 10);
        assert_eq!(bytes, encode_prepared(&input).unwrap());
        let tile = decode_prepared(input.area, input.valid, &bytes).unwrap();
        assert_eq!(tile.cells()[0].temperature_base_centi, i32::MAX);
        assert_eq!(tile.cells()[last].temperature_base_centi, i32::MIN);
        assert_eq!(tile.cells()[0].height.raw(), i32::MIN);
        assert_eq!(tile.cells()[last].height.raw(), i32::MAX);
    }
    #[test]
    fn every_short_prefix_trailing_byte_and_changed_header_is_rejected() {
        let input = terrain(2, 3);
        let bytes = encode_prepared(&input).unwrap();
        for n in 0..bytes.len() {
            assert_eq!(
                decode_prepared(input.area, input.valid, &bytes[..n]),
                Err(PreparedFormatError::Length)
            );
        }
        let mut extra = bytes.clone();
        extra.push(0);
        assert_eq!(
            decode_prepared(input.area, input.valid, &extra),
            Err(PreparedFormatError::Length)
        );
        for i in 0..32 {
            let mut bad = bytes.clone();
            bad[i] ^= 1;
            assert_eq!(
                decode_prepared(input.area, input.valid, &bad),
                Err(PreparedFormatError::Header)
            );
        }
        assert_eq!(
            decode_prepared(AreaCoord::new(1, 0), input.valid, &bytes),
            Err(PreparedFormatError::Extent)
        );
        let mut expected = input.valid;
        expected.boundary.west = false;
        assert_eq!(
            decode_prepared(AreaCoord::new(1, 0), expected, &bytes),
            Err(PreparedFormatError::Header)
        );
    }
    #[test]
    fn invalid_extent_arrays_and_world_overshoot_fail_before_encoding() {
        let mut input = terrain(2, 3);
        input.valid.width = 0;
        assert_eq!(encode_prepared(&input), Err(PreparedFormatError::Extent));
        input.valid.width = 513;
        assert_eq!(encode_prepared(&input), Err(PreparedFormatError::Extent));
        input.valid.width = 2;
        input.heights.pop();
        assert_eq!(
            encode_prepared(&input),
            Err(PreparedFormatError::ArrayShape)
        );
        let mut input = terrain(65, 64);
        input.area = AreaCoord::new(78, 78);
        input.valid.boundary.north = false;
        input.valid.boundary.west = false;
        assert_eq!(encode_prepared(&input), Err(PreparedFormatError::Extent));
        input.valid.width = 64;
        assert!(encode_prepared(&input).is_ok());
    }
}
