//! Construction preflight ceilings and the work actually requested.
use super::*;

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
