//! `areas/<ax>_<ay>/water.bin` — stored river and lake forms (logic/02
//! §world-water). Optional: worlds without it (older recipes) load with no
//! forms.
//!
//! Layout, little-endian: magic `ARDAWTR\0`, version `u32` = 1, four `u32`
//! counts (segments, lakes, deltas, dolines), then fixed rows:
//! segment 19 bytes (pattern u8, width dm u32, depth cm u32, belt dm u32,
//! sinuosity ‰ u16, slope ppm u32); lake 2 bytes (origin u8, terminal u8);
//! delta 12 bytes (x u16, y u16, radius m u32, catchment km² u32);
//! doline 8 bytes (x u16, y u16, radius dm u16, depth dm u16).

use super::{put_u16, put_u32, take_u16, take_u32, take_u8};
use crate::coords::CellCoord;
use crate::error::FormatError;
use crate::water::{
    AreaWater, ChannelPattern, DeltaForm, Doline, LakeForm, LakeOrigin, SegmentForm,
};

/// Leading bytes of every water-forms layer.
pub const WATER_MAGIC: &[u8; 8] = b"ARDAWTR\0";
/// Layout version.
pub const WATER_VERSION: u32 = 1;
const HEADER: usize = 8 + 4 + 16;
const SEGMENT: usize = 19;
const LAKE: usize = 2;
const DELTA: usize = 12;
const DOLINE: usize = 8;

/// Encodes one area's water forms.
#[must_use]
pub fn encode_water(w: &AreaWater) -> Vec<u8> {
    let mut out = Vec::with_capacity(
        HEADER
            + w.segments.len() * SEGMENT
            + w.lakes.len() * LAKE
            + w.deltas.len() * DELTA
            + w.dolines.len() * DOLINE,
    );
    out.extend(WATER_MAGIC);
    put_u32(&mut out, WATER_VERSION);
    for n in [
        w.segments.len(),
        w.lakes.len(),
        w.deltas.len(),
        w.dolines.len(),
    ] {
        put_u32(&mut out, u32::try_from(n).unwrap_or(u32::MAX));
    }
    for s in &w.segments {
        out.push(s.pattern as u8);
        put_u32(&mut out, s.bankfull_width_dm);
        put_u32(&mut out, s.bankfull_depth_cm);
        put_u32(&mut out, s.belt_width_dm);
        put_u16(&mut out, s.sinuosity_permille);
        put_u32(&mut out, s.slope_ppm);
    }
    for l in &w.lakes {
        out.push(l.origin as u8);
        out.push(u8::from(l.terminal));
    }
    for d in &w.deltas {
        put_u16(&mut out, d.apex.x());
        put_u16(&mut out, d.apex.y());
        put_u32(&mut out, d.radius_m);
        put_u32(&mut out, d.catchment_km2);
    }
    for d in &w.dolines {
        put_u16(&mut out, d.at.x());
        put_u16(&mut out, d.at.y());
        put_u16(&mut out, d.radius_dm);
        put_u16(&mut out, d.depth_dm);
    }
    out
}

/// Decodes one area's water forms.
///
/// # Errors
/// Wrong magic or version, truncated or trailing bytes, unknown enum
/// discriminants or out-of-area coordinates.
pub fn decode_water(bytes: &[u8], path: &str) -> Result<AreaWater, FormatError> {
    let eof = |expected: usize| FormatError::UnexpectedEof {
        path: path.to_owned(),
        read: bytes.len(),
        expected,
    };
    if bytes.len() < HEADER {
        return Err(eof(HEADER));
    }
    if &bytes[..8] != WATER_MAGIC {
        return Err(FormatError::BadMagic {
            path: path.to_owned(),
            layer: "water",
        });
    }
    let mut at = 8;
    let version = take_u32(bytes, &mut at);
    if version != WATER_VERSION {
        return Err(FormatError::UnknownDiscriminant {
            path: path.to_owned(),
            field: "water version",
            value: u16::try_from(version).unwrap_or(u16::MAX),
        });
    }
    let mut counts = [0_usize; 4];
    for c in &mut counts {
        *c = take_u32(bytes, &mut at) as usize;
    }
    let expected = counts[0]
        .checked_mul(SEGMENT)
        .and_then(|v| v.checked_add(counts[1].checked_mul(LAKE)?))
        .and_then(|v| v.checked_add(counts[2].checked_mul(DELTA)?))
        .and_then(|v| v.checked_add(counts[3].checked_mul(DOLINE)?))
        .and_then(|v| v.checked_add(HEADER))
        .ok_or_else(|| eof(usize::MAX))?;
    if bytes.len() != expected {
        return Err(eof(expected));
    }
    let unknown = |field: &'static str, value: u8| FormatError::UnknownDiscriminant {
        path: path.to_owned(),
        field,
        value: u16::from(value),
    };
    let cell = |x: u16, y: u16| {
        CellCoord::new(x, y).ok_or(FormatError::UnknownDiscriminant {
            path: path.to_owned(),
            field: "water cell",
            value: x.max(y),
        })
    };
    let mut w = AreaWater::default();
    for _ in 0..counts[0] {
        let p = take_u8(bytes, &mut at);
        w.segments.push(SegmentForm {
            pattern: ChannelPattern::from_u8(p).ok_or_else(|| unknown("channel pattern", p))?,
            bankfull_width_dm: take_u32(bytes, &mut at),
            bankfull_depth_cm: take_u32(bytes, &mut at),
            belt_width_dm: take_u32(bytes, &mut at),
            sinuosity_permille: take_u16(bytes, &mut at),
            slope_ppm: take_u32(bytes, &mut at),
        });
    }
    for _ in 0..counts[1] {
        let o = take_u8(bytes, &mut at);
        let t = take_u8(bytes, &mut at);
        if t > 1 {
            return Err(unknown("lake terminal flag", t));
        }
        w.lakes.push(LakeForm {
            origin: LakeOrigin::from_u8(o).ok_or_else(|| unknown("lake origin", o))?,
            terminal: t == 1,
        });
    }
    for _ in 0..counts[2] {
        let (x, y) = (take_u16(bytes, &mut at), take_u16(bytes, &mut at));
        w.deltas.push(DeltaForm {
            apex: cell(x, y)?,
            radius_m: take_u32(bytes, &mut at),
            catchment_km2: take_u32(bytes, &mut at),
        });
    }
    for _ in 0..counts[3] {
        let (x, y) = (take_u16(bytes, &mut at), take_u16(bytes, &mut at));
        w.dolines.push(Doline {
            at: cell(x, y)?,
            radius_dm: take_u16(bytes, &mut at),
            depth_dm: take_u16(bytes, &mut at),
        });
    }
    Ok(w)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> AreaWater {
        AreaWater {
            segments: vec![
                SegmentForm {
                    pattern: ChannelPattern::Meandering,
                    bankfull_width_dm: 146,
                    bankfull_depth_cm: 85,
                    belt_width_dm: 146,
                    sinuosity_permille: 1_630,
                    slope_ppm: 420,
                },
                SegmentForm {
                    pattern: ChannelPattern::Braided,
                    bankfull_width_dm: 90,
                    bankfull_depth_cm: 60,
                    belt_width_dm: 4_000,
                    sinuosity_permille: 1_040,
                    slope_ppm: 9_000,
                },
            ],
            lakes: vec![LakeForm {
                origin: LakeOrigin::Oxbow,
                terminal: false,
            }],
            deltas: vec![DeltaForm {
                apex: CellCoord::new(3, 511).unwrap(),
                radius_m: 2_800,
                catchment_km2: 565,
            }],
            dolines: vec![Doline {
                at: CellCoord::new(100, 7).unwrap(),
                radius_dm: 450,
                depth_dm: 120,
            }],
        }
    }

    #[test]
    fn water_forms_round_trip_exactly() {
        let w = sample();
        let bytes = encode_water(&w);
        assert_eq!(bytes.len(), HEADER + 2 * SEGMENT + LAKE + DELTA + DOLINE);
        assert_eq!(decode_water(&bytes, "w").unwrap(), w);
        assert_eq!(
            decode_water(&encode_water(&AreaWater::default()), "w").unwrap(),
            AreaWater::default()
        );
    }

    #[test]
    fn malformed_water_forms_are_refused() {
        let bytes = encode_water(&sample());
        assert!(decode_water(&bytes[..bytes.len() - 1], "w").is_err());
        let mut extra = bytes.clone();
        extra.push(0);
        assert!(decode_water(&extra, "w").is_err());
        let mut magic = bytes.clone();
        magic[0] = b'X';
        assert!(matches!(
            decode_water(&magic, "w"),
            Err(FormatError::BadMagic { .. })
        ));
        let mut pattern = bytes.clone();
        pattern[HEADER] = 9;
        assert!(decode_water(&pattern, "w").is_err());
        let mut version = bytes;
        version[8] = 2;
        assert!(decode_water(&version, "w").is_err());
    }
}
