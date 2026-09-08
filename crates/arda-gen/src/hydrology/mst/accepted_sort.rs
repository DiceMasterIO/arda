//! Bounded external ordering of accepted physical MST witnesses.
use super::routing::{CellIndex, Extent};
use super::saddles::{Node, Saddle};
use std::{
    cmp::Ordering,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
const FAN_IN: u64 = 8;
const BLOCK: usize = 4096;
const ROW: u64 = 32;
const MAGIC: &[u8; 8] = b"ARDAMST1";
type Result<T> = std::result::Result<T, SortError>;
/// Private-stage rejection. Partial files are owned by the parent transaction.
#[derive(Debug, thiserror::Error)]
pub enum SortError {
    /// An actual filesystem operation failed.
    #[error("accepted-edge storage failed: {0}")]
    Io(#[from] std::io::Error),
    /// A count, byte offset or resource reservation overflowed.
    #[error("accepted-edge arithmetic overflow")]
    Overflow,
    /// The supplied edge, capacity or phase is invalid.
    #[error("invalid accepted-edge input: {0}")]
    Invalid(&'static str),
    /// A declared resource ceiling was insufficient or exhausted.
    #[error("accepted-edge resource limit: {0}")]
    Limit(&'static str),
    /// Two records have the same canonical hierarchy key.
    #[error("duplicate accepted edge")]
    Duplicate,
    /// Private bytes violate the checked codec or ordering.
    #[error("corrupt accepted-edge stream: {0}")]
    Corrupt(&'static str),
    /// A prior operation aborted this owner.
    #[error("accepted-edge owner is poisoned")]
    Poisoned,
}
fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or(SortError::Overflow)
}
fn mul(a: u64, b: u64) -> Result<u64> {
    a.checked_mul(b).ok_or(SortError::Overflow)
}
fn size(a: u64) -> Result<usize> {
    usize::try_from(a).map_err(|_| SortError::Overflow)
}
/// Unique hierarchy ordering for an edge already validated in this extent.
#[must_use]
pub fn edge_key(extent: Extent, e: Saddle) -> (i32, u64, u64, u64, u64) {
    let node = |v| match v {
        Node::Closed(at) => extent.anchor_key(at),
        Node::Exterior => u64::MAX,
    };
    let a = extent.anchor_key(e.from);
    let b = e.to.map_or(u64::MAX, |at| extent.anchor_key(at));
    (e.sill_mm, node(e.left), node(e.right), a.min(b), a.max(b))
}
/// Construction and one complete sequential final-reader reservation.
#[derive(Clone, Copy, Debug)]
pub struct SortLimits {
    /// Requested maximum in-memory accepted-edge sort batch.
    pub buffer_records: u64,
    /// Owned payload and explicit metadata/path allowance.
    pub ram_bytes: u64,
    /// Peak simultaneous private file bytes, including retained final run.
    pub scratch_bytes: u64,
    /// Attempted buffered read/write bytes, including all headers.
    pub io_bytes: u128,
    /// Explicit attempted filesystem calls, including metadata and sync.
    pub io_operations: u64,
    /// Key comparisons, including comparisons inside merge sorting.
    pub comparisons: u64,
}
impl SortLimits {
    /// Checks all construction plus one-pass final-reader bounds before mutation.
    ///
    /// # Errors
    /// Rejects a zero buffer or an unrepresentable count/reservation.
    pub fn required(directory: &Path, expected_edges: u64, buffer_records: u64) -> Result<Self> {
        if buffer_records == 0 {
            return Err(SortError::Invalid("zero run buffer"));
        }
        let r = buffer_records.min(expected_edges.max(1));
        let first_runs = expected_edges.div_ceil(r).max(1);
        let (mut width, mut runs, mut all_runs, mut levels, mut blocks) = (r, first_runs, 0, 0, 0);
        loop {
            all_runs = add(all_runs, runs)?;
            levels = add(levels, 1)?;
            // Sum exact header-inclusive writer blocks and data-reader blocks.
            let full = expected_edges / width;
            let tail = expected_edges % width;
            let full_blocks = add(add(width, 1)?.div_ceil(128), width.div_ceil(128))?;
            let mut at_level = mul(full, full_blocks)?;
            if tail != 0 {
                at_level = add(
                    at_level,
                    add(add(tail, 1)?.div_ceil(128), tail.div_ceil(128))?,
                )?;
            }
            if expected_edges == 0 {
                at_level = 1;
            }
            blocks = add(blocks, at_level)?;
            if runs == 1 {
                break;
            }
            runs = runs.div_ceil(FAN_IN);
            width = u64::try_from(
                (u128::from(width) * u128::from(FAN_IN)).min(u128::from(expected_edges.max(1))),
            )
            .map_err(|_| SortError::Overflow)?;
        }
        let path = u64::try_from(directory.as_os_str().as_encoded_bytes().len())
            .map_err(|_| SortError::Overflow)?;
        let row_memory =
            u64::try_from(std::mem::size_of::<Saddle>()).map_err(|_| SortError::Overflow)?;
        let ram_bytes = add(add(mul(mul(2, row_memory)?, r)?, 65536)?, mul(3, path)?)?;
        size(ram_bytes)?;
        let log = if r <= 1 {
            0
        } else {
            u64::from(64 - (r - 1).leading_zeros())
        };
        let comparisons = mul(expected_edges, add(add(log, mul(9, levels - 1)?)?, 2)?)?;
        let io_bytes = u128::from(
            64u64
                .checked_mul(expected_edges)
                .ok_or(SortError::Overflow)?,
        )
        .checked_mul(u128::from(levels))
        .and_then(|v| v.checked_add(64 * u128::from(all_runs)))
        .ok_or(SortError::Overflow)?;
        Ok(Self {
            buffer_records: r,
            ram_bytes,
            scratch_bytes: add(mul(64, expected_edges)?, mul(64, first_runs)?)?,
            io_bytes,
            io_operations: add(add(mul(7, all_runs)?, blocks)?, 2)?,
            comparisons,
        })
    }
}
/// Separate reservation for each additional pass over the retained accepted file.
/// No sorting or scratch-file mutation is performed by a replay.
#[derive(Debug, Clone, Copy)]
pub struct ReadLimits {
    /// One 4096-byte buffer plus reader/path/allocator metadata allowance.
    pub ram_bytes: u64,
    /// Exact attempted header and complete row bytes.
    pub io_bytes: u128,
    /// Open, metadata, header-read and buffered row-read calls.
    pub io_operations: u64,
    /// One order comparison after each row except the first.
    pub comparisons: u64,
}
impl ReadLimits {
    /// Admission for one complete additional checked pass, before opening its file.
    ///
    /// # Errors
    /// Rejects count, offset or path reservation overflow.
    pub fn required(path: &Path, expected_edges: u64) -> Result<Self> {
        let path_bytes = u64::try_from(path.as_os_str().as_encoded_bytes().len())
            .map_err(|_| SortError::Overflow)?;
        let bytes = add(32, mul(ROW, expected_edges)?)?;
        let ram_bytes = add(8192, path_bytes)?;
        size(ram_bytes)?;
        Ok(Self {
            ram_bytes,
            io_bytes: u128::from(bytes),
            io_operations: add(3, expected_edges.div_ceil(128))?,
            comparisons: expected_edges.saturating_sub(1),
        })
    }
}
/// Actual requested work, including a failed attempted I/O call.
#[derive(Clone, Copy, Debug, Default)]
pub struct SortWork {
    /// Attempted buffered read/write bytes.
    pub io_bytes: u128,
    /// Attempted explicit filesystem calls.
    pub io_operations: u64,
    /// Canonical edge-key comparisons.
    pub comparisons: u64,
}
struct Budget {
    limits: SortLimits,
    work: SortWork,
}
impl Budget {
    fn io(&mut self, n: usize) -> Result<()> {
        let bytes = self
            .work
            .io_bytes
            .checked_add(n as u128)
            .ok_or(SortError::Overflow)?;
        if bytes > self.limits.io_bytes || self.work.io_operations >= self.limits.io_operations {
            return Err(SortError::Limit("I/O"));
        }
        self.work.io_bytes = bytes;
        self.work.io_operations += 1;
        Ok(())
    }
    fn compare(&mut self, extent: Extent, a: Saddle, b: Saddle) -> Result<Ordering> {
        if self.work.comparisons >= self.limits.comparisons {
            return Err(SortError::Limit("comparisons"));
        }
        self.work.comparisons += 1;
        Ok(edge_key(extent, a).cmp(&edge_key(extent, b)))
    }
}
fn header(extent: Extent, count: u64) -> Result<[u8; 32]> {
    let last = CellIndex::new(extent.cells() - 1, extent).ok_or(SortError::Invalid("extent"))?;
    let (x, y) = extent.coordinates(last);
    let mut b = [0; 32];
    b[..8].copy_from_slice(MAGIC);
    b[8..12].copy_from_slice(&(x + 1).to_le_bytes());
    b[12..16].copy_from_slice(&(y + 1).to_le_bytes());
    b[16..24].copy_from_slice(&count.to_le_bytes());
    b[24..28].copy_from_slice(&32u32.to_le_bytes());
    Ok(b)
}
fn checksum(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf29ce484222325u64, |h, &v| {
        (h ^ u64::from(v)).wrapping_mul(0x100000001b3)
    })
}
fn encode(e: Saddle) -> [u8; 32] {
    let mut b = [0; 32];
    b[..24].copy_from_slice(&e.encode());
    let c = checksum(&b[..24]);
    b[24..].copy_from_slice(&c.to_le_bytes());
    b
}
fn decode(b: &[u8], extent: Extent) -> Result<Saddle> {
    if b.len() != 32 {
        return Err(SortError::Corrupt("row width"));
    }
    let c = u64::from_le_bytes(
        b[24..]
            .try_into()
            .map_err(|_| SortError::Corrupt("checksum width"))?,
    );
    if checksum(&b[..24]) != c {
        return Err(SortError::Corrupt("checksum"));
    }
    Saddle::decode(
        b[..24]
            .try_into()
            .map_err(|_| SortError::Corrupt("payload width"))?,
        extent,
    )
    .ok_or(SortError::Corrupt("saddle"))
}
struct Writer {
    file: File,
    buffer: Vec<u8>,
    written: u64,
}
impl Writer {
    fn create(path: &Path, extent: Extent, count: u64, budget: &mut Budget) -> Result<Self> {
        budget.io(0)?;
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        let mut buffer = Vec::with_capacity(BLOCK);
        buffer.extend_from_slice(&header(extent, count)?);
        Ok(Self {
            file,
            buffer,
            written: 0,
        })
    }
    fn row(&mut self, e: Saddle, budget: &mut Budget) -> Result<()> {
        if self.buffer.len() + 32 > BLOCK {
            self.flush(budget)?;
        }
        self.buffer.extend_from_slice(&encode(e));
        Ok(())
    }
    fn flush(&mut self, budget: &mut Budget) -> Result<()> {
        if !self.buffer.is_empty() {
            budget.io(self.buffer.len())?;
            self.file.write_all(&self.buffer)?;
            self.written = add(self.written, self.buffer.len() as u64)?;
            self.buffer.clear();
        }
        Ok(())
    }
    fn finish(mut self, count: u64, budget: &mut Budget) -> Result<()> {
        self.flush(budget)?;
        let expected = add(32, mul(ROW, count)?)?;
        if self.written != expected {
            return Err(SortError::Corrupt("writer count"));
        }
        budget.io(0)?;
        self.file.sync_all()?;
        budget.io(0)?;
        if self.file.metadata()?.len() != expected {
            return Err(SortError::Corrupt("writer length"));
        }
        Ok(())
    }
}
// No flush-on-drop: a failed private buffer is never silently retried.
struct Reader {
    file: File,
    buffer: Vec<u8>,
    at: usize,
    end: usize,
    remaining: u64,
    previous: Option<Saddle>,
    head: Option<Saddle>,
}
impl Reader {
    fn open(path: &Path, extent: Extent, count: u64, budget: &mut Budget) -> Result<Self> {
        budget.io(0)?;
        let mut file = File::open(path)?;
        budget.io(0)?;
        if file.metadata()?.len() != add(32, mul(ROW, count)?)? {
            return Err(SortError::Corrupt("file length"));
        }
        let mut h = [0; 32];
        budget.io(32)?;
        file.read_exact(&mut h)?;
        if h != header(extent, count)? {
            return Err(SortError::Corrupt("header"));
        }
        Ok(Self {
            file,
            buffer: vec![0; BLOCK],
            at: 0,
            end: 0,
            remaining: count,
            previous: None,
            head: None,
        })
    }
    fn advance(&mut self, extent: Extent, budget: &mut Budget) -> Result<()> {
        if self.at == self.end {
            if self.remaining == 0 {
                self.head = None;
                return Ok(());
            }
            let rows = self.remaining.min(128);
            let bytes = size(mul(ROW, rows)?)?;
            budget.io(bytes)?;
            self.file.read_exact(&mut self.buffer[..bytes])?;
            self.remaining -= rows;
            self.at = 0;
            self.end = bytes;
        }
        let e = decode(&self.buffer[self.at..self.at + 32], extent)?;
        self.at += 32;
        if let Some(old) = self.previous {
            match budget.compare(extent, old, e)? {
                Ordering::Less => {}
                Ordering::Equal => return Err(SortError::Duplicate),
                Ordering::Greater => return Err(SortError::Corrupt("run order")),
            }
        }
        self.previous = Some(e);
        self.head = Some(e);
        Ok(())
    }
}
/// Accepted edges only, with bounded runs and no candidate-edge archive.
/// The parent owns file cleanup; every failed push permanently poisons this owner.
pub struct AcceptedSorter {
    directory: PathBuf,
    extent: Extent,
    expected: u64,
    budget: Budget,
    pairs: Vec<Saddle>,
    scratch: Vec<Saddle>,
    count: u64,
    runs: u64,
    poisoned: bool,
}
impl AcceptedSorter {
    /// Claims a fresh private namespace after complete resource admission.
    ///
    /// # Errors
    /// Fails before file creation on invalid bounds; refuses existing files and I/O failure.
    pub fn create(
        directory: &Path,
        extent: Extent,
        expected_edges: u64,
        limits: SortLimits,
    ) -> Result<Self> {
        let need = SortLimits::required(directory, expected_edges, limits.buffer_records)?;
        for (ok, name) in [
            (limits.ram_bytes >= need.ram_bytes, "RAM"),
            (limits.scratch_bytes >= need.scratch_bytes, "scratch"),
            (limits.io_bytes >= need.io_bytes, "I/O bytes"),
            (limits.io_operations >= need.io_operations, "I/O operations"),
            (limits.comparisons >= need.comparisons, "comparisons"),
        ] {
            if !ok {
                return Err(SortError::Limit(name));
            }
        }
        let mut budget = Budget {
            limits: SortLimits {
                buffer_records: need.buffer_records,
                ..limits
            },
            work: SortWork::default(),
        };
        budget.io(0)?;
        if !fs::metadata(directory)?.is_dir() {
            return Err(SortError::Invalid("directory"));
        }
        budget.io(0)?;
        let claim = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("accepted-sort.lock"))?;
        budget.io(0)?;
        claim.sync_all()?;
        let n = size(need.buffer_records)?;
        let at = CellIndex::new(0, extent).ok_or(SortError::Invalid("extent"))?;
        let fill = Saddle {
            left: Node::Closed(at),
            right: Node::Exterior,
            sill_mm: 0,
            from: at,
            to: None,
        };
        Ok(Self {
            directory: directory.to_path_buf(),
            extent,
            expected: expected_edges,
            budget,
            pairs: Vec::with_capacity(n),
            scratch: vec![fill; n],
            count: 0,
            runs: 0,
            poisoned: false,
        })
    }
    /// Current attempted work before the owner is consumed by `finish`.
    #[must_use]
    pub fn work(&self) -> SortWork {
        self.budget.work
    }
    fn path(&self, generation: u64, index: u64) -> PathBuf {
        self.directory
            .join(format!("accepted-run-{generation}-{index}.bin"))
    }
    /// Appends one accepted edge; duplicate keys are errors, never deduplicated.
    ///
    /// # Errors
    /// Any invalid edge, excess count, comparison/I/O limit or actual I/O failure poisons this owner.
    pub fn push(&mut self, edge: Saddle) -> Result<()> {
        let r = self.push_inner(edge);
        if r.is_err() {
            self.poisoned = true;
        }
        r
    }
    fn push_inner(&mut self, edge: Saddle) -> Result<()> {
        if self.poisoned {
            return Err(SortError::Poisoned);
        }
        if self.count >= self.expected {
            return Err(SortError::Limit("edges"));
        }
        if Saddle::decode(edge.encode(), self.extent) != Some(edge) {
            return Err(SortError::Invalid("saddle"));
        }
        self.pairs.push(edge);
        self.count += 1;
        if self.pairs.len() == size(self.budget.limits.buffer_records)? {
            self.flush_run()?;
        }
        Ok(())
    }
    fn sort_batch(&mut self) -> Result<()> {
        let n = self.pairs.len();
        let mut width = 1;
        while width < n {
            let mut start = 0;
            while start < n {
                let mid = (start + width).min(n);
                let end = (mid + width).min(n);
                let (mut a, mut b) = (start, mid);
                for at in start..end {
                    let take_a = if a == mid {
                        false
                    } else if b == end {
                        true
                    } else {
                        self.budget
                            .compare(self.extent, self.pairs[a], self.pairs[b])?
                            != Ordering::Greater
                    };
                    self.scratch[at] = if take_a {
                        let v = self.pairs[a];
                        a += 1;
                        v
                    } else {
                        let v = self.pairs[b];
                        b += 1;
                        v
                    };
                }
                start = end;
            }
            self.pairs.copy_from_slice(&self.scratch[..n]);
            width = width.checked_mul(2).ok_or(SortError::Overflow)?;
        }
        Ok(())
    }
    fn flush_run(&mut self) -> Result<()> {
        if self.pairs.is_empty() {
            return Ok(());
        }
        self.sort_batch()?;
        let count = self.pairs.len() as u64;
        let mut w = Writer::create(
            &self.path(0, self.runs),
            self.extent,
            count,
            &mut self.budget,
        )?;
        let mut prior = None;
        for &e in &self.pairs {
            if let Some(old) = prior {
                if self.budget.compare(self.extent, old, e)? == Ordering::Equal {
                    return Err(SortError::Duplicate);
                }
            }
            w.row(e, &mut self.budget)?;
            prior = Some(e);
        }
        w.finish(count, &mut self.budget)?;
        self.runs = add(self.runs, 1)?;
        self.pairs.clear();
        Ok(())
    }
    fn run_rows(&self, width: u64, index: u64) -> Result<u64> {
        Ok(self
            .count
            .checked_sub(mul(width, index)?)
            .ok_or(SortError::Corrupt("run index"))?
            .min(width))
    }
    fn merge(&mut self, generation: u64, width: u64, runs: u64) -> Result<u64> {
        let outputs = runs.div_ceil(FAN_IN);
        for out in 0..outputs {
            let first = mul(out, FAN_IN)?;
            let end = add(first, FAN_IN)?.min(runs);
            let mut readers = Vec::with_capacity(size(FAN_IN)?);
            let mut rows = 0;
            for index in first..end {
                let n = self.run_rows(width, index)?;
                rows = add(rows, n)?;
                let mut r = Reader::open(
                    &self.path(generation, index),
                    self.extent,
                    n,
                    &mut self.budget,
                )?;
                r.advance(self.extent, &mut self.budget)?;
                readers.push(r);
            }
            let mut w = Writer::create(
                &self.path(add(generation, 1)?, out),
                self.extent,
                rows,
                &mut self.budget,
            )?;
            let mut previous = None;
            loop {
                let mut best = None;
                for (i, r) in readers.iter().enumerate() {
                    if let Some(e) = r.head {
                        if let Some((_, old)) = best {
                            if self.budget.compare(self.extent, e, old)? == Ordering::Less {
                                best = Some((i, e));
                            }
                        } else {
                            best = Some((i, e));
                        }
                    }
                }
                let Some((i, e)) = best else {
                    break;
                };
                if let Some(old) = previous {
                    if self.budget.compare(self.extent, old, e)? == Ordering::Equal {
                        return Err(SortError::Duplicate);
                    }
                }
                w.row(e, &mut self.budget)?;
                previous = Some(e);
                readers[i].advance(self.extent, &mut self.budget)?;
            }
            w.finish(rows, &mut self.budget)?;
            drop(readers);
            for index in first..end {
                self.budget.io(0)?;
                fs::remove_file(self.path(generation, index))?;
            }
        }
        Ok(outputs)
    }
    /// Completes ordering and transfers the remaining budget to one bounded reader.
    /// The final sorted private run remains owned by the parent transaction.
    ///
    /// # Errors
    /// Refuses poisoned/count-mismatched state, duplicate/corrupt edges and exhausted or failed work.
    pub fn finish(mut self) -> Result<EdgeReader> {
        if self.poisoned {
            return Err(SortError::Poisoned);
        }
        if self.count != self.expected {
            return Err(SortError::Invalid("expected edge count"));
        }
        self.flush_run()?;
        if self.count == 0 {
            Writer::create(&self.path(0, 0), self.extent, 0, &mut self.budget)?
                .finish(0, &mut self.budget)?;
            self.runs = 1;
        }
        let (mut generation, mut width, mut runs) =
            (0, self.budget.limits.buffer_records, self.runs);
        while runs > 1 {
            runs = self.merge(generation, width, runs)?;
            generation = add(generation, 1)?;
            width =
                u64::try_from((u128::from(width) * u128::from(FAN_IN)).min(u128::from(self.count)))
                    .map_err(|_| SortError::Overflow)?;
        }
        let path = self.path(generation, 0);
        let reader = Reader::open(&path, self.extent, self.count, &mut self.budget)?;
        Ok(EdgeReader {
            reader,
            path,
            extent: self.extent,
            count: self.count,
            remaining: self.count,
            budget: self.budget,
            failed: false,
        })
    }
}
/// One bounded, fallible sequential pass over retained accepted edges.
/// A read error is yielded once; later iteration terminates without returning more edges.
pub struct EdgeReader {
    reader: Reader,
    path: PathBuf,
    extent: Extent,
    count: u64,
    remaining: u64,
    budget: Budget,
    failed: bool,
}
impl EdgeReader {
    /// Opens one independently admitted checked pass over a retained accepted file.
    /// Caller supplies the same extent/count used by the producer. The exact header,
    /// file length, row checksums and canonical order are revalidated on every pass.
    /// Parent transaction ownership determines how long the file remains available.
    ///
    /// # Errors
    /// Rejects inadequate admission before opening; preserves file/codec/work failures.
    pub fn open(path: &Path, extent: Extent, count: u64, limits: ReadLimits) -> Result<Self> {
        let need = ReadLimits::required(path, count)?;
        for (ok, name) in [
            (limits.ram_bytes >= need.ram_bytes, "replay RAM"),
            (limits.io_bytes >= need.io_bytes, "replay I/O bytes"),
            (
                limits.io_operations >= need.io_operations,
                "replay I/O operations",
            ),
            (limits.comparisons >= need.comparisons, "replay comparisons"),
        ] {
            if !ok {
                return Err(SortError::Limit(name));
            }
        }
        let mut budget = Budget {
            limits: SortLimits {
                buffer_records: 0,
                ram_bytes: limits.ram_bytes,
                scratch_bytes: 0,
                io_bytes: limits.io_bytes,
                io_operations: limits.io_operations,
                comparisons: limits.comparisons,
            },
            work: SortWork::default(),
        };
        let reader = Reader::open(path, extent, count, &mut budget)?;
        Ok(Self {
            reader,
            path: path.to_path_buf(),
            extent,
            count,
            remaining: count,
            budget,
            failed: false,
        })
    }
    /// Starts a separately budgeted pass without changing this reader's position.
    /// A failed reader cannot be used to bypass its poison state.
    ///
    /// # Errors
    /// Rejects poisoned state, insufficient new admission, missing/corrupt files or I/O failure.
    pub fn replay(&self, limits: ReadLimits) -> Result<Self> {
        if self.failed {
            return Err(SortError::Poisoned);
        }
        Self::open(&self.path, self.extent, self.count, limits)
    }
    /// Exact total accepted edges promised by this stream.
    #[must_use]
    pub fn expected_count(&self) -> u64 {
        self.count
    }
    /// Final private path; parent ownership governs lifetime and publication.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Construction and final-read work charged so far.
    #[must_use]
    pub fn work(&self) -> SortWork {
        self.budget.work
    }
}
impl Iterator for EdgeReader {
    type Item = Result<Saddle>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.failed || self.remaining == 0 {
            return None;
        }
        let result = self
            .reader
            .advance(self.extent, &mut self.budget)
            .and_then(|()| self.reader.head.ok_or(SortError::Corrupt("early end")));
        if result.is_err() {
            self.failed = true;
        } else {
            self.remaining -= 1;
        }
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let id = NEXT.fetch_add(1, AtomicOrdering::Relaxed);
            let p = std::env::temp_dir().join(format!(
                "arda-accepted-sort-{}-{now}-{id}",
                std::process::id()
            ));
            fs::create_dir(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn extent() -> Extent {
        Extent::new(1024, 3).unwrap()
    }
    fn edge(i: u32) -> Saddle {
        let e = extent();
        let from = (i * 17) % 1000;
        Saddle {
            left: Node::Closed(CellIndex::new(i % 101 + 1, e).unwrap()),
            right: Node::Exterior,
            sill_mm: i32::try_from(i % 7).unwrap() - 3,
            from: CellIndex::new(from, e).unwrap(),
            to: Some(CellIndex::new(from + 1, e).unwrap()),
        }
    }
    fn collect(reader: &mut EdgeReader) -> Vec<Saddle> {
        reader.map(|v| v.unwrap()).collect()
    }
    #[test]
    fn exact_witness_bytes_across_tiny_runs_and_multiple_generations() {
        let e = extent();
        let mut expected: Vec<Saddle> = (0..257).map(edge).collect();
        expected.sort_unstable_by_key(|&a| edge_key(e, a));
        let expected_bytes: Vec<u8> = expected.iter().flat_map(|a| encode(*a)).collect();
        for buffer in [1, 2, 17, 512] {
            let dir = Temp::new();
            let limits = SortLimits::required(&dir.0, 257, buffer).unwrap();
            let mut sort = AcceptedSorter::create(&dir.0, e, 257, limits).unwrap();
            for i in 0..257 {
                sort.push(edge(i * 73 % 257)).unwrap();
            }
            let mut reader = sort.finish().unwrap();
            assert_eq!(reader.expected_count(), 257);
            assert_eq!(collect(&mut reader), expected);
            assert!(reader.next().is_none());
            let bytes = fs::read(reader.path()).unwrap();
            assert_eq!(&bytes[..32], &header(e, 257).unwrap());
            assert_eq!(&bytes[32..], &expected_bytes);
            let work = reader.work();
            assert_eq!(work.io_bytes, limits.io_bytes);
            assert_eq!(work.io_operations, limits.io_operations);
            assert!(work.comparisons <= limits.comparisons);
            assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
        }
    }
    #[test]
    fn exact_reservations_cover_empty_short_and_partial_runs() {
        for n in 0..35 {
            for buffer in [1, 2, 3, 8, 64] {
                let dir = Temp::new();
                let limits = SortLimits::required(&dir.0, n, buffer).unwrap();
                let mut sort = AcceptedSorter::create(&dir.0, extent(), n, limits).unwrap();
                for i in (0..u32::try_from(n).unwrap()).rev() {
                    sort.push(edge(i)).unwrap();
                }
                let mut reader = sort.finish().unwrap();
                assert_eq!(collect(&mut reader).len(), usize::try_from(n).unwrap());
                assert_eq!(reader.work().io_bytes, limits.io_bytes);
                assert_eq!(reader.work().io_operations, limits.io_operations);
                assert!(reader.work().comparisons <= limits.comparisons);
            }
        }
    }
    #[test]
    fn duplicates_in_one_batch_and_across_runs_poison() {
        for buffer in [1, 2, 66] {
            let dir = Temp::new();
            let limits = SortLimits::required(&dir.0, 66, buffer).unwrap();
            let mut sort = AcceptedSorter::create(&dir.0, extent(), 66, limits).unwrap();
            let mut failed = false;
            for i in 0..66 {
                if let Err(err) = sort.push(edge(if i == 65 { 0 } else { i })) {
                    assert!(matches!(err, SortError::Duplicate));
                    failed = true;
                    break;
                }
            }
            if failed {
                assert!(matches!(sort.push(edge(90)), Err(SortError::Poisoned)));
                assert!(matches!(sort.finish(), Err(SortError::Poisoned)));
            } else {
                assert!(matches!(sort.finish(), Err(SortError::Duplicate)));
            }
        }
    }
    #[test]
    fn admission_limits_and_namespace_collisions_precede_mutation() {
        for field in 0..5 {
            let dir = Temp::new();
            let mut limits = SortLimits::required(&dir.0, 100, 9).unwrap();
            match field {
                0 => limits.ram_bytes -= 1,
                1 => limits.scratch_bytes -= 1,
                2 => limits.io_bytes -= 1,
                3 => limits.io_operations -= 1,
                _ => limits.comparisons -= 1,
            };
            assert!(matches!(
                AcceptedSorter::create(&dir.0, extent(), 100, limits),
                Err(SortError::Limit(_))
            ));
            assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 0);
        }
        let dir = Temp::new();
        assert!(SortLimits::required(&dir.0, 1, 0).is_err());
        assert!(SortLimits::required(&dir.0, u64::MAX, 1).is_err());
        let limits = SortLimits::required(&dir.0, 1, 1).unwrap();
        fs::write(dir.0.join("accepted-sort.lock"), b"old").unwrap();
        assert!(matches!(
            AcceptedSorter::create(&dir.0, extent(), 1, limits),
            Err(SortError::Io(_))
        ));
        assert_eq!(fs::read(dir.0.join("accepted-sort.lock")).unwrap(), b"old");
        let dir = Temp::new();
        let limits = SortLimits::required(&dir.0, 1, 1).unwrap();
        let mut sort = AcceptedSorter::create(&dir.0, extent(), 1, limits).unwrap();
        fs::write(dir.0.join("accepted-run-0-0.bin"), b"old").unwrap();
        assert!(matches!(sort.push(edge(0)), Err(SortError::Io(_))));
        assert_eq!(
            fs::read(dir.0.join("accepted-run-0-0.bin")).unwrap(),
            b"old"
        );
    }
    #[test]
    fn sort_comparison_and_real_dirty_write_fail_without_retry() {
        let dir = Temp::new();
        let limits = SortLimits::required(&dir.0, 4, 4).unwrap();
        let mut sort = AcceptedSorter::create(&dir.0, extent(), 4, limits).unwrap();
        sort.budget.limits.comparisons = 2;
        for i in 0..3 {
            sort.push(edge(i)).unwrap();
        }
        assert!(matches!(
            sort.push(edge(3)),
            Err(SortError::Limit("comparisons"))
        ));
        assert_eq!(sort.work().comparisons, 2);
        assert!(matches!(sort.finish(), Err(SortError::Poisoned)));
        let dir = Temp::new();
        let limits = SortLimits::required(&dir.0, 1, 1).unwrap();
        let mut budget = Budget {
            limits,
            work: SortWork::default(),
        };
        let path = dir.0.join("real-error.bin");
        let mut writer = Writer::create(&path, extent(), 1, &mut budget).unwrap();
        writer.row(edge(0), &mut budget).unwrap();
        writer.file = File::open(&path).unwrap();
        assert!(matches!(
            writer.finish(1, &mut budget),
            Err(SortError::Io(_))
        ));
        assert_eq!(budget.work.io_bytes, 64);
        assert_eq!(fs::metadata(path).unwrap().len(), 0);
    }
    #[test]
    fn invalid_edges_counts_and_truncated_runs_abort() {
        let dir = Temp::new();
        let limits = SortLimits::required(&dir.0, 2, 1).unwrap();
        let mut sort = AcceptedSorter::create(&dir.0, extent(), 2, limits).unwrap();
        sort.push(edge(0)).unwrap();
        assert!(matches!(
            sort.finish(),
            Err(SortError::Invalid("expected edge count"))
        ));
        let dir = Temp::new();
        let limits = SortLimits::required(&dir.0, 1, 1).unwrap();
        let mut sort = AcceptedSorter::create(&dir.0, extent(), 1, limits).unwrap();
        let mut bad = edge(0);
        bad.right = bad.left;
        assert!(matches!(sort.push(bad), Err(SortError::Invalid("saddle"))));
        assert!(matches!(sort.finish(), Err(SortError::Poisoned)));
        let dir = Temp::new();
        let limits = SortLimits::required(&dir.0, 2, 1).unwrap();
        let mut sort = AcceptedSorter::create(&dir.0, extent(), 2, limits).unwrap();
        sort.push(edge(0)).unwrap();
        sort.push(edge(1)).unwrap();
        OpenOptions::new()
            .write(true)
            .open(dir.0.join("accepted-run-0-0.bin"))
            .unwrap()
            .set_len(63)
            .unwrap();
        assert!(matches!(
            sort.finish(),
            Err(SortError::Corrupt("file length"))
        ));
    }
    #[test]
    fn final_reader_checks_payload_header_order_and_owned_budget() {
        for mutation in 0..4 {
            let dir = Temp::new();
            let limits = SortLimits::required(&dir.0, 2, 2).unwrap();
            let mut sort = AcceptedSorter::create(&dir.0, extent(), 2, limits).unwrap();
            sort.push(edge(0)).unwrap();
            sort.push(edge(1)).unwrap();
            if mutation == 0 {
                let p = dir.0.join("accepted-run-0-0.bin");
                let mut b = fs::read(&p).unwrap();
                b[8] ^= 1;
                fs::write(&p, b).unwrap();
                assert!(matches!(sort.finish(), Err(SortError::Corrupt("header"))));
                continue;
            }
            let mut reader = sort.finish().unwrap();
            if mutation == 3 {
                reader.budget.limits.io_bytes = reader.work().io_bytes;
                assert!(matches!(reader.next(), Some(Err(SortError::Limit("I/O")))));
                assert!(reader.next().is_none());
                continue;
            }
            let mut b = fs::read(reader.path()).unwrap();
            if mutation == 1 {
                b[32 + 8] ^= 1;
            } else {
                let a = b[32..64].to_vec();
                b.copy_within(64..96, 32);
                b[64..96].copy_from_slice(&a);
            }
            fs::write(reader.path(), b).unwrap();
            if mutation == 1 {
                assert!(matches!(
                    reader.next(),
                    Some(Err(SortError::Corrupt("checksum")))
                ));
            } else {
                assert!(reader.next().unwrap().is_ok());
                assert!(matches!(
                    reader.next(),
                    Some(Err(SortError::Corrupt("run order")))
                ));
            }
            assert!(reader.next().is_none());
        }
    }
    #[test]
    fn hierarchy_key_preserves_real_export_witness_and_signed_sill() {
        let e = extent();
        let mut a = edge(0);
        a.from = CellIndex::new(1023, e).unwrap();
        a.to = None;
        a.sill_mm = i32::MIN;
        assert_eq!(decode(&encode(a), e).unwrap(), a);
        let k = edge_key(e, a);
        assert_eq!(k.0, i32::MIN);
        assert_eq!(k.2, u64::MAX);
        assert_eq!(k.3, 1023);
        assert_eq!(k.4, u64::MAX);
        let mut b = edge(1);
        b.left = Node::Closed(CellIndex::new(5, e).unwrap());
        b.right = Node::Closed(CellIndex::new(1025, e).unwrap());
        b.from = CellIndex::new(1024, e).unwrap();
        b.to = Some(CellIndex::new(1, e).unwrap());
        assert_eq!(decode(&encode(b), e).unwrap(), b);
        assert_eq!(edge_key(e, b).3, 1);
        assert_eq!(edge_key(e, b).4, 1u64 << 32);
    }
    #[test]
    fn retained_file_replays_with_exact_independent_pass_admission() {
        let t = Temp::new();
        let n = 257;
        let limits = SortLimits::required(&t.0, n, 2).unwrap();
        let mut sorter = AcceptedSorter::create(&t.0, extent(), n, limits).unwrap();
        for i in (0..u32::try_from(n).unwrap()).rev() {
            sorter.push(edge(i)).unwrap();
        }
        let mut original = sorter.finish().unwrap();
        let first = original.next().unwrap().unwrap();
        let read_limits = ReadLimits::required(original.path(), n).unwrap();
        let mut replay = original.replay(read_limits).unwrap();
        let all = collect(&mut replay);
        assert_eq!(all.len(), usize::try_from(n).unwrap());
        assert_eq!(all[0], first);
        let work = replay.work();
        assert_eq!(work.io_bytes, read_limits.io_bytes);
        assert_eq!(work.io_operations, read_limits.io_operations);
        assert_eq!(work.comparisons, read_limits.comparisons);
        assert_eq!(collect(&mut original), all[1..]);
        let mut second = EdgeReader::open(replay.path(), extent(), n, read_limits).unwrap();
        assert_eq!(collect(&mut second), all);
        assert!(replay.path().exists());
    }
    #[test]
    fn replay_admission_precedes_open_and_codec_failure_poison_is_preserved() {
        let t = Temp::new();
        let missing = t.0.join("missing");
        let exact = ReadLimits::required(&missing, 2).unwrap();
        for insufficient in [
            ReadLimits {
                ram_bytes: exact.ram_bytes - 1,
                ..exact
            },
            ReadLimits {
                io_bytes: exact.io_bytes - 1,
                ..exact
            },
            ReadLimits {
                io_operations: exact.io_operations - 1,
                ..exact
            },
            ReadLimits {
                comparisons: exact.comparisons - 1,
                ..exact
            },
        ] {
            assert!(matches!(
                EdgeReader::open(&missing, extent(), 2, insufficient),
                Err(SortError::Limit(_))
            ));
        }
        let mut sorter =
            AcceptedSorter::create(&t.0, extent(), 2, SortLimits::required(&t.0, 2, 2).unwrap())
                .unwrap();
        sorter.push(edge(1)).unwrap();
        sorter.push(edge(2)).unwrap();
        let original = sorter.finish().unwrap();
        let limits = ReadLimits::required(original.path(), 2).unwrap();
        let mut bytes = fs::read(original.path()).unwrap();
        bytes[40] ^= 1;
        fs::write(original.path(), bytes).unwrap();
        let mut replay = original.replay(limits).unwrap();
        assert!(matches!(replay.next().unwrap(), Err(SortError::Corrupt(_))));
        assert!(replay.next().is_none());
        assert!(matches!(replay.replay(limits), Err(SortError::Poisoned)));
        assert!(matches!(
            EdgeReader::open(
                original.path(),
                extent(),
                3,
                ReadLimits::required(original.path(), 3).unwrap()
            ),
            Err(SortError::Corrupt(_))
        ));
        assert!(matches!(
            ReadLimits::required(&missing, u64::MAX),
            Err(SortError::Overflow)
        ));
    }
    #[test]
    fn empty_accepted_file_replays_as_an_exact_header_only_pass() {
        let t = Temp::new();
        let original =
            AcceptedSorter::create(&t.0, extent(), 0, SortLimits::required(&t.0, 0, 1).unwrap())
                .unwrap()
                .finish()
                .unwrap();
        let limits = ReadLimits::required(original.path(), 0).unwrap();
        let mut replay = original.replay(limits).unwrap();
        assert!(replay.next().is_none());
        assert_eq!(replay.work().io_bytes, 32);
        assert_eq!(replay.work().io_operations, 3);
        assert_eq!(replay.work().comparisons, 0);
    }
}
