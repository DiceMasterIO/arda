//! Accepted physical MST from replayed final receiver-owned saddle witnesses.
use super::{
    accepted_sort::{edge_key, AcceptedSorter, EdgeReader, SortError, SortLimits},
    hierarchy::Minimum,
    io::{self, IoWork, Meter, Scratch, StageError},
    routing::{CellIndex, Extent, OutletKind, Receiver, RoutingStore},
    saddles::{self, Node, Saddle, ScanError},
    slots::{SlotStore, SlotWork},
};
use std::path::Path;

mod minimums;
pub use minimums::MinimumReader;
/// Whole-stage reservations; accepted sorting receives a checked partition.
#[derive(Debug, Clone, Copy)]
pub struct MstLimits {
    /// Maximum closed terminals admitted by the enclosing world.
    pub max_leaves: u64,
    /// Combined live ordinary RAM for cache, paths, fixed buffers and sorter.
    pub ram_bytes: u64,
    /// Combined producer and accepted-sort scratch lengths.
    pub scratch_bytes: u64,
    /// Requested producer+sorter I/O payload budget, including final readers.
    pub io_bytes: u128,
    /// Producer+sorter file-operation budget, including final readers.
    pub io_operations: u64,
    /// Number of resident64-slot pages; positive even for an empty closed set.
    pub cache_pages: u64,
    /// Bounded accepted-edge construction buffer size.
    pub sort_buffer_records: u64,
    /// Checked fixed-slot reads.
    pub slot_reads: u64,
    /// Checked fixed-slot writes.
    pub slot_writes: u64,
    /// Producer and sorter total explicit key comparisons.
    pub comparisons: u64,
    /// Actual routing record reads across counting, indexing and all saddle replays.
    pub routing_reads: u64,
    /// Candidate witnesses across all saddle replays.
    pub scan_candidates: u64,
}
/// Successful producer work; final reader work remains inspectable on each reader.
#[derive(Debug, Default, Clone, Copy)]
pub struct Work {
    /// Actual closed physical minima.
    pub leaves: u32,
    /// Completed Boruvka rounds.
    pub rounds: u8,
    /// Count/index/replay routing records read.
    pub routing_reads: u64,
    /// Actual saddle witnesses inspected across complete scans.
    pub scan_candidates: u64,
    /// Producer-only canonical edge comparisons, excluding sorter counters.
    pub comparisons: u64,
    /// Exact accepted physical edges, equal to leaves on success.
    pub accepted: u32,
    /// Producer fixed-slot accesses.
    pub slots: SlotWork,
    /// Producer private I/O before returning the two final streams.
    pub io: IoWork,
}
/// Typed failure from the routing authority, producer or accepted-edge sorter.
#[derive(Debug, thiserror::Error)]
pub enum MstError<R> {
    /// Caller-owned routing storage failed.
    #[error("MST routing backend failed")]
    Routing(#[source] R),
    /// Producer input, graph, budget or private I/O failed.
    #[error(transparent)]
    Stage(#[from] StageError),
    /// Accepted-edge sorting or its private I/O failed.
    #[error(transparent)]
    Sort(#[from] SortError),
}
/// Two bounded final streams with their exact domain and successful build work.
/// Pass `&mut minima` and `&mut edges` to the later hierarchy builder after mapping
/// their typed errors into its backend error. No full minima/edge vector is loaded.
pub struct MstProduct {
    /// Indexed minima in stable packed-coordinate order.
    pub minima: MinimumReader,
    /// Canonically sorted exact accepted witnesses.
    pub edges: EdgeReader,
    /// Actual modeled extent.
    pub extent: Extent,
    /// Completed producer measurements.
    pub work: Work,
}
fn minimum_scan<S: RoutingStore>(
    routing: &mut S,
    limit: u64,
    work: &mut Work,
    mut sink: impl FnMut(Minimum) -> Result<(), StageError>,
) -> Result<u32, MstError<S::Error>> {
    let e = routing.extent();
    let mut count = 0_u32;
    for raw in 0..e.cells() {
        if work.routing_reads >= limit {
            return Err(StageError::Limit("routing reads").into());
        }
        work.routing_reads += 1;
        let at = CellIndex::new(raw, e).ok_or(StageError::Invalid("minimum ordinal"))?;
        let r = routing.read(at).map_err(MstError::Routing)?;
        if r.owner().is_none() {
            return Err(StageError::Invalid("unresolved receiver owner").into());
        }
        match r
            .receiver(e, at)
            .ok_or(StageError::Invalid("unresolved receiver"))?
        {
            Receiver::Stop(OutletKind::ClosedDepression) => {
                if r.owner() != Some(at) || r.is_marine() {
                    return Err(StageError::Invalid("invalid closed terminal").into());
                }
                sink(Minimum {
                    at,
                    floor_mm: r.height(),
                })?;
                count = count
                    .checked_add(1)
                    .ok_or(StageError::Limit("minimum count"))?;
            }
            Receiver::Stop(OutletKind::MarineEntry) => {
                if !r.is_marine() || r.owner() != Some(at) {
                    return Err(StageError::Invalid("invalid marine terminal").into());
                }
            }
            Receiver::Stop(OutletKind::DomainExport) => {
                if r.is_marine() || r.owner() != Some(at) {
                    return Err(StageError::Invalid("invalid export terminal").into());
                }
            }
            Receiver::Cell(_) => {}
        }
    }
    Ok(count)
}
fn node(store: &mut SlotStore, n: Node) -> Result<u32, StageError> {
    match n {
        Node::Closed(at) => store.bind(at),
        Node::Exterior => Ok(store.count() - 1),
    }
}
fn offer(store: &mut SlotStore, e: Extent, edge: Saddle) -> Result<(), StageError> {
    if Saddle::decode(edge.encode(), e) != Some(edge) {
        return Err(StageError::Invalid("candidate witness"));
    }
    let a = node(store, edge.left)?;
    let b = node(store, edge.right)?;
    let a = store.read(a)?.frozen;
    let b = store.read(b)?.frozen;
    if a == b {
        return Ok(());
    }
    for root in [a, b] {
        let mut row = store.read(root)?;
        if row.frozen != root {
            return Err(StageError::Invalid("frozen root"));
        }
        let replace = if let Some(old) = row.best {
            store.compare()?;
            edge_key(e, edge) < edge_key(e, old)
        } else {
            true
        };
        if replace {
            row.best = Some(edge);
            store.write(root, row)?;
        }
    }
    Ok(())
}
fn finish_round(
    store: &mut SlotStore,
    accepted: &mut AcceptedSorter,
    work: &mut Work,
) -> Result<u32, MstError<std::convert::Infallible>> {
    for i in 0..store.count() {
        let r = store.read(i)?;
        if r.frozen == i && r.best.is_none() {
            return Err(StageError::Disconnected.into());
        }
    }
    let mut joined = 0_u32;
    for i in 0..store.count() {
        let r = store.read(i)?;
        if r.frozen != i {
            continue;
        }
        let edge = r.best.ok_or(StageError::Disconnected)?;
        let a = node(store, edge.left)?;
        let b = node(store, edge.right)?;
        let ra = store.read(a)?.frozen;
        let rb = store.read(b)?.frozen;
        let other = if ra == i {
            rb
        } else if rb == i {
            ra
        } else {
            return Err(StageError::Invalid("minimum edge leaves its component").into());
        };
        store.compare()?;
        if store.read(other)?.best == Some(edge) && other < i {
            continue;
        }
        if !store.join(a, b)? {
            return Err(StageError::Cycle.into());
        }
        accepted.push(edge)?;
        joined = joined
            .checked_add(1)
            .ok_or(StageError::Limit("accepted count"))?;
        work.accepted = work
            .accepted
            .checked_add(1)
            .ok_or(StageError::Limit("accepted count"))?;
    }
    Ok(joined)
}
/// Produce the canonical accepted physical MST without retaining candidate edges.
///
/// Count actual terminals first, read-only; then admit every live RAM/scratch
/// reservation before file creation. Component labels remain frozen throughout
/// each complete real saddle scan. The unique edge key agrees with hierarchy.
///
/// # Errors
/// Fails on unresolved/malformed input, unknown owners, disconnected/cyclic
/// components, exhausted declared resources, routing errors and private I/O.
pub fn produce<S: RoutingStore>(
    routing: &mut S,
    directory: &Path,
    limits: MstLimits,
) -> Result<MstProduct, MstError<S::Error>> {
    let extent = routing.extent();
    let mut work = Work::default();
    let leaves = minimum_scan(routing, limits.routing_reads, &mut work, |_| Ok(()))?;
    work.leaves = leaves;
    if u64::from(leaves) > limits.max_leaves {
        return Err(StageError::ClosedLeafLimit {
            observed: leaves,
            limit: limits.max_leaves,
        }
        .into());
    }
    if limits
        .routing_reads
        .checked_sub(work.routing_reads)
        .is_none_or(|remaining| remaining < u64::from(extent.cells()))
    {
        return Err(StageError::Limit("minimum indexing reads").into());
    }
    let sort_limits =
        SortLimits::required(directory, u64::from(leaves), limits.sort_buffer_records)?;
    let producer_ram = SlotStore::required_ram(directory, limits.cache_pages)?
        .checked_add(8192)
        .ok_or(StageError::Limit("RAM bytes"))?;
    if producer_ram
        .checked_add(sort_limits.ram_bytes)
        .ok_or(StageError::Limit("RAM bytes"))?
        > limits.ram_bytes
    {
        return Err(StageError::Limit("RAM bytes").into());
    }
    let n = leaves
        .checked_add(1)
        .ok_or(StageError::Limit("slot count"))?;
    let min_bytes = 64 + u64::from(leaves) * 16;
    let producer_scratch = SlotStore::required_bytes(n)
        .checked_add(min_bytes)
        .ok_or(StageError::Limit("scratch bytes"))?;
    if producer_scratch
        .checked_add(sort_limits.scratch_bytes)
        .ok_or(StageError::Limit("scratch bytes"))?
        > limits.scratch_bytes
    {
        return Err(StageError::Limit("scratch bytes").into());
    }
    let io_bytes = limits
        .io_bytes
        .checked_sub(sort_limits.io_bytes)
        .ok_or(StageError::Limit("sort I/O reservation"))?;
    let io_operations = limits
        .io_operations
        .checked_sub(sort_limits.io_operations)
        .ok_or(StageError::Limit("sort I/O reservation"))?;
    let comparison_limit = limits
        .comparisons
        .checked_sub(sort_limits.comparisons)
        .ok_or(StageError::Limit("sort comparison reservation"))?;
    let pages = u64::from(n).div_ceil(64);
    let l = u64::from(leaves);
    // Includes both initial scans' table I/O, slot initialization and the complete
    // final minimum reader. Routing-authority reads have their independent limit.
    let initial_bytes = 256_u128 + 48 * u128::from(leaves) + u128::from(pages) * 4096;
    let initial_ops = 12 + 2 * (2 * l.div_ceil(256) + l.div_ceil(64) + pages);
    if initial_bytes > io_bytes || initial_ops > io_operations {
        return Err(StageError::Limit("known minimum I/O reservation").into());
    }
    let mut meter = Meter::new(io_bytes, io_operations);
    let mut minima = Scratch::create(directory.join("mst-minima.rows"), &mut meter)?;
    minima.write(
        0,
        &io::header(extent, 0, u64::from(leaves), 16)?,
        &mut meter,
    )?;
    let mut buffer = [0_u8; 4096];
    let mut buffered = 0usize;
    let mut written = 0u64;
    let actual = minimum_scan(routing, limits.routing_reads, &mut work, |m| {
        buffer[buffered..buffered + 16].copy_from_slice(&io::minimum_bytes(m));
        buffered += 16;
        if buffered == buffer.len() {
            minima.write(64 + written, &buffer, &mut meter)?;
            written += buffered as u64;
            buffered = 0;
        }
        Ok(())
    })?;
    if actual != leaves {
        return Err(StageError::Invalid("terminal authority changed while indexing").into());
    }
    if buffered > 0 {
        minima.write(64 + written, &buffer[..buffered], &mut meter)?;
    }
    let mut slots = SlotStore::create(
        directory,
        extent,
        leaves,
        &mut minima,
        meter,
        super::slots::Limits {
            cache_pages: limits.cache_pages,
            reads: limits.slot_reads,
            writes: limits.slot_writes,
            comparisons: comparison_limit,
        },
    )?;
    let mut sorter = AcceptedSorter::create(directory, extent, u64::from(leaves), sort_limits)?;
    let mut components = n;
    while components > 1 {
        if work.rounds >= 32 {
            return Err(StageError::Invalid("Boruvka round bound").into());
        }
        for i in 0..n {
            let root = slots.root(i)?;
            let mut r = slots.read(i)?;
            r.frozen = root;
            r.best = None;
            slots.write(i, r)?;
        }
        let scan_limits = saddles::Limits {
            reads: limits
                .routing_reads
                .checked_sub(work.routing_reads)
                .ok_or(StageError::Limit("routing reads"))?,
            candidates: limits
                .scan_candidates
                .checked_sub(work.scan_candidates)
                .ok_or(StageError::Limit("candidate count"))?,
        };
        let scan = saddles::scan(routing, scan_limits, |edge| offer(&mut slots, extent, edge));
        let scan = match scan {
            Ok(v) => v,
            Err(ScanError::Storage(e)) => return Err(MstError::Routing(e)),
            Err(ScanError::Output(e)) => return Err(MstError::Stage(e)),
            Err(ScanError::WorkLimit) => return Err(StageError::Limit("saddle scan").into()),
            Err(ScanError::InvalidOwnership) => {
                return Err(StageError::Invalid("saddle ownership").into())
            }
        };
        work.routing_reads = work
            .routing_reads
            .checked_add(scan.reads)
            .ok_or(StageError::Limit("routing reads"))?;
        work.scan_candidates = work
            .scan_candidates
            .checked_add(scan.candidates)
            .ok_or(StageError::Limit("candidate count"))?;
        let joined = match finish_round(&mut slots, &mut sorter, &mut work) {
            Ok(n) => n,
            Err(MstError::Routing(never)) => match never {},
            Err(MstError::Stage(e)) => return Err(e.into()),
            Err(MstError::Sort(e)) => return Err(e.into()),
        };
        let after = components
            .checked_sub(joined)
            .ok_or(StageError::Invalid("component count"))?;
        if after == 0 || after > components / 2 {
            return Err(StageError::Invalid("Boruvka halving invariant").into());
        }
        components = after;
        work.rounds += 1;
    }
    if work.accepted != leaves {
        return Err(StageError::Invalid("accepted MST count").into());
    }
    work.slots = slots.work();
    work.comparisons = work.slots.comparisons;
    let meter = slots.finish()?;
    let edges = sorter.finish()?;
    let minima = MinimumReader::new(
        minima,
        extent,
        leaves,
        meter,
        work.comparisons,
        comparison_limit,
    )?;
    work.io = minima.io_work();
    Ok(MstProduct {
        minima,
        edges,
        extent,
        work,
    })
}

#[cfg(test)]
mod tests;
