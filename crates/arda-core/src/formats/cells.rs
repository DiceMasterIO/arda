//! `areas/<ax>_<ay>/cells.bin` — 512x512 fixed-layout rows.
//!
//! Fields are written one at a time in little-endian order. No `unsafe`
//! transmute (`code-prefs.md` §Q1), so the layout is host-independent by
//! construction. Row layout is documented in `02-models.md`.

use super::{
    put_i16, put_i32, put_u16, put_u32, put_u64, take_i16, take_i32, take_u16, take_u32, take_u64,
    take_u8,
};
use crate::cell::{Cell, Cover, RoadClass, TerrainKind};
use crate::coords::{CellCoord, AREA_CELLS};
use crate::error::FormatError;
use crate::fixed::{DischargeMilli, HeightMm, RainfallMm, TempCentiC};

/// Bytes per stored cell row.
pub const CELL_BYTES: usize = 39;

/// Cells in one area layer.
const CELL_COUNT: usize = AREA_CELLS as usize * AREA_CELLS as usize;

/// One area tile's full cell grid, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaCells {
    cells: Vec<Cell>,
}

impl AreaCells {
    /// Builds a grid with every cell set to `fill`.
    #[must_use]
    pub fn flat(fill: Cell) -> Self {
        Self {
            cells: vec![fill; CELL_COUNT],
        }
    }

    /// Reads one cell.
    ///
    /// Invariant: `CellCoord` is bounds-checked at construction and the
    /// vector is always `CELL_COUNT` long, so the index cannot escape.
    #[must_use]
    pub fn get(&self, at: CellCoord) -> &Cell {
        &self.cells[at.index()]
    }

    /// Overwrites one cell.
    pub fn set(&mut self, at: CellCoord, cell: Cell) {
        self.cells[at.index()] = cell;
    }

    /// Iterates cells in row-major order.
    pub fn iter(&self) -> impl Iterator<Item = &Cell> {
        self.cells.iter()
    }
}

/// Appends one cell row to `out`.
pub(crate) fn put_cell(out: &mut Vec<u8>, c: &Cell) {
    put_i32(out, c.height.raw());
    out.push(c.terrain as u8);
    out.push(c.cover as u8);
    put_u16(out, c.slope_milli_deg);
    put_u16(out, c.aspect_deg);
    put_i16(out, c.temperature.raw());
    put_u16(out, c.rainfall.raw());
    out.push(c.moisture);
    out.push(c.forest_density);
    put_u32(out, c.drainage_area_cells);
    put_u64(out, c.discharge.raw());
    out.push(c.watercourse_order);
    put_u32(out, c.watercourse_width_dm);
    put_u16(out, c.height_above_river_dm);
    out.push(c.wetness);
    out.push(c.road as u8);
    put_u16(out, c.built_by);
}

/// Reads one cell row at `at`, advancing the cursor.
///
/// # Errors
/// Returns [`FormatError::UnknownDiscriminant`] for a stored enum byte this
/// build does not know.
pub(crate) fn take_cell(path: &str, src: &[u8], at: &mut usize) -> Result<Cell, FormatError> {
    let unknown = |field: &'static str, value: u8| FormatError::UnknownDiscriminant {
        path: path.to_owned(),
        field,
        value: u16::from(value),
    };

    let height = HeightMm::new(take_i32(src, at));
    let terrain_raw = take_u8(src, at);
    let terrain =
        TerrainKind::from_u8(terrain_raw).ok_or_else(|| unknown("terrain", terrain_raw))?;
    let cover_raw = take_u8(src, at);
    let cover = Cover::from_u8(cover_raw).ok_or_else(|| unknown("cover", cover_raw))?;
    let slope_milli_deg = take_u16(src, at);
    let aspect_deg = take_u16(src, at);
    let temperature = TempCentiC::new(take_i16(src, at));
    let rainfall = RainfallMm::new(take_u16(src, at));
    let moisture = take_u8(src, at);
    let forest_density = take_u8(src, at);
    let drainage_area_cells = take_u32(src, at);
    let discharge = DischargeMilli::new(take_u64(src, at));
    let watercourse_order = take_u8(src, at);
    let watercourse_width_dm = take_u32(src, at);
    let height_above_river_dm = take_u16(src, at);
    let wetness = take_u8(src, at);
    let road_raw = take_u8(src, at);
    let road = RoadClass::from_u8(road_raw).ok_or_else(|| unknown("road", road_raw))?;
    let built_by = take_u16(src, at);

    Ok(Cell {
        height,
        terrain,
        cover,
        slope_milli_deg,
        aspect_deg,
        temperature,
        rainfall,
        moisture,
        forest_density,
        drainage_area_cells,
        discharge,
        watercourse_order,
        watercourse_width_dm,
        height_above_river_dm,
        wetness,
        road,
        built_by,
    })
}

/// Encodes a full area cell layer.
#[must_use]
pub fn encode_cells(cells: &AreaCells) -> Vec<u8> {
    let mut out = Vec::with_capacity(CELL_COUNT * CELL_BYTES);
    for cell in cells.iter() {
        put_cell(&mut out, cell);
    }
    out
}

/// Decodes a full area cell layer.
///
/// # Errors
/// - [`FormatError::UnexpectedEof`] when the layer is not exactly the
///   expected length.
/// - [`FormatError::UnknownDiscriminant`] for an unrecognised enum byte.
pub fn decode_cells(path: &str, bytes: &[u8]) -> Result<AreaCells, FormatError> {
    let expected = CELL_COUNT * CELL_BYTES;
    if bytes.len() != expected {
        return Err(FormatError::UnexpectedEof {
            path: path.to_owned(),
            read: bytes.len(),
            expected,
        });
    }
    let mut cells = Vec::with_capacity(CELL_COUNT);
    let mut at = 0usize;
    for _ in 0..CELL_COUNT {
        cells.push(take_cell(path, bytes, &mut at)?);
    }
    Ok(AreaCells { cells })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn distinctive() -> Cell {
        Cell {
            height: HeightMm::new(-1_234_567),
            terrain: TerrainKind::Lake,
            cover: Cover::Marsh,
            slope_milli_deg: 41_234,
            aspect_deg: 359,
            temperature: TempCentiC::new(-3_912),
            rainfall: RainfallMm::new(64_000),
            moisture: 199,
            forest_density: 7,
            drainage_area_cells: 4_000_000_009,
            discharge: DischargeMilli::new(u64::from(u32::MAX) + 17),
            watercourse_order: 6,
            watercourse_width_dm: u32::from(u16::MAX) + 17,
            height_above_river_dm: 12_345,
            wetness: 254,
            road: RoadClass::Highway,
            built_by: 65_533,
        }
    }

    fn cc(x: u16, y: u16) -> CellCoord {
        CellCoord::new(x, y).unwrap()
    }

    #[test]
    fn row_is_thirty_nine_bytes() {
        let mut out = Vec::new();
        put_cell(&mut out, &distinctive());
        assert_eq!(out.len(), CELL_BYTES);
    }

    #[test]
    fn every_field_round_trips() {
        let mut cells = AreaCells::flat(Cell::default());
        let at = cc(7, 9);
        cells.set(at, distinctive());

        let bytes = encode_cells(&cells);
        let back = decode_cells("cells.bin", &bytes).unwrap();

        assert_eq!(*back.get(at), distinctive());
        assert_eq!(*back.get(cc(0, 0)), Cell::default());
    }

    #[test]
    fn encoding_is_stable_across_calls() {
        // logic/04: byte-identity. Same cells in, same bytes out, always.
        let cells = AreaCells::flat(distinctive());
        assert_eq!(encode_cells(&cells), encode_cells(&cells));
    }

    #[test]
    fn full_layer_is_the_expected_size() {
        let cells = AreaCells::flat(Cell::default());
        assert_eq!(encode_cells(&cells).len(), 512 * 512 * CELL_BYTES);
    }

    #[test]
    fn truncated_layer_is_refused_naming_the_file() {
        let bytes = encode_cells(&AreaCells::flat(Cell::default()));
        let err = decode_cells("areas/00_00/cells.bin", &bytes[..bytes.len() - 1]).unwrap_err();
        match err {
            FormatError::UnexpectedEof { path, expected, .. } => {
                assert_eq!(path, "areas/00_00/cells.bin");
                assert_eq!(expected, 512 * 512 * CELL_BYTES);
            }
            other => panic!("expected UnexpectedEof, got {other:?}"),
        }
    }

    #[test]
    fn unknown_terrain_discriminant_is_refused() {
        let mut bytes = encode_cells(&AreaCells::flat(Cell::default()));
        bytes[4] = 200;
        let err = decode_cells("cells.bin", &bytes).unwrap_err();
        match err {
            FormatError::UnknownDiscriminant { field, value, .. } => {
                assert_eq!(field, "terrain");
                assert_eq!(value, 200);
            }
            other => panic!("expected UnknownDiscriminant, got {other:?}"),
        }
    }

    /// Property-style sweep: every discriminant combination survives a
    /// round trip (`code-prefs.md` §Q6 welcomes property tests on formats).
    #[test]
    fn all_enum_combinations_round_trip() {
        let terrains = [TerrainKind::Sea, TerrainKind::Land, TerrainKind::Lake];
        let covers = [
            Cover::Bare,
            Cover::Grass,
            Cover::Scrub,
            Cover::Forest,
            Cover::Marsh,
            Cover::Rock,
            Cover::Ice,
        ];
        let roads = [
            RoadClass::None,
            RoadClass::Track,
            RoadClass::Road,
            RoadClass::Highway,
        ];
        for t in terrains {
            for c in covers {
                for r in roads {
                    let cell = Cell {
                        terrain: t,
                        cover: c,
                        road: r,
                        ..Cell::default()
                    };
                    let mut out = Vec::new();
                    put_cell(&mut out, &cell);
                    let mut at = 0;
                    assert_eq!(take_cell("cells.bin", &out, &mut at).unwrap(), cell);
                }
            }
        }
    }
}
