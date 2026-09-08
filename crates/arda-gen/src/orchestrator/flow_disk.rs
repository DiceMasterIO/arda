//! Bounded private fine-flow records; no fine-cell flow array is retained in RAM.
#![deny(missing_docs)]

use crate::hydrology::fine_flow::{FlowMetrics, FlowRecord, FlowStore};
use crate::hydrology::routing::{CellIndex, Extent};
use arda_core::hydrology::BasinId;
use std::collections::{BTreeMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const HEADER: usize = 64;
const PAGE: usize = 4096;
const ROW: usize = 80;
const PER_PAGE: u64 = 51;

/// Admission applies to this handle; the transaction also charges every replay handle.
#[derive(Debug, Clone, Copy)]
pub struct FlowLimits {
    /// Maximum resident pages, including dirty pages awaiting successful eviction.
    pub cache_pages: u32,
    /// Cache, transient page, index, queue and path payload reservation.
    pub ram_bytes: u64,
    /// Complete private file length.
    pub scratch_bytes: u64,
    /// Attempted payload bytes, including failed actual I/O.
    pub io_bytes: u128,
    /// Attempted opens, metadata, seeks and payload operations.
    pub io_operations: u64,
}

/// Actual attempted private I/O; cache hits do not incur file I/O.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FlowWork {
    /// Charged payload bytes.
    pub bytes: u128,
    /// Charged filesystem operations.
    pub operations: u64,
}

/// A checked private-record, admission or filesystem failure.
#[derive(Debug, thiserror::Error)]
pub enum FlowDiskError {
    /// File or record authority is inconsistent.
    #[error("invalid fine-flow storage: {0}")]
    Invalid(&'static str),
    /// Admission, allocation or I/O ceiling failed.
    #[error("fine-flow storage exceeded {0}")]
    Limit(&'static str),
    /// Original filesystem failure at the exact owned path.
    #[error("fine-flow I/O on {path}: {source}")]
    Io {
        /// Actual private file involved.
        path: PathBuf,
        /// Original operating-system error.
        #[source]
        source: std::io::Error,
    },
}
type Result<T> = std::result::Result<T, FlowDiskError>;

/// A source error remains distinct from failure to persist its generated record.
#[derive(Debug, thiserror::Error)]
pub enum CreateError<E> {
    /// Original prepared/forcing/annual source failure.
    #[error("fine-flow source failed")]
    Input(#[source] E),
    /// Exact private storage failure.
    #[error(transparent)]
    Storage(#[from] FlowDiskError),
}

fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}
fn invalid(reason: &'static str) -> FlowDiskError {
    FlowDiskError::Invalid(reason)
}
fn get<const N: usize>(b: &[u8], at: usize) -> Result<[u8; N]> {
    b.get(at..at + N)
        .ok_or_else(|| invalid("short record"))?
        .try_into()
        .map_err(|_| invalid("record slice"))
}

/// Complete file payload, including initialized final-page padding.
#[must_use]
pub fn scratch_required(extent: Extent) -> u64 {
    64 + u64::from(extent.cells()).div_ceil(PER_PAGE) * 4096
}
/// Conservative cache and temporary-buffer bound checked before cloning paths.
pub fn ram_required(path: &Path, cache_pages: u32) -> Result<u64> {
    if cache_pages == 0 {
        return Err(invalid("empty cache"));
    }
    let path_bytes = u64::try_from(path.as_os_str().as_encoded_bytes().len())
        .map_err(|_| FlowDiskError::Limit("path length"))?;
    u64::from(cache_pages)
        .checked_mul(8192)
        .and_then(|n| n.checked_add(16384))
        .and_then(|n| path_bytes.checked_mul(4).and_then(|p| n.checked_add(p)))
        .ok_or(FlowDiskError::Limit("RAM arithmetic"))
}
fn admit(path: &Path, extent: Extent, limits: FlowLimits) -> Result<()> {
    if ram_required(path, limits.cache_pages)? > limits.ram_bytes
        || scratch_required(extent) > limits.scratch_bytes
    {
        return Err(FlowDiskError::Limit("RAM or scratch"));
    }
    Ok(())
}

fn header(extent: Extent) -> Result<[u8; HEADER]> {
    let last = CellIndex::new(extent.cells() - 1, extent).ok_or_else(|| invalid("empty domain"))?;
    let (x, y) = extent.coordinates(last);
    let mut b = [0; HEADER];
    b[..8].copy_from_slice(b"ARDAFL01");
    b[8..12].copy_from_slice(&(x + 1).to_le_bytes());
    b[12..16].copy_from_slice(&(y + 1).to_le_bytes());
    b[16..24].copy_from_slice(&u64::from(extent.cells()).to_le_bytes());
    b[24..28].copy_from_slice(&80_u32.to_le_bytes());
    b[28..32].copy_from_slice(&4096_u32.to_le_bytes());
    let sum = checksum(&b[..56]);
    b[56..64].copy_from_slice(&sum.to_le_bytes());
    Ok(b)
}

/// Explicit little-endian scratch row; unused payload has one canonical zero encoding.
pub fn encode_record(record: FlowRecord, extent: Extent) -> Result<[u8; ROW]> {
    if record.parent.is_some() && !record.visited || record.lake.is_none() && record.surface_mm != 0
    {
        return Err(invalid("record state"));
    }
    let m = record.metrics;
    if record
        .parent
        .is_some_and(|p| CellIndex::new(p.raw(), extent).is_none())
        || m.hand_at
            .is_some_and(|p| CellIndex::new(p.raw(), extent).is_none())
    {
        return Err(invalid("reference outside domain"));
    }
    // D8 confluences can have eight equal-order tributaries. Two is the
    // Strahler promotion threshold, not the maximum stored contribution count.
    if m.pending > 8 || m.max_in_ties > 8 || (m.hand_at.is_none() && m.hand_distance_mm != u64::MAX)
    {
        return Err(invalid("metrics state"));
    }
    let mut b = [0; ROW];
    b[..16].copy_from_slice(&record.net.to_le_bytes());
    if let Some(p) = record.parent {
        b[16..20].copy_from_slice(&p.raw().to_le_bytes());
    }
    b[20] = u8::from(record.visited)
        | (u8::from(record.parent.is_some()) << 1)
        | (u8::from(record.exterior) << 2)
        | (u8::from(record.lake.is_some()) << 3);
    b[21] = record.selected;
    b[22..26].copy_from_slice(&record.surface_mm.to_le_bytes());
    if let Some(id) = record.lake {
        b[26..34].copy_from_slice(&id.0.to_le_bytes());
    }
    b[34] = u8::from(m.hand_at.is_some());
    b[35] = m.pending;
    b[36] = m.max_in_order;
    b[37] = m.max_in_ties;
    b[38] = m.order;
    b[40..44].copy_from_slice(&m.catchment_cells.to_le_bytes());
    b[44..48].copy_from_slice(&m.drainage_cells.to_le_bytes());
    b[48..56].copy_from_slice(&m.hand_distance_mm.to_le_bytes());
    if let Some(at) = m.hand_at {
        b[56..60].copy_from_slice(&at.raw().to_le_bytes());
    }
    b[60..68].copy_from_slice(&m.scalar_annual.to_le_bytes());
    let sum = checksum(&b[..72]);
    b[72..80].copy_from_slice(&sum.to_le_bytes());
    Ok(b)
}

/// Validate checksum, flags, padding and references before any record enters the cache.
pub fn decode_record(b: &[u8], extent: Extent) -> Result<FlowRecord> {
    if b.len() != ROW
        || b[20] & !15 != 0
        || b[34] > 1
        || b[39] != 0
        || b[68..72].iter().any(|&n| n != 0)
        || u64::from_le_bytes(get(b, 72)?) != checksum(&b[..72])
    {
        return Err(invalid("record integrity"));
    }
    let parent_raw = u32::from_le_bytes(get(b, 16)?);
    let parent = if b[20] & 2 != 0 {
        Some(CellIndex::new(parent_raw, extent).ok_or_else(|| invalid("parent ordinal"))?)
    } else {
        if parent_raw != 0 {
            return Err(invalid("absent parent payload"));
        }
        None
    };
    let id = u64::from_le_bytes(get(b, 26)?);
    let lake = if b[20] & 8 != 0 {
        Some(BasinId(id))
    } else {
        if id != 0 {
            return Err(invalid("absent lake payload"));
        }
        None
    };
    let hand_raw = u32::from_le_bytes(get(b, 56)?);
    let hand_at = if b[34] == 1 {
        Some(CellIndex::new(hand_raw, extent).ok_or_else(|| invalid("HAND ordinal"))?)
    } else {
        if hand_raw != 0 {
            return Err(invalid("absent HAND payload"));
        }
        None
    };
    let record = FlowRecord {
        net: i128::from_le_bytes(get(b, 0)?),
        parent,
        visited: b[20] & 1 != 0,
        selected: b[21],
        exterior: b[20] & 4 != 0,
        lake,
        surface_mm: i32::from_le_bytes(get(b, 22)?),
        metrics: FlowMetrics {
            catchment_cells: u32::from_le_bytes(get(b, 40)?),
            drainage_cells: u32::from_le_bytes(get(b, 44)?),
            pending: b[35],
            max_in_order: b[36],
            max_in_ties: b[37],
            order: b[38],
            hand_distance_mm: u64::from_le_bytes(get(b, 48)?),
            hand_at,
            scalar_annual: u64::from_le_bytes(get(b, 60)?),
        },
    };
    encode_record(record, extent)?;
    Ok(record)
}

struct Meter {
    limits: FlowLimits,
    work: FlowWork,
}
impl Meter {
    fn charge(&mut self, bytes: u128) -> Result<()> {
        let n = self
            .work
            .operations
            .checked_add(1)
            .ok_or(FlowDiskError::Limit("I/O operations"))?;
        let b = self
            .work
            .bytes
            .checked_add(bytes)
            .ok_or(FlowDiskError::Limit("I/O bytes"))?;
        if n > self.limits.io_operations || b > self.limits.io_bytes {
            return Err(FlowDiskError::Limit("I/O"));
        }
        self.work = FlowWork {
            bytes: b,
            operations: n,
        };
        Ok(())
    }
}
struct CachedPage {
    bytes: Box<[u8; PAGE]>,
    dirty: bool,
}

/// A single private fixed-record file with bounded FIFO pages and explicit fallible flush.
/// Drop closes handles; it never hides a failed write by silently flushing dirty records.
pub struct DiskFlowStore {
    path: PathBuf,
    extent: Extent,
    file: File,
    cache: BTreeMap<u64, CachedPage>,
    fifo: VecDeque<u64>,
    meter: Meter,
}
impl DiskFlowStore {
    fn io_error(&self, source: std::io::Error) -> FlowDiskError {
        FlowDiskError::Io {
            path: self.path.clone(),
            source,
        }
    }
    /// Fresh exclusive creation from a canonical fallible source, one record per ordinal.
    /// An interrupted file remains private and cannot be used as completed solved input.
    pub fn create<E>(
        path: &Path,
        extent: Extent,
        input: impl IntoIterator<Item = std::result::Result<FlowRecord, E>>,
        limits: FlowLimits,
    ) -> std::result::Result<Self, CreateError<E>> {
        admit(path, extent, limits)?;
        let length = scratch_required(extent);
        let page_count = u64::from(extent.cells()).div_ceil(PER_PAGE);
        if limits.io_bytes < u128::from(length) || limits.io_operations < 2 + page_count {
            return Err(FlowDiskError::Limit("initialization I/O").into());
        }
        let mut meter = Meter {
            limits,
            work: FlowWork::default(),
        };
        meter.charge(0)?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|source| FlowDiskError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        meter.charge(64)?;
        file.write_all(&header(extent)?)
            .map_err(|source| FlowDiskError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        let mut input = input.into_iter();
        for page in 0..page_count {
            let mut bytes = [0; PAGE];
            for slot in 0..PER_PAGE {
                let raw = page * PER_PAGE + slot;
                if raw >= u64::from(extent.cells()) {
                    break;
                }
                let record = input
                    .next()
                    .ok_or_else(|| invalid("short source"))?
                    .map_err(CreateError::Input)?;
                if record.visited
                    || record.parent.is_some()
                    || record.selected != 0
                    || record.exterior
                    || record.metrics
                        != (FlowMetrics {
                            scalar_annual: record.metrics.scalar_annual,
                            ..FlowMetrics::default()
                        })
                {
                    return Err(invalid("source already routed").into());
                }
                let start =
                    usize::try_from(slot).map_err(|_| FlowDiskError::Limit("page slot"))? * ROW;
                bytes[start..start + ROW].copy_from_slice(&encode_record(record, extent)?);
            }
            meter.charge(PAGE as u128)?;
            file.write_all(&bytes).map_err(|source| FlowDiskError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        }
        if input
            .next()
            .transpose()
            .map_err(CreateError::Input)?
            .is_some()
        {
            return Err(invalid("long source").into());
        }
        Self::from_file(path, extent, file, meter).map_err(CreateError::Storage)
    }

    fn from_file(path: &Path, extent: Extent, file: File, meter: Meter) -> Result<Self> {
        let mut fifo = VecDeque::new();
        fifo.try_reserve_exact(
            usize::try_from(meter.limits.cache_pages).map_err(|_| FlowDiskError::Limit("cache"))?,
        )
        .map_err(|_| FlowDiskError::Limit("cache allocation"))?;
        Ok(Self {
            path: path.to_path_buf(),
            extent,
            file,
            cache: BTreeMap::new(),
            fifo,
            meter,
        })
    }

    /// Reopen an explicitly completed/flushed private source with a separately charged budget.
    // Retained for storage validation and optional stage diagnostics.
    #[allow(dead_code)]
    pub fn open(path: &Path, extent: Extent, limits: FlowLimits) -> Result<Self> {
        admit(path, extent, limits)?;
        let mut meter = Meter {
            limits,
            work: FlowWork::default(),
        };
        meter.charge(0)?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|source| FlowDiskError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        meter.charge(0)?;
        if file
            .metadata()
            .map_err(|source| FlowDiskError::Io {
                path: path.to_path_buf(),
                source,
            })?
            .len()
            != scratch_required(extent)
        {
            return Err(invalid("file length"));
        }
        let mut h = [0; HEADER];
        meter.charge(HEADER as u128)?;
        file.read_exact(&mut h)
            .map_err(|source| FlowDiskError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        if h != header(extent)? {
            return Err(invalid("header"));
        }
        Self::from_file(path, extent, file, meter)
    }
    /// Actual attempted I/O for the transaction's global ledger.
    #[must_use]
    // Retained for storage validation and optional stage diagnostics.
    #[allow(dead_code)]
    pub fn work(&self) -> FlowWork {
        self.meter.work
    }

    fn save(&mut self, page: u64) -> Result<()> {
        let cached = self
            .cache
            .get(&page)
            .ok_or_else(|| invalid("cache queue"))?;
        if !cached.dirty {
            return Ok(());
        }
        // Retain both the dirty page and its FIFO slot until the write succeeds.
        self.meter.charge(0)?;
        self.file
            .seek(SeekFrom::Start(64 + page * 4096))
            .map_err(|e| self.io_error(e))?;
        self.meter.charge(PAGE as u128)?;
        let cached = self
            .cache
            .get(&page)
            .ok_or_else(|| invalid("cache queue"))?;
        self.file
            .write_all(cached.bytes.as_ref())
            .map_err(|e| self.io_error(e))?;
        self.cache
            .get_mut(&page)
            .ok_or_else(|| invalid("cache queue"))?
            .dirty = false;
        Ok(())
    }
    fn load(&mut self, page: u64) -> Result<()> {
        if self.cache.contains_key(&page) {
            return Ok(());
        }
        if self.cache.len()
            >= usize::try_from(self.meter.limits.cache_pages)
                .map_err(|_| FlowDiskError::Limit("cache count"))?
        {
            let oldest = *self.fifo.front().ok_or_else(|| invalid("cache FIFO"))?;
            self.save(oldest)?;
            self.fifo.pop_front();
            self.cache.remove(&oldest);
        }
        let mut bytes = Box::new([0; PAGE]);
        self.meter.charge(0)?;
        self.file
            .seek(SeekFrom::Start(64 + page * 4096))
            .map_err(|e| self.io_error(e))?;
        self.meter.charge(PAGE as u128)?;
        self.file
            .read_exact(bytes.as_mut())
            .map_err(|e| self.io_error(e))?;
        for slot in 0..PER_PAGE {
            let start = usize::try_from(slot).map_err(|_| FlowDiskError::Limit("page slot"))? * ROW;
            if page * PER_PAGE + slot < u64::from(self.extent.cells()) {
                decode_record(&bytes[start..start + ROW], self.extent)?;
            } else if bytes[start..start + ROW].iter().any(|&b| b != 0) {
                return Err(invalid("unused record padding"));
            }
        }
        if bytes[4080..].iter().any(|&b| b != 0) {
            return Err(invalid("page padding"));
        }
        self.cache.insert(
            page,
            CachedPage {
                bytes,
                dirty: false,
            },
        );
        self.fifo.push_back(page);
        Ok(())
    }
    /// Persist every dirty page in canonical FIFO order; failures retain unflushed state.
    pub fn flush(&mut self) -> Result<()> {
        while let Some(&page) = self.fifo.front() {
            self.save(page)?;
            self.fifo.pop_front();
            self.cache.remove(&page);
        }
        Ok(())
    }
}

impl FlowStore for DiskFlowStore {
    type Error = FlowDiskError;
    fn extent(&self) -> Extent {
        self.extent
    }
    fn read(&mut self, at: CellIndex) -> Result<FlowRecord> {
        if CellIndex::new(at.raw(), self.extent).is_none() {
            return Err(invalid("cell ordinal"));
        }
        let page = u64::from(at.raw()) / PER_PAGE;
        let slot = usize::try_from(u64::from(at.raw()) % PER_PAGE)
            .map_err(|_| FlowDiskError::Limit("page slot"))?;
        self.load(page)?;
        let b = &self
            .cache
            .get(&page)
            .ok_or_else(|| invalid("loaded page"))?
            .bytes;
        decode_record(&b[slot * ROW..(slot + 1) * ROW], self.extent)
    }
    fn write(&mut self, at: CellIndex, record: FlowRecord) -> Result<()> {
        if CellIndex::new(at.raw(), self.extent).is_none() {
            return Err(invalid("cell ordinal"));
        }
        let encoded = encode_record(record, self.extent)?;
        let page = u64::from(at.raw()) / PER_PAGE;
        let slot = usize::try_from(u64::from(at.raw()) % PER_PAGE)
            .map_err(|_| FlowDiskError::Limit("page slot"))?;
        self.load(page)?;
        let cached = self
            .cache
            .get_mut(&page)
            .ok_or_else(|| invalid("loaded page"))?;
        cached.bytes[slot * ROW..(slot + 1) * ROW].copy_from_slice(&encoded);
        cached.dirty = true;
        Ok(())
    }
}

#[cfg(test)]
#[path = "flow_disk_tests.rs"]
mod flow_disk_tests;
