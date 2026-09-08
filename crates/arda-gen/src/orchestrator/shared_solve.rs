//! Concrete private annual solve; final area/global publication belongs to the caller.
#![deny(missing_docs)]
use super::{
    annual_source::{self, AnnualSource, CellSourceError, SourceError, SourceLimits, SourceWork},
    child_links::{self, ChildLimits, DiskChildLinks},
    flow_disk::{self, DiskFlowStore},
    prepared_files::{PreparedError, PreparedReader},
    routing_disk::{self, DiskRoutingStore},
};
use crate::continent::{climate::ContinentClimate, ContinentGrid};
use crate::hydrology::{
    annual::{self, AnnualInput, AnnualLimits, AnnualNode, AnnualSolution},
    annual_aggregation::{self, AggregationLimits, Aggregator},
    annual_records::{self, RecordLimits},
    annual_topology::{self, TopologyLimits},
    annual_transfers::{self, LeafFlow, TransferLimits},
    fine_flow::{self, FlowRecord, FlowTarget},
    flow_metrics,
    handoff::{self, HandoffError},
    hierarchy::{self},
    hierarchy_disk::{self, DiskHierarchyStore},
    mst::{self, MstLimits, ReadLimits},
    ocean,
    routing::{self, Extent},
    saddles::Saddle,
    witness_binding::{BindingError, BreakpointRegistry},
};
use arda_core::{
    hydrology::{AnnualWaterBalance, BasinId, GlobalLake, Litres},
    GenerateConfig,
};
use std::{
    convert::Infallible,
    fs::{self, File},
    io::Read,
    path::Path,
};
type RoutingError = routing_disk::DiskError;
type HierarchyError = hierarchy_disk::DiskError;

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
/// All completed physical authorities needed by the caller's read-only extraction.
/// No final area/global file has been published. Drop/abort remains caller-owned.
pub struct SharedArtifacts {
    /// Physical heights, final marine membership and original immutable receiver ownership.
    pub routing: DiskRoutingStore,
    /// Final signed flow and completed drainage/order/HAND scratch.
    pub flow: DiskFlowStore,
    /// Finalized hierarchy rows and retained child table in the hierarchy directory.
    pub hierarchy: DiskHierarchyStore,
    /// Actual canonical dry spill junctions.
    #[allow(dead_code)] // Retained solved topology for diagnostics.
    pub registry: BreakpointRegistry,
    /// Sorted retained physical nodes for potential-spill/catchment identity lookup.
    pub nodes: Vec<AnnualNode>,
    /// Static representative membership and immutable leaf accounts.
    pub solution: AnnualSolution,
    /// Canonical final connected-lake records.
    pub lakes: Vec<GlobalLake>,
    /// Reconciled whole-domain annual water ledger, including direct sea/domain catchments.
    pub balance: AnnualWaterBalance,
    /// Completed logical stage measurements.
    #[allow(dead_code)] // Optional caller-facing stage diagnostics.
    pub work: SharedWork,
}
/// Completed logical work; retained disk handles expose their actual I/O counters separately.
#[derive(Debug)]
// Stage reports are retained even when the batch caller ignores individual counters.
#[allow(dead_code)]
pub struct SharedWork {
    /// Fine marine flood.
    pub ocean: ocean::Work,
    /// Fine receiver ownership.
    pub routing: routing::Work,
    /// Accepted physical tree production.
    pub mst: mst::Work,
    /// Hierarchy construction.
    pub hierarchy: hierarchy::Work,
    /// Actual witness source reads.
    pub binding_reads: u64,
    /// Annual and fine-source passes in that order.
    pub sources: [SourceWork; 2],
    /// Final physical flow.
    pub flow: fine_flow::FlowWork,
    /// Final metric work.
    pub metrics: flow_metrics::MetricsWork,
    /// Counted glue payload reads.
    pub bridge_bytes: u128,
    /// Counted glue filesystem calls.
    pub bridge_operations: u64,
}
/// Original source or exact outward-ledger failure inside fine-flow callbacks.
#[derive(Debug, thiserror::Error)]
pub enum FineInputError {
    /// Prepared forcing/routing input failure.
    #[error(transparent)]
    Source(#[from] CellSourceError<RoutingError>),
    /// Whole-world outward account cannot be represented.
    #[error("fine export sum overflow")]
    Overflow,
}
/// Any error abandons this private transaction; no partially initialized file is reopenable.
#[derive(Debug, thiserror::Error)]
pub enum SharedError {
    /// Failed request/cap/known-count consistency check.
    #[error("invalid shared solve: {0}")]
    Invalid(&'static str),
    /// Glue allocation, arithmetic or declared reservation failure.
    #[error("shared solve exceeded {0}")]
    Limit(&'static str),
    /// Exact glue filesystem failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Fresh source-to-routing file creation failure.
    #[error("routing initialization failed")]
    RoutingCreate(#[source] routing_disk::CreateError<PreparedError>),
    /// Actual routing storage error.
    #[error(transparent)]
    Routing(#[from] RoutingError),
    /// Fine marine flood failure.
    #[error("ocean connectivity failed")]
    Ocean(#[source] ocean::OceanError<RoutingError>),
    /// Physical routing failure.
    #[error("physical routing failed")]
    Route(#[source] routing::RoutingError<RoutingError>),
    /// Physical accepted-tree production failure.
    #[error("physical MST failed")]
    Mst(#[source] mst::MstError<RoutingError>),
    /// Exact accepted-file replay failure.
    #[error(transparent)]
    Replay(#[from] mst::SortError),
    /// Child sorting failure.
    #[error(transparent)]
    Child(#[from] child_links::ChildError),
    /// Hierarchy backend failure.
    #[error(transparent)]
    HierarchyStore(#[from] HierarchyError),
    /// Actual breakpoint admission failure.
    #[error("spill registry failed")]
    Binding(#[source] BindingError<RoutingError>),
    /// Hierarchy/witness handoff failure.
    #[error("hierarchy construction failed")]
    Hierarchy(#[source] hierarchy::BuildError<HandoffError<HierarchyError, RoutingError>>),
    /// Retained topology adaptation failure.
    #[error("annual topology failed")]
    Topology(#[source] annual_topology::TopologyError<HierarchyError>),
    /// Source/forcing admission or incomplete pass.
    #[error(transparent)]
    Source(#[from] SourceError),
    /// Actual canonical annual cell read.
    #[error(transparent)]
    Cell(#[from] CellSourceError<RoutingError>),
    /// Source/band aggregation failure.
    #[error(transparent)]
    Aggregation(#[from] annual_aggregation::AggregationError),
    /// Static representative solve failure.
    #[error(transparent)]
    Annual(#[from] annual::AnnualError),
    /// Accepted-tree normalization failure.
    #[error(transparent)]
    Transfers(#[from] annual_transfers::TransferError),
    /// Connected row or final ledger failure.
    #[error("annual record reconciliation failed")]
    Records(#[source] annual_records::RecordError<RoutingError>),
    /// Fresh flow-file creation failure.
    #[error("flow initialization failed")]
    FlowCreate(#[source] flow_disk::CreateError<Infallible>),
    /// Exact mutable flow/metric storage failure.
    #[error(transparent)]
    FlowStore(#[from] flow_disk::FlowDiskError),
    /// Actual source/tree fine-flow failure.
    #[error("fine flow failed")]
    Flow(#[source] fine_flow::FlowError<RoutingError, flow_disk::FlowDiskError, FineInputError>),
    /// Final metric failure.
    #[error("flow metrics failed")]
    Metrics(#[source] flow_metrics::MetricsError<RoutingError, flow_disk::FlowDiskError>),
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
struct Bridge {
    limits: BridgeLimits,
    bytes: u128,
    operations: u64,
}
impl Bridge {
    fn io(&mut self, bytes: u64) -> Result<(), SharedError> {
        self.bytes = self
            .bytes
            .checked_add(u128::from(bytes))
            .ok_or(SharedError::Limit("bridge bytes"))?;
        self.operations = self
            .operations
            .checked_add(1)
            .ok_or(SharedError::Limit("bridge calls"))?;
        if self.bytes > self.limits.io_bytes || self.operations > self.limits.io_operations {
            return Err(SharedError::Limit("bridge I/O"));
        }
        Ok(())
    }
    fn children(&mut self, path: &Path, count: u64) -> Result<Vec<BasinId>, SharedError> {
        let bytes = count
            .checked_mul(8)
            .ok_or(SharedError::Limit("child length"))?;
        self.io(0)?;
        let mut file = File::open(path)?;
        self.io(0)?;
        if file.metadata()?.len() != bytes {
            return Err(SharedError::Invalid("final child file length"));
        }
        let mut rows = reserved(count)?;
        let mut buffer = [0u8; 4096];
        let mut left = bytes;
        while left > 0 {
            let n = left.min(4096);
            self.io(n)?;
            let n = usize::try_from(n).map_err(|_| SharedError::Limit("child chunk"))?;
            file.read_exact(&mut buffer[..n])?;
            for b in buffer[..n].chunks_exact(8) {
                rows.push(BasinId(u64::from_le_bytes(
                    b.try_into()
                        .map_err(|_| SharedError::Invalid("child row"))?,
                )));
            }
            left -= u64::try_from(n).map_err(|_| SharedError::Limit("child chunk"))?;
        }
        Ok(rows)
    }
}
fn reserved<T>(count: u64) -> Result<Vec<T>, SharedError> {
    let mut v = Vec::new();
    v.try_reserve_exact(usize::try_from(count).map_err(|_| SharedError::Limit("vector count"))?)
        .map_err(|_| SharedError::Limit("vector allocation"))?;
    Ok(v)
}
/// Solve only inside an existing private directory. Fixed subdirectories use create_dir
/// and all file writers use fresh-file contracts; an existing stage is never overwritten.
/// Failed transactions are caller-aborted, never reopened from file existence/length.
/// On success both mutable stores are flushed before returning extraction handles.
#[allow(clippy::too_many_lines)]
pub fn solve(
    prepared: &mut PreparedReader,
    grid: &ContinentGrid,
    climate: &ContinentClimate,
    config: GenerateConfig,
    directory: &Path,
    limits: SharedLimits,
) -> Result<SharedArtifacts, SharedError> {
    let domain = prepared.domain();
    let expected = crate::hydrology::prepared_domain::PreparedDomain::for_config(config)
        .map_err(|_| SharedError::Invalid("request domain"))?;
    if domain != expected {
        return Err(SharedError::Invalid("prepared request"));
    }
    let extent =
        Extent::new(domain.width(), domain.height()).ok_or(SharedError::Invalid("extent"))?;
    limits.reservations()?;
    if bridge_ram(directory, limits.topology.leaves, limits.topology.nodes)
        .is_none_or(|n| n > limits.bridge.ram_bytes)
        || annual_source::required_ram(domain).is_none_or(|n| n > limits.source.ram_bytes)
        || annual_source::required_operations(domain).is_none_or(|n| n > limits.source.operations)
    {
        return Err(SharedError::Limit("bridge/source preflight"));
    }
    let mut bridge = Bridge {
        limits: limits.bridge,
        bytes: 0,
        operations: 0,
    };
    let routing_dir = directory.join("routing");
    let mst_dir = directory.join("mst");
    let hierarchy_dir = directory.join("hierarchy");
    for p in [&routing_dir, &mst_dir, &hierarchy_dir] {
        bridge.io(0)?;
        fs::create_dir(p)?;
    }
    let mut routes = DiskRoutingStore::try_create(
        &routing_dir,
        extent,
        prepared.routing_records(),
        limits.routing_storage,
    )
    .map_err(SharedError::RoutingCreate)?;
    let ocean_work = ocean::connect(&mut routes, limits.ocean).map_err(SharedError::Ocean)?;
    let routing_work =
        routing::route_and_own(&mut routes, limits.routing).map_err(SharedError::Route)?;
    routes.flush()?;
    let product = mst::produce(&mut routes, &mst_dir, limits.mst).map_err(SharedError::Mst)?;
    let mst_work = product.work;
    let leaves = u64::from(mst_work.leaves);
    if leaves > limits.topology.leaves || leaves > limits.flow.accepted_edges {
        return Err(SharedError::Limit("accepted leaves"));
    }
    let mut accepted = reserved(leaves)?;
    for edge in product.edges.replay(limits.replay)? {
        if u64::try_from(accepted.len()).map_err(|_| SharedError::Limit("edges"))? >= leaves {
            return Err(SharedError::Invalid("extra accepted edge"));
        }
        accepted.push(edge?);
    }
    if u64::try_from(accepted.len()).map_err(|_| SharedError::Limit("edges"))? != leaves {
        return Err(SharedError::Invalid("missing accepted edge"));
    }
    let children = DiskChildLinks::create(&hierarchy_dir, limits.child)?;
    let mut hierarchy =
        DiskHierarchyStore::new(&hierarchy_dir, extent, limits.hierarchy_storage, children)?;
    let mut registry = BreakpointRegistry::new(limits.binding_records, limits.binding_ram)
        .map_err(SharedError::Binding)?;
    let (hierarchy_work, binding_reads) = handoff::build_from_mst(
        product,
        &mut routes,
        &mut hierarchy,
        &mut registry,
        limits.hierarchy,
        limits.binding_reads,
    )
    .map_err(SharedError::Hierarchy)?;
    hierarchy.finish()?;
    let (nodes, elders) = hierarchy.output_counts()?;
    if nodes > limits.topology.nodes || elders != leaves || hierarchy_work.links > nodes {
        return Err(SharedError::Limit("retained topology"));
    }
    let children = bridge.children(
        &hierarchy_dir.join(child_links::CHILD_FILE),
        hierarchy_work.links,
    )?;
    let topology = annual_topology::adapt(
        extent,
        (0..nodes).map(|i| hierarchy.output_node(i)),
        &children,
        accepted.iter().copied().map(Ok::<_, HierarchyError>),
        &registry,
        TopologyLimits {
            nodes,
            leaves,
            ..limits.topology
        },
    )
    .map_err(SharedError::Topology)?;
    let mut aggregator = Aggregator::new(extent, topology.nodes, limits.aggregation)?;
    let source_work = {
        let mut source = AnnualSource::new(prepared, grid, climate, config, limits.source)?;
        for raw in 0..extent.cells() {
            let at = routing::CellIndex::new(raw, extent)
                .ok_or(SharedError::Invalid("source ordinal"))?;
            aggregator.push(&source.cell(&mut routes, at)?)?;
        }
        source.finish()?
    };
    let aggregate = aggregator.finish()?;
    let mut solution = annual::solve_annual(
        AnnualInput {
            nodes: &aggregate.nodes,
            children: &children,
            bands: &aggregate.bands,
            joins: &topology.joins,
        },
        limits.annual,
    )?;
    let mut leaf_flow = reserved(leaves)?;
    for l in &solution.leaf_net {
        leaf_flow.push(LeafFlow {
            leaf: l.leaf,
            net_litres: l.net_litres,
            lake: l.lake,
            surface: l.surface,
        });
    }
    let transfers =
        annual_transfers::normalize_transfers(extent, &leaf_flow, &accepted, limits.transfers)?;
    drop(leaf_flow);
    let lakes = annual_records::lake_records(
        &mut routes,
        &aggregate.nodes,
        &solution,
        &transfers.transfers,
        limits.records,
    )
    .map_err(SharedError::Records)?;
    drop(transfers);
    drop(children);
    drop(topology.joins);
    let annual_aggregation::Aggregation {
        nodes,
        bands,
        catchment_cells,
        sea,
        domain: direct_domain,
        ..
    } = aggregate;
    drop(bands);
    drop(catchment_cells);
    let mut flow = DiskFlowStore::create(
        &directory.join("flow.bin"),
        extent,
        (0..extent.cells()).map(|_| Ok::<_, Infallible>(FlowRecord::default())),
        limits.flow_storage,
    )
    .map_err(SharedError::FlowCreate)?;
    let (mut sea_out, mut domain_out) = (0u128, 0u128);
    let (flow_work, fine_source_work) = {
        let mut source = AnnualSource::new(prepared, grid, climate, config, limits.source)?;
        let work = fine_flow::run_with_source(
            &mut routes,
            &mut flow,
            |routing, at| {
                source
                    .fine_cell(routing, at, &solution.leaf_net)
                    .map_err(FineInputError::Source)
            },
            accepted.iter().copied().map(Ok::<_, FineInputError>),
            fine_flow::FlowLimits {
                accepted_edges: leaves,
                ..limits.flow
            },
            |edge| {
                let target = match edge.to {
                    FlowTarget::Sea(_) => Some(&mut sea_out),
                    FlowTarget::DomainExport => Some(&mut domain_out),
                    FlowTarget::Cell(_) => None,
                };
                if let Some(sum) = target {
                    *sum = sum
                        .checked_add(edge.annual.0)
                        .ok_or(FineInputError::Overflow)?;
                }
                Ok(())
            },
        )
        .map_err(SharedError::Flow)?;
        (work, source.finish()?)
    };
    drop(accepted);
    let balance = annual_records::reconcile_balance(
        solution.balance,
        sea,
        direct_domain,
        Litres(sea_out),
        Litres(domain_out),
    )
    .map_err(SharedError::Records)?;
    if flow_work.exterior_litres
        != sea_out
            .checked_add(domain_out)
            .ok_or(SharedError::Limit("export sum"))?
    {
        return Err(SharedError::Invalid("fine export callback disagreement"));
    }
    solution.balance = balance;
    let metrics_work = flow_metrics::run(&mut routes, &mut flow, &lakes, limits.metrics)
        .map_err(SharedError::Metrics)?;
    flow.flush()?;
    routes.flush()?;
    Ok(SharedArtifacts {
        routing: routes,
        flow,
        hierarchy,
        registry,
        nodes,
        solution,
        lakes,
        balance,
        work: SharedWork {
            ocean: ocean_work,
            routing: routing_work,
            mst: mst_work,
            hierarchy: hierarchy_work,
            binding_reads,
            sources: [source_work, fine_source_work],
            flow: flow_work,
            metrics: metrics_work,
            bridge_bytes: bridge.bytes,
            bridge_operations: bridge.operations,
        },
    })
}
