//! One concrete bounded paged Boruvka slot store; no world-sized label vector.
use super::{
    io::{self, Meter, Result, Scratch, StageError},
    routing::{CellIndex, Extent},
    saddles::Saddle,
};
use std::{
    collections::{BTreeMap, VecDeque},
    path::Path,
};
const PAGE: usize = 4096;
const PER: u64 = 64;
const WIDTH: usize = 64;
const HEADER: u64 = 64;
/// Immutable minimum plus mutable union/frozen-round/minimum-edge fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    /// Actual minimum ordinal; None only at the final Exterior slot.
    pub minimum: Option<CellIndex>,
    /// Physical minimum elevation; Exterior uses i32::MIN.
    pub floor_mm: i32,
    /// Current union parent ordinal.
    pub parent: u32,
    /// Union-by-rank value, at most31 within the supported domain.
    pub rank: u8,
    /// Immutable component label during the current candidate scan.
    pub frozen: u32,
    /// Canonical outgoing edge for this frozen component, when present.
    pub best: Option<Saddle>,
}
#[cfg(test)]
#[path = "slot_tests.rs"]
mod tests;
fn encode(v: Slot, e: Extent, at: u32, n: u32) -> Result<[u8; 64]> {
    let mut b = [0; 64];
    b[..4].copy_from_slice(&v.minimum.map_or(u32::MAX, CellIndex::raw).to_le_bytes());
    b[4..8].copy_from_slice(&v.floor_mm.to_le_bytes());
    b[8..12].copy_from_slice(&v.parent.to_le_bytes());
    b[12..16].copy_from_slice(&v.frozen.to_le_bytes());
    b[16] = v.rank;
    b[17] = u8::from(v.best.is_some());
    b[20..24].copy_from_slice(&at.to_le_bytes());
    if let Some(edge) = v.best {
        b[24..48].copy_from_slice(&edge.encode());
    }
    let h = io::checksum(&b[..56]);
    b[56..].copy_from_slice(&h.to_le_bytes());
    decode(&b, e, at, n)?;
    Ok(b)
}
fn decode(b: &[u8], e: Extent, at: u32, n: u32) -> Result<Slot> {
    if b.len() != 64 {
        return Err(StageError::Invalid("slot width"));
    }
    let u = |i| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let h = u64::from_le_bytes(
        b[56..]
            .try_into()
            .map_err(|_| StageError::Invalid("slot checksum"))?,
    );
    if h != io::checksum(&b[..56])
        || b[18..20].iter().chain(&b[48..56]).any(|&v| v != 0)
        || u(20) != at
    {
        return Err(StageError::Invalid("slot checksum/padding"));
    }
    let minimum = if u(0) == u32::MAX {
        None
    } else {
        Some(CellIndex::new(u(0), e).ok_or(StageError::Invalid("slot minimum extent"))?)
    };
    let floor_mm = i32::from_le_bytes(
        b[4..8]
            .try_into()
            .map_err(|_| StageError::Invalid("floor width"))?,
    );
    let parent = u(8);
    let frozen = u(12);
    let rank = b[16];
    if n == 0
        || at >= n
        || parent >= n
        || frozen >= n
        || rank > 31
        || minimum.is_some() != (at + 1 < n)
        || (minimum.is_none() && floor_mm != i32::MIN)
    {
        return Err(StageError::Invalid("slot identity/range"));
    }
    let best = match b[17] {
        0 => {
            if b[24..48].iter().any(|&v| v != 0) {
                return Err(StageError::Invalid("absent best payload"));
            }
            None
        }
        1 => Some(
            Saddle::decode(
                b[24..48]
                    .try_into()
                    .map_err(|_| StageError::Invalid("edge width"))?,
                e,
            )
            .ok_or(StageError::Invalid("best witness"))?,
        ),
        _ => return Err(StageError::Invalid("best tag")),
    };
    Ok(Slot {
        minimum,
        floor_mm,
        parent,
        rank,
        frozen,
        best,
    })
}
struct Page {
    bytes: Box<[u8; PAGE]>,
    dirty: bool,
}
/// Counted fixed-slot operations, independent of file I/O counters.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SlotWork {
    /// Initialized slots read, including repeated cached accesses.
    pub reads: u64,
    /// Updated slots written, including cached updates.
    pub writes: u64,
    /// Explicit ordinal/edge/union ordering comparisons.
    pub comparisons: u64,
}
#[derive(Clone, Copy)]
pub(crate) struct Limits {
    pub cache_pages: u64,
    pub reads: u64,
    pub writes: u64,
    pub comparisons: u64,
}
/// Bounded coherent FIFO cache over exactly L+1 encoded fixed slots.
pub struct SlotStore {
    file: Scratch,
    extent: Extent,
    count: u32,
    capacity: usize,
    cache: BTreeMap<u64, Page>,
    fifo: VecDeque<u64>,
    meter: Meter,
    read_limit: u64,
    write_limit: u64,
    comparison_limit: u64,
    work: SlotWork,
}
impl SlotStore {
    /// Exact paged-file size including its checked64-byte header.
    pub fn required_bytes(count: u32) -> u64 {
        HEADER + u64::from(count).div_ceil(PER) * PAGE as u64
    }
    /// Conservative cache/metadata/transient and owned path payload admission.
    pub fn required_ram(directory: &Path, cache_pages: u64) -> Result<u64> {
        if cache_pages == 0 {
            return Err(StageError::Limit("cache pages"));
        }
        let paths = u64::try_from(directory.as_os_str().len())
            .ok()
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| n.checked_add(256))
            .ok_or(StageError::Limit("path bytes"))?;
        cache_pages
            .checked_add(1)
            .and_then(|n| n.checked_mul(8192))
            .and_then(|n| n.checked_add(paths))
            .ok_or(StageError::Limit("cache bytes"))
    }
    pub(crate) fn create(
        directory: &Path,
        e: Extent,
        leaves: u32,
        minima: &mut Scratch,
        mut meter: Meter,
        limits: Limits,
    ) -> Result<Self> {
        let count = leaves
            .checked_add(1)
            .ok_or(StageError::Limit("slot count"))?;
        if u64::from(leaves) > u64::from(e.cells()) {
            return Err(StageError::Invalid("leaf count"));
        }
        let capacity =
            usize::try_from(limits.cache_pages).map_err(|_| StageError::Limit("cache count"))?;
        if capacity == 0 {
            return Err(StageError::Limit("cache pages"));
        }
        let mut header = [0; 64];
        minima.read(0, &mut header, &mut meter)?;
        if header != io::header(e, 0, u64::from(leaves), 16)?
            || minima.length(&mut meter)? != 64 + u64::from(leaves) * 16
        {
            return Err(StageError::Invalid("minimum header/length"));
        }
        let mut file = Scratch::create(directory.join("mst-slots.pages"), &mut meter)?;
        file.write(0, &io::header(e, 1, u64::from(count), 64)?, &mut meter)?;
        let mut previous = None;
        let mut work = SlotWork::default();
        for page in 0..u64::from(count).div_ceil(PER) {
            let mut b = [0; PAGE];
            let start = page * PER;
            let closed = u64::from(leaves).saturating_sub(start).min(PER);
            let mut input = [0; 1024];
            if closed > 0 {
                minima.read(
                    HEADER + start * 16,
                    &mut input
                        [..usize::try_from(closed).map_err(|_| StageError::Invalid("batch"))? * 16],
                    &mut meter,
                )?;
            }
            for i in 0..PER {
                let at = start + i;
                if at >= u64::from(count) {
                    break;
                }
                let at = u32::try_from(at).map_err(|_| StageError::Invalid("slot ordinal"))?;
                let (minimum, floor_mm) = if at < leaves {
                    let j = usize::try_from(i).map_err(|_| StageError::Invalid("slot position"))?;
                    let m = io::minimum(&input[j * 16..j * 16 + 16], e)?;
                    if previous.is_some() {
                        if work.comparisons >= limits.comparisons {
                            return Err(StageError::Limit("key comparisons"));
                        }
                        work.comparisons += 1;
                    }
                    if previous.is_some_and(|old| old >= m.at) {
                        return Err(StageError::Invalid("minimum order"));
                    }
                    previous = Some(m.at);
                    (Some(m.at), m.floor_mm)
                } else {
                    (None, i32::MIN)
                };
                let offset =
                    usize::try_from(i).map_err(|_| StageError::Invalid("slot position"))? * WIDTH;
                b[offset..offset + WIDTH].copy_from_slice(&encode(
                    Slot {
                        minimum,
                        floor_mm,
                        parent: at,
                        rank: 0,
                        frozen: at,
                        best: None,
                    },
                    e,
                    at,
                    count,
                )?);
            }
            file.write(HEADER + page * PAGE as u64, &b, &mut meter)?;
        }
        Ok(Self {
            file,
            extent: e,
            count,
            capacity,
            cache: BTreeMap::new(),
            fifo: VecDeque::new(),
            meter,
            read_limit: limits.reads,
            write_limit: limits.writes,
            comparison_limit: limits.comparisons,
            work,
        })
    }
    /// Number of actual components/minima slots, including Exterior.
    pub fn count(&self) -> u32 {
        self.count
    }
    /// Slot work already charged.
    pub fn work(&self) -> SlotWork {
        self.work
    }
    fn save(&mut self, p: u64) -> Result<()> {
        let page = self
            .cache
            .get(&p)
            .ok_or(StageError::Invalid("cache entry"))?;
        if page.dirty {
            self.file
                .write(HEADER + p * PAGE as u64, &page.bytes[..], &mut self.meter)?;
        }
        Ok(())
    }
    fn ensure(&mut self, p: u64) -> Result<()> {
        if self.cache.contains_key(&p) {
            return Ok(());
        }
        if self.cache.len() == self.capacity {
            let old = *self.fifo.front().ok_or(StageError::Invalid("cache FIFO"))?;
            self.save(old)?;
            self.cache.remove(&old);
            self.fifo.pop_front();
        }
        let mut h = [0; 64];
        self.file.read(0, &mut h, &mut self.meter)?;
        if h != io::header(self.extent, 1, u64::from(self.count), 64)?
            || self.file.length(&mut self.meter)? != Self::required_bytes(self.count)
        {
            return Err(StageError::Invalid("slot header/length"));
        }
        let mut bytes = Box::new([0; PAGE]);
        self.file
            .read(HEADER + p * PAGE as u64, &mut bytes[..], &mut self.meter)?;
        for i in 0..PER {
            let at = p * PER + i;
            let j = usize::try_from(i).map_err(|_| StageError::Invalid("slot position"))?;
            let row = &bytes[j * WIDTH..j * WIDTH + WIDTH];
            if at < u64::from(self.count) {
                decode(
                    row,
                    self.extent,
                    u32::try_from(at).map_err(|_| StageError::Invalid("slot ordinal"))?,
                    self.count,
                )?;
            } else if row.iter().any(|&v| v != 0) {
                return Err(StageError::Invalid("page padding"));
            }
        }
        self.cache.insert(
            p,
            Page {
                bytes,
                dirty: false,
            },
        );
        self.fifo.push_back(p);
        Ok(())
    }
    /// Read an initialized slot, charging before any access.
    pub fn read(&mut self, at: u32) -> Result<Slot> {
        if at >= self.count {
            return Err(StageError::Invalid("slot index"));
        }
        if self.work.reads >= self.read_limit {
            return Err(StageError::Limit("slot reads"));
        }
        self.work.reads += 1;
        let page = u64::from(at) / PER;
        self.ensure(page)?;
        let j = usize::try_from(u64::from(at) % PER)
            .map_err(|_| StageError::Invalid("slot position"))?
            * WIDTH;
        decode(
            &self
                .cache
                .get(&page)
                .ok_or(StageError::Invalid("cache entry"))?
                .bytes[j..j + WIDTH],
            self.extent,
            at,
            self.count,
        )
    }
    /// Write a valid indexed slot, preserving its immutable physical minimum.
    pub fn write(&mut self, at: u32, row: Slot) -> Result<()> {
        if at >= self.count {
            return Err(StageError::Invalid("slot index"));
        }
        if self.work.writes >= self.write_limit {
            return Err(StageError::Limit("slot writes"));
        }
        self.work.writes += 1;
        let b = encode(row, self.extent, at, self.count)?;
        let page = u64::from(at) / PER;
        self.ensure(page)?;
        let j = usize::try_from(u64::from(at) % PER)
            .map_err(|_| StageError::Invalid("slot position"))?
            * WIDTH;
        let p = self
            .cache
            .get_mut(&page)
            .ok_or(StageError::Invalid("cache entry"))?;
        let old = decode(&p.bytes[j..j + WIDTH], self.extent, at, self.count)?;
        if (old.minimum, old.floor_mm) != (row.minimum, row.floor_mm) {
            return Err(StageError::Invalid("changed immutable minimum"));
        }
        p.bytes[j..j + WIDTH].copy_from_slice(&b);
        p.dirty = true;
        Ok(())
    }
    /// Charge one explicit algorithmic ordering/equality comparison before execution.
    pub fn compare(&mut self) -> Result<()> {
        if self.work.comparisons >= self.comparison_limit {
            return Err(StageError::Limit("key comparisons"));
        }
        self.work.comparisons += 1;
        Ok(())
    }
    /// Current union representative with a fixed rank-derived traversal ceiling.
    pub fn root(&mut self, mut at: u32) -> Result<u32> {
        for _ in 0..32 {
            let r = self.read(at)?;
            if r.parent == at {
                return Ok(at);
            }
            at = r.parent;
        }
        Err(StageError::Cycle)
    }
    /// Union two distinct live components, retaining deterministic rank/ordinal ties.
    pub fn join(&mut self, a: u32, b: u32) -> Result<bool> {
        let a = self.root(a)?;
        let b = self.root(b)?;
        if a == b {
            return Ok(false);
        }
        let ua = self.read(a)?;
        let ub = self.read(b)?;
        self.compare()?;
        let (root, mut ur, child, mut uc) =
            if (ua.rank, std::cmp::Reverse(a)) >= (ub.rank, std::cmp::Reverse(b)) {
                (a, ua, b, ub)
            } else {
                (b, ub, a, ua)
            };
        self.compare()?;
        if ur.rank == uc.rank {
            ur.rank = ur
                .rank
                .checked_add(1)
                .filter(|&n| n <= 31)
                .ok_or(StageError::Invalid("union rank"))?;
        }
        uc.parent = root;
        self.write(root, ur)?;
        self.write(child, uc)?;
        Ok(true)
    }
    /// Exact indexed binding from an actual closed receiver terminal to its slot.
    pub fn bind(&mut self, at: CellIndex) -> Result<u32> {
        let (mut low, mut high) = (0, self.count - 1);
        while low < high {
            let mid = low + (high - low) / 2;
            let m = self
                .read(mid)?
                .minimum
                .ok_or(StageError::Invalid("missing minimum"))?;
            self.compare()?;
            if m < at {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        if low == self.count - 1 {
            return Err(StageError::Invalid("unknown closed owner"));
        }
        let found = self.read(low)?.minimum;
        self.compare()?;
        if found != Some(at) {
            return Err(StageError::Invalid("unknown closed owner"));
        }
        Ok(low)
    }
    /// Explicitly flush dirty pages; failed pages remain coherent and readable.
    pub fn flush(&mut self) -> Result<()> {
        while let Some(&p) = self.fifo.front() {
            self.save(p)?;
            self.cache.remove(&p);
            self.fifo.pop_front();
        }
        Ok(())
    }
    pub(crate) fn finish(mut self) -> Result<Meter> {
        self.flush()?;
        Ok(self.meter)
    }
}
