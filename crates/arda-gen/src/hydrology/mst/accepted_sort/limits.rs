//! Sort and replay reservations, and the work actually requested.
use super::*;

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
