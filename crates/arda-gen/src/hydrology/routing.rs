//! Exact global flat routing and receiver-chain ownership for prepared terrain.
//!
//! The backend owns only private prepared/scratch pages, never final area files.
//! Physical heights and the already-solved marine mask are immutable during routing.
//! Before routing, the same scratch supports a checked connected-ocean flood.
#![deny(missing_docs)]

mod memory;
mod pass;
pub use memory::{MemoryError, MemoryPages};
pub use pass::{resolve_ownership, route_and_own};

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

#[cfg(test)]
mod tests;
