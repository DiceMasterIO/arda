//! Small deterministic backend for direct physical controls; never a world-sized fallback.
use super::fine_flow::{FlowRecord, FlowStore};
use super::routing::{CellIndex, Extent};
/// Checked tiny-control scratch, capped at 4096 cells independently of RAM admission.
pub struct MemoryFlowStore {
    extent: Extent,
    rows: Vec<FlowRecord>,
}
/// Memory admission or indexed access failed.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MemoryFlowError {
    /// The explicit small-control count/RAM reservation was exceeded.
    #[error("memory flow limit")]
    Limit,
    /// The ordinal is outside this store.
    #[error("memory flow index")]
    Index,
}
impl MemoryFlowStore {
    /// Create only a small admitted control store; initializes all records canonically.
    /// # Errors
    /// Rejects more than 4096 cells, insufficient RAM, arithmetic/allocation failures.
    pub fn new(extent: Extent, ram_bytes: u64) -> Result<Self, MemoryFlowError> {
        let n = usize::try_from(extent.cells()).map_err(|_| MemoryFlowError::Limit)?;
        let required = u64::from(extent.cells())
            .checked_mul(
                u64::try_from(std::mem::size_of::<FlowRecord>())
                    .map_err(|_| MemoryFlowError::Limit)?,
            )
            .and_then(|n| n.checked_add(4096))
            .ok_or(MemoryFlowError::Limit)?;
        if n > 4096 || required > ram_bytes {
            return Err(MemoryFlowError::Limit);
        }
        let mut rows = Vec::new();
        rows.try_reserve_exact(n)
            .map_err(|_| MemoryFlowError::Limit)?;
        rows.resize(n, FlowRecord::default());
        Ok(Self { extent, rows })
    }
}
impl FlowStore for MemoryFlowStore {
    type Error = MemoryFlowError;
    fn extent(&self) -> Extent {
        self.extent
    }
    fn read(&mut self, at: CellIndex) -> Result<FlowRecord, Self::Error> {
        self.rows
            .get(at.raw() as usize)
            .copied()
            .ok_or(MemoryFlowError::Index)
    }
    fn write(&mut self, at: CellIndex, row: FlowRecord) -> Result<(), Self::Error> {
        *self
            .rows
            .get_mut(at.raw() as usize)
            .ok_or(MemoryFlowError::Index)? = row;
        Ok(())
    }
}
