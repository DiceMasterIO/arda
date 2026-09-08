//! Area format-4 API and the retained continent object codec.
//! Unknown continent sections retain their length-bounded additive contract.
pub use super::area_objects_v4::{
    decode_objects, decode_objects_with_limits, encode_objects, encode_objects_with_limits,
    ObjectsFormatError, ObjectsLimits, OBJECTS_MAGIC,
};
use super::{put_u16, put_u32, put_u64, take_u16, take_u32, take_u64};
use crate::continent::{ContinentObjects, ContinentRiver};
use crate::coords::KmCoord;
use crate::error::FormatError;
use crate::fixed::DischargeMilli;

fn need(path: &str, src: &[u8], at: usize, extra: usize) -> Result<(), FormatError> {
    // Mirrors the `need` closure in `overview.rs`: `at.checked_add(extra)`
    // so a crafted section length can't wrap `usize` past the EOF guard;
    // `None` just means "past EOF", reported with the honest saturated
    // requirement rather than a fabricated sentinel.
    match at.checked_add(extra) {
        Some(end) if end <= src.len() => Ok(()),
        Some(end) => Err(FormatError::UnexpectedEof {
            path: path.to_owned(),
            read: src.len(),
            expected: end,
        }),
        None => Err(FormatError::UnexpectedEof {
            path: path.to_owned(),
            read: src.len(),
            expected: at.saturating_add(extra),
        }),
    }
}

/// Container magic for `continent/objects.bin`.
pub const CONTINENT_OBJECTS_MAGIC: &[u8; 8] = b"ARDACOB\0";

const KIND_CONTINENT_RIVERS: u16 = 1;

fn put_km_course(out: &mut Vec<u8>, course: &[KmCoord]) {
    put_u32(out, u32::try_from(course.len()).unwrap_or(u32::MAX));
    for c in course {
        put_u16(out, c.x);
        put_u16(out, c.y);
    }
}

fn take_km_course(path: &str, src: &[u8], at: &mut usize) -> Result<Vec<KmCoord>, FormatError> {
    need(path, src, *at, 4)?;
    let n = take_u32(src, at) as usize;
    // `n` comes from a `u32` field, so `n * 4` cannot itself overflow a
    // 64-bit `usize`; `saturating_mul` still reports the genuine
    // requirement rather than a fabricated sentinel if that ever changes.
    let byte_len = n.checked_mul(4).ok_or_else(|| FormatError::UnexpectedEof {
        path: path.to_owned(),
        read: src.len(),
        expected: n.saturating_mul(4),
    })?;
    need(path, src, *at, byte_len)?;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let x = take_u16(src, at);
        let y = take_u16(src, at);
        out.push(KmCoord::new(x, y));
    }
    Ok(out)
}

/// Encodes the continent object lists.
#[must_use]
pub fn encode_continent_objects(objects: &ContinentObjects) -> Vec<u8> {
    let mut rivers = Vec::new();
    for r in &objects.rivers {
        put_u16(&mut rivers, r.id);
        put_u32(&mut rivers, r.catchment_km2);
        put_u64(&mut rivers, r.discharge.raw());
        put_u16(&mut rivers, r.feeds.unwrap_or(0));
        put_km_course(&mut rivers, &r.course);
    }
    let mut out = Vec::with_capacity(CONTINENT_OBJECTS_MAGIC.len() + 12 + rivers.len());
    out.extend_from_slice(CONTINENT_OBJECTS_MAGIC);
    put_u16(&mut out, 1);
    put_u16(&mut out, KIND_CONTINENT_RIVERS);
    put_u32(
        &mut out,
        u32::try_from(objects.rivers.len()).unwrap_or(u32::MAX),
    );
    put_u32(&mut out, u32::try_from(rivers.len()).unwrap_or(u32::MAX));
    out.extend_from_slice(&rivers);
    out
}

/// Decodes the continent object lists.
///
/// # Errors
/// Same classes as [`decode_objects`], against the continent magic.
pub fn decode_continent_objects(path: &str, bytes: &[u8]) -> Result<ContinentObjects, FormatError> {
    let magic_len = CONTINENT_OBJECTS_MAGIC.len();
    if bytes.len() < magic_len || &bytes[..magic_len] != CONTINENT_OBJECTS_MAGIC {
        return Err(FormatError::BadMagic {
            path: path.to_owned(),
            layer: "continent objects",
        });
    }
    let mut at = magic_len;
    need(path, bytes, at, 2)?;
    let sections = take_u16(bytes, &mut at);
    let mut out = ContinentObjects::empty();
    for _ in 0..sections {
        need(path, bytes, at, 10)?;
        let kind = take_u16(bytes, &mut at);
        let record_ct = take_u32(bytes, &mut at) as usize;
        let byte_len = take_u32(bytes, &mut at) as usize;
        need(path, bytes, at, byte_len)?;
        let end = at + byte_len;
        if kind == KIND_CONTINENT_RIVERS {
            let section = &bytes[at..end];
            let mut local = 0;
            for _ in 0..record_ct {
                need(path, section, local, 16)?;
                let id = take_u16(section, &mut local);
                let catchment_km2 = take_u32(section, &mut local);
                let discharge = DischargeMilli::new(take_u64(section, &mut local));
                let feeds_raw = take_u16(section, &mut local);
                let course = take_km_course(path, section, &mut local)?;
                out.rivers.push(ContinentRiver {
                    id,
                    catchment_km2,
                    discharge,
                    feeds: (feeds_raw != 0).then_some(feeds_raw),
                    course,
                });
            }
        }
        at = end;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Continent objects tests
    fn km(x: u16, y: u16) -> KmCoord {
        KmCoord::new(x, y)
    }

    fn continent_sample() -> ContinentObjects {
        ContinentObjects {
            rivers: vec![
                ContinentRiver {
                    id: 1,
                    catchment_km2: 9_385,
                    discharge: DischargeMilli::new(38_000_000),
                    feeds: None,
                    course: vec![km(10, 3), km(10, 4), km(11, 5)],
                },
                ContinentRiver {
                    id: 2,
                    catchment_km2: 1_020,
                    discharge: DischargeMilli::new(4_100_000),
                    feeds: Some(1),
                    course: vec![km(9, 4), km(10, 4)],
                },
            ],
        }
    }

    #[test]
    fn continent_objects_round_trip() {
        let bytes = encode_continent_objects(&continent_sample());
        assert_eq!(
            decode_continent_objects("continent/objects.bin", &bytes).unwrap(),
            continent_sample()
        );
    }

    #[test]
    fn empty_continent_objects_round_trip() {
        let bytes = encode_continent_objects(&ContinentObjects::empty());
        assert_eq!(
            decode_continent_objects("continent/objects.bin", &bytes).unwrap(),
            ContinentObjects::empty()
        );
    }

    #[test]
    fn continent_magic_differs_from_area_magic() {
        // A continent objects file handed to the area decoder must be
        // refused, and vice versa — the magics are the diagnostic.
        let bytes = encode_continent_objects(&continent_sample());
        assert!(matches!(
            decode_objects("x", &bytes).unwrap_err(),
            FormatError::BadMagic { .. }
        ));
    }

    #[test]
    fn unknown_continent_section_kind_is_skipped() {
        // Additive evolution within a major (`02-models.md` Schema).
        let mut bytes = encode_continent_objects(&continent_sample());
        let at = CONTINENT_OBJECTS_MAGIC.len();
        let count = u16::from_le_bytes([bytes[at], bytes[at + 1]]);
        bytes[at..at + 2].copy_from_slice(&(count + 1).to_le_bytes());
        bytes.extend_from_slice(&777u16.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&[1, 2, 3, 4]);
        assert_eq!(
            decode_continent_objects("continent/objects.bin", &bytes).unwrap(),
            continent_sample()
        );
    }

    #[test]
    fn continent_record_cannot_consume_following_section_header() {
        let mut bytes = CONTINENT_OBJECTS_MAGIC.to_vec();
        put_u16(&mut bytes, 2);
        put_u16(&mut bytes, KIND_CONTINENT_RIVERS);
        put_u32(&mut bytes, 1);
        put_u32(&mut bytes, 16); // Prefix only: course count is absent.
        put_u16(&mut bytes, 1);
        put_u32(&mut bytes, 100);
        put_u64(&mut bytes, 40);
        put_u16(&mut bytes, 0);
        // A whole-file parser used to borrow these zeros as an empty course.
        put_u16(&mut bytes, 0);
        put_u32(&mut bytes, 0);
        put_u32(&mut bytes, 0);
        assert!(matches!(
            decode_continent_objects("continent.bin", &bytes),
            Err(FormatError::UnexpectedEof { .. })
        ));
    }
    #[test]
    fn continent_wide_discharge_keeps_existing_api_contract() {
        let mut o = continent_sample();
        o.rivers[0].discharge = DischargeMilli::new(33_249_619_483);
        assert_eq!(
            decode_continent_objects("continent.bin", &encode_continent_objects(&o)).unwrap(),
            o
        );
    }
}
