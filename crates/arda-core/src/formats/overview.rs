//! `continent/overview.bin` — the persisted 1 km grid (feature 02 §Q6).
//!
//! Header: magic, `u32` width, `u32` height. Then `width × height`
//! fixed 18-byte little-endian records, row-major:
//! `height_mm: i32`, `temperature: i16`, `rainfall_mm: u16`,
//! `regime: u8`, `downstream: u8` (0–7 fixed neighbour order,
//! 255 = none), `catchment_km2: u32`, `discharge_l_s: u32`.

use super::{put_i16, put_i32, put_u16, put_u32, take_i16, take_i32, take_u16, take_u32, take_u8};
use crate::continent::{ClimateRegime, ContinentCell, ContinentOverview};
use crate::error::FormatError;
use crate::fixed::{DischargeMilli, HeightMm, RainfallMm, TempCentiC};

/// Layer magic.
pub const OVERVIEW_MAGIC: &[u8; 8] = b"ARDAOVR\0";

/// Bytes per cell record.
pub const OVERVIEW_CELL_BYTES: usize = 18;

/// Stored value for "no downstream".
const NO_DOWNSTREAM: u8 = 255;

/// Encodes the continent grid.
#[must_use]
pub fn encode_overview(overview: &ContinentOverview) -> Vec<u8> {
    debug_assert_eq!(
        overview.cells.len(),
        overview.width.unsigned_abs() as usize * overview.height.unsigned_abs() as usize,
        "overview cells must fill width x height"
    );
    let count = overview.cells.len();
    let mut out = Vec::with_capacity(16 + count * OVERVIEW_CELL_BYTES);
    out.extend_from_slice(OVERVIEW_MAGIC);
    put_u32(&mut out, u32::try_from(overview.width).unwrap_or(0));
    put_u32(&mut out, u32::try_from(overview.height).unwrap_or(0));
    for c in &overview.cells {
        put_i32(&mut out, c.height.raw());
        put_i16(&mut out, c.temperature.raw());
        put_u16(&mut out, c.rainfall.raw());
        out.push(c.regime as u8);
        out.push(c.downstream.unwrap_or(NO_DOWNSTREAM));
        put_u32(&mut out, c.catchment_km2);
        put_u32(&mut out, c.discharge.raw());
    }
    out
}

/// Decodes the continent grid.
///
/// # Errors
/// - [`FormatError::BadMagic`] when the file is not an overview layer.
/// - [`FormatError::UnexpectedEof`] when the grid runs past the end.
/// - [`FormatError::UnknownDiscriminant`] on a bad regime or direction.
/// - [`FormatError::DimensionsOverflow`] when a declared dimension cannot
///   be represented as the `i32` the rest of the codebase expects.
pub fn decode_overview(path: &str, bytes: &[u8]) -> Result<ContinentOverview, FormatError> {
    if bytes.len() < OVERVIEW_MAGIC.len() || &bytes[..OVERVIEW_MAGIC.len()] != OVERVIEW_MAGIC {
        return Err(FormatError::BadMagic {
            path: path.to_owned(),
            layer: "overview",
        });
    }
    let mut at = OVERVIEW_MAGIC.len();
    // `at.checked_add(extra)` so this guard itself can never wrap, no matter
    // what sizes a future caller feeds it — `None` just means "past EOF".
    let need = |at: usize, extra: usize| match at.checked_add(extra) {
        Some(end) if end <= bytes.len() => Ok(()),
        Some(end) => Err(FormatError::UnexpectedEof {
            path: path.to_owned(),
            read: bytes.len(),
            expected: end,
        }),
        None => Err(FormatError::UnexpectedEof {
            path: path.to_owned(),
            read: bytes.len(),
            expected: at.saturating_add(extra),
        }),
    };
    need(at, 8)?;
    let width = take_u32(bytes, &mut at);
    let height = take_u32(bytes, &mut at);
    // File-supplied dims: chain every multiply/add through checked
    // arithmetic so a crafted header can't wrap `usize` and slip past an
    // EOF guard (release, no overflow-checks) or panic on the add (debug).
    // Each step below reports the genuine (saturated where unrepresentable)
    // byte requirement rather than a fabricated sentinel.
    let count = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| FormatError::UnexpectedEof {
            path: path.to_owned(),
            read: bytes.len(),
            expected: (width as usize).saturating_mul(height as usize),
        })?;
    let byte_len =
        count
            .checked_mul(OVERVIEW_CELL_BYTES)
            .ok_or_else(|| FormatError::UnexpectedEof {
                path: path.to_owned(),
                read: bytes.len(),
                // Saturated: the true requirement exceeds usize::MAX.
                expected: count.saturating_mul(OVERVIEW_CELL_BYTES),
            })?;
    need(at, byte_len)?;

    let mut cells = Vec::with_capacity(count);
    for _ in 0..count {
        let h = take_i32(bytes, &mut at);
        let t = take_i16(bytes, &mut at);
        let r = take_u16(bytes, &mut at);
        let regime_raw = take_u8(bytes, &mut at);
        let regime =
            ClimateRegime::from_u8(regime_raw).ok_or(FormatError::UnknownDiscriminant {
                path: path.to_owned(),
                field: "climate regime",
                value: u16::from(regime_raw),
            })?;
        let dir_raw = take_u8(bytes, &mut at);
        let downstream = match dir_raw {
            NO_DOWNSTREAM => None,
            0..=7 => Some(dir_raw),
            _ => {
                return Err(FormatError::UnknownDiscriminant {
                    path: path.to_owned(),
                    field: "downstream direction",
                    value: u16::from(dir_raw),
                })
            }
        };
        let catchment_km2 = take_u32(bytes, &mut at);
        let discharge = DischargeMilli::new(take_u32(bytes, &mut at));
        cells.push(ContinentCell {
            height: HeightMm::new(h),
            temperature: TempCentiC::new(t),
            rainfall: RainfallMm::new(r),
            regime,
            downstream,
            catchment_km2,
            discharge,
        });
    }
    // Refuse dims this build cannot represent honestly, rather than
    // silently wrapping a `u32::MAX` width into a negative `i32` (the
    // struct's field type — see `ContinentOverview`).
    const I32_MAX_AS_U32: u32 = i32::MAX.unsigned_abs();
    if width > I32_MAX_AS_U32 || height > I32_MAX_AS_U32 {
        return Err(FormatError::DimensionsOverflow {
            path: path.to_owned(),
            width,
            height,
        });
    }
    #[allow(clippy::cast_possible_wrap)] // just refused > i32::MAX above
    Ok(ContinentOverview {
        width: width as i32,
        height: height as i32,
        cells,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continent::{ClimateRegime, ContinentCell, ContinentOverview};
    use crate::fixed::{DischargeMilli, HeightMm, RainfallMm, TempCentiC};

    fn sample() -> ContinentOverview {
        let mut cells = vec![ContinentCell::default(); 6];
        cells[4] = ContinentCell {
            height: HeightMm::new(812_000),
            temperature: TempCentiC::new(1_150),
            rainfall: RainfallMm::new(940),
            regime: ClimateRegime::Mediterranean,
            downstream: Some(6),
            catchment_km2: 4_211,
            discharge: DischargeMilli::new(66_000),
        };
        cells[1] = ContinentCell {
            height: HeightMm::new(-2_400_000),
            ..ContinentCell::default()
        };
        ContinentOverview {
            width: 3,
            height: 2,
            cells,
        }
    }

    #[test]
    fn overview_round_trips() {
        let bytes = encode_overview(&sample());
        assert_eq!(decode_overview("overview.bin", &bytes).unwrap(), sample());
    }

    #[test]
    fn encoding_is_stable_across_calls() {
        assert_eq!(encode_overview(&sample()), encode_overview(&sample()));
    }

    #[test]
    fn bad_magic_is_refused_naming_the_layer() {
        let mut bytes = encode_overview(&sample());
        bytes[0] = b'X';
        match decode_overview("continent/overview.bin", &bytes).unwrap_err() {
            crate::error::FormatError::BadMagic { path, layer } => {
                assert_eq!(path, "continent/overview.bin");
                assert_eq!(layer, "overview");
            }
            other => panic!("expected BadMagic, got {other:?}"),
        }
    }

    #[test]
    fn truncated_grid_is_refused() {
        let bytes = encode_overview(&sample());
        let err = decode_overview("overview.bin", &bytes[..bytes.len() - 5]).unwrap_err();
        assert!(matches!(
            err,
            crate::error::FormatError::UnexpectedEof { .. }
        ));
    }

    #[test]
    fn an_unknown_regime_discriminant_is_refused() {
        let mut bytes = encode_overview(&sample());
        // Record 4, regime byte: header 16 + 4*18 + offset 8 (i32+i16+u16).
        bytes[16 + 4 * 18 + 8] = 9;
        let err = decode_overview("overview.bin", &bytes).unwrap_err();
        assert!(matches!(
            err,
            crate::error::FormatError::UnknownDiscriminant {
                field: "climate regime",
                ..
            }
        ));
    }

    #[test]
    fn huge_dims_that_overflow_the_byte_count_are_refused_not_panicked() {
        // A crafted header whose width * height * cell-size overflows usize
        // must be refused with UnexpectedEof, never panic or abort.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(OVERVIEW_MAGIC);
        put_u32(&mut bytes, u32::MAX);
        put_u32(&mut bytes, u32::MAX);
        let err = decode_overview("overview.bin", &bytes).unwrap_err();
        assert!(matches!(
            err,
            crate::error::FormatError::UnexpectedEof { .. }
        ));
    }

    #[test]
    fn dims_at_the_exact_wrap_window_are_refused_not_panicked() {
        // floor(u64::MAX / 18) == 1_024_819_115_206_086_200 is the one count
        // value where `count * 18` fits in u64 (it lands 15 bytes short of
        // u64::MAX) but `count * 18 + 16` (the header) wraps past it. That
        // count factors into two u32s — 238_795_480 * 4_291_618_565 — so a
        // real width/height header can reach the wrap window. This must be
        // refused as UnexpectedEof, not panic (debug) or wrap past the EOF
        // guard into a capacity-overflow abort (release).
        let mut bytes = Vec::new();
        bytes.extend_from_slice(OVERVIEW_MAGIC);
        put_u32(&mut bytes, 238_795_480);
        put_u32(&mut bytes, 4_291_618_565);
        let err = decode_overview("overview.bin", &bytes).unwrap_err();
        assert!(matches!(
            err,
            crate::error::FormatError::UnexpectedEof { .. }
        ));
    }

    #[test]
    fn a_width_past_i32_max_is_refused_not_wrapped_negative() {
        // width = u32::MAX, height = 0, empty body: the byte-length checks
        // all pass (0 cells needed), so without an explicit dims check this
        // used to "succeed" with `width: -1` (i32 wrap). Must be refused.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(OVERVIEW_MAGIC);
        put_u32(&mut bytes, u32::MAX);
        put_u32(&mut bytes, 0);
        let err = decode_overview("overview.bin", &bytes).unwrap_err();
        assert!(matches!(
            err,
            crate::error::FormatError::DimensionsOverflow {
                width: u32::MAX,
                height: 0,
                ..
            }
        ));
    }

    #[test]
    fn an_invalid_downstream_direction_is_refused() {
        let mut bytes = encode_overview(&sample());
        bytes[16 + 4 * 18 + 9] = 8; // valid values are 0–7 and 255
        let err = decode_overview("overview.bin", &bytes).unwrap_err();
        assert!(matches!(
            err,
            crate::error::FormatError::UnknownDiscriminant {
                field: "downstream direction",
                ..
            }
        ));
    }
}
