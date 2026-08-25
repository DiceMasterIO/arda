//! `areas/<ax>_<ay>/objects.bin` — a tagged section container.
//!
//! Unknown section kinds are skipped so later stages can add record types
//! without a format major bump (`02-models.md` Schema: additive within a
//! major).

use super::{put_i32, put_u16, put_u32, take_i32, take_u16, take_u32, take_u8};
use crate::coords::CellCoord;
use crate::error::FormatError;
use crate::fixed::{DischargeMilli, HeightMm};
use crate::objects::{AreaObjects, Lake, RiverSegment, Terminus};

/// Container magic.
pub const OBJECTS_MAGIC: &[u8; 8] = b"ARDAOBJ\0";

const KIND_RIVERS: u16 = 1;
const KIND_LAKES: u16 = 2;

fn need(path: &str, src: &[u8], at: usize, extra: usize) -> Result<(), FormatError> {
    if at + extra > src.len() {
        Err(FormatError::UnexpectedEof {
            path: path.to_owned(),
            read: src.len(),
            expected: at + extra,
        })
    } else {
        Ok(())
    }
}

fn put_course(out: &mut Vec<u8>, course: &[CellCoord]) {
    put_u32(out, u32::try_from(course.len()).unwrap_or(u32::MAX));
    for c in course {
        put_u16(out, c.x());
        put_u16(out, c.y());
    }
}

fn take_course(path: &str, src: &[u8], at: &mut usize) -> Result<Vec<CellCoord>, FormatError> {
    need(path, src, *at, 4)?;
    let n = take_u32(src, at) as usize;
    need(path, src, *at, n * 4)?;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let x = take_u16(src, at);
        let y = take_u16(src, at);
        let coord = CellCoord::new(x, y).ok_or(FormatError::UnknownDiscriminant {
            path: path.to_owned(),
            field: "cell coordinate",
            value: x.max(y),
        })?;
        out.push(coord);
    }
    Ok(out)
}

/// Encodes an area's object lists.
#[must_use]
pub fn encode_objects(objects: &AreaObjects) -> Vec<u8> {
    let mut rivers = Vec::new();
    for r in &objects.rivers {
        put_u16(&mut rivers, r.id);
        rivers.push(r.order);
        put_u16(&mut rivers, r.width_dm);
        put_u32(&mut rivers, r.discharge.raw());
        put_u16(&mut rivers, r.feeds.unwrap_or(0));
        rivers.push(r.ends as u8);
        put_course(&mut rivers, &r.course);
    }

    let mut lakes = Vec::new();
    for l in &objects.lakes {
        put_u16(&mut lakes, l.id);
        put_i32(&mut lakes, l.surface.raw());
        put_u32(&mut lakes, l.depth_mm);
        match l.outlet {
            Some(c) => {
                lakes.push(1);
                put_u16(&mut lakes, c.x());
                put_u16(&mut lakes, c.y());
            }
            None => {
                lakes.push(0);
                put_u16(&mut lakes, 0);
                put_u16(&mut lakes, 0);
            }
        }
        put_course(&mut lakes, &l.cells);
    }

    let mut out = Vec::with_capacity(OBJECTS_MAGIC.len() + 22 + rivers.len() + lakes.len());
    out.extend_from_slice(OBJECTS_MAGIC);
    put_u16(&mut out, 2);

    put_u16(&mut out, KIND_RIVERS);
    put_u32(
        &mut out,
        u32::try_from(objects.rivers.len()).unwrap_or(u32::MAX),
    );
    put_u32(&mut out, u32::try_from(rivers.len()).unwrap_or(u32::MAX));
    out.extend_from_slice(&rivers);

    put_u16(&mut out, KIND_LAKES);
    put_u32(
        &mut out,
        u32::try_from(objects.lakes.len()).unwrap_or(u32::MAX),
    );
    put_u32(&mut out, u32::try_from(lakes.len()).unwrap_or(u32::MAX));
    out.extend_from_slice(&lakes);

    out
}

/// Decodes an area's object lists.
///
/// # Errors
/// - [`FormatError::BadMagic`] when the file is not an objects layer.
/// - [`FormatError::UnexpectedEof`] when a section runs past the end.
pub fn decode_objects(path: &str, bytes: &[u8]) -> Result<AreaObjects, FormatError> {
    if bytes.len() < OBJECTS_MAGIC.len() || &bytes[..OBJECTS_MAGIC.len()] != OBJECTS_MAGIC {
        return Err(FormatError::BadMagic {
            path: path.to_owned(),
            layer: "objects",
        });
    }

    let mut at = OBJECTS_MAGIC.len();
    need(path, bytes, at, 2)?;
    let sections = take_u16(bytes, &mut at);

    let mut out = AreaObjects::empty();
    for _ in 0..sections {
        need(path, bytes, at, 10)?;
        let kind = take_u16(bytes, &mut at);
        let record_ct = take_u32(bytes, &mut at) as usize;
        let byte_len = take_u32(bytes, &mut at) as usize;
        need(path, bytes, at, byte_len)?;
        let end = at + byte_len;

        match kind {
            KIND_RIVERS => {
                for _ in 0..record_ct {
                    need(path, bytes, at, 12)?;
                    let id = take_u16(bytes, &mut at);
                    let order = take_u8(bytes, &mut at);
                    let width_dm = take_u16(bytes, &mut at);
                    let discharge = DischargeMilli::new(take_u32(bytes, &mut at));
                    let feeds_raw = take_u16(bytes, &mut at);
                    let ends_raw = take_u8(bytes, &mut at);
                    let ends =
                        Terminus::from_u8(ends_raw).ok_or(FormatError::UnknownDiscriminant {
                            path: path.to_owned(),
                            field: "terminus",
                            value: u16::from(ends_raw),
                        })?;
                    let course = take_course(path, bytes, &mut at)?;
                    out.rivers.push(RiverSegment {
                        id,
                        order,
                        width_dm,
                        discharge,
                        feeds: (feeds_raw != 0).then_some(feeds_raw),
                        ends,
                        course,
                    });
                }
            }
            KIND_LAKES => {
                for _ in 0..record_ct {
                    need(path, bytes, at, 15)?;
                    let id = take_u16(bytes, &mut at);
                    let surface = HeightMm::new(take_i32(bytes, &mut at));
                    let depth_mm = take_u32(bytes, &mut at);
                    let has_outlet = take_u8(bytes, &mut at) == 1;
                    let ox = take_u16(bytes, &mut at);
                    let oy = take_u16(bytes, &mut at);
                    let outlet = if has_outlet {
                        Some(
                            CellCoord::new(ox, oy).ok_or(FormatError::UnknownDiscriminant {
                                path: path.to_owned(),
                                field: "lake outlet",
                                value: ox.max(oy),
                            })?,
                        )
                    } else {
                        None
                    };
                    let cells = take_course(path, bytes, &mut at)?;
                    out.lakes.push(Lake {
                        id,
                        surface,
                        depth_mm,
                        outlet,
                        cells,
                    });
                }
            }
            // Forward compatibility: a section a later build wrote and this
            // one does not know (`02-models.md` additive evolution).
            _ => {}
        }
        at = end;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cc(x: u16, y: u16) -> CellCoord {
        CellCoord::new(x, y).unwrap()
    }

    fn sample() -> AreaObjects {
        AreaObjects {
            rivers: vec![
                RiverSegment {
                    id: 1,
                    order: 3,
                    width_dm: 240,
                    discharge: DischargeMilli::new(88_000),
                    feeds: None,
                    ends: Terminus::Sea,
                    course: vec![cc(0, 0), cc(1, 0), cc(1, 1), cc(511, 511)],
                },
                RiverSegment {
                    id: 2,
                    order: 1,
                    width_dm: 15,
                    discharge: DischargeMilli::new(400),
                    feeds: Some(1),
                    ends: Terminus::Junction,
                    course: vec![cc(100, 200)],
                },
            ],
            lakes: vec![Lake {
                id: 1,
                surface: HeightMm::new(214_000),
                depth_mm: 4_200,
                outlet: Some(cc(52, 51)),
                cells: vec![cc(50, 50), cc(51, 50), cc(50, 51)],
            }],
        }
    }

    #[test]
    fn objects_round_trip() {
        let bytes = encode_objects(&sample());
        assert_eq!(decode_objects("objects.bin", &bytes).unwrap(), sample());
    }

    #[test]
    fn empty_objects_round_trip() {
        let bytes = encode_objects(&AreaObjects::empty());
        assert_eq!(
            decode_objects("objects.bin", &bytes).unwrap(),
            AreaObjects::empty()
        );
    }

    #[test]
    fn encoding_is_stable_across_calls() {
        assert_eq!(encode_objects(&sample()), encode_objects(&sample()));
    }

    #[test]
    fn bad_magic_is_refused_naming_the_layer() {
        let mut bytes = encode_objects(&sample());
        bytes[0] = b'X';
        let err = decode_objects("areas/00_00/objects.bin", &bytes).unwrap_err();
        match err {
            FormatError::BadMagic { path, layer } => {
                assert_eq!(path, "areas/00_00/objects.bin");
                assert_eq!(layer, "objects");
            }
            other => panic!("expected BadMagic, got {other:?}"),
        }
    }

    #[test]
    fn truncated_container_is_refused() {
        let bytes = encode_objects(&sample());
        let err = decode_objects("objects.bin", &bytes[..bytes.len() - 3]).unwrap_err();
        assert!(matches!(err, FormatError::UnexpectedEof { .. }));
    }

    /// Forward compatibility: a section this build does not know is skipped,
    /// so build-order step 5 can add settlements without a format major bump.
    #[test]
    fn segment_links_and_termini_round_trip() {
        // Artifact, Water: "every segment knows which segment it feeds and
        // how it ends"; lakes record surface, depth, and their outlet cell.
        let back = decode_objects("objects.bin", &encode_objects(&sample())).unwrap();
        assert_eq!(back.rivers[0].ends, Terminus::Sea);
        assert_eq!(back.rivers[0].feeds, None);
        assert_eq!(back.rivers[1].ends, Terminus::Junction);
        assert_eq!(back.rivers[1].feeds, Some(1));
        assert_eq!(back.lakes[0].depth_mm, 4_200);
        assert_eq!(back.lakes[0].outlet, Some(cc(52, 51)));
    }

    #[test]
    fn a_lake_spilling_off_tile_has_no_outlet_cell() {
        let mut o = sample();
        o.lakes[0].outlet = None;
        let back = decode_objects("objects.bin", &encode_objects(&o)).unwrap();
        assert_eq!(back.lakes[0].outlet, None);
    }

    #[test]
    fn unknown_section_kind_is_skipped() {
        let mut bytes = encode_objects(&sample());
        let count = u16::from_le_bytes([bytes[8], bytes[9]]);
        bytes[8..10].copy_from_slice(&(count + 1).to_le_bytes());
        bytes.extend_from_slice(&999u16.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);

        assert_eq!(decode_objects("objects.bin", &bytes).unwrap(), sample());
    }
}
