//! Bounded external child-link ordering for a private hierarchy transaction.
use arda_core::{formats::hydrology::TableSpan, hydrology::BasinId};
use std::{
    cmp::Ordering,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

const FAN_IN: u64 = 8;
const BLOCK: usize = 4096;
const PAGE_ROWS: u64 = 170;
const SPAN_BYTES: usize = 24;
const PAGE_BYTES: usize = 170 * SPAN_BYTES;
/// Final fixed-width child identifiers, retained after successful completion.
pub const CHILD_FILE: &str = "children.u64le";
/// Final `(parent, child offset, child count)` index, all little-endian u64.
pub const SPAN_FILE: &str = "child-spans.bin";

#[derive(Debug, thiserror::Error)]
/// Private-stage failure; a failed instance cannot subsequently publish success.
pub enum ChildError {
    /// An actual filesystem operation failed.
    #[error("child-link storage failed: {0}")]
    Io(#[from] std::io::Error),
    /// Checked count, offset or reservation arithmetic overflowed.
    #[error("child-link arithmetic overflow")]
    Overflow,
    /// The supplied capacity or phase is invalid.
    #[error("invalid child-link input: {0}")]
    Invalid(&'static str),
    /// A declared resource reservation is insufficient or exhausted.
    #[error("child-link resource limit: {0}")]
    Limit(&'static str),
    /// One parent/child pair appears more than once.
    #[error("duplicate child-link pair")]
    Duplicate,
    /// Private bytes do not match their known extent or sorted codec.
    #[error("corrupt child-link file: {0}")]
    Corrupt(&'static str),
    /// An earlier error has permanently aborted this private instance.
    #[error("child-link instance is poisoned")]
    Poisoned,
}
type Result<T> = std::result::Result<T, ChildError>;
fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or(ChildError::Overflow)
}
fn mul(a: u64, b: u64) -> Result<u64> {
    a.checked_mul(b).ok_or(ChildError::Overflow)
}
fn size(v: u64) -> Result<usize> {
    usize::try_from(v).map_err(|_| ChildError::Overflow)
}
fn ceil(n: u64, d: u64) -> u64 {
    n.div_ceil(d)
}

#[derive(Clone, Copy, Debug)]
/// Construction preflight and cumulative runtime ceilings. Lookup calls consume
/// the remaining I/O/key-comparison budget after `finish`.
pub struct ChildLimits {
    /// Maximum in-memory sort batch; two 16-byte-row buffers are admitted.
    pub buffer_records: u64,
    /// Maximum total parent/child links accepted by this instance.
    pub max_links: u64,
    /// Peak owned payload and explicitly reserved metadata/path bytes.
    pub ram_bytes: u64,
    /// Peak simultaneous private file bytes, including final tables.
    pub scratch_bytes: u64,
    /// Requested bytes across all explicit buffered read/write calls.
    pub io_bytes: u128,
    /// Explicit filesystem calls, including metadata, seek, sync and deletion.
    pub io_operations: u64,
    /// Parent/pair key comparisons, including comparisons inside sorting.
    pub comparisons: u64,
}
impl ChildLimits {
    /// Conservative construction reservation for a known directory and capacity.
    /// Callers add lookup allowances before construction; no lookup budget is guessed.
    ///
    /// # Errors
    /// Rejects a zero run buffer or an unrepresentable reservation.
    pub fn required(directory: &Path, buffer_records: u64, max_links: u64) -> Result<Self> {
        if buffer_records == 0 {
            return Err(ChildError::Invalid("zero run buffer"));
        }
        let buffer_records = buffer_records.min(max_links.max(1));
        let mut runs = ceil(max_links, buffer_records);
        let mut all_runs = runs;
        let mut generations = 0;
        while runs > 1 {
            runs = ceil(runs, FAN_IN);
            all_runs = add(all_runs, runs)?;
            generations = add(generations, 1)?;
        }
        let levels = if max_links == 0 {
            0
        } else {
            add(generations, 1)?
        };
        let blocks = mul(levels, ceil(max_links, 256))?;
        let io_operations = add(
            add(mul(8, all_runs)?, mul(2, blocks)?)?,
            add(add(ceil(max_links, 512), ceil(max_links, 170))?, 11)?,
        )?;
        let log = if buffer_records <= 1 {
            0
        } else {
            u64::from(64 - (buffer_records - 1).leading_zeros())
        };
        let comparisons = mul(max_links, add(add(log, mul(9, generations)?)?, 3)?)?;
        let path = u64::try_from(directory.as_os_str().as_encoded_bytes().len())
            .map_err(|_| ChildError::Overflow)?;
        // 64 KiB covers eight Reader descriptors/buffers, writer/cache buffers,
        // the owner, fixed merge heads and temporary filename overhead. Actual
        // directory text is separately charged for three simultaneous paths.
        let ram_bytes = add(add(mul(32, buffer_records)?, 65_536)?, mul(3, path)?)?;
        size(ram_bytes)?;
        Ok(Self {
            buffer_records,
            max_links,
            ram_bytes,
            scratch_bytes: mul(48, max_links)?,
            io_bytes: u128::from(max_links) * (64 + 32 * u128::from(generations)),
            io_operations,
            comparisons,
        })
    }
    /// Adds a worst-case reservation for this many cold-cache parent lookups.
    /// Runtime work is still charged exactly; unused reservation is not spent.
    ///
    /// # Errors
    /// Rejects unrepresentable cumulative I/O or key-comparison reservations.
    pub fn with_lookups(mut self, lookups: u64) -> Result<Self> {
        let depth = if self.max_links == 0 {
            0
        } else {
            u64::from(64 - self.max_links.leading_zeros())
        };
        let loads = mul(lookups, depth)?;
        self.io_bytes = self
            .io_bytes
            .checked_add(u128::from(loads) * PAGE_BYTES as u128)
            .ok_or(ChildError::Overflow)?;
        self.io_operations = add(self.io_operations, mul(2, loads)?)?;
        self.comparisons = add(self.comparisons, mul(PAGE_ROWS, loads)?)?;
        Ok(self)
    }
}
#[derive(Default, Clone, Copy, Debug)]
/// Requested work charged before attempting the operation, including failed I/O.
pub struct ChildWork {
    /// Requested buffered read/write bytes.
    pub io_bytes: u128,
    /// Explicit filesystem calls.
    pub io_operations: u64,
    /// Pair/parent key comparisons.
    pub comparisons: u64,
}
struct Budget {
    limit: ChildLimits,
    work: ChildWork,
}
impl Budget {
    fn io(&mut self, bytes: usize) -> Result<()> {
        let next = self
            .work
            .io_bytes
            .checked_add(bytes as u128)
            .ok_or(ChildError::Overflow)?;
        if next > self.limit.io_bytes || self.work.io_operations >= self.limit.io_operations {
            return Err(ChildError::Limit("I/O"));
        }
        self.work.io_bytes = next;
        self.work.io_operations += 1;
        Ok(())
    }
    fn compare<T: Ord>(&mut self, a: &T, b: &T) -> Result<Ordering> {
        if self.work.comparisons >= self.limit.comparisons {
            return Err(ChildError::Limit("comparisons"));
        }
        self.work.comparisons += 1;
        Ok(a.cmp(b))
    }
}
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Pair {
    parent: u64,
    child: u64,
}
impl Pair {
    fn encode(self) -> [u8; 16] {
        let mut b = [0; 16];
        b[..8].copy_from_slice(&self.parent.to_le_bytes());
        b[8..].copy_from_slice(&self.child.to_le_bytes());
        b
    }
    fn decode(b: &[u8]) -> Result<Self> {
        if b.len() != 16 {
            return Err(ChildError::Corrupt("pair width"));
        }
        let parent = u64::from_le_bytes(
            b[..8]
                .try_into()
                .map_err(|_| ChildError::Corrupt("parent"))?,
        );
        let child = u64::from_le_bytes(
            b[8..]
                .try_into()
                .map_err(|_| ChildError::Corrupt("child"))?,
        );
        if parent == child {
            return Err(ChildError::Corrupt("self child"));
        }
        Ok(Self { parent, child })
    }
}
struct Writer {
    file: File,
    buffer: Vec<u8>,
    written: u64,
}
impl Writer {
    fn create(path: &Path, budget: &mut Budget) -> Result<Self> {
        budget.io(0)?;
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        Ok(Self {
            file,
            buffer: Vec::with_capacity(BLOCK),
            written: 0,
        })
    }
    fn row(&mut self, row: &[u8], budget: &mut Budget) -> Result<()> {
        if row.len() > BLOCK {
            return Err(ChildError::Invalid("row exceeds I/O buffer"));
        }
        if self.buffer.len() + row.len() > BLOCK {
            self.flush(budget)?;
        }
        self.buffer.extend_from_slice(row);
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
    fn finish(mut self, expected: u64, budget: &mut Budget) -> Result<()> {
        self.flush(budget)?;
        if self.written != expected {
            return Err(ChildError::Corrupt("writer count"));
        }
        budget.io(0)?;
        self.file.sync_all()?;
        budget.io(0)?;
        if self.file.metadata()?.len() != expected {
            return Err(ChildError::Corrupt("writer length"));
        }
        Ok(())
    }
    // Drop only closes the file. Failed/dirty buffers are never implicitly retried.
}
struct Reader {
    file: File,
    buffer: Vec<u8>,
    at: usize,
    end: usize,
    remaining: u64,
    previous: Option<Pair>,
    head: Option<Pair>,
}
impl Reader {
    fn open(path: &Path, rows: u64, budget: &mut Budget) -> Result<Self> {
        budget.io(0)?;
        let file = File::open(path)?;
        budget.io(0)?;
        if file.metadata()?.len() != mul(rows, 16)? {
            return Err(ChildError::Corrupt("run length"));
        }
        Ok(Self {
            file,
            buffer: vec![0; BLOCK],
            at: 0,
            end: 0,
            remaining: rows,
            previous: None,
            head: None,
        })
    }
    fn advance(&mut self, budget: &mut Budget) -> Result<()> {
        if self.at == self.end {
            if self.remaining == 0 {
                self.head = None;
                return Ok(());
            }
            let rows = self.remaining.min(256);
            let bytes = size(mul(rows, 16)?)?;
            budget.io(bytes)?;
            self.file.read_exact(&mut self.buffer[..bytes])?;
            self.at = 0;
            self.end = bytes;
            self.remaining -= rows;
        }
        let p = Pair::decode(&self.buffer[self.at..self.at + 16])?;
        self.at += 16;
        if let Some(old) = self.previous {
            match budget.compare(&old, &p)? {
                Ordering::Greater => return Err(ChildError::Corrupt("unordered run")),
                Ordering::Equal => return Err(ChildError::Duplicate),
                Ordering::Less => {}
            }
        }
        self.previous = Some(p);
        self.head = Some(p);
        Ok(())
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Writing,
    Ready,
    Poisoned,
}
/// One private external sort plus a bounded page cache for parent-span lookups.
/// The parent transaction owns cleanup, including all files left after any error.
pub struct DiskChildLinks {
    directory: PathBuf,
    budget: Budget,
    stage: Stage,
    pairs: Vec<Pair>,
    scratch: Vec<Pair>,
    count: u64,
    runs: u64,
    span_file: Option<File>,
    span_count: u64,
    cache: [u8; PAGE_BYTES],
    cache_page: Option<u64>,
    cache_rows: usize,
}
impl DiskChildLinks {
    /// Claims fresh private filenames in an existing transaction directory.
    ///
    /// # Errors
    /// Rejects insufficient construction reservations before any filesystem mutation.
    pub fn create(directory: &Path, limits: ChildLimits) -> Result<Self> {
        let need = ChildLimits::required(directory, limits.buffer_records, limits.max_links)?;
        if limits.ram_bytes < need.ram_bytes {
            return Err(ChildError::Limit("RAM"));
        }
        if limits.scratch_bytes < need.scratch_bytes {
            return Err(ChildError::Limit("scratch"));
        }
        if limits.io_bytes < need.io_bytes || limits.io_operations < need.io_operations {
            return Err(ChildError::Limit("construction I/O"));
        }
        if limits.comparisons < need.comparisons {
            return Err(ChildError::Limit("construction comparisons"));
        }
        let mut budget = Budget {
            limit: ChildLimits {
                buffer_records: need.buffer_records,
                ..limits
            },
            work: ChildWork::default(),
        };
        budget.io(0)?;
        if !directory.metadata()?.is_dir() {
            return Err(ChildError::Invalid("directory"));
        }
        budget.io(0)?;
        let claim = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("child-links.lock"))?;
        budget.io(0)?;
        claim.sync_all()?;
        let n = size(need.buffer_records)?;
        Ok(Self {
            directory: directory.to_path_buf(),
            budget,
            stage: Stage::Writing,
            pairs: Vec::with_capacity(n),
            scratch: vec![Pair::default(); n],
            count: 0,
            runs: 0,
            span_file: None,
            span_count: 0,
            cache: [0; PAGE_BYTES],
            cache_page: None,
            cache_rows: 0,
        })
    }
    /// Current attempted work, including the operation which returned an I/O error.
    #[must_use]
    pub fn work(&self) -> ChildWork {
        self.budget.work
    }
    fn writing(&self) -> Result<()> {
        match self.stage {
            Stage::Writing => Ok(()),
            Stage::Ready => Err(ChildError::Invalid("already finished")),
            Stage::Poisoned => Err(ChildError::Poisoned),
        }
    }
    fn run_path(&self, generation: u64, index: u64) -> PathBuf {
        self.directory
            .join(format!("child-run-{generation}-{index}.bin"))
    }
    fn abort<T>(&mut self, result: Result<T>) -> Result<T> {
        if result.is_err() {
            self.stage = Stage::Poisoned;
        }
        result
    }
    /// Appends one immutable parent/child relation; a full batch becomes a sorted run.
    ///
    /// # Errors
    /// Invalid stage, self-links, duplicates and all admitted-resource/I/O failures abort the instance.
    pub fn push(&mut self, parent: BasinId, child: BasinId) -> Result<()> {
        let result = self.push_inner(parent, child);
        self.abort(result)
    }
    fn push_inner(&mut self, parent: BasinId, child: BasinId) -> Result<()> {
        self.writing()?;
        if parent == child {
            return Err(ChildError::Invalid("self child"));
        }
        if self.count >= self.budget.limit.max_links {
            return Err(ChildError::Limit("links"));
        }
        self.pairs.push(Pair {
            parent: parent.0,
            child: child.0,
        });
        self.count += 1;
        if self.pairs.len() == size(self.budget.limit.buffer_records)? {
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
                        self.budget.compare(&self.pairs[a], &self.pairs[b])? != Ordering::Greater
                    };
                    self.scratch[at] = if take_a {
                        let value = self.pairs[a];
                        a += 1;
                        value
                    } else {
                        let value = self.pairs[b];
                        b += 1;
                        value
                    };
                }
                start = end;
            }
            self.pairs.copy_from_slice(&self.scratch[..n]);
            width = width.checked_mul(2).ok_or(ChildError::Overflow)?;
        }
        Ok(())
    }
    fn flush_run(&mut self) -> Result<()> {
        if self.pairs.is_empty() {
            return Ok(());
        }
        self.sort_batch()?;
        let mut writer = Writer::create(&self.run_path(0, self.runs), &mut self.budget)?;
        let mut previous = None;
        for &pair in &self.pairs {
            if let Some(old) = previous {
                if self.budget.compare(&old, &pair)? == Ordering::Equal {
                    return Err(ChildError::Duplicate);
                }
            }
            writer.row(&pair.encode(), &mut self.budget)?;
            previous = Some(pair);
        }
        writer.finish(mul(self.pairs.len() as u64, 16)?, &mut self.budget)?;
        self.runs = add(self.runs, 1)?;
        self.pairs.clear();
        Ok(())
    }
    fn run_rows(&self, width: u64, index: u64) -> Result<u64> {
        Ok(self
            .count
            .checked_sub(mul(width, index)?)
            .ok_or(ChildError::Corrupt("run index"))?
            .min(width))
    }
    fn merge_generation(&mut self, generation: u64, width: u64, runs: u64) -> Result<u64> {
        let outputs = ceil(runs, FAN_IN);
        for output in 0..outputs {
            let first = mul(output, FAN_IN)?;
            let end = add(first, FAN_IN)?.min(runs);
            let mut readers = Vec::with_capacity(size(FAN_IN)?);
            let mut rows = 0;
            for index in first..end {
                let count = self.run_rows(width, index)?;
                rows = add(rows, count)?;
                let mut reader =
                    Reader::open(&self.run_path(generation, index), count, &mut self.budget)?;
                reader.advance(&mut self.budget)?;
                readers.push(reader);
            }
            let mut writer = Writer::create(
                &self.run_path(add(generation, 1)?, output),
                &mut self.budget,
            )?;
            let mut previous = None;
            loop {
                let mut best = None;
                for (i, reader) in readers.iter().enumerate() {
                    if let Some(pair) = reader.head {
                        if let Some((_, old)) = best {
                            if self.budget.compare(&pair, &old)? == Ordering::Less {
                                best = Some((i, pair));
                            }
                        } else {
                            best = Some((i, pair));
                        }
                    }
                }
                let Some((i, pair)) = best else { break };
                if let Some(old) = previous {
                    if self.budget.compare(&old, &pair)? == Ordering::Equal {
                        return Err(ChildError::Duplicate);
                    }
                }
                writer.row(&pair.encode(), &mut self.budget)?;
                previous = Some(pair);
                readers[i].advance(&mut self.budget)?;
            }
            writer.finish(mul(rows, 16)?, &mut self.budget)?;
            drop(readers);
            for index in first..end {
                self.budget.io(0)?;
                fs::remove_file(self.run_path(generation, index))?;
            }
        }
        Ok(outputs)
    }
    /// Completes the sorted child table and parent-span index, preserving final files.
    ///
    /// # Errors
    /// Count disagreement, duplicate links, corrupt runs or any failed operation poison the instance.
    pub fn finish(&mut self, expected_count: u64) -> Result<()> {
        let result = self.finish_inner(expected_count);
        self.abort(result)
    }
    fn finish_inner(&mut self, expected_count: u64) -> Result<()> {
        self.writing()?;
        if expected_count != self.count {
            return Err(ChildError::Invalid("expected link count"));
        }
        self.flush_run()?;
        let (mut generation, mut width, mut runs) =
            (0, self.budget.limit.buffer_records, self.runs);
        while runs > 1 {
            runs = self.merge_generation(generation, width, runs)?;
            generation = add(generation, 1)?;
            width =
                u64::try_from((u128::from(width) * u128::from(FAN_IN)).min(u128::from(self.count)))
                    .map_err(|_| ChildError::Overflow)?;
        }
        let mut children = Writer::create(&self.directory.join(CHILD_FILE), &mut self.budget)?;
        let mut spans = Writer::create(&self.directory.join(SPAN_FILE), &mut self.budget)?;
        let mut parent = None;
        let mut offset = 0;
        let mut group_count = 0;
        let mut span_count = 0;
        if self.count != 0 {
            let mut reader =
                Reader::open(&self.run_path(generation, 0), self.count, &mut self.budget)?;
            reader.advance(&mut self.budget)?;
            while let Some(pair) = reader.head {
                if let Some(old) = parent {
                    if self.budget.compare(&old, &pair.parent)? != Ordering::Equal {
                        write_span(&mut spans, old, offset, group_count, &mut self.budget)?;
                        span_count = add(span_count, 1)?;
                        offset = add(offset, group_count)?;
                        group_count = 0;
                    }
                }
                parent = Some(pair.parent);
                group_count = add(group_count, 1)?;
                children.row(&pair.child.to_le_bytes(), &mut self.budget)?;
                reader.advance(&mut self.budget)?;
            }
        }
        if let Some(parent) = parent {
            write_span(&mut spans, parent, offset, group_count, &mut self.budget)?;
            span_count = add(span_count, 1)?;
        }
        children.finish(mul(self.count, 8)?, &mut self.budget)?;
        spans.finish(mul(span_count, 24)?, &mut self.budget)?;
        if self.count != 0 {
            self.budget.io(0)?;
            fs::remove_file(self.run_path(generation, 0))?;
        }
        self.budget.io(0)?;
        let file = File::open(self.directory.join(SPAN_FILE))?;
        self.budget.io(0)?;
        if file.metadata()?.len() != mul(span_count, 24)? {
            return Err(ChildError::Corrupt("span extent"));
        }
        self.span_file = Some(file);
        self.span_count = span_count;
        self.pairs = Vec::new();
        self.scratch = Vec::new();
        self.stage = Stage::Ready;
        Ok(())
    }
    fn span_row(&mut self, index: u64) -> Result<(u64, TableSpan)> {
        let page = index / PAGE_ROWS;
        if self.cache_page != Some(page) {
            let first = mul(page, PAGE_ROWS)?;
            let rows = (self.span_count - first).min(PAGE_ROWS);
            let bytes = size(mul(rows, 24)?)?;
            self.budget.io(0)?;
            let file = self
                .span_file
                .as_mut()
                .ok_or(ChildError::Invalid("missing span file"))?;
            file.seek(SeekFrom::Start(mul(first, 24)?))?;
            self.budget.io(bytes)?;
            file.read_exact(&mut self.cache[..bytes])?;
            let mut previous = None;
            for row in self.cache[..bytes].as_chunks::<24>().0 {
                let (p, s) = decode_span(row, self.count)?;
                if let Some((old, prior)) = previous {
                    if self.budget.compare(&old, &p)? != Ordering::Less || prior != s.offset {
                        return Err(ChildError::Corrupt("span page order"));
                    }
                }
                previous = Some((p, add(s.offset, s.count)?));
            }
            let (_, initial) = decode_span(&self.cache[..24], self.count)?;
            let (_, final_row) = decode_span(&self.cache[bytes - 24..bytes], self.count)?;
            if (first == 0 && initial.offset != 0)
                || (first + rows == self.span_count
                    && add(final_row.offset, final_row.count)? != self.count)
            {
                return Err(ChildError::Corrupt("span coverage"));
            }
            self.cache_page = Some(page);
            self.cache_rows = size(rows)?;
        }
        let at = size(index % PAGE_ROWS)?;
        if at >= self.cache_rows {
            return Err(ChildError::Corrupt("span row index"));
        }
        decode_span(&self.cache[at * 24..(at + 1) * 24], self.count)
    }
    /// Returns the exact child slice; an absent parent has canonical span `(0,0)`.
    ///
    /// # Errors
    /// Unfinished/poisoned state, malformed private bytes and exhausted runtime lookup budgets fail.
    pub fn span(&mut self, parent: BasinId) -> Result<TableSpan> {
        let result = self.span_inner(parent);
        self.abort(result)
    }
    fn span_inner(&mut self, parent: BasinId) -> Result<TableSpan> {
        match self.stage {
            Stage::Ready => {}
            Stage::Writing => return Err(ChildError::Invalid("unfinished")),
            Stage::Poisoned => return Err(ChildError::Poisoned),
        }
        let (mut lo, mut hi) = (0, self.span_count);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let (key, value) = self.span_row(mid)?;
            match self.budget.compare(&key, &parent.0)? {
                Ordering::Less => lo = mid + 1,
                Ordering::Greater => hi = mid,
                Ordering::Equal => return Ok(value),
            }
        }
        Ok(TableSpan {
            offset: 0,
            count: 0,
        })
    }
}
fn write_span(
    writer: &mut Writer,
    parent: u64,
    offset: u64,
    count: u64,
    budget: &mut Budget,
) -> Result<()> {
    let mut bytes = [0; 24];
    bytes[..8].copy_from_slice(&parent.to_le_bytes());
    bytes[8..16].copy_from_slice(&offset.to_le_bytes());
    bytes[16..].copy_from_slice(&count.to_le_bytes());
    writer.row(&bytes, budget)
}
fn decode_span(bytes: &[u8], total: u64) -> Result<(u64, TableSpan)> {
    if bytes.len() != 24 {
        return Err(ChildError::Corrupt("span width"));
    }
    let mut values = [0u64; 3];
    for (i, v) in values.iter_mut().enumerate() {
        *v = u64::from_le_bytes(
            bytes[i * 8..i * 8 + 8]
                .try_into()
                .map_err(|_| ChildError::Corrupt("span integer"))?,
        );
    }
    if values[2] == 0 || add(values[1], values[2])? > total {
        return Err(ChildError::Corrupt("span bounds"));
    }
    Ok((
        values[0],
        TableSpan {
            offset: values[1],
            count: values[2],
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
    static SERIAL: AtomicU64 = AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let id = SERIAL.fetch_add(1, AtomicOrdering::Relaxed);
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let p = std::env::temp_dir().join(format!(
                "arda-child-links-{}-{stamp}-{id}",
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
    fn generous(dir: &Path, buffer: u64, n: u64) -> ChildLimits {
        let mut l = ChildLimits::required(dir, buffer, n).unwrap();
        l.io_bytes += 10_000_000;
        l.io_operations += 100_000;
        l.comparisons += 1_000_000;
        l
    }
    fn child_bytes(pairs: &[(u64, u64)]) -> Vec<u8> {
        pairs.iter().flat_map(|p| p.1.to_le_bytes()).collect()
    }
    #[test]
    fn declared_lookup_reservation_covers_cold_pages_and_absent_keys() {
        let temp = Temp::new();
        let n = 1000;
        let queries = 1200;
        let limits = ChildLimits::required(&temp.0, 9, n)
            .unwrap()
            .with_lookups(queries)
            .unwrap();
        let mut links = DiskChildLinks::create(&temp.0, limits).unwrap();
        for id in (0..n).rev() {
            links.push(BasinId(2 * id + 10_000), BasinId(id)).unwrap();
        }
        links.finish(n).unwrap();
        for q in 0..queries {
            let id = (q * 683) % 2200;
            links.cache_page = None;
            let span = links.span(BasinId(10_000 + id)).unwrap();
            assert_eq!(span.count, u64::from(id < 2 * n && id % 2 == 0));
        }
        assert!(links.work().io_bytes <= limits.io_bytes);
        assert!(links.work().io_operations <= limits.io_operations);
        assert!(links.work().comparisons <= limits.comparisons);
        assert!(matches!(
            ChildLimits::required(&temp.0, 9, n)
                .unwrap()
                .with_lookups(u64::MAX),
            Err(ChildError::Overflow)
        ));
    }
    #[test]
    fn multiple_runs_and_generations_have_exact_canonical_spans() {
        for buffer in [1, 2, 17, 512] {
            let temp = Temp::new();
            let n = 257;
            let required = ChildLimits::required(&temp.0, buffer, n).unwrap();
            let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, buffer, n)).unwrap();
            let mut expected = Vec::new();
            for i in 0..n {
                let child = (i * 73) % n;
                let parent = 10_000 + child % 17;
                links.push(BasinId(parent), BasinId(child)).unwrap();
                expected.push((parent, child));
            }
            links.finish(n).unwrap();
            expected.sort_unstable();
            assert_eq!(
                fs::read(temp.0.join(CHILD_FILE)).unwrap(),
                child_bytes(&expected)
            );
            let work = links.work();
            assert!(work.io_bytes <= required.io_bytes);
            assert!(work.io_operations <= required.io_operations);
            assert!(work.comparisons <= required.comparisons);
            let mut index = Vec::new();
            let mut offset = 0;
            for parent in 10_000..10_017 {
                let count = expected.iter().filter(|p| p.0 == parent).count() as u64;
                let span = links.span(BasinId(parent)).unwrap();
                assert_eq!(span, TableSpan { offset, count });
                index.extend_from_slice(&parent.to_le_bytes());
                index.extend_from_slice(&offset.to_le_bytes());
                index.extend_from_slice(&count.to_le_bytes());
                offset += count;
            }
            assert_eq!(fs::read(temp.0.join(SPAN_FILE)).unwrap(), index);
            assert_eq!(
                links.span(BasinId(99)).unwrap(),
                TableSpan {
                    offset: 0,
                    count: 0
                }
            );
            let names: Vec<_> = fs::read_dir(&temp.0)
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect();
            assert_eq!(names.len(), 3);
        }
    }
    #[test]
    fn empty_one_link_and_large_parent_use_the_same_stream_contract() {
        for n in [0, 1, 1025] {
            let temp = Temp::new();
            let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, 7, n)).unwrap();
            for child in (0..n).rev() {
                links.push(BasinId(9000), BasinId(child)).unwrap();
            }
            links.finish(n).unwrap();
            assert_eq!(
                links.span(BasinId(9000)).unwrap(),
                TableSpan {
                    offset: 0,
                    count: n
                }
            );
            assert_eq!(fs::metadata(temp.0.join(CHILD_FILE)).unwrap().len(), 8 * n);
            assert_eq!(
                fs::metadata(temp.0.join(SPAN_FILE)).unwrap().len(),
                if n == 0 { 0 } else { 24 }
            );
        }
    }
    #[test]
    fn exact_construction_reservations_cover_short_and_partial_batches() {
        for n in 0..35 {
            for buffer in [1, 2, 3, 8, 64] {
                let temp = Temp::new();
                let limits = ChildLimits::required(&temp.0, buffer, n).unwrap();
                let mut links = DiskChildLinks::create(&temp.0, limits).unwrap();
                for child in (0..n).rev() {
                    links.push(BasinId(100), BasinId(child)).unwrap();
                }
                links.finish(n).unwrap();
                let work = links.work();
                assert!(work.io_bytes <= limits.io_bytes);
                assert!(work.io_operations <= limits.io_operations);
                assert!(work.comparisons <= limits.comparisons);
            }
        }
    }
    #[test]
    fn duplicate_pairs_within_and_across_runs_abort_permanently() {
        let temp = Temp::new();
        let mut same = DiskChildLinks::create(&temp.0, generous(&temp.0, 2, 2)).unwrap();
        same.push(BasinId(9), BasinId(1)).unwrap();
        assert!(matches!(
            same.push(BasinId(9), BasinId(1)),
            Err(ChildError::Duplicate)
        ));
        assert!(matches!(same.finish(2), Err(ChildError::Poisoned)));
        let temp = Temp::new();
        let mut apart = DiskChildLinks::create(&temp.0, generous(&temp.0, 1, 66)).unwrap();
        for child in 0..65 {
            apart.push(BasinId(9000), BasinId(child)).unwrap();
        }
        apart.push(BasinId(9000), BasinId(0)).unwrap();
        assert!(matches!(apart.finish(66), Err(ChildError::Duplicate)));
        assert!(matches!(
            apart.span(BasinId(9000)),
            Err(ChildError::Poisoned)
        ));
    }
    #[test]
    fn admission_and_namespace_failures_do_not_overwrite_files() {
        let temp = Temp::new();
        let required = ChildLimits::required(&temp.0, 2, 4).unwrap();
        for field in 0..5 {
            let mut l = required;
            match field {
                0 => l.ram_bytes -= 1,
                1 => l.scratch_bytes -= 1,
                2 => l.io_bytes -= 1,
                3 => l.io_operations -= 1,
                _ => l.comparisons -= 1,
            };
            assert!(matches!(
                DiskChildLinks::create(&temp.0, l),
                Err(ChildError::Limit(_))
            ));
            assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 0);
        }
        assert!(matches!(
            ChildLimits::required(&temp.0, 0, 1),
            Err(ChildError::Invalid(_))
        ));
        assert!(matches!(
            ChildLimits::required(&temp.0, u64::MAX, u64::MAX),
            Err(ChildError::Overflow)
        ));
        fs::write(temp.0.join("child-links.lock"), b"existing").unwrap();
        assert!(matches!(
            DiskChildLinks::create(&temp.0, required),
            Err(ChildError::Io(_))
        ));
        assert_eq!(
            fs::read(temp.0.join("child-links.lock")).unwrap(),
            b"existing"
        );
        let temp = Temp::new();
        let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, 1, 1)).unwrap();
        fs::write(temp.0.join("child-run-0-0.bin"), b"existing run").unwrap();
        assert!(matches!(
            links.push(BasinId(2), BasinId(1)),
            Err(ChildError::Io(_))
        ));
        assert_eq!(
            fs::read(temp.0.join("child-run-0-0.bin")).unwrap(),
            b"existing run"
        );
        assert!(matches!(links.finish(1), Err(ChildError::Poisoned)));
    }
    #[test]
    fn comparison_and_dirty_buffer_limits_stop_inside_the_operation() {
        let temp = Temp::new();
        let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, 8, 8)).unwrap();
        links.budget.limit.comparisons = 2;
        for child in 0..7 {
            links.push(BasinId(100), BasinId(7 - child)).unwrap();
        }
        assert!(matches!(
            links.push(BasinId(100), BasinId(0)),
            Err(ChildError::Limit("comparisons"))
        ));
        assert_eq!(links.work().comparisons, 2);
        assert!(!temp.0.join("child-run-0-0.bin").exists());
        let temp = Temp::new();
        let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, 2, 2)).unwrap();
        links.budget.limit.io_bytes = 0;
        links.push(BasinId(10), BasinId(2)).unwrap();
        assert!(matches!(
            links.push(BasinId(10), BasinId(1)),
            Err(ChildError::Limit("I/O"))
        ));
        assert_eq!(
            fs::metadata(temp.0.join("child-run-0-0.bin"))
                .unwrap()
                .len(),
            0
        );
        drop(links);
        assert_eq!(
            fs::metadata(temp.0.join("child-run-0-0.bin"))
                .unwrap()
                .len(),
            0
        );
    }
    #[test]
    fn counts_truncated_runs_and_corrupt_span_pages_are_rejected() {
        let temp = Temp::new();
        let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, 1, 2)).unwrap();
        links.push(BasinId(10), BasinId(1)).unwrap();
        assert!(matches!(
            links.finish(2),
            Err(ChildError::Invalid("expected link count"))
        ));
        assert!(!temp.0.join(CHILD_FILE).exists());
        let temp = Temp::new();
        let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, 1, 2)).unwrap();
        for i in 0..2 {
            links.push(BasinId(10), BasinId(i)).unwrap();
        }
        fs::write(temp.0.join("child-run-0-0.bin"), [0; 15]).unwrap();
        assert!(matches!(
            links.finish(2),
            Err(ChildError::Corrupt("run length"))
        ));
        let temp = Temp::new();
        let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, 3, 3)).unwrap();
        for i in 0..3 {
            links.push(BasinId(10 + i), BasinId(i)).unwrap();
        }
        links.finish(3).unwrap();
        let mut bytes = fs::read(temp.0.join(SPAN_FILE)).unwrap();
        bytes[16..24].fill(0);
        fs::write(temp.0.join(SPAN_FILE), bytes).unwrap();
        assert!(matches!(
            links.span(BasinId(10)),
            Err(ChildError::Corrupt("span bounds"))
        ));
        assert!(matches!(links.span(BasinId(11)), Err(ChildError::Poisoned)));
    }
    #[test]
    fn lookup_pages_are_bounded_and_budget_exhaustion_poisoned() {
        let temp = Temp::new();
        let n = 1000;
        let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, 31, n)).unwrap();
        for i in (0..n).rev() {
            links.push(BasinId(10_000 + i), BasinId(i)).unwrap();
        }
        links.finish(n).unwrap();
        for i in [0, 169, 170, 340, 511, 999, 341] {
            assert_eq!(
                links.span(BasinId(10_000 + i)).unwrap(),
                TableSpan {
                    offset: i,
                    count: 1
                }
            );
            assert!(links.cache_rows <= 170);
        }
        links.budget.limit.comparisons = links.work().comparisons;
        assert!(matches!(
            links.span(BasinId(10_501)),
            Err(ChildError::Limit("comparisons"))
        ));
        assert!(matches!(
            links.span(BasinId(10_501)),
            Err(ChildError::Poisoned)
        ));
    }
    #[test]
    fn real_buffered_write_error_is_charged_and_never_retried_on_drop() {
        let temp = Temp::new();
        let mut budget = Budget {
            limit: generous(&temp.0, 2, 2),
            work: ChildWork::default(),
        };
        let path = temp.0.join("read-only-writer.bin");
        let mut writer = Writer::create(&path, &mut budget).unwrap();
        writer.row(&[7; 32], &mut budget).unwrap();
        writer.file = File::open(&path).unwrap();
        assert!(matches!(
            writer.finish(32, &mut budget),
            Err(ChildError::Io(_))
        ));
        assert_eq!(budget.work.io_bytes, 32);
        assert_eq!(fs::metadata(path).unwrap().len(), 0);
    }
    #[test]
    fn individually_bounded_span_cannot_omit_the_child_table_prefix() {
        let temp = Temp::new();
        let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, 3, 3)).unwrap();
        for child in 0..3 {
            links.push(BasinId(10), BasinId(child)).unwrap();
        }
        links.finish(3).unwrap();
        let mut bytes = fs::read(temp.0.join(SPAN_FILE)).unwrap();
        bytes[8..16].copy_from_slice(&1_u64.to_le_bytes());
        bytes[16..24].copy_from_slice(&2_u64.to_le_bytes());
        fs::write(temp.0.join(SPAN_FILE), bytes).unwrap();
        assert!(matches!(
            links.span(BasinId(10)),
            Err(ChildError::Corrupt("span coverage"))
        ));
    }
}
