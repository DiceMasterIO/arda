//! Paged private hierarchy Store with explicit rows and failure-preserving writes.
use super::{
    hierarchy::{ElderLink, NodeRow, Store, UnionRow},
    private_rows::{self, RowError, ELDER_BYTES, NODE_BYTES, UNION_BYTES},
    routing::Extent,
};
use crate::orchestrator::child_links::{ChildError, DiskChildLinks};
use arda_core::{
    formats::hydrology::{decode_record, encode_record, BasinNodeRow, TableSpan},
    hydrology::BasinId,
};
use std::{
    collections::{BTreeMap, VecDeque},
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
const PAGE: usize = 4096;
const HEADER: u64 = 64;
const CACHE_CHARGE: u64 = 8192;
const BASIN_BYTES: usize = 76;
/// Independent reservations for these hierarchy files and cache, excluding child sorting.
#[derive(Debug, Clone, Copy)]
pub struct DiskLimits {
    /// Conservative cache allowance, 8192 bytes per page plus one transient page.
    pub cache_bytes: u64,
    /// Maximum combined logical lengths of private scratch and output files.
    pub scratch_bytes: u64,
    /// Requested payload read/write bytes, charged even on failure.
    pub io_bytes: u128,
    /// Seek, metadata and payload operations, charged before each call.
    pub io_operations: u64,
}
/// Counted hierarchy file I/O; child-sort I/O is reported by its separate owner.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DiskWork {
    /// Requested payload bytes.
    pub bytes: u128,
    /// Seek, metadata and payload calls attempted after successful admission.
    pub operations: u64,
}
/// Checked record, reservation, child-sort or actual filesystem failure.
#[derive(Debug, thiserror::Error)]
pub enum DiskError {
    /// Invalid lifecycle, index, header or table order.
    #[error("invalid hierarchy scratch: {0}")]
    Invalid(&'static str),
    /// A declared resource reservation was exhausted.
    #[error("hierarchy scratch exceeded {0}")]
    Limit(&'static str),
    /// Explicit private row failed schema or integrity validation.
    #[error(transparent)]
    Row(#[from] RowError),
    /// External child sorting/index lookup failed.
    #[error(transparent)]
    Child(#[from] ChildError),
    /// Actual operation failed on this private file.
    #[error("hierarchy I/O on {path}: {source}")]
    Io {
        /// Private file path.
        path: PathBuf,
        /// Underlying operating-system failure.
        #[source]
        source: std::io::Error,
    },
}
type Result<T> = std::result::Result<T, DiskError>;
struct Meter {
    limits: DiskLimits,
    work: DiskWork,
}
impl Meter {
    fn charge(&mut self, n: u128) -> Result<()> {
        let bytes = self
            .work
            .bytes
            .checked_add(n)
            .ok_or(DiskError::Limit("I/O bytes"))?;
        let operations = self
            .work
            .operations
            .checked_add(1)
            .ok_or(DiskError::Limit("I/O operations"))?;
        if bytes > self.limits.io_bytes {
            return Err(DiskError::Limit("I/O bytes"));
        }
        if operations > self.limits.io_operations {
            return Err(DiskError::Limit("I/O operations"));
        }
        self.work = DiskWork { bytes, operations };
        Ok(())
    }
}
struct Scratch {
    file: File,
    path: PathBuf,
}
impl Scratch {
    fn length(&self, m: &mut Meter) -> Result<u64> {
        m.charge(0)?;
        self.file
            .metadata()
            .map(|v| v.len())
            .map_err(|source| DiskError::Io {
                path: self.path.clone(),
                source,
            })
    }
    fn create(path: PathBuf) -> Result<Self> {
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
    fn seek(&mut self, at: u64, m: &mut Meter) -> Result<()> {
        m.charge(0)?;
        self.file
            .seek(SeekFrom::Start(at))
            .map_err(|source| DiskError::Io {
                path: self.path.clone(),
                source,
            })?;
        Ok(())
    }
    fn write(&mut self, at: u64, b: &[u8], m: &mut Meter) -> Result<()> {
        self.seek(at, m)?;
        m.charge(b.len() as u128)?;
        self.file.write_all(b).map_err(|source| DiskError::Io {
            path: self.path.clone(),
            source,
        })
    }
    fn read(&mut self, at: u64, b: &mut [u8], m: &mut Meter) -> Result<()> {
        self.seek(at, m)?;
        m.charge(b.len() as u128)?;
        self.file.read_exact(b).map_err(|source| DiskError::Io {
            path: self.path.clone(),
            source,
        })
    }
}
struct Cached {
    bytes: Box<[u8; PAGE]>,
    dirty: bool,
}
fn slot_bytes(kind: usize) -> usize {
    if kind == 0 {
        UNION_BYTES
    } else {
        NODE_BYTES
    }
}
fn slots(kind: usize) -> u64 {
    (PAGE / slot_bytes(kind)) as u64
}
fn pages(rows: u64, kind: usize) -> u64 {
    rows.div_ceil(slots(kind))
}
fn header(e: Extent, kind: usize, count: u64, width: u32) -> Result<[u8; 64]> {
    let mut h = [0; 64];
    h[..8].copy_from_slice(b"ARDAHSP1");
    h[8] = u8::try_from(kind).map_err(|_| DiskError::Invalid("header kind"))?;
    h[16..24].copy_from_slice(&count.to_le_bytes());
    h[24..28].copy_from_slice(&width.to_le_bytes());
    h[28..32].copy_from_slice(
        &u32::try_from(PAGE)
            .map_err(|_| DiskError::Invalid("page width"))?
            .to_le_bytes(),
    );
    let at =
        super::routing::CellIndex::new(e.cells() - 1, e).ok_or(DiskError::Invalid("extent"))?;
    let (x, y) = e.coordinates(at);
    h[32..36].copy_from_slice(&(x + 1).to_le_bytes());
    h[36..40].copy_from_slice(&(y + 1).to_le_bytes());
    let sum = private_rows::checksum(&h[..56]);
    h[56..].copy_from_slice(&sum.to_le_bytes());
    Ok(h)
}
/// Two paged scratch tables plus streamed checked containment/elder outputs.
///
/// The concrete child sorter is passed by its caller after whole-stage admission.
/// Its storage/resources remain independent. No file is created by `new`.
/// Call `finish` explicitly before exposing output; Drop never swallows flush errors.
pub struct DiskHierarchyStore {
    directory: PathBuf,
    extent: Extent,
    meter: Meter,
    children: DiskChildLinks,
    files: Option<[Scratch; 4]>,
    unions: u64,
    nodes: u64,
    capacity: usize,
    cache: BTreeMap<(usize, u64), Cached>,
    fifo: VecDeque<(usize, u64)>,
    emitted_nodes: u64,
    emitted_elders: u64,
    last_node: Option<u64>,
    last_elder: Option<u64>,
    finished: bool,
    children_finished: bool,
    child_count: u64,
    emitted_child_count: u64,
}
impl DiskHierarchyStore {
    /// Construct a file-free adapter with a pre-created independently admitted child sorter.
    pub fn new(
        directory: &Path,
        extent: Extent,
        limits: DiskLimits,
        children: DiskChildLinks,
    ) -> Result<Self> {
        if limits.cache_bytes < Self::cache_required(directory, 1)? {
            return Err(DiskError::Limit("cache/path bytes"));
        }
        Ok(Self {
            directory: directory.to_path_buf(),
            extent,
            meter: Meter {
                limits,
                work: DiskWork::default(),
            },
            children,
            files: None,
            unions: 0,
            nodes: 0,
            capacity: 0,
            cache: BTreeMap::new(),
            fifo: VecDeque::new(),
            emitted_nodes: 0,
            emitted_elders: 0,
            last_node: None,
            last_elder: None,
            finished: false,
            children_finished: false,
            child_count: 0,
            emitted_child_count: 0,
        })
    }
    fn path_charge(directory: &Path) -> Result<u64> {
        // Base path plus four file paths, doubled for path growth and one error
        // payload; the fixed suffix/separator allowance exceeds all four names.
        u64::try_from(directory.as_os_str().len())
            .ok()
            .and_then(|n| n.checked_mul(5))
            .and_then(|n| n.checked_add(128))
            .and_then(|n| n.checked_mul(2))
            .ok_or(DiskError::Limit("path bytes"))
    }
    /// Cache plus actual owned path admission; one transient page is included.
    pub fn cache_required(directory: &Path, cached_pages: u64) -> Result<u64> {
        if cached_pages == 0 {
            return Err(DiskError::Limit("cache pages"));
        }
        cached_pages
            .checked_add(1)
            .and_then(|n| n.checked_mul(CACHE_CHARGE))
            .and_then(|n| n.checked_add(Self::path_charge(directory).ok()?))
            .ok_or(DiskError::Limit("cache/path bytes"))
    }
    /// Required maximum physical file lengths, including page padding and all output rows.
    pub fn required_bytes(unions: u64, nodes: u64) -> Result<u64> {
        if unions == 0 {
            return Err(DiskError::Invalid("missing Exterior union row"));
        }
        let p = pages(unions, 0)
            .checked_add(pages(nodes, 1))
            .ok_or(DiskError::Limit("scratch bytes"))?;
        HEADER
            .checked_mul(4)
            .and_then(|v| v.checked_add(p.checked_mul(PAGE as u64)?))
            .and_then(|v| v.checked_add(nodes.checked_mul(BASIN_BYTES as u64)?))
            .and_then(|v| v.checked_add((unions - 1).checked_mul(ELDER_BYTES as u64)?))
            .ok_or(DiskError::Limit("scratch bytes"))
    }
    /// Actual requested file I/O so far, including initialization and failed attempts.
    pub fn work(&self) -> DiskWork {
        self.meter.work
    }
    /// Checked maximum file-length reservation for the currently initialized tables.
    pub fn reserved_bytes(&self) -> Result<u64> {
        Self::required_bytes(self.unions, self.nodes)
    }
    fn active(&self) -> Result<()> {
        if self.files.is_none() {
            Err(DiskError::Invalid("not reserved"))
        } else if self.finished {
            Err(DiskError::Invalid("already finished"))
        } else {
            Ok(())
        }
    }
    fn count(&self, kind: usize) -> u64 {
        if kind == 0 {
            self.unions
        } else {
            self.nodes
        }
    }
    fn save(&mut self, key: (usize, u64)) -> Result<()> {
        let page = self
            .cache
            .get(&key)
            .ok_or(DiskError::Invalid("cache entry"))?;
        if page.dirty {
            let files = self
                .files
                .as_mut()
                .ok_or(DiskError::Invalid("not reserved"))?;
            files[key.0].write(
                HEADER + key.1 * PAGE as u64,
                &page.bytes[..],
                &mut self.meter,
            )?;
        }
        Ok(())
    }
    fn ensure(&mut self, key: (usize, u64)) -> Result<()> {
        if self.cache.contains_key(&key) {
            return Ok(());
        }
        if self.cache.len() == self.capacity {
            let old = *self.fifo.front().ok_or(DiskError::Invalid("cache FIFO"))?;
            self.save(old)?;
            self.cache.remove(&old);
            self.fifo.pop_front();
        }
        let mut bytes = Box::new([0; PAGE]);
        let files = self
            .files
            .as_mut()
            .ok_or(DiskError::Invalid("not reserved"))?;
        let mut h = [0; 64];
        files[key.0].read(0, &mut h, &mut self.meter)?;
        if h != header(
            self.extent,
            key.0,
            self.count(key.0),
            u32::try_from(slot_bytes(key.0)).map_err(|_| DiskError::Invalid("slot width"))?,
        )? {
            return Err(DiskError::Invalid("scratch header"));
        }
        self.files
            .as_mut()
            .ok_or(DiskError::Invalid("not reserved"))?[key.0]
            .read(
                HEADER + key.1 * PAGE as u64,
                &mut bytes[..],
                &mut self.meter,
            )?;
        let expected = HEADER + pages(self.count(key.0), key.0) * PAGE as u64;
        if self
            .files
            .as_ref()
            .ok_or(DiskError::Invalid("not reserved"))?[key.0]
            .length(&mut self.meter)?
            != expected
        {
            return Err(DiskError::Invalid("scratch file length"));
        }
        let width = slot_bytes(key.0);
        let nslots = PAGE / width;
        for i in 0..nslots {
            let at = key.1 * slots(key.0) + i as u64;
            let row = &bytes[i * width..(i + 1) * width];
            if row.iter().all(|&v| v == 0) {
                continue;
            }
            if at >= self.count(key.0) {
                return Err(DiskError::Invalid("nonzero page padding"));
            }
            if key.0 == 0 {
                private_rows::decode_union(row, self.extent, at, self.unions, self.nodes)?;
            } else {
                private_rows::decode_node(row, self.extent, at, self.unions - 1, self.nodes)?;
            }
        }
        if bytes[nslots * width..].iter().any(|&b| b != 0) {
            return Err(DiskError::Invalid("page tail padding"));
        }
        self.cache.insert(
            key,
            Cached {
                bytes,
                dirty: false,
            },
        );
        self.fifo.push_back(key);
        Ok(())
    }
    fn location(&self, kind: usize, at: u64) -> Result<((usize, u64), usize)> {
        self.active()?;
        if kind > 1 || at >= self.count(kind) {
            return Err(DiskError::Invalid("record index"));
        }
        let per = slots(kind);
        let offset = usize::try_from(at % per).map_err(|_| DiskError::Invalid("slot index"))?
            * slot_bytes(kind);
        Ok(((kind, at / per), offset))
    }
    fn read_slot<const N: usize>(&mut self, kind: usize, at: u64) -> Result<[u8; N]> {
        let (key, offset) = self.location(kind, at)?;
        self.ensure(key)?;
        self.cache
            .get(&key)
            .ok_or(DiskError::Invalid("cache entry"))?
            .bytes[offset..offset + N]
            .try_into()
            .map_err(|_| DiskError::Invalid("slot width"))
    }
    fn put_slot(&mut self, kind: usize, at: u64, b: &[u8]) -> Result<()> {
        let (key, offset) = self.location(kind, at)?;
        self.ensure(key)?;
        let page = self
            .cache
            .get_mut(&key)
            .ok_or(DiskError::Invalid("cache entry"))?;
        page.bytes[offset..offset + b.len()].copy_from_slice(b);
        page.dirty = true;
        Ok(())
    }
    /// Flush in FIFO order; dirty pages remain readable after any failed write.
    pub fn flush(&mut self) -> Result<()> {
        self.active()?;
        while let Some(&key) = self.fifo.front() {
            self.save(key)?;
            self.cache.remove(&key);
            self.fifo.pop_front();
        }
        Ok(())
    }
    /// Finalize headers only after all elder rows and dirty scratch writes succeed.
    pub fn finish(&mut self) -> Result<()> {
        self.active()?;
        if !self.children_finished
            || self.emitted_elders != self.unions - 1
            || self.emitted_nodes < self.unions - 1
            || self.emitted_child_count != self.child_count
        {
            return Err(DiskError::Invalid("incomplete elder output"));
        }
        self.flush()?;
        let mut headers = [[0; 64]; 2];
        headers[0] = header(
            self.extent,
            2,
            self.emitted_nodes,
            u32::try_from(BASIN_BYTES).map_err(|_| DiskError::Invalid("basin width"))?,
        )?;
        headers[1] = header(
            self.extent,
            3,
            self.emitted_elders,
            u32::try_from(ELDER_BYTES).map_err(|_| DiskError::Invalid("elder width"))?,
        )?;
        let f = self
            .files
            .as_mut()
            .ok_or(DiskError::Invalid("not reserved"))?;
        if f[2].length(&mut self.meter)? != HEADER + self.emitted_nodes * BASIN_BYTES as u64
            || f[3].length(&mut self.meter)? != HEADER + self.emitted_elders * ELDER_BYTES as u64
        {
            return Err(DiskError::Invalid("output file length"));
        }
        f[2].write(0, &headers[0], &mut self.meter)?;
        f[3].write(0, &headers[1], &mut self.meter)?;
        self.finished = true;
        Ok(())
    }
    /// Number of complete public containment rows; usable only after finish.
    pub fn output_counts(&self) -> Result<(u64, u64)> {
        if !self.finished {
            return Err(DiskError::Invalid("unfinished output"));
        }
        Ok((self.emitted_nodes, self.emitted_elders))
    }
    fn output<const N: usize>(&mut self, kind: usize, at: u64, count: u64) -> Result<[u8; N]> {
        if !self.finished || at >= count {
            return Err(DiskError::Invalid("output index/state"));
        }
        let f = self
            .files
            .as_mut()
            .ok_or(DiskError::Invalid("not reserved"))?;
        let mut h = [0; 64];
        f[kind].read(0, &mut h, &mut self.meter)?;
        if h != header(
            self.extent,
            kind,
            count,
            u32::try_from(N).map_err(|_| DiskError::Invalid("row width"))?,
        )? {
            return Err(DiskError::Invalid("output header"));
        }
        if f[kind].length(&mut self.meter)? != HEADER + count * N as u64 {
            return Err(DiskError::Invalid("output file length"));
        }
        let mut b = [0; N];
        f[kind].read(HEADER + at * N as u64, &mut b, &mut self.meter)?;
        Ok(b)
    }
    /// Exact finalized private table paths for a bounded sequential consumer.
    /// Each starts with the validated 64-byte private header; remaining rows have
    /// the published fixed widths. No directory enumeration or whole-table load.
    pub fn output_paths(&self) -> Result<(&Path, &Path)> {
        self.output_counts()?;
        let f = self
            .files
            .as_ref()
            .ok_or(DiskError::Invalid("not reserved"))?;
        Ok((&f[2].path, &f[3].path))
    }
    /// Read one checked finalized containment row without loading its whole table.
    pub fn output_node(&mut self, at: u64) -> Result<BasinNodeRow> {
        let b = self.output::<BASIN_BYTES>(2, at, self.emitted_nodes)?;
        decode_record(&b).map_err(|_| DiskError::Invalid("public basin row"))
    }
    /// Read one checked finalized exclusive elder row.
    pub fn output_elder(&mut self, at: u64) -> Result<ElderLink> {
        let b = self.output::<ELDER_BYTES>(3, at, self.emitted_elders)?;
        Ok(private_rows::decode_elder(&b, self.extent)?)
    }
}
impl Store for DiskHierarchyStore {
    type Error = DiskError;
    fn reserve(&mut self, unions: u64, nodes: u64) -> Result<()> {
        if self.files.is_some() {
            return Err(DiskError::Invalid("repeated reserve"));
        }
        if unions == 0
            || unions - 1 > u64::from(self.extent.cells())
            || nodes != (unions - 1).saturating_mul(2).saturating_sub(1)
        {
            return Err(DiskError::Invalid("table counts"));
        }
        let available = self
            .meter
            .limits
            .cache_bytes
            .checked_sub(Self::path_charge(&self.directory)?)
            .ok_or(DiskError::Limit("path bytes"))?
            / CACHE_CHARGE;
        if available < 2 {
            return Err(DiskError::Limit("cache bytes"));
        }
        let capacity =
            usize::try_from(available - 1).map_err(|_| DiskError::Limit("cache count"))?;
        if Self::required_bytes(unions, nodes)? > self.meter.limits.scratch_bytes {
            return Err(DiskError::Limit("scratch bytes"));
        }
        let n_pages = pages(unions, 0) + pages(nodes, 1);
        let initial = 4 * HEADER + n_pages * PAGE as u64;
        if u128::from(initial) > self.meter.limits.io_bytes {
            return Err(DiskError::Limit("initial I/O bytes"));
        }
        if 2 * (4 + n_pages) > self.meter.limits.io_operations {
            return Err(DiskError::Limit("initial I/O operations"));
        }
        let mut files = [
            Scratch::create(self.directory.join("hierarchy-unions.pages"))?,
            Scratch::create(self.directory.join("hierarchy-nodes.pages"))?,
            Scratch::create(self.directory.join("hierarchy-basins.rows"))?,
            Scratch::create(self.directory.join("hierarchy-elders.rows"))?,
        ];
        for (kind, f) in files.iter_mut().enumerate() {
            let (count, width) = match kind {
                0 => (unions, UNION_BYTES),
                1 => (nodes, NODE_BYTES),
                2 => (0, BASIN_BYTES),
                _ => (0, ELDER_BYTES),
            };
            f.write(
                0,
                &header(
                    self.extent,
                    kind,
                    count,
                    u32::try_from(width).map_err(|_| DiskError::Invalid("row width"))?,
                )?,
                &mut self.meter,
            )?;
            if kind < 2 {
                for page in 0..pages(count, kind) {
                    f.write(HEADER + page * PAGE as u64, &[0; PAGE], &mut self.meter)?;
                }
            }
        }
        self.files = Some(files);
        self.unions = unions;
        self.nodes = nodes;
        self.capacity = capacity;
        Ok(())
    }
    fn union(&mut self, at: u64) -> Result<UnionRow> {
        let b = self.read_slot::<UNION_BYTES>(0, at)?;
        Ok(private_rows::decode_union(
            &b,
            self.extent,
            at,
            self.unions,
            self.nodes,
        )?)
    }
    fn put_union(&mut self, at: u64, row: UnionRow) -> Result<()> {
        self.active()?;
        let b = private_rows::encode_union(row, self.extent, at, self.unions, self.nodes)?;
        self.put_slot(0, at, &b)
    }
    fn node(&mut self, at: u64) -> Result<NodeRow> {
        let b = self.read_slot::<NODE_BYTES>(1, at)?;
        Ok(private_rows::decode_node(
            &b,
            self.extent,
            at,
            self.unions - 1,
            self.nodes,
        )?)
    }
    fn put_node(&mut self, at: u64, row: NodeRow) -> Result<()> {
        self.active()?;
        let b = private_rows::encode_node(row, self.extent, at, self.unions - 1, self.nodes)?;
        self.put_slot(1, at, &b)
    }
    fn link(&mut self, parent: BasinId, child: BasinId) -> Result<()> {
        self.active()?;
        Ok(self.children.push(parent, child)?)
    }
    fn finish_links(&mut self, count: u64) -> Result<()> {
        self.active()?;
        self.children.finish(count)?;
        self.child_count = count;
        self.children_finished = true;
        Ok(())
    }
    fn child_span(&mut self, parent: BasinId) -> Result<TableSpan> {
        self.active()?;
        Ok(self.children.span(parent)?)
    }
    fn emit(&mut self, row: BasinNodeRow) -> Result<()> {
        self.active()?;
        if !self.children_finished
            || self.emitted_nodes >= self.nodes
            || self.last_node.is_some_and(|id| id >= row.id.0)
        {
            return Err(DiskError::Invalid("basin output count/order"));
        }
        let child_total = self
            .emitted_child_count
            .checked_add(row.children.count)
            .ok_or(DiskError::Invalid("child count overflow"))?;
        if child_total > self.child_count {
            return Err(DiskError::Invalid("too many child references"));
        }
        let b = encode_record(&row).map_err(|_| DiskError::Invalid("public basin codec"))?;
        self.files
            .as_mut()
            .ok_or(DiskError::Invalid("not reserved"))?[2]
            .write(
                HEADER + self.emitted_nodes * BASIN_BYTES as u64,
                &b,
                &mut self.meter,
            )?;
        self.emitted_nodes += 1;
        self.last_node = Some(row.id.0);
        self.emitted_child_count = child_total;
        Ok(())
    }
    fn emit_elder(&mut self, row: ElderLink) -> Result<()> {
        self.active()?;
        if self.emitted_elders >= self.unions - 1
            || self.last_elder.is_some_and(|id| id >= row.leaf.0)
        {
            return Err(DiskError::Invalid("elder output count/order"));
        }
        let b = private_rows::encode_elder(row, self.extent)?;
        self.files
            .as_mut()
            .ok_or(DiskError::Invalid("not reserved"))?[3]
            .write(
                HEADER + self.emitted_elders * ELDER_BYTES as u64,
                &b,
                &mut self.meter,
            )?;
        self.emitted_elders += 1;
        self.last_elder = Some(row.leaf.0);
        Ok(())
    }
}

#[cfg(test)]
#[path = "disk_tests.rs"]
mod tests;
