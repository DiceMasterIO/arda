//! The streamed final-minimum file read back with the producer's remaining
//! I/O budget.

use super::*;

/// One4096-byte minimum stream buffer with the producer's remaining I/O budget.
pub struct MinimumReader {
    file: Scratch,
    extent: Extent,
    count: u32,
    at: u32,
    buffer: Box<[u8; 4096]>,
    start: u32,
    loaded: u32,
    pub(super) meter: Meter,
    previous: Option<CellIndex>,
    comparisons: u64,
    comparison_limit: u64,
    failed: bool,
}
impl MinimumReader {
    pub(super) fn new(
        mut file: Scratch,
        extent: Extent,
        count: u32,
        mut meter: Meter,
        comparisons: u64,
        comparison_limit: u64,
    ) -> Result<Self, StageError> {
        let mut h = [0; 64];
        file.read(0, &mut h, &mut meter)?;
        if h != io::header(extent, 0, u64::from(count), 16)?
            || file.length(&mut meter)? != 64 + u64::from(count) * 16
        {
            return Err(StageError::Invalid("minimum header/length"));
        }
        Ok(Self {
            file,
            extent,
            count,
            at: 0,
            buffer: Box::new([0; 4096]),
            start: 0,
            loaded: 0,
            meter,
            previous: None,
            comparisons,
            comparison_limit,
            failed: false,
        })
    }
    /// Declared exact number of closed minima.
    pub fn count(&self) -> u32 {
        self.count
    }
    /// Producer I/O including minimum stream reads already consumed.
    pub fn io_work(&self) -> IoWork {
        self.meter.work
    }
    /// Producer ordering comparisons including final minimum-order validation.
    pub fn comparisons(&self) -> u64 {
        self.comparisons
    }
    /// Exact private table path; no directory discovery is needed.
    pub fn path(&self) -> &Path {
        &self.file.path
    }
    fn read_next(&mut self) -> Result<Minimum, StageError> {
        if self.at >= self.start + self.loaded {
            self.start = self.at;
            self.loaded = (self.count - self.at).min(256);
            let n = usize::try_from(self.loaded)
                .map_err(|_| StageError::Invalid("minimum batch"))?
                * 16;
            self.file.read(
                64 + u64::from(self.at) * 16,
                &mut self.buffer[..n],
                &mut self.meter,
            )?;
        }
        let i = usize::try_from(self.at - self.start)
            .map_err(|_| StageError::Invalid("minimum index"))?
            * 16;
        let m = io::minimum(&self.buffer[i..i + 16], self.extent)?;
        if self.previous.is_some() {
            if self.comparisons >= self.comparison_limit {
                return Err(StageError::Limit("key comparisons"));
            }
            self.comparisons += 1;
        }
        if self.previous.is_some_and(|p| p >= m.at) {
            return Err(StageError::Invalid("minimum order"));
        }
        self.previous = Some(m.at);
        self.at += 1;
        Ok(m)
    }
}
impl Iterator for MinimumReader {
    type Item = Result<Minimum, StageError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.failed || self.at == self.count {
            return None;
        }
        let result = self.read_next();
        if result.is_err() {
            self.failed = true;
        }
        Some(result)
    }
}
