//! Pure whole-generation admission before any private or published output is created.
#![deny(missing_docs)]
use super::{
    annual_source,
    child_links::ChildLimits,
    flow_disk, prepared_files, routing_disk,
    shared_solve::{self, BridgeLimits, SharedLimits},
};
use crate::hydrology::{
    annual, annual_aggregation, annual_records, annual_topology, annual_transfers,
    area_output::{self, AreaLimits},
    extraction,
    final_index::{self, IndexLimits},
    fine_flow, flow_metrics, hierarchy,
    hierarchy_disk::DiskHierarchyStore,
    mst::{self, slots::SlotStore},
    ocean,
    prepared_domain::{DomainError, PreparedDomain},
    routing::{self, Extent},
    types::HydrologyLimits,
};
use arda_core::GenerateConfig;
use std::path::Path;
const MIB: u64 = 1024 * 1024;
const TILE: u64 = 512 * 512;
/// Complete concrete stage reservations, derived without touching the filesystem.
#[derive(Debug, Clone, Copy)]
pub struct GenerationAdmission {
    /// Request-derived modeled and exported rectangles.
    pub domain: PreparedDomain,
    /// Fine spatial ordinal domain.
    pub extent: Extent,
    /// One shared prepared writer/reader lifetime, including later area reads.
    pub prepared: prepared_files::Limits,
    /// All private shared solver stages and retained scratch handles.
    pub shared: SharedLimits,
    /// Completed feature and area-reference index.
    pub index: IndexLimits,
    /// Sequential composition of one area at a time.
    pub area: AreaLimits,
    /// Conservative simultaneous payload and complete declared work/I/O totals.
    pub reservations: GenerationReservations,
}
/// Explicit sums; no temporal-reuse credit is used for stage-owned reservations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationReservations {
    /// Sum of stage reservations plus continent/preparation and final encoding buffers.
    pub ram_bytes: u64,
    /// Prepared files and all simultaneously retained solve scratch.
    pub scratch_bytes: u64,
    /// Routing/MST/hierarchy/flow scratch, excluding separately staged prepared files.
    pub spatial_scratch_bytes: u64,
    /// Complete declared prepared, solve and final-file payload I/O ceilings.
    pub io_bytes: u128,
    /// Complete declared filesystem-operation ceilings.
    pub io_operations: u64,
    /// Sum of declared stage logical-work counters, including extraction/composition.
    pub logical_operations: u64,
    /// Shared initial terrain sampling and all bounded physical evolution steps.
    pub terrain_operations: u64,
    /// Continent, complete shared terrain evolution and one prepared-area payload.
    pub continent_preparation_bytes: u64,
    /// One128MiB objects encoding plus cells/compression and fixed writer buffers.
    pub encoding_bytes: u64,
    /// Final file payload allowance, checked by bounded output encoders/callers.
    pub final_io_bytes: u128,
    /// Final file operation allowance, including all configured areas and fixed global tables.
    pub final_io_operations: u64,
}
/// Invalid request, arithmetic/helper failure or explicit whole-generation resource refusal.
#[derive(Debug, thiserror::Error)]
pub enum AdmissionError {
    /// The requested modeled domain is unsupported.
    #[error(transparent)]
    Domain(#[from] DomainError),
    /// Physical evolution rejected its rectangle or allocation arithmetic.
    #[error(transparent)]
    Terrain(#[from] crate::hydrology::HydrologyError),
    /// A known count or reservation cannot be represented.
    #[error("generation admission overflow: {0}")]
    Overflow(&'static str),
    /// A complete reservation exceeds the user's explicit capacity.
    #[error("generation {resource} requires {required}, limit {limit}")]
    Limit {
        /// Resource being admitted.
        resource: &'static str,
        /// Complete required reservation.
        required: u128,
        /// Public caller allowance.
        limit: u128,
    },
    /// Prepared helper rejected its cache/path/count.
    #[error(transparent)]
    Prepared(#[from] prepared_files::PreparedError),
    /// Exact MST private storage helper failed.
    #[error(transparent)]
    Mst(#[from] mst::StageError),
    /// Exact accepted sorting/replay helper failed.
    #[error(transparent)]
    Sort(#[from] mst::SortError),
    /// Exact child-link sorting/index helper failed.
    #[error(transparent)]
    Child(#[from] super::child_links::ChildError),
    /// Exact hierarchy cache/row helper failed.
    #[error(transparent)]
    Hierarchy(#[from] crate::hydrology::hierarchy_disk::DiskError),
    /// Exact fine-flow cache/row helper failed.
    #[error(transparent)]
    Flow(#[from] flow_disk::FlowDiskError),
    /// Annual model payload arithmetic failed.
    #[error(transparent)]
    Annual(#[from] annual::AnnualError),
    /// Summed concrete solver reservations failed.
    #[error(transparent)]
    Shared(#[from] shared_solve::SharedError),
}
type Result<T> = std::result::Result<T, AdmissionError>;
fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or(AdmissionError::Overflow("sum"))
}
fn mul(a: u64, b: u64) -> Result<u64> {
    a.checked_mul(b).ok_or(AdmissionError::Overflow("product"))
}
fn need(value: Option<u64>) -> Result<u64> {
    value.ok_or(AdmissionError::Overflow("payload helper"))
}
fn sum(values: &[u64]) -> Result<u64> {
    values.iter().try_fold(0, |a, &b| add(a, b))
}
fn cap(resource: &'static str, required: u128, limit: u128) -> Result<()> {
    if required > limit {
        return Err(AdmissionError::Limit {
            resource,
            required,
            limit,
        });
    }
    Ok(())
}
fn cold(operations: u64, initial: u64) -> Result<(u128, u64)> {
    // One logical record access may evict a dirty page and load another; each
    // payload access seeks. Fixed creation/flush calls and initial bytes are separate.
    Ok((
        u128::from(operations) * 8192 + u128::from(initial),
        add(
            mul(operations, 4)?,
            add(mul(initial.div_ceil(4096), 2)?, 64)?,
        )?,
    ))
}
/// Derive concrete limits without filesystem reads/writes or physical-model changes.
///
/// Count ceilings are capacities, not promises that arbitrary terrain completes.
/// An observed excess later returns the owning stage's typed resource error; no
/// capacity is silently lowered, spilled into unreserved RAM or used to change water.
/// Preparation and composition are sequential; introducing workers requires new admission.
/// # Errors
/// Rejects invalid requests, unrepresentable counts/paths or any complete resource sum
/// beyond the supplied public limits before the caller creates its output transaction.
#[allow(clippy::too_many_lines)]
pub fn admit(
    config: GenerateConfig,
    scratch: &Path,
    limits: HydrologyLimits,
) -> Result<GenerationAdmission> {
    let path_bytes = u64::try_from(scratch.as_os_str().as_encoded_bytes().len())
        .map_err(|_| AdmissionError::Overflow("path bytes"))?;
    let path_payload = mul(add(path_bytes, 128)?, 16)?;
    cap(
        "RAM bytes",
        u128::from(path_payload),
        u128::from(limits.ram_bytes),
    )?;
    let domain = PreparedDomain::for_config(config)?;
    let extent =
        Extent::new(domain.width(), domain.height()).ok_or(AdmissionError::Overflow("extent"))?;
    let n = u64::from(extent.cells());
    let areas = mul(
        u64::try_from(config.areas_wide()).map_err(|_| AdmissionError::Overflow("areas"))?,
        u64::try_from(config.areas_high()).map_err(|_| AdmissionError::Overflow("areas"))?,
    )?;
    let leaves = limits.max_closed_leaves.min(n);
    let nodes = mul(leaves, 2)?.saturating_sub(1);
    let bands = limits.max_bands.min(n);
    let records = limits.max_feature_records.min(mul(n, 4)?);
    let references = limits.max_area_references.min(mul(records, areas)?);
    let lake_edges = limits.max_lake_boundary_edges.min(n);
    let prepared_dir = scratch.join("prepared");
    let solve_dir = scratch.join("solve");
    let mst_dir = solve_dir.join("mst");
    let hierarchy_dir = solve_dir.join("hierarchy");
    let flow_path = solve_dir.join("flow.bin");
    let prepared_scratch = prepared_files::scratch_required(domain);
    let queries = add(mul(n, 3)?, add(areas, u64::from(domain.count()))?)?;
    let prepared = prepared_files::Limits {
        ram_bytes: prepared_files::ram_required(&prepared_dir, domain, domain.columns())?,
        scratch_bytes: prepared_scratch,
        cache_tiles: domain.columns(),
        // Valid even if every tile query misses; row caching only reduces actual I/O.
        io_bytes: u128::from(queries) * u128::from(32 + 10 * TILE)
            + 2 * u128::from(prepared_scratch),
        io_operations: add(
            mul(queries, 3)?,
            add(mul(u64::from(domain.count()), 8)?, 32)?,
        )?,
        tile_queries: queries,
    };
    let depth = u64::from(64 - (leaves + 1).leading_zeros());
    let rounds = if leaves == 0 {
        0
    } else {
        u64::from(64 - leaves.leading_zeros())
    };
    let sort = mst::SortLimits::required(&mst_dir, leaves, 4096)?;
    let slot_pages = (leaves + 1).div_ceil(64).min(1024);
    let slot_reads = add(mul(mul(rounds, 256)?, n)?, mul(leaves, 4096)?)?;
    let slot_writes = add(mul(mul(rounds, 64)?, n)?, mul(leaves, 2048)?)?;
    let mst_initial = add(
        add(256, mul(leaves, 48)?)?,
        mul((leaves + 1).div_ceil(64), 4096)?,
    )?;
    let (mst_io, mst_calls) = cold(add(slot_reads, slot_writes)?, mst_initial)?;
    let mst = mst::MstLimits {
        max_leaves: leaves,
        ram_bytes: sum(&[
            SlotStore::required_ram(&mst_dir, slot_pages)?,
            8192,
            sort.ram_bytes,
        ])?,
        scratch_bytes: sum(&[
            SlotStore::required_bytes(
                u32::try_from(leaves + 1).map_err(|_| AdmissionError::Overflow("slot count"))?,
            ),
            64 + 16 * leaves,
            sort.scratch_bytes,
        ])?,
        io_bytes: mst_io
            .checked_add(sort.io_bytes)
            .ok_or(AdmissionError::Overflow("MST I/O"))?,
        io_operations: sum(&[
            mst_calls,
            sort.io_operations,
            12 + 2 * (2 * leaves.div_ceil(256) + leaves.div_ceil(64) + (leaves + 1).div_ceil(64)),
        ])?,
        cache_pages: slot_pages,
        sort_buffer_records: 4096,
        slot_reads,
        slot_writes,
        comparisons: add(mul(slot_reads, 2)?, sort.comparisons)?,
        routing_reads: add(mul(n, 2)?, mul(mul(rounds, 12)?, n)?)?,
        scan_candidates: mul(mul(rounds, 5)?, n)?,
    };
    // All possible actual final names have shorter decimal components than u64::MAX.
    let replay = mst::ReadLimits::required(
        &mst_dir.join("accepted-run-18446744073709551615-18446744073709551615.bin"),
        leaves,
    )?;
    let child = ChildLimits::required(&hierarchy_dir, 4096, nodes)?.with_lookups(mul(nodes, 2)?)?;
    let hierarchy_ops = add(mul(mul(add(depth, 1)?, 1024)?, add(leaves, 1)?)?, 4096)?;
    let hierarchy_bytes = DiskHierarchyStore::required_bytes(leaves + 1, nodes)?;
    let (hierarchy_io, hierarchy_calls) =
        cold(add(hierarchy_ops, mul(nodes, 2)?)?, hierarchy_bytes)?;
    let topology_ops = add(mul(nodes, 4096)?, 4096)?;
    let aggregation_ops = sum(&[mul(n, 128)?, mul(nodes, 1024)?, mul(bands, 8)?, 4096])?;
    let annual_ops = sum(&[mul(n, 64)?, mul(bands, 512)?, mul(nodes, 4096)?, 4096])?;
    let flow_ops = add(mul(n, 110)?, 3)?;
    let metric_ops = sum(&[
        mul(n, 2048)?,
        mul(lake_edges, 64)?,
        mul(leaves, 1024)?,
        4096,
    ])?;
    let index_ops = sum(&[
        mul(n, extraction::MAX_CELL_WORK + 32)?,
        mul(records, 64)?,
        mul(references, 64)?,
        mul(areas, 4096)?,
        nodes,
        leaves,
        4096,
    ])?;
    let context = add(records, leaves)?.min(1_048_576);
    let channels = records.min(262_144);
    let area_ops = sum(&[
        mul(TILE, extraction::MAX_CELL_WORK + 64)?,
        mul(context, 64)?,
        mul(channels, 64)?,
        4096,
    ])?;
    let index = IndexLimits {
        ram_bytes: need(final_index::required_ram(records, references, areas))?,
        records,
        area_references: references,
        operations: index_ops,
    };
    let area = AreaLimits {
        ram_bytes: need(area_output::required_ram(TILE, context, channels))?,
        operations: area_ops,
        context_records: u32::try_from(context).map_err(|_| AdmissionError::Overflow("context"))?,
        channels: u32::try_from(channels).map_err(|_| AdmissionError::Overflow("channels"))?,
    };
    let ocean = ocean::Limits::for_extent(extent);
    let routing = routing::Limits::for_extent(extent);
    let source = annual_source::SourceLimits {
        ram_bytes: need(annual_source::required_ram(domain))?,
        operations: need(annual_source::required_operations(domain))?,
    };
    let final_queries = add(index_ops, mul(area_ops, areas)?)?;
    let binding_reads = mul(add(nodes, leaves)?, 4)?;
    let routing_access = sum(&[
        ocean.reads,
        mul(ocean.marks, 2)?,
        ocean.tape_operations,
        routing.cell_reads,
        routing.cell_writes,
        routing.tape_reads,
        routing.tape_writes,
        mst.routing_reads,
        binding_reads,
        mul(n, 4)?,
        flow_ops,
        metric_ops,
        final_queries,
        mul(leaves, 4)?,
    ])?;
    let flow_access = sum(&[flow_ops, metric_ops, final_queries])?;
    let routing_bytes = routing_disk::DiskRoutingStore::scratch_required(extent);
    let (routing_io, routing_calls) = cold(routing_access, routing_bytes)?;
    let flow_bytes = flow_disk::scratch_required(extent);
    let (flow_io, flow_calls) = cold(flow_access, flow_bytes)?;
    let flow_pages = u32::try_from(n.div_ceil(51).min(4096))
        .map_err(|_| AdmissionError::Overflow("flow cache"))?;
    let shared = SharedLimits {
        routing_storage: routing_disk::DiskLimits {
            cache_bytes: mul(add(n.div_ceil(256).min(4096), 1)?, 8192)?,
            scratch_bytes: routing_bytes,
            io_bytes: routing_io,
            io_operations: routing_calls,
        },
        ocean,
        routing,
        mst,
        replay,
        child,
        hierarchy_storage: crate::hydrology::hierarchy_disk::DiskLimits {
            cache_bytes: DiskHierarchyStore::cache_required(&hierarchy_dir, 1024)?,
            scratch_bytes: hierarchy_bytes,
            io_bytes: hierarchy_io,
            io_operations: hierarchy_calls,
        },
        hierarchy: hierarchy::Limits {
            leaves,
            store_operations: hierarchy_ops,
        },
        binding_records: add(nodes, leaves)?,
        binding_ram: add(mul(add(nodes, leaves)?, 256)?, 4096)?,
        binding_reads,
        topology: annual_topology::TopologyLimits {
            nodes,
            leaves,
            ram_bytes: need(annual_topology::required_ram(nodes))?,
            operations: topology_ops,
        },
        aggregation: annual_aggregation::AggregationLimits {
            ram_bytes: need(annual_aggregation::required_ram(nodes, bands))?,
            bands,
            operations: aggregation_ops,
        },
        annual: annual::AnnualLimits {
            ram_bytes: annual::required_ram(nodes, bands)?,
            nodes,
            bands,
            events: annual_ops,
        },
        transfers: annual_transfers::TransferLimits {
            ram_bytes: need(annual_transfers::required_ram(leaves))?,
            leaves,
        },
        records: annual_records::RecordLimits {
            ram_bytes: add(mul(leaves, 256)?, 4096)?,
            source_reads: mul(leaves, 4)?,
        },
        source,
        flow_storage: flow_disk::FlowLimits {
            cache_pages: flow_pages,
            ram_bytes: flow_disk::ram_required(&flow_path, flow_pages)?,
            scratch_bytes: flow_bytes,
            io_bytes: flow_io,
            io_operations: flow_calls,
        },
        flow: fine_flow::FlowLimits {
            operations: flow_ops,
            absolute_litres: 1 << 80,
            accepted_edges: leaves,
        },
        metrics: flow_metrics::MetricsLimits {
            ram_bytes: need(flow_metrics::required_ram(leaves, lake_edges))?,
            lakes: leaves,
            lake_edges,
            operations: metric_ops,
        },
        bridge: BridgeLimits {
            ram_bytes: need(shared_solve::bridge_ram(&solve_dir, leaves, nodes))?,
            io_bytes: u128::from(mul(nodes, 8)?),
            io_operations: add(nodes.div_ceil(512), 16)?,
        },
    };
    let stage = shared.reservations()?;
    // Existing coarse arrays, heaps/queues/river extraction and retained climate/hydrology
    // fit the conservative256-byte/site payload envelope. Logic/02 shared terrain
    // correction additionally owns two fine i32 inputs plus its admitted scratch.
    // The fixed preparation allowance includes one sequential bundle, including
    // its two outside strips and four corners (4160 added owned bytes on 64-bit).
    let size = config.size_km();
    let coarse = mul(u64::from(size.width), u64::from(size.height))?;
    let evolution_scratch = u64::try_from(crate::area::evolution::scratch_bytes(
        domain.width() as usize,
        domain.height() as usize,
    )?)
    .map_err(|_| AdmissionError::Overflow("terrain scratch"))?;
    let continent_preparation_bytes =
        sum(&[mul(coarse, 256)?, 128 * MIB, mul(n, 8)?, evolution_scratch])?;
    let terrain_operations =
        crate::area::evolution::work_operations(domain.width() as usize, domain.height() as usize)?;
    let encoding_bytes = add(128 * MIB, add(mul(TILE, 2 * 40)?, 3 * MIB)?)?;
    let final_io_bytes = u128::from(areas) * u128::from(encoding_bytes)
        + u128::from(records) * 256
        + u128::from(nodes) * 256
        + u128::from(coarse) * 256
        + MIB as u128;
    // Fixed-record writers issue one payload call per row; the repeated child
    // read and lake/child/node writes are covered independently from the bridge.
    let final_io_operations = sum(&[
        // cells, objects, blocks and water forms: four calls each per file.
        mul(areas, 16)?,
        records,
        mul(nodes, 4)?,
        mul(leaves, 2)?,
        256,
    ])?;
    let logical_operations = sum(&[
        terrain_operations,
        prepared.tile_queries,
        ocean.reads,
        mul(ocean.marks, 2)?,
        ocean.tape_operations,
        routing.cell_reads,
        routing.cell_writes,
        routing.tape_reads,
        routing.tape_writes,
        mst.slot_reads,
        mst.slot_writes,
        mst.comparisons,
        mst.routing_reads,
        mst.scan_candidates,
        replay.comparisons,
        child.comparisons,
        hierarchy_ops,
        binding_reads,
        topology_ops,
        aggregation_ops,
        annual_ops,
        mul(source.operations, 2)?,
        flow_ops,
        metric_ops,
        index_ops,
        mul(area_ops, areas)?,
    ])?;
    let reservations = GenerationReservations {
        ram_bytes: sum(&[
            prepared.ram_bytes,
            stage.ram_bytes,
            index.ram_bytes,
            area.ram_bytes,
            continent_preparation_bytes,
            encoding_bytes,
            path_payload,
        ])?,
        scratch_bytes: add(prepared.scratch_bytes, stage.scratch_bytes)?,
        spatial_scratch_bytes: stage.scratch_bytes,
        io_bytes: prepared
            .io_bytes
            .checked_add(stage.io_bytes)
            .and_then(|v| v.checked_add(final_io_bytes))
            .ok_or(AdmissionError::Overflow("I/O sum"))?,
        io_operations: sum(&[
            prepared.io_operations,
            stage.io_operations,
            final_io_operations,
        ])?,
        logical_operations,
        terrain_operations,
        continent_preparation_bytes,
        encoding_bytes,
        final_io_bytes,
        final_io_operations,
    };
    cap(
        "RAM bytes",
        u128::from(reservations.ram_bytes),
        u128::from(limits.ram_bytes),
    )?;
    cap(
        "spatial scratch bytes",
        u128::from(reservations.spatial_scratch_bytes),
        u128::from(limits.spatial_scratch_bytes),
    )?;
    cap(
        "combined scratch bytes",
        u128::from(reservations.scratch_bytes),
        u128::from(limits.combined_scratch_bytes),
    )?;
    cap(
        "logical operations",
        u128::from(reservations.logical_operations),
        u128::from(limits.global_event_operations),
    )?;
    cap("I/O bytes", reservations.io_bytes, limits.global_io_bytes)?;
    cap(
        "I/O operations",
        u128::from(reservations.io_operations),
        u128::from(limits.global_io_operations),
    )?;
    Ok(GenerationAdmission {
        domain,
        extent,
        prepared,
        shared,
        index,
        area,
        reservations,
    })
}
