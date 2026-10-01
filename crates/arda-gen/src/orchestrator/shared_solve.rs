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
mod limits;
pub use limits::{bridge_ram, BridgeLimits, SharedLimits};

type RoutingError = routing_disk::DiskError;
type HierarchyError = hierarchy_disk::DiskError;

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
            for b in buffer[..n].as_chunks::<8>().0 {
                rows.push(BasinId(u64::from_le_bytes(*b)));
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
