//! Bounded external child-link ordering for a private hierarchy transaction.
use arda_core::{formats::hydrology::TableSpan, hydrology::BasinId};
use std::{
    cmp::Ordering,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

mod limits;
mod runs;
mod spans;
pub use limits::{ChildLimits, ChildWork};
use runs::{Pair, Reader, Writer};
use spans::write_span;

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
}

#[cfg(test)]
mod tests;
