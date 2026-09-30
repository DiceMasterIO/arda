//! Canonical fine-terrain access in bounded per-area windows (contract §fine).
//!
//! A window holds the exact lattice samples covering one area's cell
//! footprints (cell `(gx, gy)` covers `[100·gx, 100·gx + 100)` m, vocabulary
//! I1) and one extra lattice step, so every query on it equals the file
//! reader's integer bilinear rule bit for bit.
//!
//! Queries take world micrometres and read the lattice through the I1 cell
//! frame ([`arda_core::FINE_FRAME_OFFSET_UM`]): lattice node `(0, 0)` is
//! the centre of cell `(0, 0)`, so a cell's fine centre height is exactly
//! its stored height and the surface agrees with arda-refine's blocks.

use crate::contract::FineHeights;
use crate::error::{ServerError, ServerResult};
use arda_core::{
    AreaCoord, HeightMm, TerrainField, TerrainFileReader, TerrainPoint, FINE_FRAME_OFFSET_UM,
};
use std::io::{Read, Seek};

/// Transient reader budget: verification chunk plus two decoded rows.
pub const FINE_READER_BYTES: u64 = 1 << 20;
/// One area edge in micrometres (512 × 100 m).
pub const AREA_SPAN_UM: i64 = 51_200_000_000;
/// One cell in micrometres.
pub const CELL_UM: i64 = 100_000_000;
/// Half a cell: the offset of a cell's centre from its north-west corner.
pub const HALF_CELL_UM: i64 = CELL_UM / 2;
/// Largest window accepted, in samples (a full area is about 1,318²).
pub const MAX_WINDOW_SAMPLES: usize = 1_500 * 1_500;

fn index_range(area: i32, origin: i64, step: i64, count: u32) -> ServerResult<(u32, u32)> {
    let start = i64::from(area) * AREA_SPAN_UM - FINE_FRAME_OFFSET_UM;
    let lo = (start - step - origin).div_euclid(step).max(0);
    let end = start + 512 * CELL_UM + step - origin;
    let hi = (end + step - 1).div_euclid(step).min(i64::from(count) - 1);
    if hi <= lo {
        return Err(ServerError::NotFound(
            "fine terrain does not cover this area".into(),
        ));
    }
    let as_u32 = |v: i64| u32::try_from(v).map_err(|_| ServerError::Internal("window".into()));
    Ok((as_u32(lo)?, as_u32(hi)?))
}

/// Reads the window of exact lattice samples around area `at`.
///
/// # Errors
/// Missing coverage, the window size limit, allocation refusal, or file I/O.
pub fn read_window<R: Read + Seek>(
    reader: &mut TerrainFileReader<R>,
    at: AreaCoord,
) -> ServerResult<TerrainField> {
    let step = i64::from(reader.spacing_um());
    let origin = reader.origin();
    let (x0, x1) = index_range(at.x, origin.x_um, step, reader.width())?;
    let (y0, y1) = index_range(at.y, origin.y_um, step, reader.height())?;
    let (cols, rows) = (x1 - x0 + 1, y1 - y0 + 1);
    let count = usize::try_from(u64::from(cols) * u64::from(rows))
        .map_err(|_| ServerError::ResourceLimit("fine window".into()))?;
    if count > MAX_WINDOW_SAMPLES {
        return Err(ServerError::ResourceLimit(format!(
            "fine window of {count} samples exceeds {MAX_WINDOW_SAMPLES}"
        )));
    }
    let mut heights = Vec::new();
    heights
        .try_reserve_exact(count)
        .map_err(|_| ServerError::ResourceLimit("fine window allocation".into()))?;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let point = TerrainPoint {
                x_um: origin.x_um + i64::from(x) * step,
                y_um: origin.y_um + i64::from(y) * step,
            };
            let h = reader
                .sample(point)?
                .ok_or_else(|| ServerError::Internal("lattice point outside coverage".into()))?;
            heights.push(h);
        }
    }
    let window_origin = TerrainPoint {
        x_um: origin.x_um + i64::from(x0) * step,
        y_um: origin.y_um + i64::from(y0) * step,
    };
    TerrainField::new(window_origin, reader.spacing_um(), cols, rows, heights)
        .map_err(|e| ServerError::Internal(format!("fine window: {e}")))
}

fn bounds(field: &TerrainField) -> (TerrainPoint, TerrainPoint) {
    let step = i64::from(field.spacing_um());
    let o = field.origin();
    (
        o,
        TerrainPoint {
            x_um: o.x_um + i64::from(field.width() - 1) * step,
            y_um: o.y_um + i64::from(field.height() - 1) * step,
        },
    )
}

/// Where the piecewise-bilinear surface can take its extremes along one axis:
/// the clipped footprint ends plus every lattice line strictly inside.
fn candidates(lo: i64, hi: i64, origin: i64, step: i64) -> Vec<i64> {
    let mut out = vec![lo];
    let mut line = (lo - origin).div_euclid(step) * step + origin + step;
    while line < hi {
        out.push(line);
        line += step;
    }
    if hi != lo {
        out.push(hi);
    }
    out
}

/// Fine heights of cell `(gx, gy)`: the bilinear height at its centre
/// `(100·gx + 50, 100·gy + 50)` m and the exact extremes of the bilinear
/// surface over its footprint `[100·gx, 100·gx + 100]` m, clipped to terrain
/// coverage. In the I1 frame fine coverage runs from the first cell's
/// centre to the last cell's centre, so the outer half cells are clipped.
///
/// `None` when the footprint lies outside the window.
#[must_use]
pub fn footprint(field: &TerrainField, gx: u32, gy: u32) -> Option<FineHeights> {
    let (min, max) = bounds(field);
    let (x0, y0) = (
        i64::from(gx) * CELL_UM - FINE_FRAME_OFFSET_UM,
        i64::from(gy) * CELL_UM - FINE_FRAME_OFFSET_UM,
    );
    if x0 > max.x_um || y0 > max.y_um || x0 + CELL_UM < min.x_um || y0 + CELL_UM < min.y_um {
        return None;
    }
    // `point_clamped` takes world coordinates: the world centre of the cell.
    let centre = point_clamped(
        field,
        i64::from(gx) * CELL_UM + HALF_CELL_UM,
        i64::from(gy) * CELL_UM + HALF_CELL_UM,
    );
    let step = i64::from(field.spacing_um());
    let xs = candidates(
        x0.max(min.x_um),
        (x0 + CELL_UM).min(max.x_um),
        min.x_um,
        step,
    );
    let ys = candidates(
        y0.max(min.y_um),
        (y0 + CELL_UM).min(max.y_um),
        min.y_um,
        step,
    );
    let (mut lo, mut hi) = (centre.raw(), centre.raw());
    for &y_um in &ys {
        for &x_um in &xs {
            let h = field.sample(TerrainPoint { x_um, y_um })?.raw();
            lo = lo.min(h);
            hi = hi.max(h);
        }
    }
    let m = |h: i32| crate::contract::convert::mm_to_m(i64::from(h));
    Some(FineHeights {
        centre_m: m(centre.raw()),
        min_m: m(lo),
        max_m: m(hi),
    })
}

/// Bilinear fine height at a world point, `None` outside the window.
#[must_use]
pub fn point(field: &TerrainField, x_um: i64, y_um: i64) -> Option<HeightMm> {
    field.sample(TerrainPoint { x_um, y_um }.fine_of_world())
}

/// Bilinear fine height at a world point clamped to the window, so a point
/// in the outer half cells of the world, which the lattice does not cover,
/// still reads the nearest covered surface.
#[must_use]
pub fn point_clamped(field: &TerrainField, x_um: i64, y_um: i64) -> HeightMm {
    let (min, max) = bounds(field);
    let p = TerrainPoint { x_um, y_um }.fine_of_world();
    let p = TerrainPoint {
        x_um: p.x_um.clamp(min.x_um, max.x_um),
        y_um: p.y_um.clamp(min.y_um, max.y_um),
    };
    field.sample(p).unwrap_or(HeightMm::SEA_LEVEL)
}

/// Heap bytes held by a window.
#[must_use]
pub fn window_bytes(field: &TerrainField) -> usize {
    std::mem::size_of_val(field.heights())
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::TerrainFileWriter;
    use std::io::Cursor;

    const STEP: u32 = 39_062_500;

    /// A plane rising 1 m per metre east plus one lattice node raised by 100 m.
    fn reader(w: u32, h: u32) -> TerrainFileReader<Cursor<Vec<u8>>> {
        let mut writer = TerrainFileWriter::new(
            Cursor::new(Vec::new()),
            TerrainPoint { x_um: 0, y_um: 0 },
            STEP,
            w,
            h,
        )
        .unwrap();
        for y in 0..h {
            let row: Vec<_> = (0..w)
                .map(|x| {
                    let bump = if (x, y) == (3, 3) { 100_000 } else { 0 };
                    HeightMm::new(
                        i32::try_from(u64::from(x) * u64::from(STEP) / 1000).unwrap() + bump,
                    )
                })
                .collect();
            writer.write_row(&row).unwrap();
        }
        TerrainFileReader::open(writer.finish().unwrap(), FINE_READER_BYTES).unwrap()
    }

    #[test]
    fn window_values_equal_the_file_reader_bit_for_bit() {
        let mut r = reader(40, 40);
        let field = read_window(&mut r, AreaCoord::new(0, 0)).unwrap();
        for (x_um, y_um) in [(0, 0), (123_456_789, 98_765_432), (1_000_000_000, 7)] {
            let p = TerrainPoint { x_um, y_um };
            assert_eq!(field.sample(p), r.sample(p).unwrap());
        }
    }

    #[test]
    fn point_interpolates_bilinearly_between_lattice_nodes() {
        let mut r = reader(40, 40);
        let field = read_window(&mut r, AreaCoord::new(0, 0)).unwrap();
        // On the plane, height in mm equals the lattice x in metres, and
        // the lattice starts at the centre of cell (0, 0): world 50 m (I1).
        let (w, y) = (FINE_FRAME_OFFSET_UM, FINE_FRAME_OFFSET_UM);
        assert_eq!(
            point(&field, w + 20_000_000, y),
            Some(HeightMm::new(20_000))
        );
        assert_eq!(
            point(&field, w + 19_531_250, y),
            Some(HeightMm::new(19_531))
        );
        assert_eq!(point(&field, w - 1, y), None);
    }

    #[test]
    fn footprint_extremes_catch_an_interior_lattice_peak() {
        let mut r = reader(40, 40);
        let field = read_window(&mut r, AreaCoord::new(0, 0)).unwrap();
        // Cell (1,1) covers world [100, 200] m, lattice [50, 150] m, with
        // its centre on lattice 100 m (its stored sample point); lattice
        // node (3,3) at 117.1875 m, raised by 100 m, is inside it and lifts
        // the centre above the plane.
        let f = footprint(&field, 1, 1).unwrap();
        assert!(f.centre_m > 100.0 && f.centre_m < f.max_m, "{}", f.centre_m);
        assert!(
            (f.max_m - 217.187).abs() < 1e-9,
            "raised node beats the east edge"
        );
        assert!((f.min_m - 50.0).abs() < 1e-9, "west edge on lattice 50 m");
        // Cell (0,0): its centre is the lattice origin; the west half cell
        // lies outside the lattice and is clipped.
        let corner = footprint(&field, 0, 0).unwrap();
        assert!(corner.min_m.abs() < 1e-9);
        assert!(corner.centre_m.abs() < 1e-9);
        // Coverage ends on lattice 39 × 39.0625 m = 1523.4375 m: cell 15's
        // centre (lattice 1500 m) is covered, cell 16 starts beyond it.
        let edge = footprint(&field, 15, 0).unwrap();
        assert!((edge.centre_m - 1500.0).abs() < 1e-9, "{}", edge.centre_m);
        assert!(footprint(&field, 16, 0).is_none());
    }
}
