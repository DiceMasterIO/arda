//! Checked format-4 area object sections. The continent object codec is separate.
#![deny(missing_docs)]
use crate::coords::CellCoord;
use crate::error::FormatError;
use crate::fixed::{DischargeMilli, HeightMm};
use crate::formats::hydrology::{
    decode_area_context, decode_record, encode_area_context_with_limits, encode_record,
    ContextLimits, FixedRecord, HydrologyFormatError,
};
use crate::hydrology::{BasinId, ChannelEdge, ReachId};
use crate::objects::{AreaObjects, Lake, RiverSegment, Terminus};
use std::collections::BTreeMap;

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
fn validate_local(o: &AreaObjects) -> Result<()> {
    for pair in o.rivers.windows(2) {
        require(
            pair[0].id < pair[1].id,
            "river IDs are not unique ascending",
        )?;
    }
    for pair in o.lakes.windows(2) {
        require(pair[0].id < pair[1].id, "lake IDs are not unique ascending")?;
    }
    for pair in o.channel_edges.windows(2) {
        require(
            pair[0].key() < pair[1].key(),
            "channel edges are not unique canonical pairs",
        )?;
    }
    let global_reaches: BTreeMap<_, _> = o.global.reaches.iter().map(|r| (r.id, r)).collect();
    let global_lakes: BTreeMap<_, _> = o.global.lakes.iter().map(|l| (l.basin, l)).collect();
    let local_index: BTreeMap<_, _> = o
        .rivers
        .iter()
        .enumerate()
        .map(|(i, r)| (r.id, i))
        .collect();
    let mut visits = vec![0_usize; CELLS];
    let mut next = vec![None; o.rivers.len()];
    for (i, r) in o.rivers.iter().enumerate() {
        require(
            r.id > 0 && r.order > 0 && r.width_dm > 0 && r.discharge.raw() >= 40,
            "invalid local river fields",
        )?;
        let g = global_reaches
            .get(&r.global_id)
            .ok_or(ObjectsFormatError::Invalid(
                "river lacks copied global reach",
            ))?;
        require(
            r.discharge == g.mean_discharge,
            "local river discharge disagrees with global authority",
        )?;
        require(
            !r.course.is_empty() && r.course.len() <= CELLS,
            "invalid local course length",
        )?;
        for &c in &r.course {
            require(visits[c.index()] != i + 1, "river course repeats a cell")?;
            visits[c.index()] = i + 1;
        }
        for p in r.course.windows(2) {
            require(
                cell_neighbor(p[0], p[1]),
                "river course is not contiguous D8",
            )?;
        }
        if g.id.is_point() {
            require(
                r.course.len() == 1
                    && u32::from(r.course[0].x()) == g.from.x % 512
                    && u32::from(r.course[0].y()) == g.from.y % 512,
                "point course does not own its source cell",
            )?;
            require(
                matches!(
                    (g.receiving, r.ends),
                    (crate::hydrology::ReceivingAccount::Lake(_), Terminus::Basin)
                        | (
                            crate::hydrology::ReceivingAccount::DomainExport,
                            Terminus::OffTile
                        )
                ),
                "point course terminus disagrees with global authority",
            )?;
        }
        if r.ends == Terminus::Basin {
            let crate::hydrology::ReceivingAccount::Lake(basin) = g.receiving else {
                return Err(ObjectsFormatError::Invalid(
                    "basin terminus lacks physical basin destination",
                ));
            };
            require(
                o.global
                    .catchments
                    .iter()
                    .any(|owner| owner.basin == Some(basin) && owner.representative_lake.is_none()),
                "basin terminus lacks copied dry physical basin",
            )?;
        }
        if r.ends == Terminus::Divergence {
            let crate::hydrology::ReceivingAccount::Junction(junction) = g.receiving else {
                return Err(ObjectsFormatError::Invalid(
                    "divergence lacks junction destination",
                ));
            };
            require(
                g.to == junction.cell(),
                "divergence junction disagrees with endpoint",
            )?;
            require(
                r.course.len() == 1
                    && u32::from(r.course[0].x()) == g.from.x % 512
                    && u32::from(r.course[0].y()) == g.from.y % 512,
                "divergence course does not own its source cell",
            )?;
            let base = junction
                .0
                .checked_mul(8)
                .ok_or(ObjectsFormatError::Invalid(
                    "junction reach identity overflow",
                ))?;
            let end = base.checked_add(7).ok_or(ObjectsFormatError::Invalid(
                "junction reach identity overflow",
            ))?;
            let mut outgoing = 0_u8;
            for (_, target) in global_reaches.range(ReachId(base)..=ReachId(end)) {
                require(
                    target.from == junction.cell() && target.annual_volume.0 > 0,
                    "divergence outgoing reach is not a real source at its junction",
                )?;
                outgoing += 1;
            }
            let point_id = ReachId::point(junction.cell()).ok_or(ObjectsFormatError::Invalid(
                "junction point identity overflow",
            ))?;
            if let Some(target) = global_reaches.get(&point_id) {
                require(
                    target.from == junction.cell()
                        && target.receiving == crate::hydrology::ReceivingAccount::DomainExport
                        && target.annual_volume.0 > 0,
                    "divergence point is not a positive domain export at its junction",
                )?;
                outgoing += 1;
            }
            require(
                (2..=9).contains(&outgoing),
                "divergence lacks multiple copied outgoing reaches",
            )?;
        }
        match (r.ends, r.feeds) {
            (Terminus::Junction, Some(id)) => {
                let j = *local_index.get(&id).ok_or(ObjectsFormatError::Invalid(
                    "feeds names a missing local river",
                ))?;
                require(j != i, "river feeds itself")?;
                let tail = r
                    .course
                    .last()
                    .ok_or(ObjectsFormatError::Invalid("empty course"))?;
                let head = o.rivers[j]
                    .course
                    .first()
                    .ok_or(ObjectsFormatError::Invalid("empty downstream course"))?;
                require(
                    tail == head || cell_neighbor(*tail, *head),
                    "local feeds connection is not adjacent",
                )?;
                next[i] = Some(j);
            }
            (Terminus::Junction, None) => {
                return Err(ObjectsFormatError::Invalid(
                    "junction lacks local feeds target",
                ))
            }
            (_, None) => {}
            (_, Some(_)) => {
                return Err(ObjectsFormatError::Invalid(
                    "non-junction carries local feeds target",
                ))
            }
        }
    }
    // Functional-graph colouring is linear and nonrecursive; shared endpoints are allowed.
    let mut state = vec![0_u8; o.rivers.len()];
    for start in 0..state.len() {
        if state[start] != 0 {
            continue;
        }
        let mut chain = Vec::new();
        let mut at = Some(start);
        while let Some(i) = at {
            if state[i] == 2 {
                break;
            }
            require(state[i] != 1, "local river feeds cycle")?;
            state[i] = 1;
            chain.push(i);
            at = next[i];
        }
        for i in chain {
            state[i] = 2;
        }
    }
    let mut lake_owner = vec![false; CELLS];
    let mut lake_counts = BTreeMap::<BasinId, usize>::new();
    for l in &o.lakes {
        require(
            l.id > 0 && !l.cells.is_empty(),
            "invalid local lake identity or membership",
        )?;
        let g = global_lakes
            .get(&l.global_id)
            .ok_or(ObjectsFormatError::Invalid("lake lacks copied global lake"))?;
        require(
            l.surface == g.surface,
            "local lake level disagrees with exact global surface",
        )?;
        let max_depth = i64::from(g.surface.raw()) - i64::from(g.deepest_bed.raw());
        require(
            max_depth >= 0 && i64::from(l.depth_mm) <= max_depth,
            "local depth exceeds global maximum",
        )?;
        if l.outlet.is_some() {
            require(
                g.annual_outflow.0 > 0 && g.outlet.is_some(),
                "global lake without supported annual outflow has local outlet",
            )?;
        }
        for p in l.cells.windows(2) {
            require(
                p[0].index() < p[1].index(),
                "lake members are not unique row-major cells",
            )?;
        }
        for c in &l.cells {
            require(!lake_owner[c.index()], "local lake memberships overlap")?;
            lake_owner[c.index()] = true;
        }
        let n = lake_counts.entry(l.global_id).or_default();
        *n = n
            .checked_add(l.cells.len())
            .ok_or(ObjectsFormatError::Overflow)?;
        require(
            *n <= usize::try_from(g.submerged_cells).map_err(|_| ObjectsFormatError::Overflow)?,
            "local fragments exceed global wet membership",
        )?;
    }
    Ok(())
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
fn decode_inner(bytes: &[u8], limits: ObjectsLimits) -> Result<AreaObjects> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hydrology::{
        AnnualCatchment, AreaHydrologyContext, CatchmentId, GlobalLake, GlobalReach, Litres,
        ReceivingAccount, SpillConnection,
    };
    use crate::GlobalCell;
    fn cc(x: u16, y: u16) -> CellCoord {
        CellCoord::new(x, y).unwrap()
    }
    fn sample() -> AreaObjects {
        let q = DischargeMilli::new(u64::from(u32::MAX) + 8);
        let reach = GlobalReach {
            id: ReachId(4),
            from: GlobalCell { x: 0, y: 0 },
            to: GlobalCell { x: 2, y: 1 },
            receiving: ReceivingAccount::Sea,
            catchment: CatchmentId(5),
            drainage_cells: 100,
            annual_volume: Litres(u128::from(q.raw()) * 31_536_000),
            mean_discharge: q,
        };
        let lake = GlobalLake {
            basin: BasinId(99),
            surface: HeightMm::new(3000),
            deepest_bed: HeightMm::new(2999),
            submerged_cells: 2,
            outlet: None,
            annual_outflow: Litres(0),
            mean_outflow: DischargeMilli::new(0),
        };
        let state = AnnualCatchment {
            catchment: CatchmentId(5),
            terminal: GlobalCell { x: 2, y: 1 },
            contributing_cells: 100,
            basin: Some(BasinId(99)),
            representative_lake: Some(BasinId(99)),
            potential_spill: Some(SpillConnection {
                from: GlobalCell { x: 3, y: 1 },
                to: Some(GlobalCell { x: 4, y: 1 }),
                sill: HeightMm::new(4000),
                receiving: ReceivingAccount::Sea,
            }),
            receiving: ReceivingAccount::Sea,
        };
        AreaObjects {
            rivers: vec![RiverSegment {
                global_id: reach.id,
                id: 70000,
                order: 2,
                width_dm: 80000,
                discharge: q,
                feeds: None,
                ends: Terminus::Lake,
                course: vec![cc(0, 0), cc(1, 0), cc(2, 1)],
            }],
            lakes: vec![Lake {
                global_id: lake.basin,
                id: 90000,
                surface: lake.surface,
                depth_mm: 1,
                outlet: None,
                cells: vec![cc(2, 1), cc(3, 1)],
            }],
            channel_edges: vec![ChannelEdge {
                from: GlobalCell { x: 0, y: 0 },
                to: GlobalCell { x: 1, y: 0 },
                from_width_dm: 80000,
                to_width_dm: 80000,
                discharge: q,
            }],
            global: AreaHydrologyContext {
                reaches: vec![reach],
                lakes: vec![lake],
                catchments: vec![state],
                ..AreaHydrologyContext::default()
            },
        }
    }
    #[derive(Clone)]
    struct Section {
        kind: u16,
        count: u32,
        body: Vec<u8>,
    }
    fn sections(bytes: &[u8]) -> Vec<Section> {
        let mut r = Reader::new(bytes, 0);
        r.take(8).unwrap();
        let n = r.u16().unwrap();
        let mut out = Vec::new();
        for _ in 0..n {
            let kind = r.u16().unwrap();
            let count = r.u32().unwrap();
            let len = usize::try_from(r.u32().unwrap()).unwrap();
            out.push(Section {
                kind,
                count,
                body: r.take(len).unwrap().to_vec(),
            });
        }
        r.finish().unwrap();
        out
    }
    fn pack(parts: &[Section]) -> Vec<u8> {
        let mut out = OBJECTS_MAGIC.to_vec();
        out.extend(u16::try_from(parts.len()).unwrap().to_le_bytes());
        for s in parts {
            out.extend(s.kind.to_le_bytes());
            out.extend(s.count.to_le_bytes());
            out.extend(u32::try_from(s.body.len()).unwrap().to_le_bytes());
            out.extend(&s.body);
        }
        out
    }
    fn decode(bytes: &[u8]) -> Result<AreaObjects> {
        decode_inner(bytes, ObjectsLimits::default())
    }
    fn rejects_change(change: impl FnOnce(&mut AreaObjects)) {
        let mut o = sample();
        change(&mut o);
        assert!(encode_objects(&o).is_err());
    }
    #[test]
    fn roundtrip_preserves_widened_fields_global_ids_and_whole_mm_depth() {
        let o = sample();
        let b = encode_objects(&o).unwrap();
        assert_eq!(decode(&b).unwrap(), o);
        assert_eq!(encode_objects(&o).unwrap(), b);
        let parts = sections(&b);
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0].body.len(), 34 + 12);
        assert_eq!(parts[1].body.len(), 29 + 8);
        assert_eq!(parts[2].body.len(), 32);
        assert_eq!(parts[3].count, 1);
        assert_eq!(&parts[0].body[..8], &4_u64.to_le_bytes());
        assert_eq!(&parts[0].body[8..12], &70000_u32.to_le_bytes());
        assert_eq!(&parts[0].body[13..17], &80000_u32.to_le_bytes());
        assert_eq!(
            &parts[0].body[17..25],
            &(u64::from(u32::MAX) + 8).to_le_bytes()
        );
        assert_eq!(&parts[0].body[30..34], &3_u32.to_le_bytes());
        assert_eq!(o.lakes[0].depth_mm, 1);
        assert!(o.rivers[0].course.contains(&o.lakes[0].cells[0]));
    }
    #[test]
    fn empty_container_requires_new_sections_and_revision_two() {
        let o = AreaObjects::empty();
        let b = encode_objects(&o).unwrap();
        assert_eq!(b.len(), 78);
        assert_eq!(decode(&b).unwrap(), o);
        assert_eq!(o.global.model_revision, 2);
    }
    #[test]
    fn every_prefix_trailing_data_and_path_errors_are_rejected() {
        let b = encode_objects(&sample()).unwrap();
        for n in 0..b.len() {
            assert!(decode(&b[..n]).is_err(), "accepted prefix {n}");
        }
        let mut tail = b.clone();
        tail.push(0);
        assert!(matches!(
            decode(&tail),
            Err(ObjectsFormatError::TrailingBytes)
        ));
        let mut bad = b.clone();
        bad[0] = 0;
        assert!(
            matches!(decode_objects("area/file",&bad),Err(FormatError::BadMagic{path,layer:"objects"}) if path=="area/file")
        );
        assert!(
            matches!(decode_objects("area/file",&b[..b.len()-1]),Err(FormatError::Objects{path,..}) if path=="area/file")
        );
    }
    #[test]
    fn record_cannot_borrow_bytes_from_following_section() {
        let b = encode_objects(&sample()).unwrap();
        let mut p = sections(&b);
        // Keep the next section intact; a declared river body ends before its
        // course's final coordinate. It must fail inside this exact slice.
        p[0].body.truncate(42);
        assert!(matches!(
            decode(&pack(&p)),
            Err(ObjectsFormatError::Truncated {
                offset: 54,
                needed: 12,
                available: 8
            })
        ));
        let mut p = sections(&b);
        p[1].body.truncate(33);
        assert!(matches!(
            decode(&pack(&p)),
            Err(ObjectsFormatError::Truncated {
                needed: 8,
                available: 4,
                ..
            })
        ));
    }
    #[test]
    fn required_sections_unique_order_and_single_context_are_enforced() {
        let b = encode_objects(&sample()).unwrap();
        for k in 1..=4 {
            let mut p = sections(&b);
            p.retain(|s| s.kind != k);
            assert!(matches!(decode(&pack(&p)),Err(ObjectsFormatError::MissingSection(v)) if v==k));
        }
        let mut p = sections(&b);
        p.insert(1, p[0].clone());
        assert!(matches!(
            decode(&pack(&p)),
            Err(ObjectsFormatError::SectionOrder)
        ));
        let mut p = sections(&b);
        p.swap(0, 1);
        assert!(matches!(
            decode(&pack(&p)),
            Err(ObjectsFormatError::SectionOrder)
        ));
        for n in [0, 2] {
            let mut p = sections(&b);
            p[3].count = n;
            assert!(decode(&pack(&p)).is_err());
        }
    }
    #[test]
    fn known_sections_consume_exactly_unknown_sections_are_length_bounded() {
        let b = encode_objects(&sample()).unwrap();
        for i in 0..4 {
            let mut p = sections(&b);
            p[i].body.push(0);
            assert!(decode(&pack(&p)).is_err());
        }
        let mut p = sections(&b);
        p.push(Section {
            kind: 999,
            count: u32::MAX,
            body: vec![1, 2, 3],
        });
        let good = pack(&p);
        assert_eq!(decode(&good).unwrap(), sample());
        assert!(decode(&good[..good.len() - 1]).is_err());
        p.push(p[4].clone());
        assert!(matches!(
            decode(&pack(&p)),
            Err(ObjectsFormatError::SectionOrder)
        ));
    }
    #[test]
    fn malformed_counts_fail_before_count_driven_allocation() {
        let b = encode_objects(&sample()).unwrap();
        for i in 0..3 {
            let mut p = sections(&b);
            p[i].count = u32::MAX;
            assert!(matches!(
                decode(&pack(&p)),
                Err(ObjectsFormatError::Limit(_))
            ));
        }
        for (i, offset) in [(0, 30), (1, 25)] {
            let mut p = sections(&b);
            p[i].body[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert!(matches!(
                decode(&pack(&p)),
                Err(ObjectsFormatError::Limit(_))
            ));
        }
        let mut p = sections(&b);
        p[2].count = 0;
        assert!(decode(&pack(&p)).is_err());
        let limits = ObjectsLimits {
            max_bytes: b.len() - 1,
            ..ObjectsLimits::default()
        };
        assert!(encode_objects_with_limits(&sample(), limits).is_err());
        assert!(decode_objects_with_limits("x", &b, limits).is_err());
        let limits = ObjectsLimits {
            max_sections: 3,
            ..ObjectsLimits::default()
        };
        assert!(encode_objects_with_limits(&sample(), limits).is_err());
        assert!(decode_objects_with_limits("x", &b, limits).is_err());
        let limits = ObjectsLimits {
            max_course_cells: 2,
            ..ObjectsLimits::default()
        };
        assert!(encode_objects_with_limits(&sample(), limits).is_err());
        assert!(decode_objects_with_limits("x", &b, limits).is_err());
        let limits = ObjectsLimits {
            context: ContextLimits {
                max_records: 0,
                ..ContextLimits::default()
            },
            ..ObjectsLimits::default()
        };
        assert!(encode_objects_with_limits(&sample(), limits).is_err());
        assert!(decode_objects_with_limits("x", &b, limits).is_err());
    }
    #[test]
    fn unknown_tags_noncanonical_none_and_bad_coordinates_fail() {
        let b = encode_objects(&sample()).unwrap();
        for (section, offset, value) in [(0, 29, 9), (1, 20, 2), (1, 21, 1)] {
            let mut p = sections(&b);
            p[section].body[offset] = value;
            assert!(decode(&pack(&p)).is_err());
        }
        for (section, offset) in [(0, 34), (1, 29)] {
            let mut p = sections(&b);
            p[section].body[offset..offset + 2].copy_from_slice(&512_u16.to_le_bytes());
            assert!(decode(&pack(&p)).is_err());
        }
    }
    #[test]
    fn local_records_require_matching_copied_authority() {
        rejects_change(|o| o.rivers[0].global_id = ReachId(123));
        rejects_change(|o| o.rivers[0].discharge = DischargeMilli::new(41));
        rejects_change(|o| o.lakes[0].global_id = BasinId(123));
        rejects_change(|o| o.lakes[0].surface = HeightMm::new(3001));
        rejects_change(|o| o.lakes[0].depth_mm = 2);
        rejects_change(|o| o.lakes[0].outlet = Some(cc(2, 1)));
        rejects_change(|o| o.rivers[0].id = 0);
    }
    #[test]
    fn dry_basin_endpoint_and_tiny_supported_lake_outlet_roundtrip() {
        let mut o = sample();
        o.rivers[0].ends = Terminus::Basin;
        o.global.reaches[0].receiving = ReceivingAccount::Lake(BasinId(99));
        o.lakes.clear();
        o.global.lakes.clear();
        o.global.catchments[0].representative_lake = None;
        let bytes = encode_objects(&o).unwrap();
        assert_eq!(decode(&bytes).unwrap(), o);
        assert_eq!(Terminus::from_u8(4), Some(Terminus::Basin));
        o.global.catchments[0].basin = None;
        assert!(encode_objects(&o).is_err());
        let mut o = sample();
        let at = o.lakes[0].cells[0];
        o.lakes[0].outlet = Some(at);
        let global = &mut o.global.lakes[0];
        global.annual_outflow = Litres(1);
        global.outlet = Some(SpillConnection {
            from: GlobalCell {
                x: u32::from(at.x()),
                y: u32::from(at.y()),
            },
            to: Some(GlobalCell {
                x: u32::from(at.x()) + 1,
                y: u32::from(at.y()),
            }),
            sill: global.surface,
            receiving: ReceivingAccount::Sea,
        });
        assert_eq!(global.mean_outflow.raw(), 0);
        let bytes = encode_objects(&o).unwrap();
        assert_eq!(decode(&bytes).unwrap(), o);
    }
    #[test]
    fn courses_are_nonempty_simple_directed_d8() {
        rejects_change(|o| o.rivers[0].course.clear());
        rejects_change(|o| o.rivers[0].course.push(cc(400, 400)));
        rejects_change(|o| o.rivers[0].course.push(cc(1, 0)));
        rejects_change(|o| o.rivers[0].ends = Terminus::Junction);
        rejects_change(|o| o.rivers[0].feeds = Some(70000));
    }
    fn connected() -> AreaObjects {
        let mut o = sample();
        let mut upstream = o.rivers[0].clone();
        upstream.id = 1;
        upstream.ends = Terminus::Junction;
        upstream.feeds = Some(70000);
        upstream.course = vec![cc(0, 1), cc(0, 0)];
        o.rivers.insert(0, upstream);
        o
    }
    #[test]
    fn shared_endpoints_and_multiple_fragments_per_global_id_are_valid() {
        let mut o = connected();
        let mut lake = o.lakes[0].clone();
        o.lakes[0].cells.truncate(1);
        lake.id += 1;
        lake.cells.remove(0);
        o.lakes.push(lake);
        let b = encode_objects(&o).unwrap();
        assert_eq!(decode(&b).unwrap(), o);
        o.rivers[0].course = vec![cc(0, 1)];
        assert!(encode_objects(&o).is_ok());
    }
    #[test]
    fn feeds_targets_are_present_connected_and_acyclic() {
        let mut o = connected();
        o.rivers[0].feeds = Some(123);
        assert!(encode_objects(&o).is_err());
        let mut o = connected();
        o.rivers[0].feeds = Some(1);
        assert!(encode_objects(&o).is_err());
        let mut o = connected();
        o.rivers[0].course = vec![cc(10, 10)];
        assert!(encode_objects(&o).is_err());
        let mut o = connected();
        o.rivers[0].course = vec![cc(0, 0)];
        o.rivers[1].course = vec![cc(1, 0)];
        o.rivers[1].ends = Terminus::Junction;
        o.rivers[1].feeds = Some(1);
        assert!(matches!(
            encode_objects(&o),
            Err(ObjectsFormatError::Invalid("local river feeds cycle"))
        ));
    }
    #[test]
    fn lake_memberships_are_canonical_disjoint_and_bounded_by_global_count() {
        rejects_change(|o| o.lakes[0].cells.reverse());
        rejects_change(|o| o.lakes[0].cells.push(cc(4, 1)));
        rejects_change(|o| {
            let mut l = o.lakes[0].clone();
            l.id += 1;
            o.lakes.push(l);
        });
        rejects_change(|o| o.channel_edges.push(o.channel_edges[0]));
    }
    #[test]
    fn divergence_roundtrips_real_branches_without_false_single_feed_or_recursive_context() {
        let mut o = sample();
        o.lakes.clear();
        o.global.lakes.clear();
        o.global.catchments[0].representative_lake = None;
        o.rivers[0].ends = Terminus::Divergence;
        o.rivers[0].course.truncate(1);
        let at = o.global.reaches[0].to;
        o.global.reaches[0].receiving =
            ReceivingAccount::Junction(crate::hydrology::JunctionId::at(at));
        let template = o.global.reaches[0].clone();
        for to in [GlobalCell { x: 3, y: 1 }, GlobalCell { x: 3, y: 2 }] {
            let mut branch = template.clone();
            branch.id = ReachId::from_step(at, to).unwrap();
            branch.from = at;
            branch.to = to;
            branch.annual_volume.0 /= 2;
            branch.mean_discharge = DischargeMilli::new(template.mean_discharge.raw() / 2);
            // The next junction is external to this bounded context. Its branches
            // need not be recursively copied merely to describe this divergence.
            branch.receiving = ReceivingAccount::Junction(crate::hydrology::JunctionId::at(to));
            o.global.reaches.push(branch);
        }
        o.global.reaches.sort_by_key(|r| r.id);
        assert_eq!(Terminus::from_u8(5), Some(Terminus::Divergence));
        let bytes = encode_objects(&o).unwrap();
        assert_eq!(decode(&bytes).unwrap(), o);
        let mut bad = o.clone();
        bad.rivers[0].feeds = Some(70000);
        assert!(encode_objects(&bad).is_err());
        let mut bad = o.clone();
        bad.global.reaches.pop();
        assert!(encode_objects(&bad).is_err());
        let mut bad = o.clone();
        bad.global.reaches[0].receiving = ReceivingAccount::Sea;
        assert!(encode_objects(&bad).is_err());
        let mut bad = o.clone();
        bad.rivers[0].course[0] = cc(2, 1);
        assert!(encode_objects(&bad).is_err());
        let mut bad = o.clone();
        bad.global.reaches[1].from.x += 1;
        assert!(encode_objects(&bad).is_err());
        let mut bad = o.clone();
        bad.global.reaches[1].annual_volume = Litres(0);
        bad.global.reaches[1].mean_discharge = DischargeMilli::new(0);
        assert!(encode_objects(&bad).is_err());
        let mut bad = o.clone();
        bad.global.reaches[2].id = bad.global.reaches[1].id;
        assert!(encode_objects(&bad).is_err());
        // Eight actual D8 branches plus one distinct physical exterior point fit.
        let mut full = o.clone();
        full.global.reaches.truncate(1);
        for (dx, dy) in [
            (-1_i32, -1_i32),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ] {
            let to = GlobalCell {
                x: u32::try_from(i64::from(at.x) + i64::from(dx)).unwrap(),
                y: u32::try_from(i64::from(at.y) + i64::from(dy)).unwrap(),
            };
            let mut branch = o.global.reaches[1].clone();
            branch.id = ReachId::from_step(at, to).unwrap();
            branch.to = to;
            branch.receiving = ReceivingAccount::Sea;
            full.global.reaches.push(branch);
        }
        full.global.reaches.sort_by_key(|r| r.id);
        assert!(encode_objects(&full).is_ok());
        let mut point = full.global.reaches[1].clone();
        point.id = ReachId::point(at).unwrap();
        point.to = at;
        point.receiving = ReceivingAccount::DomainExport;
        point.annual_volume = Litres(1);
        point.mean_discharge = DischargeMilli::new(0);
        full.global.reaches.push(point.clone());
        full.global.reaches.sort_by_key(|r| r.id);
        let bytes = encode_objects(&full).unwrap();
        assert_eq!(decode(&bytes).unwrap(), full);
        let mut bad = full.clone();
        bad.global.reaches.last_mut().unwrap().receiving = ReceivingAccount::Lake(BasinId(55));
        assert!(encode_objects(&bad).is_err());
        let mut bad = full.clone();
        bad.global.reaches.push(point);
        assert!(encode_objects(&bad).is_err());
        // A below-threshold domain exit may be the second branch by itself.
        let mut pair = full.clone();
        pair.global.reaches.retain(|g| {
            g.id == template.id || g.id == full.global.reaches[1].id || g.id.is_point()
        });
        assert!(encode_objects(&pair).is_ok());
    }
    #[test]
    fn point_local_course_owns_exact_terminal_and_keeps_dry_basin_semantics() {
        let mut o = sample();
        o.lakes.clear();
        o.global.lakes.clear();
        o.global.catchments[0].representative_lake = None;
        let g = &mut o.global.reaches[0];
        g.id = ReachId::point(g.from).unwrap();
        g.to = g.from;
        g.receiving = ReceivingAccount::Lake(BasinId(99));
        o.rivers[0].global_id = g.id;
        o.rivers[0].course.truncate(1);
        o.rivers[0].ends = Terminus::Basin;
        assert_eq!(decode(&encode_objects(&o).unwrap()).unwrap(), o);
        let mut bad = o.clone();
        bad.rivers[0].course[0] = cc(1, 0);
        assert!(encode_objects(&bad).is_err());
        let mut bad = o.clone();
        bad.rivers[0].ends = Terminus::Lake;
        assert!(encode_objects(&bad).is_err());
        o.global.reaches[0].receiving = ReceivingAccount::DomainExport;
        o.rivers[0].ends = Terminus::OffTile;
        assert_eq!(decode(&encode_objects(&o).unwrap()).unwrap(), o);
        o.rivers[0].ends = Terminus::Sea;
        assert!(encode_objects(&o).is_err());
    }
}
