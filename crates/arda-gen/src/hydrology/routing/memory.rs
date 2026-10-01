//! The in-memory reference [`RoutingStore`]: checked 4 KiB record pages and
//! the four routing tapes.

use super::*;

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
    pub(super) pages: Vec<Box<[CellRecord; PAGE_CELLS]>>,
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
