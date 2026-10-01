//! Stage limits for the shared solve and their summed admission: the
//! bridge ceilings and the conservative reservation total.

use super::*;

/// Additional accepted/child-vector and fixed file bridge admission.
#[derive(Debug, Clone, Copy)]
pub struct BridgeLimits {
    /// Accepted-edge/LeafFlow/child slices, fixed buffer and owned paths.
    pub ram_bytes: u64,
    /// Child-table payload bytes; other readers own their independent allowances.
    pub io_bytes: u128,
    /// Three subdirectory creates, child open/metadata and buffered reads.
    pub io_operations: u64,
}
/// Exact stage reservations; no stage silently borrows another stage's unused quota.
#[derive(Debug, Clone, Copy)]
pub struct SharedLimits {
    /// Private physical routing records and two reusable tapes, for all stages.
    pub routing_storage: routing_disk::DiskLimits,
    /// Fine marine connectivity work.
    pub ocean: ocean::Limits,
    /// Physical receiver and ownership work.
    pub routing: routing::Limits,
    /// Accepted-witness producer and sorter.
    pub mst: MstLimits,
    /// One additional accepted-file pass into the bounded retained edge slice.
    pub replay: ReadLimits,
    /// External child table sorter/index, including hierarchy lookups.
    pub child: ChildLimits,
    /// Hierarchy records/cache and finalized row reads.
    pub hierarchy_storage: hierarchy_disk::DiskLimits,
    /// Hierarchy work and maximum closed leaves.
    pub hierarchy: hierarchy::Limits,
    /// Maximum real dry spill junctions.
    pub binding_records: u64,
    /// Full registry payload allowance.
    pub binding_ram: u64,
    /// Actual witness-resolution routing reads.
    pub binding_reads: u64,
    /// Maximum retained nodes/leaves here; exact observed counts are passed to adapt.
    pub topology: TopologyLimits,
    /// Streamed grouped annual support bands.
    pub aggregation: AggregationLimits,
    /// Static support-cost model.
    pub annual: AnnualLimits,
    /// Final accepted-tree flow normalization.
    pub transfers: TransferLimits,
    /// Final connected lake rows.
    pub records: RecordLimits,
    /// Each of two annual/fine source passes receives this allowance afresh.
    pub source: SourceLimits,
    /// One mutable flow/metric file for all later stages and caller extraction.
    pub flow_storage: flow_disk::FlowLimits,
    /// Final physical fine flow; accepted_edges is a cap here, exact count at the call.
    pub flow: fine_flow::FlowLimits,
    /// Final drainage/order/HAND.
    pub metrics: flow_metrics::MetricsLimits,
    /// Additional bounded glue storage/I/O.
    pub bridge: BridgeLimits,
}
/// Conservative sum of all stage reservations, with no temporal reuse credit.
/// PreparedReader and borrowed continent inputs are separately owned by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reservations {
    /// Includes simultaneous owned stage output/cache reservations.
    pub ram_bytes: u64,
    /// Private files retained by the transaction.
    pub scratch_bytes: u64,
    /// Sum of actual backend/bridge requested-I/O ceilings.
    pub io_bytes: u128,
    /// Sum of actual backend/bridge file-operation ceilings.
    pub io_operations: u64,
}

/// Extra bridge payload ceiling for accepted edges, temporary leaf-flow projection and children.
#[must_use]
pub fn bridge_ram(directory: &Path, leaves: u64, nodes: u64) -> Option<u64> {
    let edge = u64::try_from(std::mem::size_of::<Saddle>()).ok()?;
    let leaf = u64::try_from(std::mem::size_of::<LeafFlow>()).ok()?;
    leaves
        .checked_mul(edge.checked_add(leaf)?.checked_mul(2)?)?
        .checked_add(nodes.checked_mul(16)?)?
        .checked_add(8192)?
        .checked_add(
            u64::try_from(directory.as_os_str().len())
                .ok()?
                .checked_add(128)?
                .checked_mul(8)?,
        )
}
impl SharedLimits {
    /// Check summed stage-owned resources before the parent admits its complete transaction.
    pub fn reservations(&self) -> Result<Reservations, SharedError> {
        let sum = |v: &[u64]| {
            v.iter().try_fold(0u64, |a, b| {
                a.checked_add(*b)
                    .ok_or(SharedError::Limit("reservation sum"))
            })
        };
        let ram_bytes = sum(&[
            self.routing_storage.cache_bytes,
            self.mst.ram_bytes,
            self.replay.ram_bytes,
            self.child.ram_bytes,
            self.hierarchy_storage.cache_bytes,
            self.binding_ram,
            self.topology.ram_bytes,
            self.aggregation.ram_bytes,
            self.annual.ram_bytes,
            self.transfers.ram_bytes,
            self.records.ram_bytes,
            self.source.ram_bytes,
            self.flow_storage.ram_bytes,
            self.metrics.ram_bytes,
            self.bridge.ram_bytes,
        ])?;
        let scratch_bytes = sum(&[
            self.routing_storage.scratch_bytes,
            self.mst.scratch_bytes,
            self.child.scratch_bytes,
            self.hierarchy_storage.scratch_bytes,
            self.flow_storage.scratch_bytes,
        ])?;
        let io_bytes = [
            self.routing_storage.io_bytes,
            self.mst.io_bytes,
            self.replay.io_bytes,
            self.child.io_bytes,
            self.hierarchy_storage.io_bytes,
            self.flow_storage.io_bytes,
            self.bridge.io_bytes,
        ]
        .into_iter()
        .try_fold(0u128, |a, b| {
            a.checked_add(b).ok_or(SharedError::Limit("I/O sum"))
        })?;
        let io_operations = sum(&[
            self.routing_storage.io_operations,
            self.mst.io_operations,
            self.replay.io_operations,
            self.child.io_operations,
            self.hierarchy_storage.io_operations,
            self.flow_storage.io_operations,
            self.bridge.io_operations,
        ])?;
        Ok(Reservations {
            ram_bytes,
            scratch_bytes,
            io_bytes,
            io_operations,
        })
    }
}
