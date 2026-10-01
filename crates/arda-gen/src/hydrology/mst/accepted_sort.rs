//! Bounded external ordering of accepted physical MST witnesses.
use super::routing::{CellIndex, Extent};
use super::saddles::{Node, Saddle};
use std::{
    cmp::Ordering,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
mod limits;
mod reader;
mod runs;
pub use limits::{ReadLimits, SortLimits, SortWork};
pub use reader::EdgeReader;
use runs::{Reader, Writer};

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

#[cfg(test)]
mod tests;
