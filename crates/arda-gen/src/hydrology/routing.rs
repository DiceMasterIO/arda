//! Exact global flat routing and receiver-chain ownership for prepared terrain.
//!
//! The backend owns only private prepared/scratch pages, never final area files.
//! Physical heights and the already-solved marine mask are immutable during routing.
//! Before routing, the same scratch supports a checked connected-ocean flood.
#![deny(missing_docs)]

const NONE: u32 = u32::MAX;
const UNSET: u8 = u8::MAX;
const CLOSED: u8 = 8;
const MARINE: u8 = 9;
const EXPORT: u8 = 10;
const DIRS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];
/// Cells per fixed 4096-byte scratch page; each record has an explicit 16-byte codec.
pub const PAGE_CELLS: usize = 256;

/// Temporary row-major ordinal within one validated modeled domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellIndex(u32);
impl CellIndex {
    /// Checked ordinal; persisted identity is obtained from the domain coordinates.
    pub fn new(raw: u32, extent: Extent) -> Option<Self> {
        (raw < extent.cells()).then_some(Self(raw))
    }
    /// Raw ordinal for fixed-width private scratch records.
    pub fn raw(self) -> u32 {
        self.0
    }
}
/// Exact rectangular domain, including requested fringe and exported overshoot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent {
    width: u32,
    height: u32,
}
impl Extent {
    /// Validates the supported positive axes and the reserved ordinal sentinel.
    pub fn new(width: u32, height: u32) -> Option<Self> {
        if width == 0 || height == 0 || width > 40000 || height > 40000 {
            return None;
        }
        width
            .checked_mul(height)
            .filter(|&n| n < NONE)
            .map(|_| Self { width, height })
    }
    /// Number of actual modeled cells, excluding page padding.
    pub fn cells(self) -> u32 {
        self.width * self.height
    }
    /// Absolute fine-cell coordinates represented by a temporary ordinal.
    pub fn coordinates(self, at: CellIndex) -> (u32, u32) {
        (at.0 % self.width, at.0 / self.width)
    }
    /// Stable packed coordinate identity, independent of domain-width indexing.
    pub fn anchor_key(self, at: CellIndex) -> u64 {
        let (x, y) = self.coordinates(at);
        (u64::from(y) << 32) | u64::from(x)
    }
    fn boundary(self, at: CellIndex) -> bool {
        let (x, y) = self.coordinates(at);
        x == 0 || y == 0 || x + 1 == self.width || y + 1 == self.height
    }
    fn neighbor(self, at: CellIndex, dir: usize) -> Option<CellIndex> {
        let (x, y) = self.coordinates(at);
        let (dx, dy) = DIRS[dir];
        let x = i64::from(x) + i64::from(dx);
        let y = i64::from(y) + i64::from(dy);
        let x = u32::try_from(x).ok()?;
        let y = u32::try_from(y).ok()?;
        (x < self.width && y < self.height).then(|| CellIndex(y * self.width + x))
    }
}
/// Terminal receiving account; a marine entry/export remains spatially distinct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutletKind {
    /// Canonical minimum of a closed physical depression/plateau.
    ClosedDepression,
    /// Actual modeled marine cell reached by the descending course.
    MarineEntry,
    /// Actual outer-rim cell with no physically lower modeled neighbor.
    DomainExport,
}
/// Final receiver semantics; outside exports never invent a ghost coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Receiver {
    /// Actual D8 neighbor, with strictly decreasing physical height or flat rank.
    Cell(CellIndex),
    /// Terminal at this cell; its coordinate identifies the receiving account.
    Stop(OutletKind),
}
/// Fixed-width scratch record. Height never changes; marine connectivity is set
/// only by the checked pre-routing flood and is immutable throughout routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRecord {
    height: i32,
    distance: u32,
    owner: u32,
    receiver: u8,
    flags: u8,
}
impl CellRecord {
    /// Begins a fresh scratch record from physical millimetres and connected-ocean state.
    pub fn prepared(height: i32, marine: bool) -> Self {
        Self {
            height,
            distance: NONE,
            owner: NONE,
            receiver: UNSET,
            flags: u8::from(marine),
        }
    }
    /// Original physical elevation in millimetres; never includes routing rank.
    pub fn height(self) -> i32 {
        self.height
    }
    /// Whether this cell belongs to the globally connected marine mask.
    pub fn is_marine(self) -> bool {
        self.flags & 1 != 0
    }
    /// Exact plateau distance after routing; absent before its plateau is processed.
    pub fn distance(self) -> Option<u32> {
        (self.distance != NONE).then_some(self.distance)
    }
    /// Coordinate ordinal of the eventual terminal receiving account.
    pub fn owner(self) -> Option<CellIndex> {
        (self.owner != NONE).then_some(CellIndex(self.owner))
    }
    /// Decodes a final receiver, rejecting unset codes and outside D8 coordinates.
    pub fn receiver(self, extent: Extent, at: CellIndex) -> Option<Receiver> {
        match self.receiver {
            0..=7 => extent
                .neighbor(at, self.receiver as usize)
                .map(Receiver::Cell),
            CLOSED => Some(Receiver::Stop(OutletKind::ClosedDepression)),
            MARINE => Some(Receiver::Stop(OutletKind::MarineEntry)),
            EXPORT => extent
                .boundary(at)
                .then_some(Receiver::Stop(OutletKind::DomainExport)),
            _ => None,
        }
    }
    /// Canonical little-endian layout: height4, distance4, owner4, direction1,
    /// flags1, reserved-zero2. This is independent of Rust struct padding.
    pub fn encode(self) -> [u8; 16] {
        let mut b = [0; 16];
        b[..4].copy_from_slice(&self.height.to_le_bytes());
        b[4..8].copy_from_slice(&self.distance.to_le_bytes());
        b[8..12].copy_from_slice(&self.owner.to_le_bytes());
        b[12] = self.receiver;
        b[13] = self.flags;
        b
    }
    /// Validates a stored record before exposing it to the routing kernel.
    pub fn decode(b: [u8; 16], extent: Extent, at: CellIndex) -> Option<Self> {
        let r = Self {
            height: i32::from_le_bytes(b[..4].try_into().ok()?),
            distance: u32::from_le_bytes(b[4..8].try_into().ok()?),
            owner: u32::from_le_bytes(b[8..12].try_into().ok()?),
            receiver: b[12],
            flags: b[13],
        };
        if b[14] != 0
            || b[15] != 0
            || r.flags & !3 != 0
            || (r.distance != NONE && r.distance >= extent.cells())
            || (r.owner != NONE && r.owner >= extent.cells())
            || (r.receiver != UNSET && r.receiver(extent, at).is_none())
            || (r.receiver == MARINE && !r.is_marine())
        {
            None
        } else {
            Some(r)
        }
    }
}
/// Two reusable append-only ordinal tapes; reads may replay or walk backwards.
#[derive(Debug, Clone, Copy)]
pub enum Tape {
    /// Plateau membership, then reused for receiver-chain unwinding.
    Component,
    /// Multi-source breadth-first frontier.
    Frontier,
}
impl Tape {
    fn slot(self) -> usize {
        match self {
            Self::Component => 0,
            Self::Frontier => 1,
        }
    }
}
/// Exact paging contract shared by the checked memory backend and a disk adapter.
///
/// Cell records occupy 4096-byte pages with 256 explicit 16-byte slots. A disk
/// adapter must cache dirty pages within its caller-owned reservation, preserve
/// read-after-write coherence, and return I/O errors. Each tape is a separate
/// little-endian u32 stream of at most N entries; clear resets logical length,
/// push appends, and get supports sequential replay/reverse unwinding. No method
/// enumerates directories, reads neighboring final files, or silently evicts data.
pub trait RoutingStore {
    /// Underlying storage failure propagated without fabricating routing output.
    type Error;
    /// Validated modeled rectangle for every ordinal in this store.
    fn extent(&self) -> Extent;
    /// Reads one coherent record; page-cache policy cannot change the value.
    fn read(&mut self, at: CellIndex) -> Result<CellRecord, Self::Error>;
    /// Replaces one scratch record, retaining the same physical height/marine bit.
    fn write(&mut self, at: CellIndex, record: CellRecord) -> Result<(), Self::Error>;
    /// Marks one fresh, nonmarine cell at or below zero as marine during the
    /// pre-routing flood. Rejects any other state; ordinary writes cannot do this.
    fn mark_marine(&mut self, at: CellIndex) -> Result<(), Self::Error>;
    /// Resets a tape's length while permitting reuse of allocated pages.
    fn clear(&mut self, tape: Tape) -> Result<(), Self::Error>;
    /// Current checked logical tape length.
    fn len(&self, tape: Tape) -> u32;
    /// Appends one ordinal; adapters reject lengths beyond the modeled cell count.
    fn push(&mut self, tape: Tape, at: CellIndex) -> Result<(), Self::Error>;
    /// Reads an existing tape position; never returns unwritten capacity.
    fn get(&mut self, tape: Tape, position: u32) -> Result<CellIndex, Self::Error>;
}
/// Logical operation counters, deliberately separate from physical page I/O.
#[derive(Debug, Default, Clone, Copy)]
pub struct Work {
    /// Number of coherent cell reads, including ownership validation.
    pub cell_reads: u64,
    /// Number of changed cell records submitted to the backend.
    pub cell_writes: u64,
    /// Number of tape entries read during BFS/replay/unwinding.
    pub tape_reads: u64,
    /// Number of tape entries appended; clear operations do not add entries.
    pub tape_writes: u64,
}
/// Explicit operation ceilings; a tighter caller budget produces a typed error.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Maximum coherent cell reads for this pass.
    pub cell_reads: u64,
    /// Maximum changed cell records for this pass.
    pub cell_writes: u64,
    /// Maximum tape-entry reads for this pass.
    pub tape_reads: u64,
    /// Maximum tape-entry appends for this pass.
    pub tape_writes: u64,
}
impl Limits {
    /// Conservative linear logical-work admission for the specified implementation.
    /// Physical cache misses are separately limited/measured by the disk adapter.
    pub fn for_extent(e: Extent) -> Self {
        let n = u64::from(e.cells());
        Self {
            cell_reads: 64 * n + 128,
            cell_writes: 12 * n + 128,
            tape_reads: 8 * n + 128,
            tape_writes: 4 * n + 128,
        }
    }
}
/// Failure leaves only private scratch partially updated; publication must abort.
#[derive(Debug, thiserror::Error)]
pub enum RoutingError<E> {
    /// Original backend error, including real I/O failure in a disk adapter.
    #[error("physical routing storage failed")]
    Storage(#[source] E),
    /// A selected operation budget was exhausted before the next operation.
    #[error("physical routing work reservation exhausted")]
    WorkLimit,
    /// A record/receiver/rank violates the explicit routing invariants.
    #[error("invalid physical routing topology")]
    InvalidTopology,
}
struct Pass<'a, S: RoutingStore> {
    store: &'a mut S,
    extent: Extent,
    limits: Limits,
    work: Work,
}
impl<S: RoutingStore> Pass<'_, S> {
    fn read(&mut self, i: CellIndex) -> Result<CellRecord, RoutingError<S::Error>> {
        if self.work.cell_reads >= self.limits.cell_reads {
            return Err(RoutingError::WorkLimit);
        }
        self.work.cell_reads += 1;
        self.store.read(i).map_err(RoutingError::Storage)
    }
    fn write(&mut self, i: CellIndex, r: CellRecord) -> Result<(), RoutingError<S::Error>> {
        if self.work.cell_writes >= self.limits.cell_writes {
            return Err(RoutingError::WorkLimit);
        }
        self.work.cell_writes += 1;
        self.store.write(i, r).map_err(RoutingError::Storage)
    }
    fn push(&mut self, t: Tape, i: CellIndex) -> Result<(), RoutingError<S::Error>> {
        if self.work.tape_writes >= self.limits.tape_writes
            || self.store.len(t) >= self.extent.cells()
        {
            return Err(RoutingError::WorkLimit);
        }
        self.work.tape_writes += 1;
        self.store.push(t, i).map_err(RoutingError::Storage)
    }
    fn get(&mut self, t: Tape, p: u32) -> Result<CellIndex, RoutingError<S::Error>> {
        if self.work.tape_reads >= self.limits.tape_reads {
            return Err(RoutingError::WorkLimit);
        }
        self.work.tape_reads += 1;
        self.store.get(t, p).map_err(RoutingError::Storage)
    }
    fn clear(&mut self, t: Tape) -> Result<(), RoutingError<S::Error>> {
        self.store.clear(t).map_err(RoutingError::Storage)
    }
    fn receivers(&mut self, order: [usize; 8]) -> Result<(), RoutingError<S::Error>> {
        for raw in 0..self.extent.cells() {
            let start = CellIndex(raw);
            let mut first = self.read(start)?;
            if first.flags & 2 != 0 {
                continue;
            }
            first.flags |= 2;
            if first.is_marine() {
                first.receiver = MARINE;
                first.distance = 0;
                self.write(start, first)?;
                continue;
            }
            let level = first.height;
            self.clear(Tape::Component)?;
            self.clear(Tape::Frontier)?;
            self.write(start, first)?;
            self.push(Tape::Component, start)?;
            let mut minimum = start;
            let mut cursor = 0;
            while cursor < self.store.len(Tape::Component) {
                let i = self.get(Tape::Component, cursor)?;
                cursor += 1;
                minimum = minimum.min(i);
                for &dir in &order {
                    if let Some(j) = self.extent.neighbor(i, dir) {
                        let mut r = self.read(j)?;
                        if r.height == level && !r.is_marine() && r.flags & 2 == 0 {
                            r.flags |= 2;
                            self.write(j, r)?;
                            self.push(Tape::Component, j)?;
                        }
                    }
                }
            }
            for position in 0..self.store.len(Tape::Component) {
                let i = self.get(Tape::Component, position)?;
                let mut r = self.read(i)?;
                let mut best: Option<(i64, i64, CellIndex, usize)> = None;
                for &dir in &order {
                    if let Some(j) = self.extent.neighbor(i, dir) {
                        let z = self.read(j)?.height;
                        if z < level {
                            let drop = i64::from(level) - i64::from(z);
                            let (dx, dy) = DIRS[dir];
                            let distance = if dx != 0 && dy != 0 { 141400 } else { 100000 };
                            let better = best.is_none_or(|(old, old_distance, old_cell, _)| {
                                drop * old_distance > old * distance
                                    || (drop * old_distance == old * distance && j < old_cell)
                            });
                            if better {
                                best = Some((drop, distance, j, dir));
                            }
                        }
                    }
                }
                if let Some((_, _, _, dir)) = best {
                    r.distance = 0;
                    r.receiver = u8::try_from(dir).map_err(|_| RoutingError::InvalidTopology)?;
                } else if self.extent.boundary(i) {
                    r.distance = 0;
                    r.receiver = EXPORT;
                } else {
                    continue;
                }
                self.write(i, r)?;
                self.push(Tape::Frontier, i)?;
            }
            if self.store.len(Tape::Frontier) == 0 {
                let mut r = self.read(minimum)?;
                r.distance = 0;
                r.receiver = CLOSED;
                self.write(minimum, r)?;
                self.push(Tape::Frontier, minimum)?;
            }
            cursor = 0;
            while cursor < self.store.len(Tape::Frontier) {
                let i = self.get(Tape::Frontier, cursor)?;
                cursor += 1;
                let next = self
                    .read(i)?
                    .distance
                    .checked_add(1)
                    .ok_or(RoutingError::InvalidTopology)?;
                for &dir in &order {
                    if let Some(j) = self.extent.neighbor(i, dir) {
                        let mut r = self.read(j)?;
                        if r.height == level && !r.is_marine() && r.distance == NONE {
                            r.distance = next;
                            self.write(j, r)?;
                            self.push(Tape::Frontier, j)?;
                        }
                    }
                }
            }
            for position in 0..self.store.len(Tape::Component) {
                let i = self.get(Tape::Component, position)?;
                let mut r = self.read(i)?;
                if r.distance == 0 {
                    continue;
                }
                let mut best = None;
                for &dir in &order {
                    if let Some(j) = self.extent.neighbor(i, dir) {
                        let q = self.read(j)?;
                        if q.height == level
                            && !q.is_marine()
                            && q.distance.checked_add(1) == Some(r.distance)
                            && best.is_none_or(|(old, _)| j < old)
                        {
                            best = Some((j, dir));
                        }
                    }
                }
                r.receiver = u8::try_from(best.ok_or(RoutingError::InvalidTopology)?.1)
                    .map_err(|_| RoutingError::InvalidTopology)?;
                self.write(i, r)?;
            }
        }
        Ok(())
    }
    fn ownership(&mut self) -> Result<(), RoutingError<S::Error>> {
        for raw in 0..self.extent.cells() {
            let start = CellIndex(raw);
            if self.read(start)?.owner != NONE {
                continue;
            }
            self.clear(Tape::Component)?;
            let mut at = start;
            let owner = loop {
                let r = self.read(at)?;
                if r.distance == NONE {
                    return Err(RoutingError::InvalidTopology);
                }
                if r.owner != NONE {
                    break CellIndex(r.owner);
                }
                self.push(Tape::Component, at)?;
                match r
                    .receiver(self.extent, at)
                    .ok_or(RoutingError::InvalidTopology)?
                {
                    Receiver::Stop(_) => break at,
                    Receiver::Cell(next) => {
                        let q = self.read(next)?;
                        if !(q.height < r.height
                            || (q.height == r.height && q.distance < r.distance))
                        {
                            return Err(RoutingError::InvalidTopology);
                        }
                        at = next;
                    }
                }
            };
            for position in (0..self.store.len(Tape::Component)).rev() {
                let i = self.get(Tape::Component, position)?;
                let mut r = self.read(i)?;
                r.owner = owner.0;
                self.write(i, r)?;
            }
        }
        Ok(())
    }
}
/// Assigns exact physical receivers and immutable terminal ownership in one pass.
/// The fixed neighbor order is NW,N,NE,W,E,SW,S,SE; ties use packed-coordinate order.
pub fn route_and_own<S: RoutingStore>(
    store: &mut S,
    limits: Limits,
) -> Result<Work, RoutingError<S::Error>> {
    let extent = store.extent();
    let mut p = Pass {
        store,
        extent,
        limits,
        work: Work::default(),
    };
    p.receivers([0, 1, 2, 3, 4, 5, 6, 7])?;
    p.ownership()?;
    Ok(p.work)
}
/// Clears previous ownership, then validates and resolves written receiver chains.
pub fn resolve_ownership<S: RoutingStore>(
    store: &mut S,
    limits: Limits,
) -> Result<Work, RoutingError<S::Error>> {
    let extent = store.extent();
    let mut p = Pass {
        store,
        extent,
        limits,
        work: Work::default(),
    };
    for raw in 0..extent.cells() {
        let i = CellIndex(raw);
        let mut r = p.read(i)?;
        r.owner = NONE;
        p.write(i, r)?;
    }
    p.ownership()?;
    Ok(p.work)
}

/// Checked reference-backend access failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryError {
    /// The ordinal belongs to a different or larger domain.
    InvalidIndex,
    /// The requested tape position has not been written.
    InvalidTapePosition,
    /// A write attempted to change physical terrain or ocean connectivity.
    ImmutableTerrain,
    /// Appending would exceed the admitted number of cells.
    TapeCapacity,
}

/// Bounded in-memory implementation of the exact fixed-page/tape contract.
/// It is a reference backend, not evidence that the maximum world fits in RAM.
pub struct MemoryPages {
    extent: Extent,
    pages: Vec<Box<[CellRecord; PAGE_CELLS]>>,
    tapes: [Vec<CellIndex>; 2],
}
impl MemoryPages {
    /// Rejects mismatched inputs or a conservative 32N + 8192-byte payload reservation.
    /// This covers record pages, two exactly reserved N-entry tapes, page pointers,
    /// and the caller's 5N-byte input slices; allocator metadata/process RSS are extra.
    /// Failed vector reservations return `None`; the host allocator still governs boxes.
    pub fn new(extent: Extent, heights: &[i32], marine: &[bool], ram_limit: u64) -> Option<Self> {
        let n = extent.cells() as usize;
        if heights.len() != n
            || marine.len() != n
            || 32 * u64::from(extent.cells()) + 8192 > ram_limit
        {
            return None;
        }
        let count = n.div_ceil(PAGE_CELLS);
        let mut pages = Vec::new();
        pages.try_reserve_exact(count).ok()?;
        let mut tapes = [Vec::new(), Vec::new()];
        for tape in &mut tapes {
            tape.try_reserve_exact(n).ok()?;
        }
        for page in 0..count {
            let mut records = Box::new([CellRecord::prepared(0, false); PAGE_CELLS]);
            for (j, record) in records.iter_mut().enumerate() {
                let i = page * PAGE_CELLS + j;
                if i < n {
                    *record = CellRecord::prepared(heights[i], marine[i]);
                }
            }
            pages.push(records);
        }
        Some(Self {
            extent,
            pages,
            tapes,
        })
    }
    /// Returns one canonical 4096-byte page; padding slots encode fresh zero records.
    pub fn encode_page(&self, page: usize) -> Option<[u8; 4096]> {
        let records = self.pages.get(page)?;
        let mut out = [0; 4096];
        for (i, r) in records.iter().enumerate() {
            out[i * 16..i * 16 + 16].copy_from_slice(&r.encode());
        }
        Some(out)
    }
    /// Validates and replaces a page after a private scratch-file round trip.
    pub fn replace_page(&mut self, page: usize, bytes: [u8; 4096]) -> Option<()> {
        let records = self.pages.get_mut(page)?;
        let mut decoded = **records;
        for (i, record) in decoded.iter_mut().enumerate() {
            let ordinal = page * PAGE_CELLS + i;
            if ordinal >= self.extent.cells() as usize {
                if bytes[i * 16..i * 16 + 16] != CellRecord::prepared(0, false).encode() {
                    return None;
                }
                continue;
            }
            let replacement = CellRecord::decode(
                bytes[i * 16..i * 16 + 16].try_into().ok()?,
                self.extent,
                CellIndex(u32::try_from(ordinal).ok()?),
            )?;
            if (record.height, record.is_marine()) != (replacement.height, replacement.is_marine())
            {
                return None;
            }
            *record = replacement;
        }
        **records = decoded;
        Some(())
    }
}
impl RoutingStore for MemoryPages {
    type Error = MemoryError;
    fn extent(&self) -> Extent {
        self.extent
    }
    fn read(&mut self, at: CellIndex) -> Result<CellRecord, Self::Error> {
        if at.0 >= self.extent.cells() {
            return Err(MemoryError::InvalidIndex);
        }
        Ok(self.pages[at.0 as usize / PAGE_CELLS][at.0 as usize % PAGE_CELLS])
    }
    fn write(&mut self, at: CellIndex, r: CellRecord) -> Result<(), Self::Error> {
        let old = self.read(at)?;
        if (old.height, old.is_marine()) != (r.height, r.is_marine()) {
            return Err(MemoryError::ImmutableTerrain);
        }
        self.pages[at.0 as usize / PAGE_CELLS][at.0 as usize % PAGE_CELLS] = r;
        Ok(())
    }
    fn mark_marine(&mut self, at: CellIndex) -> Result<(), Self::Error> {
        let old = self.read(at)?;
        if old.height > 0 || old != CellRecord::prepared(old.height, false) {
            return Err(MemoryError::ImmutableTerrain);
        }
        self.pages[at.0 as usize / PAGE_CELLS][at.0 as usize % PAGE_CELLS] =
            CellRecord::prepared(old.height, true);
        Ok(())
    }
    fn clear(&mut self, t: Tape) -> Result<(), Self::Error> {
        self.tapes[t.slot()].clear();
        Ok(())
    }
    // push rejects lengths above extent.cells(), which is strictly below u32::MAX.
    #[allow(clippy::cast_possible_truncation)]
    fn len(&self, t: Tape) -> u32 {
        self.tapes[t.slot()].len() as u32
    }
    fn push(&mut self, t: Tape, i: CellIndex) -> Result<(), Self::Error> {
        if i.0 >= self.extent.cells() {
            return Err(MemoryError::InvalidIndex);
        }
        if self.tapes[t.slot()].len() >= self.extent.cells() as usize {
            return Err(MemoryError::TapeCapacity);
        }
        self.tapes[t.slot()].push(i);
        Ok(())
    }
    fn get(&mut self, t: Tape, p: u32) -> Result<CellIndex, Self::Error> {
        self.tapes[t.slot()]
            .get(p as usize)
            .copied()
            .ok_or(MemoryError::InvalidTapePosition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn store(w: u32, h: u32, z: &[i32]) -> MemoryPages {
        let e = Extent::new(w, h).unwrap();
        MemoryPages::new(e, z, &vec![false; z.len()], 1 << 28).unwrap()
    }
    fn run(s: &mut MemoryPages) -> Work {
        route_and_own(s, Limits::for_extent(s.extent())).unwrap()
    }
    #[test]
    fn flat_reaches_lower_outlet_at_higher_identity() {
        let mut s = store(
            9,
            3,
            &[
                20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 10, 10, 10, 10, 10, 10, 9, 20, 20, 20, 20,
                20, 20, 20, 20, 20, 20,
            ],
        );
        run(&mut s);
        assert_eq!(s.read(CellIndex(10)).unwrap().owner(), Some(CellIndex(16)));
        let r = s.read(CellIndex(10)).unwrap();
        assert!(
            matches!(r.receiver(s.extent(),CellIndex(10)),Some(Receiver::Cell(next)) if next.0>10)
        );
    }
    #[test]
    fn multiple_outlets_and_neighbor_order_have_identical_ownership() {
        let z = vec![
            20, 20, 20, 20, 20, 20, 20, 20, 9, 10, 10, 10, 8, 20, 20, 20, 20, 20, 20, 20, 20,
        ];
        let mut a = store(7, 3, &z);
        let mut b = store(7, 3, &z);
        run(&mut a);
        let extent = b.extent();
        let mut p = Pass {
            store: &mut b,
            extent,
            limits: Limits::for_extent(extent),
            work: Work::default(),
        };
        p.receivers([7, 6, 5, 4, 3, 2, 1, 0]).unwrap();
        p.ownership().unwrap();
        for i in 0..21 {
            assert_eq!(a.read(CellIndex(i)).unwrap(), b.read(CellIndex(i)).unwrap());
        }
        assert_eq!(a.read(CellIndex(10)).unwrap().owner(), Some(CellIndex(8)));
        assert_eq!(a.read(CellIndex(11)).unwrap().owner(), Some(CellIndex(12)));
    }
    #[test]
    fn course_crosses_pages_and_area_width_without_cycle() {
        let w = 600;
        let mut z = vec![20; w * 3];
        for x in 1..w - 1 {
            z[w + x] = 10;
        }
        z[w + w - 2] = 9;
        let mut s = store(u32::try_from(w).unwrap(), 3, &z);
        let work = run(&mut s);
        assert!(work.cell_reads < 64 * s.extent().cells() as u64);
        let mut at = CellIndex(u32::try_from(w).unwrap() + 1);
        for _ in 0..w {
            let r = s.read(at).unwrap();
            assert_eq!(
                r.owner(),
                Some(CellIndex(u32::try_from(2 * w - 2).unwrap()))
            );
            match r.receiver(s.extent(), at).unwrap() {
                Receiver::Cell(next) => at = next,
                Receiver::Stop(OutletKind::ClosedDepression) => break,
                _ => panic!("unexpected terminal"),
            }
        }
        assert_eq!(at, CellIndex(u32::try_from(2 * w - 2).unwrap()));
        for page in 0..s.pages.len() {
            let bytes = s.encode_page(page).unwrap();
            s.replace_page(page, bytes).unwrap();
        }
    }
    #[test]
    fn marine_and_actual_outer_exit_remain_distinct() {
        let e = Extent::new(3, 3).unwrap();
        let mut z = vec![20; 9];
        z[3] = -2;
        z[4] = 1;
        let mut marine = vec![false; 9];
        marine[3] = true;
        let mut s = MemoryPages::new(e, &z, &marine, 1 << 20).unwrap();
        run(&mut s);
        assert_eq!(s.read(CellIndex(4)).unwrap().owner(), Some(CellIndex(3)));
        assert_eq!(
            s.read(CellIndex(3)).unwrap().receiver(e, CellIndex(3)),
            Some(Receiver::Stop(OutletKind::MarineEntry))
        );
        let mut flat = store(3, 3, &[10; 9]);
        run(&mut flat);
        assert_eq!(flat.read(CellIndex(4)).unwrap().owner(), Some(CellIndex(0)));
        assert_eq!(
            flat.read(CellIndex(0)).unwrap().receiver(e, CellIndex(0)),
            Some(Receiver::Stop(OutletKind::DomainExport))
        );
    }
    #[test]
    fn corrupt_cycle_and_resource_exhaustion_are_errors() {
        let mut s = store(3, 3, &[10; 9]);
        run(&mut s);
        let mut a = s.read(CellIndex(4)).unwrap();
        a.distance = 0;
        a.receiver = 4;
        s.write(CellIndex(4), a).unwrap();
        let mut b = s.read(CellIndex(5)).unwrap();
        b.distance = 0;
        b.receiver = 3;
        s.write(CellIndex(5), b).unwrap();
        assert!(matches!(
            resolve_ownership(&mut s, Limits::for_extent(Extent::new(3, 3).unwrap())),
            Err(RoutingError::InvalidTopology)
        ));
        let mut s = store(3, 3, &[10; 9]);
        let mut limits = Limits::for_extent(s.extent());
        limits.cell_reads = 1;
        assert!(matches!(
            route_and_own(&mut s, limits),
            Err(RoutingError::WorkLimit)
        ));
        assert!(MemoryPages::new(s.extent(), &[10; 9], &[false; 9], 1).is_none());
    }
    #[test]
    fn closed_plateau_has_one_canonical_owner_without_physical_epsilon() {
        let mut z = vec![20; 25];
        for y in 1..4 {
            for x in 1..4 {
                z[y * 5 + x] = 10;
            }
        }
        let mut s = store(5, 5, &z);
        run(&mut s);
        for (i, &height) in z.iter().enumerate() {
            let r = s.read(CellIndex(u32::try_from(i).unwrap())).unwrap();
            assert_eq!(r.height(), height);
            assert_eq!(r.owner(), Some(CellIndex(6)));
        }
        assert_eq!(
            s.read(CellIndex(6))
                .unwrap()
                .receiver(s.extent(), CellIndex(6)),
            Some(Receiver::Stop(OutletKind::ClosedDepression))
        );
    }
    #[test]
    fn varied_physical_heights_and_flats_obey_linear_limits_and_order() {
        let mut random = 436342u64;
        for _ in 0..32 {
            let mut z = Vec::new();
            for _ in 0..23 * 19 {
                random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                z.push(((random >> 32) % 7) as i32 - 3);
            }
            let mut a = store(23, 19, &z);
            let mut b = store(23, 19, &z);
            run(&mut a);
            let extent = b.extent();
            let mut p = Pass {
                store: &mut b,
                extent,
                limits: Limits::for_extent(extent),
                work: Work::default(),
            };
            p.receivers([5, 2, 7, 0, 3, 6, 1, 4]).unwrap();
            p.ownership().unwrap();
            for i in 0..extent.cells() {
                let r = a.read(CellIndex(i)).unwrap();
                assert_eq!(r, b.read(CellIndex(i)).unwrap());
                assert!(r.owner().is_some());
            }
            resolve_ownership(&mut a, Limits::for_extent(extent)).unwrap();
        }
    }
    #[test]
    fn checked_backend_rejects_cross_domain_and_immutable_writes() {
        let mut s = store(3, 3, &[10; 9]);
        let invalid = CellIndex::new(9, Extent::new(4, 4).unwrap()).unwrap();
        assert_eq!(s.read(invalid), Err(MemoryError::InvalidIndex));
        assert_eq!(
            s.get(Tape::Component, 0),
            Err(MemoryError::InvalidTapePosition)
        );
        assert_eq!(
            s.write(CellIndex(0), CellRecord::prepared(9, false)),
            Err(MemoryError::ImmutableTerrain)
        );
        let mut page = s.encode_page(0).unwrap();
        page[..4].copy_from_slice(&9i32.to_le_bytes());
        assert!(s.replace_page(0, page).is_none());
    }
    #[test]
    fn page_decoder_rejects_ghost_export_and_unreserved_bits() {
        let e = Extent::new(3, 3).unwrap();
        let mut r = CellRecord::prepared(10, false);
        r.receiver = EXPORT;
        assert!(CellRecord::decode(r.encode(), e, CellIndex(4)).is_none());
        let mut bytes = CellRecord::prepared(10, false).encode();
        bytes[15] = 1;
        assert!(CellRecord::decode(bytes, e, CellIndex(0)).is_none());
    }
}
