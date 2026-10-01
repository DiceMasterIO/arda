//! The retained accepted-edge file read back as one bounded, fallible pass.
use super::*;

/// One bounded, fallible sequential pass over retained accepted edges.
/// A read error is yielded once; later iteration terminates without returning more edges.
pub struct EdgeReader {
    pub(super) reader: Reader,
    pub(super) path: PathBuf,
    pub(super) extent: Extent,
    pub(super) count: u64,
    pub(super) remaining: u64,
    pub(super) budget: Budget,
    pub(super) failed: bool,
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
