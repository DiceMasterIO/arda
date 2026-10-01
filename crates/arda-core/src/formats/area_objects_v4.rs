//! Checked format-4 area object sections. The continent object codec is separate.
#![deny(missing_docs)]
use crate::coords::CellCoord;
use crate::formats::hydrology::{
    encode_area_context_with_limits, encode_record, ContextLimits, FixedRecord,
    HydrologyFormatError,
};
use crate::hydrology::ChannelEdge;
use crate::objects::AreaObjects;

mod decode;
mod validate;
pub use decode::{decode_objects, decode_objects_with_limits};
use validate::validate_local;

/// Area object-container magic; the enclosing manifest enforces format major 4.
pub const OBJECTS_MAGIC: &[u8; 8] = b"ARDAOBJ\0";
const RIVERS: u16 = 1;
const LAKES: u16 = 2;
const CHANNELS: u16 = 3;
const GLOBAL: u16 = 4;
const HEADER_BYTES: usize = 10;
const SECTION_BYTES: usize = 10;
const RIVER_PREFIX: usize = 34;
const LAKE_PREFIX: usize = 29;
const CELLS: usize = 512 * 512;

/// Malformed local objects, a bounded-container failure, or invalid copied hydrology.
#[derive(Debug)]
pub enum ObjectsFormatError {
    /// Container magic is not an area objects layer.
    BadMagic,
    /// A section or record requires bytes outside its own bounded slice.
    Truncated {
        /// Absolute byte offset requested.
        offset: usize,
        /// Requested bytes at this offset.
        needed: usize,
        /// Remaining bytes in the enclosing section.
        available: usize,
    },
    /// A known section or full container was not consumed exactly.
    TrailingBytes,
    /// A known required section was absent.
    MissingSection(u16),
    /// Section tags were duplicated or not in canonical increasing order.
    SectionOrder,
    /// A boolean or terminus discriminant is unknown.
    UnknownTag {
        /// Name of the tagged field.
        field: &'static str,
        /// Invalid stored discriminant.
        value: u8,
    },
    /// A local identity, geometry, or copied-record invariant failed.
    Invalid(&'static str),
    /// Declared counts or actual encoded size exceed a hard limit.
    Limit(&'static str),
    /// Checked length/count arithmetic overflowed.
    Overflow,
    /// The shared global-record codec rejected a record or context.
    Hydrology(HydrologyFormatError),
}
impl From<HydrologyFormatError> for ObjectsFormatError {
    fn from(e: HydrologyFormatError) -> Self {
        Self::Hydrology(e)
    }
}
impl std::fmt::Display for ObjectsFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadMagic => f.write_str("bad area objects magic"),
            Self::Truncated {
                offset,
                needed,
                available,
            } => write!(
                f,
                "section truncated at {offset}: needs {needed} bytes, has {available}"
            ),
            Self::TrailingBytes => {
                f.write_str("trailing bytes in area objects section or container")
            }
            Self::MissingSection(k) => write!(f, "required area object section {k} is missing"),
            Self::SectionOrder => {
                f.write_str("area object section tags are duplicated or unordered")
            }
            Self::UnknownTag { field, value } => write!(f, "unknown {field} tag {value}"),
            Self::Invalid(s) => write!(f, "invalid area objects: {s}"),
            Self::Limit(s) => write!(f, "area object limit: {s}"),
            Self::Overflow => f.write_str("area object size arithmetic overflow"),
            Self::Hydrology(e) => std::fmt::Display::fmt(e, f),
        }
    }
}
impl std::error::Error for ObjectsFormatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Hydrology(e) => Some(e),
            _ => None,
        }
    }
}
type Result<T> = std::result::Result<T, ObjectsFormatError>;
fn require(ok: bool, reason: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(ObjectsFormatError::Invalid(reason))
    }
}

/// Hard limits apply before count-driven allocation, on both encode and decode.
#[derive(Debug, Clone, Copy)]
pub struct ObjectsLimits {
    /// Maximum bytes in the entire container, including unknown sections.
    pub max_bytes: usize,
    /// Maximum section count, including unknown future sections.
    pub max_sections: u16,
    /// Maximum local river-fragment count.
    pub max_rivers: u32,
    /// Maximum local lake-fragment count.
    pub max_lakes: u32,
    /// Maximum saved halo-edge count; matches the renderer's declared bound.
    pub max_channel_edges: u32,
    /// Maximum sum of river-course entries, allowing shared connection endpoints.
    pub max_course_cells: u32,
    /// Maximum sum of lake membership entries; lake memberships remain disjoint.
    pub max_lake_cells: u32,
    /// Independent limits for copied global hydrology records.
    pub context: ContextLimits,
}
impl Default for ObjectsLimits {
    fn default() -> Self {
        Self {
            max_bytes: 128 * 1024 * 1024,
            max_sections: 64,
            max_rivers: 262144,
            max_lakes: 262144,
            max_channel_edges: 262144,
            max_course_cells: 1048576,
            max_lake_cells: 262144,
            context: ContextLimits::default(),
        }
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
    base: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], base: usize) -> Self {
        Self { bytes, at: 0, base }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(n).ok_or(ObjectsFormatError::Overflow)?;
        let slice = self
            .bytes
            .get(self.at..end)
            .ok_or(ObjectsFormatError::Truncated {
                offset: self.base.saturating_add(self.at),
                needed: n,
                available: self.bytes.len().saturating_sub(self.at),
            })?;
        self.at = end;
        Ok(slice)
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| ObjectsFormatError::Overflow)?,
        ))
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| ObjectsFormatError::Overflow)?,
        ))
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| ObjectsFormatError::Overflow)?,
        ))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| ObjectsFormatError::Overflow)?,
        ))
    }
    fn coord(&mut self) -> Result<CellCoord> {
        let x = self.u16()?;
        let y = self.u16()?;
        CellCoord::new(x, y).ok_or(ObjectsFormatError::Invalid(
            "cell coordinate leaves 512-square area",
        ))
    }
    fn finish(self) -> Result<()> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(ObjectsFormatError::TrailingBytes)
        }
    }
    fn course(&mut self, total: &mut u32, max_total: u32) -> Result<Vec<CellCoord>> {
        let count = self.u32()?;
        if count == 0 || count > 262144 {
            return Err(ObjectsFormatError::Limit(
                "empty or oversized local cell list",
            ));
        }
        *total = total
            .checked_add(count)
            .ok_or(ObjectsFormatError::Overflow)?;
        if *total > max_total {
            return Err(ObjectsFormatError::Limit("aggregate local cell-list count"));
        }
        let n = usize::try_from(count).map_err(|_| ObjectsFormatError::Overflow)?;
        let needed = n.checked_mul(4).ok_or(ObjectsFormatError::Overflow)?;
        let start = self.at;
        let section = self.take(needed)?;
        let mut cells = Reader::new(section, self.base + start);
        let mut out = Vec::with_capacity(n);
        for _ in 0..count {
            out.push(cells.coord()?);
        }
        cells.finish()?;
        Ok(out)
    }
}
fn count(n: usize) -> Result<u32> {
    u32::try_from(n).map_err(|_| ObjectsFormatError::Overflow)
}
fn put_course(out: &mut Vec<u8>, cells: &[CellCoord]) -> Result<()> {
    out.extend(count(cells.len())?.to_le_bytes());
    for c in cells {
        out.extend(c.x().to_le_bytes());
        out.extend(c.y().to_le_bytes());
    }
    Ok(())
}
fn section_len(fixed: usize, records: usize, points: usize) -> Result<usize> {
    fixed
        .checked_mul(records)
        .and_then(|n| points.checked_mul(4).and_then(|p| n.checked_add(p)))
        .ok_or(ObjectsFormatError::Overflow)
}
fn cell_neighbor(a: CellCoord, b: CellCoord) -> bool {
    a != b && a.x().abs_diff(b.x()) <= 1 && a.y().abs_diff(b.y()) <= 1
}
fn list_counts(o: &AreaObjects, limits: ObjectsLimits) -> Result<(usize, usize)> {
    if count(o.rivers.len())? > limits.max_rivers
        || count(o.lakes.len())? > limits.max_lakes
        || count(o.channel_edges.len())? > limits.max_channel_edges
    {
        return Err(ObjectsFormatError::Limit("local object record count"));
    }
    let mut rivers = 0_usize;
    for r in &o.rivers {
        if r.course.is_empty() || r.course.len() > CELLS {
            return Err(ObjectsFormatError::Limit("empty or oversized river course"));
        }
        rivers = rivers
            .checked_add(r.course.len())
            .ok_or(ObjectsFormatError::Overflow)?;
    }
    let mut lakes = 0_usize;
    for l in &o.lakes {
        if l.cells.is_empty() || l.cells.len() > CELLS {
            return Err(ObjectsFormatError::Limit(
                "empty or oversized lake membership",
            ));
        }
        lakes = lakes
            .checked_add(l.cells.len())
            .ok_or(ObjectsFormatError::Overflow)?;
    }
    if count(rivers)? > limits.max_course_cells || count(lakes)? > limits.max_lake_cells {
        return Err(ObjectsFormatError::Limit("aggregate local cell-list count"));
    }
    Ok((rivers, lakes))
}

/// Encode all four required sections, rejecting invalid objects and unrepresentable counts.
///
/// # Errors
/// Returns typed structural, geometry, context, or resource-limit failures.
pub fn encode_objects(objects: &AreaObjects) -> Result<Vec<u8>> {
    encode_objects_with_limits(objects, ObjectsLimits::default())
}
/// Encode using explicitly selected hard limits; no record is dropped to satisfy a limit.
///
/// # Errors
/// Returns the same checked failures as [`encode_objects`].
pub fn encode_objects_with_limits(o: &AreaObjects, limits: ObjectsLimits) -> Result<Vec<u8>> {
    if limits.max_sections < 4 {
        return Err(ObjectsFormatError::Limit(
            "required sections exceed section limit",
        ));
    }
    let (course_cells, lake_cells) = list_counts(o, limits)?;
    let mut context_limits = limits.context;
    context_limits.max_bytes = context_limits
        .max_bytes
        .min(u64::try_from(limits.max_bytes).map_err(|_| ObjectsFormatError::Overflow)?);
    let global = encode_area_context_with_limits(&o.global, context_limits)?;
    validate_local(o)?;
    let sizes = [
        section_len(RIVER_PREFIX, o.rivers.len(), course_cells)?,
        section_len(LAKE_PREFIX, o.lakes.len(), lake_cells)?,
        o.channel_edges
            .len()
            .checked_mul(ChannelEdge::WIDTH)
            .ok_or(ObjectsFormatError::Overflow)?,
        global.len(),
    ];
    let total = sizes
        .iter()
        .try_fold(HEADER_BYTES + 4 * SECTION_BYTES, |n, s| {
            n.checked_add(*s).ok_or(ObjectsFormatError::Overflow)
        })?;
    if total > limits.max_bytes {
        return Err(ObjectsFormatError::Limit("object file byte limit"));
    }
    for s in sizes {
        count(s)?;
    }
    let mut out = Vec::with_capacity(total);
    out.extend(OBJECTS_MAGIC);
    out.extend(4_u16.to_le_bytes());
    let put_header = |out: &mut Vec<u8>, kind: u16, records: usize, size: usize| -> Result<()> {
        out.extend(kind.to_le_bytes());
        out.extend(count(records)?.to_le_bytes());
        out.extend(count(size)?.to_le_bytes());
        Ok(())
    };
    put_header(&mut out, RIVERS, o.rivers.len(), sizes[0])?;
    for r in &o.rivers {
        out.extend(r.global_id.0.to_le_bytes());
        out.extend(r.id.to_le_bytes());
        out.push(r.order);
        out.extend(r.width_dm.to_le_bytes());
        out.extend(r.discharge.raw().to_le_bytes());
        out.extend(r.feeds.unwrap_or(0).to_le_bytes());
        out.push(r.ends as u8);
        put_course(&mut out, &r.course)?;
    }
    put_header(&mut out, LAKES, o.lakes.len(), sizes[1])?;
    for l in &o.lakes {
        out.extend(l.global_id.0.to_le_bytes());
        out.extend(l.id.to_le_bytes());
        out.extend(l.surface.raw().to_le_bytes());
        out.extend(l.depth_mm.to_le_bytes());
        match l.outlet {
            Some(c) => {
                out.push(1);
                out.extend(c.x().to_le_bytes());
                out.extend(c.y().to_le_bytes());
            }
            None => {
                out.extend([0; 5]);
            }
        }
        put_course(&mut out, &l.cells)?;
    }
    put_header(&mut out, CHANNELS, o.channel_edges.len(), sizes[2])?;
    for e in &o.channel_edges {
        out.extend(encode_record(e)?);
    }
    put_header(&mut out, GLOBAL, 1, sizes[3])?;
    out.extend(global);
    require(
        out.len() == total,
        "object output length differs from schema",
    )?;
    Ok(out)
}

#[cfg(test)]
mod tests;
