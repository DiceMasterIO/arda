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
//!
//! Version 2 (recipe 7, arid basins) has five counts (segments, lakes,
//! deltas, dolines, pans), lake rows of 3 bytes (origin u8, terminal u8,
//! saline u8) and pan rows of 7 bytes (y u16, x0 u16, len u16, kind u8).
//! Earlier recipes keep writing version 1 byte for byte.

use super::{put_u16, put_u32, take_u16, take_u32, take_u8};
use crate::coords::CellCoord;
use crate::error::FormatError;
use crate::water::{
    AreaWater, ChannelPattern, DeltaForm, Doline, LakeForm, LakeOrigin, PanKind, PanRun,
    SegmentForm,
};

/// Leading bytes of every water-forms layer.
pub const WATER_MAGIC: &[u8; 8] = b"ARDAWTR\0";
/// Layout version written for recipe-6 worlds.
pub const WATER_VERSION: u32 = 1;
/// Layout version with saline flags and playa runs (recipe 7).
pub const WATER_VERSION_ARID: u32 = 2;
const HEADER: usize = 8 + 4 + 16;
const HEADER_V2: usize = HEADER + 4;
const SEGMENT: usize = 19;
const LAKE: usize = 2;
const LAKE_V2: usize = 3;
const DELTA: usize = 12;
const DOLINE: usize = 8;
const PAN: usize = 7;

/// Encodes one area's water forms in layout version 1 (recipe 6). Saline
/// flags and pans are not stored.
#[must_use]
pub fn encode_water(w: &AreaWater) -> Vec<u8> {
    encode(w, WATER_VERSION)
}

/// Encodes one area's water forms in layout version 2 (recipe 7).
#[must_use]
pub fn encode_water_v2(w: &AreaWater) -> Vec<u8> {
    encode(w, WATER_VERSION_ARID)
}

fn encode(w: &AreaWater, version: u32) -> Vec<u8> {
    let v2 = version >= WATER_VERSION_ARID;
    let mut out = Vec::with_capacity(
        HEADER_V2
            + w.segments.len() * SEGMENT
            + w.lakes.len() * LAKE_V2
            + w.deltas.len() * DELTA
            + w.dolines.len() * DOLINE
            + w.pans.len() * PAN,
    );
    out.extend(WATER_MAGIC);
    put_u32(&mut out, version);
    let mut counts = vec![
        w.segments.len(),
        w.lakes.len(),
        w.deltas.len(),
        w.dolines.len(),
    ];
    if v2 {
        counts.push(w.pans.len());
    }
    for n in counts {
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
        if v2 {
            out.push(u8::from(l.saline));
        }
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
    if v2 {
        for p in &w.pans {
            put_u16(&mut out, p.y);
            put_u16(&mut out, p.x0);
            put_u16(&mut out, p.len);
            out.push(p.kind as u8);
        }
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
    if version != WATER_VERSION && version != WATER_VERSION_ARID {
        return Err(FormatError::UnknownDiscriminant {
            path: path.to_owned(),
            field: "water version",
            value: u16::try_from(version).unwrap_or(u16::MAX),
        });
    }
    let v2 = version == WATER_VERSION_ARID;
    let (header, lake_row) = if v2 {
        (HEADER_V2, LAKE_V2)
    } else {
        (HEADER, LAKE)
    };
    if bytes.len() < header {
        return Err(eof(header));
    }
    let mut counts = [0_usize; 5];
    for c in counts.iter_mut().take(if v2 { 5 } else { 4 }) {
        *c = take_u32(bytes, &mut at) as usize;
    }
    let expected = counts[0]
        .checked_mul(SEGMENT)
        .and_then(|v| v.checked_add(counts[1].checked_mul(lake_row)?))
        .and_then(|v| v.checked_add(counts[2].checked_mul(DELTA)?))
        .and_then(|v| v.checked_add(counts[3].checked_mul(DOLINE)?))
        .and_then(|v| v.checked_add(counts[4].checked_mul(PAN)?))
        .and_then(|v| v.checked_add(header))
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
        // Version 1 stores no saline flag: every terminal lake is saline.
        let s = if v2 { take_u8(bytes, &mut at) } else { t };
        if s > 1 {
            return Err(unknown("lake saline flag", s));
        }
        w.lakes.push(LakeForm {
            origin: LakeOrigin::from_u8(o).ok_or_else(|| unknown("lake origin", o))?,
            terminal: t == 1,
            saline: s == 1,
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
    for _ in 0..counts[4] {
        let (y, x0, len) = (
            take_u16(bytes, &mut at),
            take_u16(bytes, &mut at),
            take_u16(bytes, &mut at),
        );
        let k = take_u8(bytes, &mut at);
        if len == 0 || u32::from(x0) + u32::from(len) > u32::from(crate::AREA_CELLS) {
            return Err(unknown(
                "pan run",
                u8::try_from(len.min(255)).unwrap_or(255),
            ));
        }
        cell(x0, y)?;
        w.pans.push(PanRun {
            y,
            x0,
            len,
            kind: PanKind::from_u8(k).ok_or_else(|| unknown("pan kind", k))?,
        });
    }
    if w.pans
        .windows(2)
        .any(|p| (p[0].y, p[0].x0) >= (p[1].y, p[1].x0))
    {
        return Err(unknown("pan run order", 0));
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
                saline: false,
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
            pans: Vec::new(),
        }
    }

    fn arid() -> AreaWater {
        let mut w = sample();
        w.lakes.push(LakeForm {
            origin: LakeOrigin::AridTerminal,
            terminal: true,
            saline: true,
        });
        w.pans = vec![
            PanRun {
                y: 40,
                x0: 10,
                len: 12,
                kind: PanKind::SaltCrust,
            },
            PanRun {
                y: 40,
                x0: 22,
                len: 3,
                kind: PanKind::Mudflat,
            },
            PanRun {
                y: 41,
                x0: 0,
                len: 512,
                kind: PanKind::Mudflat,
            },
        ];
        w
    }

    #[test]
    fn arid_layout_round_trips_saline_lakes_and_pans() {
        let w = arid();
        let bytes = encode_water_v2(&w);
        assert_eq!(
            bytes.len(),
            HEADER_V2 + 2 * SEGMENT + 2 * LAKE_V2 + DELTA + DOLINE + 3 * PAN
        );
        let back = decode_water(&bytes, "w").unwrap();
        assert_eq!(back, w);
        assert_eq!(back.pan_at(10, 40), Some(PanKind::SaltCrust));
        assert_eq!(back.pan_at(22, 40), Some(PanKind::Mudflat));
        assert_eq!(back.pan_at(25, 40), None);
        assert_eq!(back.pan_at(9, 40), None);
        assert_eq!(back.pan_at(511, 41), Some(PanKind::Mudflat));
        // Version 1 drops pans and reads saline from terminal.
        let v1 = decode_water(&encode_water(&w), "w").unwrap();
        assert!(v1.pans.is_empty());
        assert!(v1.lakes[1].saline && !v1.lakes[0].saline);
        let mut bad = bytes.clone();
        let last = bad.len() - 1;
        bad[last] = 7;
        assert!(decode_water(&bad, "w").is_err(), "unknown pan kind");
        let mut unordered = w.clone();
        unordered.pans.swap(0, 2);
        assert!(decode_water(&encode_water_v2(&unordered), "w").is_err());
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
