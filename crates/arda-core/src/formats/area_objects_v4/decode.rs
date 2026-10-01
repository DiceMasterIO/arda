//! Decoding the format-4 area object container: sections, records and
//! copied authority, under hard resource limits.

use super::*;
use crate::coords::CellCoord;
use crate::error::FormatError;
use crate::fixed::{DischargeMilli, HeightMm};
use crate::formats::hydrology::{decode_area_context, decode_record, FixedRecord};
use crate::hydrology::{BasinId, ChannelEdge, ReachId};
use crate::objects::{AreaObjects, Lake, RiverSegment, Terminus};

/// Decode a format-4 area container; error paths retain the requested file identity.
///
/// # Errors
/// Rejects wrong magic, missing/duplicate sections, truncated section-local records,
/// malformed copied authority, unsupported tags, and hard resource-limit violations.
pub fn decode_objects(path: &str, bytes: &[u8]) -> std::result::Result<AreaObjects, FormatError> {
    decode_objects_with_limits(path, bytes, ObjectsLimits::default())
}
/// Decode with explicit bounds before allocating record or course arrays.
///
/// # Errors
/// Returns the same path-carrying failures as [`decode_objects`].
pub fn decode_objects_with_limits(
    path: &str,
    bytes: &[u8],
    limits: ObjectsLimits,
) -> std::result::Result<AreaObjects, FormatError> {
    decode_inner(bytes, limits).map_err(|source| match source {
        ObjectsFormatError::BadMagic => FormatError::BadMagic {
            path: path.to_owned(),
            layer: "objects",
        },
        source => FormatError::Objects {
            path: path.to_owned(),
            source,
        },
    })
}
pub(super) fn decode_inner(bytes: &[u8], limits: ObjectsLimits) -> Result<AreaObjects> {
    if bytes.len() > limits.max_bytes {
        return Err(ObjectsFormatError::Limit("object file byte limit"));
    }
    if bytes.get(..8) != Some(OBJECTS_MAGIC.as_slice()) {
        return Err(ObjectsFormatError::BadMagic);
    }
    let mut r = Reader::new(bytes, 0);
    r.take(8)?;
    let sections = r.u16()?;
    if sections > limits.max_sections {
        return Err(ObjectsFormatError::Limit("object section count"));
    }
    let mut o = AreaObjects::empty();
    let mut seen = 0_u8;
    let mut previous = 0_u16;
    let mut courses = 0_u32;
    let mut memberships = 0_u32;
    for _ in 0..sections {
        let kind = r.u16()?;
        let records = r.u32()?;
        let length = usize::try_from(r.u32()?).map_err(|_| ObjectsFormatError::Overflow)?;
        if kind <= previous {
            return Err(ObjectsFormatError::SectionOrder);
        }
        previous = kind;
        let base = r.at;
        let section = r.take(length)?;
        let mut s = Reader::new(section, base);
        let minimum = match kind {
            RIVERS => {
                if records > limits.max_rivers {
                    return Err(ObjectsFormatError::Limit("river record count"));
                }
                RIVER_PREFIX
            }
            LAKES => {
                if records > limits.max_lakes {
                    return Err(ObjectsFormatError::Limit("lake record count"));
                }
                LAKE_PREFIX
            }
            CHANNELS => {
                if records > limits.max_channel_edges {
                    return Err(ObjectsFormatError::Limit("channel edge count"));
                }
                ChannelEdge::WIDTH
            }
            GLOBAL => {
                require(
                    records == 1,
                    "global context section must have exactly one record",
                )?;
                0
            }
            _ => 0,
        };
        let n = usize::try_from(records).map_err(|_| ObjectsFormatError::Overflow)?;
        let expected = minimum.checked_mul(n).ok_or(ObjectsFormatError::Overflow)?;
        if expected > length {
            return Err(ObjectsFormatError::Truncated {
                offset: base,
                needed: expected,
                available: length,
            });
        }
        match kind {
            RIVERS => {
                seen |= 1;
                o.rivers = Vec::with_capacity(n);
                for _ in 0..records {
                    let global_id = ReachId(s.u64()?);
                    let id = s.u32()?;
                    let order = s.u8()?;
                    let width_dm = s.u32()?;
                    let discharge = DischargeMilli::new(s.u64()?);
                    let feeds = s.u32()?;
                    let tag = s.u8()?;
                    let ends = Terminus::from_u8(tag).ok_or(ObjectsFormatError::UnknownTag {
                        field: "river terminus",
                        value: tag,
                    })?;
                    let course = s.course(&mut courses, limits.max_course_cells)?;
                    o.rivers.push(RiverSegment {
                        global_id,
                        id,
                        order,
                        width_dm,
                        discharge,
                        feeds: (feeds != 0).then_some(feeds),
                        ends,
                        course,
                    });
                }
                s.finish()?;
            }
            LAKES => {
                seen |= 2;
                o.lakes = Vec::with_capacity(n);
                for _ in 0..records {
                    let global_id = BasinId(s.u64()?);
                    let id = s.u32()?;
                    let surface = HeightMm::new(s.i32()?);
                    let depth_mm = s.u32()?;
                    let tag = s.u8()?;
                    let ox = s.u16()?;
                    let oy = s.u16()?;
                    let outlet = match tag {
                        0 => {
                            require(
                                ox == 0 && oy == 0,
                                "absent lake outlet has nonzero coordinates",
                            )?;
                            None
                        }
                        1 => Some(
                            CellCoord::new(ox, oy)
                                .ok_or(ObjectsFormatError::Invalid("lake outlet leaves area"))?,
                        ),
                        _ => {
                            return Err(ObjectsFormatError::UnknownTag {
                                field: "lake outlet",
                                value: tag,
                            })
                        }
                    };
                    let cells = s.course(&mut memberships, limits.max_lake_cells)?;
                    o.lakes.push(Lake {
                        global_id,
                        id,
                        surface,
                        depth_mm,
                        outlet,
                        cells,
                    });
                }
                s.finish()?;
            }
            CHANNELS => {
                seen |= 4;
                require(expected == length, "channel section count/length mismatch")?;
                o.channel_edges = Vec::with_capacity(n);
                for _ in 0..records {
                    o.channel_edges
                        .push(decode_record(s.take(ChannelEdge::WIDTH)?)?);
                }
                s.finish()?;
            }
            GLOBAL => {
                seen |= 8;
                o.global = decode_area_context(section, limits.context)?;
            }
            _ => {} // Additive unknown sections are bounded by their exact declared slice.
        }
    }
    r.finish()?;
    for (mask, kind) in [(1, RIVERS), (2, LAKES), (4, CHANNELS), (8, GLOBAL)] {
        if seen & mask == 0 {
            return Err(ObjectsFormatError::MissingSection(kind));
        }
    }
    list_counts(&o, limits)?;
    validate_local(&o)?;
    Ok(o)
}
