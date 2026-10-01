//! Bounded private scratch for global routing. No final area file is read.

use super::routing::{CellIndex, CellRecord, Extent, RoutingStore, Tape, PAGE_CELLS};
use std::{
    collections::{BTreeMap, VecDeque},
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

const PAGE_BYTES: usize = 4096;
const HEADER_BYTES: u64 = 32;
const CACHE_CHARGE: u64 = 8192;
const MAGIC: &[u8; 8] = b"ARDARTP1";

/// Caller-owned resource reservation, checked before creating any files.
#[derive(Debug, Clone, Copy)]
pub struct DiskLimits {
    /// Cache payload allowance; includes one transient page and conservative map overhead.
    pub cache_bytes: u64,
    /// Full page file plus both N-entry ordinal tapes, including padding/header.
    pub scratch_bytes: u64,
    /// Maximum requested read/write payload bytes, including initial page creation.
    pub io_bytes: u128,
    /// Maximum seeks and payload read/write operations.
    pub io_operations: u64,
}

/// Physical I/O counters; logical routing reads have their own separate limits.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DiskWork {
    /// Requested payload bytes; a failing operation remains charged.
    pub bytes: u128,
    /// Seeks and payload operations attempted after successful admission.
    pub operations: u64,
}

/// An invalid scratch request, corrupted private record or actual filesystem failure.
#[derive(Debug, thiserror::Error)]
pub enum DiskError {
    /// Input, record or immutable terrain contract was violated.
    #[error("invalid routing scratch: {0}")]
    Invalid(&'static str),
    /// A caller reservation was exhausted before allocating/writing the next resource.
    #[error("routing scratch exceeded {0}")]
    Limit(&'static str),
    /// A scratch file operation failed.
    #[error("routing scratch I/O on {path}: {source}")]
    Io {
        /// Exact private file involved.
        path: PathBuf,
        /// Original operating-system error.
        #[source]
        source: std::io::Error,
    },
}

/// Distinguishes a fallible prepared-source error from routing scratch failure.
#[derive(Debug, thiserror::Error)]
pub enum CreateError<E> {
    /// Original prepared-file or source-stream failure, without substituting EOF.
    #[error("routing prepared source failed")]
    Input(#[source] E),
    /// Routing scratch admission or physical I/O failure.
    #[error(transparent)]
    Storage(#[from] DiskError),
}

struct Scratch {
    file: File,
    path: PathBuf,
}
impl Scratch {
    fn create(path: PathBuf) -> Result<Self, DiskError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|source| DiskError::Io {
                path: path.clone(),
                source,
            })?;
        Ok(Self { file, path })
    }
    fn seek(&mut self, offset: u64, meter: &mut Meter) -> Result<(), DiskError> {
        meter.charge(0)?;
        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(|source| DiskError::Io {
                path: self.path.clone(),
                source,
            })?;
        Ok(())
    }
    fn read(&mut self, offset: u64, bytes: &mut [u8], meter: &mut Meter) -> Result<(), DiskError> {
        self.seek(offset, meter)?;
        meter.charge(bytes.len() as u128)?;
        self.file.read_exact(bytes).map_err(|source| DiskError::Io {
            path: self.path.clone(),
            source,
        })
    }
    fn write(&mut self, offset: u64, bytes: &[u8], meter: &mut Meter) -> Result<(), DiskError> {
        self.seek(offset, meter)?;
        meter.charge(bytes.len() as u128)?;
        self.file.write_all(bytes).map_err(|source| DiskError::Io {
            path: self.path.clone(),
            source,
        })
    }
}

struct Meter {
    limits: DiskLimits,
    work: DiskWork,
}
impl Meter {
    fn charge(&mut self, bytes: u128) -> Result<(), DiskError> {
        let next_bytes = self
            .work
            .bytes
            .checked_add(bytes)
            .ok_or(DiskError::Limit("I/O bytes"))?;
        let next_operations = self
            .work
            .operations
            .checked_add(1)
            .ok_or(DiskError::Limit("I/O operations"))?;
        if next_bytes > self.limits.io_bytes {
            return Err(DiskError::Limit("I/O bytes"));
        }
        if next_operations > self.limits.io_operations {
            return Err(DiskError::Limit("I/O operations"));
        }
        self.work = DiskWork {
            bytes: next_bytes,
            operations: next_operations,
        };
        Ok(())
    }
}

struct Page {
    bytes: Box<[u8; PAGE_BYTES]>,
    dirty: bool,
}
struct OrdinalTape {
    file: Scratch,
    length: u32,
}

/// Coherent FIFO page cache plus two bounded reusable on-disk ordinal tapes.
///
/// The directory is private to one generation transaction. File names are fixed;
/// an existing file is refused. Call `flush` before consuming the page artifact.
/// Drop only closes files and never hides an unsuccessful dirty-page write.
pub struct DiskRoutingStore {
    extent: Extent,
    records: Scratch,
    tapes: [OrdinalTape; 2],
    cache: BTreeMap<u32, Page>,
    fifo: VecDeque<u32>,
    capacity: usize,
    meter: Meter,
}

fn slot(tape: Tape) -> usize {
    match tape {
        Tape::Component => 0,
        Tape::Frontier => 1,
    }
}

impl DiskRoutingStore {
    /// Required physical file-length reservation for the domain, including page padding.
    #[must_use]
    pub fn scratch_required(extent: Extent) -> u64 {
        let n = u64::from(extent.cells());
        HEADER_BYTES + n.div_ceil(PAGE_CELLS as u64) * PAGE_BYTES as u64 + 8 * n
    }

    /// Streams fresh immutable records into private scratch within explicit limits.
    ///
    /// `prepared` supplies exactly N fresh records in row-major order. The cache
    /// starts empty, so the constructor never retains a full terrain grid.
    ///
    /// # Errors
    /// Rejects insufficient reservations, wrong record counts, non-fresh records,
    /// existing scratch files, or any failed filesystem operation.
    // Retained for storage validation and optional stage diagnostics.
    #[allow(dead_code)]
    pub fn create(
        directory: &Path,
        extent: Extent,
        prepared: impl IntoIterator<Item = CellRecord>,
        limits: DiskLimits,
    ) -> Result<Self, DiskError> {
        match Self::try_create(
            directory,
            extent,
            prepared.into_iter().map(Ok::<_, std::convert::Infallible>),
            limits,
        ) {
            Ok(store) => Ok(store),
            Err(CreateError::Storage(error)) => Err(error),
            Err(CreateError::Input(impossible)) => match impossible {},
        }
    }

    /// Streams an actual fallible prepared-file reader, preserving its original
    /// error separately from short input or a failed routing scratch operation.
    /// Source failure leaves private partial scratch and prevents publication.
    ///
    /// # Errors
    /// Returns original source failures or the same bounded scratch failures as create.
    pub fn try_create<E>(
        directory: &Path,
        extent: Extent,
        prepared: impl IntoIterator<Item = Result<CellRecord, E>>,
        limits: DiskLimits,
    ) -> Result<Self, CreateError<E>> {
        let available = limits.cache_bytes / CACHE_CHARGE;
        if available < 2 {
            return Err(DiskError::Limit("cache bytes").into());
        }
        let capacity =
            usize::try_from(available - 1).map_err(|_| DiskError::Limit("cache entries"))?;
        if Self::scratch_required(extent) > limits.scratch_bytes {
            return Err(DiskError::Limit("scratch bytes").into());
        }
        let pages = u64::from(extent.cells()).div_ceil(PAGE_CELLS as u64);
        let initial_bytes = HEADER_BYTES + pages * PAGE_BYTES as u64;
        if u128::from(initial_bytes) > limits.io_bytes {
            return Err(DiskError::Limit("I/O bytes").into());
        }
        if 2 * (1 + pages) > limits.io_operations {
            return Err(DiskError::Limit("I/O operations").into());
        }
        let mut meter = Meter {
            limits,
            work: DiskWork::default(),
        };
        let mut records = Scratch::create(directory.join("routing.pages"))?;
        let tapes = [
            OrdinalTape {
                file: Scratch::create(directory.join("component.tape"))?,
                length: 0,
            },
            OrdinalTape {
                file: Scratch::create(directory.join("frontier.tape"))?,
                length: 0,
            },
        ];
        let last =
            CellIndex::new(extent.cells() - 1, extent).ok_or(DiskError::Invalid("empty extent"))?;
        let (last_x, last_y) = extent.coordinates(last);
        let mut header = [0_u8; 32];
        header[..8].copy_from_slice(MAGIC);
        header[8..12].copy_from_slice(&(last_x + 1).to_le_bytes());
        header[12..16].copy_from_slice(&(last_y + 1).to_le_bytes());
        header[16..20].copy_from_slice(&extent.cells().to_le_bytes());
        header[20..24].copy_from_slice(&(16_u32).to_le_bytes());
        records.write(0, &header, &mut meter)?;
        let mut source = prepared.into_iter();
        for page in 0..pages {
            let mut bytes = [0; PAGE_BYTES];
            for position in 0..PAGE_CELLS {
                let ordinal = page * PAGE_CELLS as u64 + position as u64;
                let record = if ordinal < u64::from(extent.cells()) {
                    let record = source
                        .next()
                        .transpose()
                        .map_err(CreateError::Input)?
                        .ok_or(DiskError::Invalid("too few prepared records"))?;
                    if record.encode()
                        != CellRecord::prepared(record.height(), record.is_marine()).encode()
                    {
                        return Err(DiskError::Invalid("prepared record is not fresh").into());
                    }
                    record
                } else {
                    CellRecord::prepared(0, false)
                };
                bytes[position * 16..position * 16 + 16].copy_from_slice(&record.encode());
            }
            records.write(HEADER_BYTES + page * PAGE_BYTES as u64, &bytes, &mut meter)?;
        }
        if source
            .next()
            .transpose()
            .map_err(CreateError::Input)?
            .is_some()
        {
            return Err(DiskError::Invalid("too many prepared records").into());
        }
        Ok(Self {
            extent,
            records,
            tapes,
            cache: BTreeMap::new(),
            fifo: VecDeque::new(),
            capacity,
            meter,
        })
    }

    /// Exact private page path for the next stage; no directory enumeration is needed.
    #[must_use]
    // Retained for storage validation and optional stage diagnostics.
    #[allow(dead_code)]
    pub fn records_path(&self) -> &Path {
        &self.records.path
    }

    /// Requested physical I/O charged so far, including initialization.
    #[must_use]
    // Retained for storage validation and optional stage diagnostics.
    #[allow(dead_code)]
    pub fn work(&self) -> DiskWork {
        self.meter.work
    }

    fn location(&self, at: CellIndex) -> Result<(u32, usize), DiskError> {
        if at.raw() >= self.extent.cells() {
            return Err(DiskError::Invalid("cell index"));
        }
        let page = at.raw() / 256;
        let offset =
            usize::try_from(at.raw() % 256).map_err(|_| DiskError::Invalid("cell offset"))? * 16;
        Ok((page, offset))
    }
    fn save_cached(&mut self, id: u32) -> Result<(), DiskError> {
        let page = self
            .cache
            .get(&id)
            .ok_or(DiskError::Invalid("missing cache entry"))?;
        if page.dirty {
            self.records.write(
                HEADER_BYTES + u64::from(id) * PAGE_BYTES as u64,
                &page.bytes[..],
                &mut self.meter,
            )?;
        }
        Ok(())
    }
    fn ensure(&mut self, id: u32) -> Result<(), DiskError> {
        if self.cache.contains_key(&id) {
            return Ok(());
        }
        if self.cache.len() == self.capacity {
            let evicted = *self.fifo.front().ok_or(DiskError::Invalid("cache FIFO"))?;
            self.save_cached(evicted)?;
            self.cache.remove(&evicted);
            self.fifo.pop_front();
        }
        let mut bytes = Box::new([0; PAGE_BYTES]);
        self.records.read(
            HEADER_BYTES + u64::from(id) * PAGE_BYTES as u64,
            &mut bytes[..],
            &mut self.meter,
        )?;
        for position in 0..PAGE_CELLS {
            let ordinal = u64::from(id) * PAGE_CELLS as u64 + position as u64;
            let row: [u8; 16] = bytes[position * 16..position * 16 + 16]
                .try_into()
                .map_err(|_| DiskError::Invalid("record width"))?;
            if ordinal < u64::from(self.extent.cells()) {
                let at = CellIndex::new(
                    u32::try_from(ordinal).map_err(|_| DiskError::Invalid("ordinal"))?,
                    self.extent,
                )
                .ok_or(DiskError::Invalid("ordinal"))?;
                CellRecord::decode(row, self.extent, at)
                    .ok_or(DiskError::Invalid("stored record"))?;
            } else if row != CellRecord::prepared(0, false).encode() {
                return Err(DiskError::Invalid("page padding"));
            }
        }
        self.cache.insert(
            id,
            Page {
                bytes,
                dirty: false,
            },
        );
        self.fifo.push_back(id);
        Ok(())
    }

    /// Writes every dirty page before the next stage consumes its records.
    ///
    /// # Errors
    /// Propagates exhausted I/O budgets and write failures; failure aborts publication.
    pub fn flush(&mut self) -> Result<(), DiskError> {
        while let Some(&id) = self.fifo.front() {
            self.save_cached(id)?;
            self.cache.remove(&id);
            self.fifo.pop_front();
        }
        Ok(())
    }
}

impl RoutingStore for DiskRoutingStore {
    type Error = DiskError;
    fn extent(&self) -> Extent {
        self.extent
    }
    fn read(&mut self, at: CellIndex) -> Result<CellRecord, Self::Error> {
        let (page, offset) = self.location(at)?;
        self.ensure(page)?;
        let page = self
            .cache
            .get(&page)
            .ok_or(DiskError::Invalid("cache read"))?;
        let bytes = page.bytes[offset..offset + 16]
            .try_into()
            .map_err(|_| DiskError::Invalid("record width"))?;
        CellRecord::decode(bytes, self.extent, at).ok_or(DiskError::Invalid("stored record"))
    }
    fn write(&mut self, at: CellIndex, record: CellRecord) -> Result<(), Self::Error> {
        let old = self.read(at)?;
        if (old.height(), old.is_marine()) != (record.height(), record.is_marine()) {
            return Err(DiskError::Invalid("immutable terrain"));
        }
        let encoded = record.encode();
        CellRecord::decode(encoded, self.extent, at)
            .ok_or(DiskError::Invalid("replacement record"))?;
        let (page, offset) = self.location(at)?;
        let cached = self
            .cache
            .get_mut(&page)
            .ok_or(DiskError::Invalid("cache write"))?;
        cached.bytes[offset..offset + 16].copy_from_slice(&encoded);
        cached.dirty = true;
        Ok(())
    }
    fn mark_marine(&mut self, at: CellIndex) -> Result<(), Self::Error> {
        let old = self.read(at)?;
        if old.height() > 0 || old != CellRecord::prepared(old.height(), false) {
            return Err(DiskError::Invalid("fresh marine flood cell"));
        }
        let (page, offset) = self.location(at)?;
        let cached = self
            .cache
            .get_mut(&page)
            .ok_or(DiskError::Invalid("marine cache write"))?;
        cached.bytes[offset..offset + 16]
            .copy_from_slice(&CellRecord::prepared(old.height(), true).encode());
        cached.dirty = true;
        Ok(())
    }
    fn clear(&mut self, tape: Tape) -> Result<(), Self::Error> {
        self.tapes[slot(tape)].length = 0;
        Ok(())
    }
    fn len(&self, tape: Tape) -> u32 {
        self.tapes[slot(tape)].length
    }
    fn push(&mut self, tape: Tape, at: CellIndex) -> Result<(), Self::Error> {
        self.location(at)?;
        let tape = &mut self.tapes[slot(tape)];
        if tape.length >= self.extent.cells() {
            return Err(DiskError::Limit("ordinal tape entries"));
        }
        tape.file.write(
            u64::from(tape.length) * 4,
            &at.raw().to_le_bytes(),
            &mut self.meter,
        )?;
        tape.length += 1;
        Ok(())
    }
    fn get(&mut self, tape: Tape, position: u32) -> Result<CellIndex, Self::Error> {
        let tape = &mut self.tapes[slot(tape)];
        if position >= tape.length {
            return Err(DiskError::Invalid("unwritten tape position"));
        }
        let mut bytes = [0; 4];
        tape.file
            .read(u64::from(position) * 4, &mut bytes, &mut self.meter)?;
        CellIndex::new(u32::from_le_bytes(bytes), self.extent)
            .ok_or(DiskError::Invalid("stored tape ordinal"))
    }
}

#[cfg(test)]
mod tests;
