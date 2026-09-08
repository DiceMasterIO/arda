//! Hand-calculated actual-grid integration through the bounded disk topology pipeline.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::hydrology::{
    annual::{self, AnnualInput, AnnualLimits},
    annual_aggregation::{AggregationLimits, Aggregator, AnnualCell, Terminal},
    annual_topology::{self, TopologyLimits},
    annual_transfers::{self, LeafFlow, TransferLimits},
    handoff::build_from_mst,
    hierarchy,
    hierarchy_disk::{DiskHierarchyStore, DiskLimits as HierarchyLimits},
    mst::{self, MstLimits, ReadLimits},
    routing::{self, CellIndex, CellRecord, Extent, OutletKind, Receiver, RoutingStore},
    witness_binding::BreakpointRegistry,
};
use crate::orchestrator::{
    child_links::{ChildLimits, DiskChildLinks, CHILD_FILE},
    routing_disk::{DiskLimits as RoutingLimits, DiskRoutingStore},
};
use arda_core::{
    hydrology::{BasinId, Litres, ReceivingAccount},
    GlobalCell, HeightMm, RainfallMm,
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "arda-annual-integration-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn mst_limits() -> MstLimits {
    MstLimits {
        ram_bytes: 1 << 24,
        scratch_bytes: 1 << 24,
        io_bytes: 1 << 30,
        io_operations: 1_000_000,
        cache_pages: 1,
        sort_buffer_records: 1,
        slot_reads: 1_000_000,
        slot_writes: 1_000_000,
        comparisons: 1_000_000,
        routing_reads: 1_000_000,
        scan_candidates: 1_000_000,
    }
}
#[test]
fn physical_disk_pipeline_produces_one_supported_nested_lake_and_exact_outward_flux() {
    // Real 7×5 terrain: three minima 1/2/3 mm, joins at 5/10 mm,
    // ten other interior cells at 11 mm and one actual rim outlet at 12 mm.
    let extent = Extent::new(7, 5).unwrap();
    let mut heights = [20; 35];
    for y in 1..4 {
        for x in 1..6 {
            heights[y * 7 + x] = 11;
        }
    }
    heights[15] = 1;
    heights[16] = 5;
    heights[17] = 2;
    heights[18] = 10;
    heights[19] = 3;
    heights[20] = 12;
    let routing_dir = Directory::new();
    let mst_dir = Directory::new();
    let hierarchy_dir = Directory::new();
    let mut routes = DiskRoutingStore::create(
        &routing_dir.0,
        extent,
        heights.iter().map(|&h| CellRecord::prepared(h, false)),
        RoutingLimits {
            cache_bytes: 16_384,
            scratch_bytes: DiskRoutingStore::scratch_required(extent),
            io_bytes: 1 << 30,
            io_operations: 1_000_000,
        },
    )
    .unwrap();
    let routing_work =
        routing::route_and_own(&mut routes, routing::Limits::for_extent(extent)).unwrap();
    routes.flush().unwrap();
    let product = mst::produce(&mut routes, &mst_dir.0, mst_limits()).unwrap();
    assert_eq!(product.work.leaves, 3);
    // Reopen the same checked accepted file before its original stream is consumed.
    // These are real physical witnesses, never reconstructed from expected topology.
    let accepted: Vec<_> = product
        .edges
        .replay(ReadLimits::required(product.edges.path(), 3).unwrap())
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(accepted.len(), 3);
    assert_eq!(
        accepted.iter().map(|e| e.sill_mm).collect::<Vec<_>>(),
        vec![5, 10, 12]
    );
    let mut child_limits = ChildLimits::required(&hierarchy_dir.0, 2, 20).unwrap();
    child_limits.io_bytes = 1 << 30;
    child_limits.io_operations = 1_000_000;
    child_limits.comparisons = 1_000_000;
    let children = DiskChildLinks::create(&hierarchy_dir.0, child_limits).unwrap();
    let mut hierarchy = DiskHierarchyStore::new(
        &hierarchy_dir.0,
        extent,
        HierarchyLimits {
            cache_bytes: DiskHierarchyStore::cache_required(&hierarchy_dir.0, 1).unwrap(),
            scratch_bytes: 1 << 24,
            io_bytes: 1 << 30,
            io_operations: 1_000_000,
        },
        children,
    )
    .unwrap();
    let mut registry =
        BreakpointRegistry::new::<<DiskRoutingStore as RoutingStore>::Error>(20, 1 << 16).unwrap();
    let (hierarchy_work, binding_reads) = build_from_mst(
        product,
        &mut routes,
        &mut hierarchy,
        &mut registry,
        hierarchy::Limits {
            leaves: 3,
            store_operations: 100_000,
        },
        1000,
    )
    .unwrap();
    hierarchy.finish().unwrap();
    assert_eq!(hierarchy.output_counts().unwrap(), (5, 3));
    // The concrete child writer emits four u64 IDs. This tiny test deliberately
    // reads that real file; production's separately admitted reader streams it.
    let raw_children = fs::read(hierarchy_dir.0.join(CHILD_FILE)).unwrap();
    assert_eq!(raw_children.len(), 32);
    let children: Vec<_> = raw_children
        .as_chunks::<8>()
        .0
        .iter()
        .map(|b| BasinId(u64::from_le_bytes(*b)))
        .collect();
    let rows: Vec<_> = (0..5).map(|i| hierarchy.output_node(i).unwrap()).collect();
    let topology = annual_topology::adapt(
        extent,
        rows.iter().cloned().map(Ok::<_, std::convert::Infallible>),
        &children,
        accepted
            .iter()
            .copied()
            .map(Ok::<_, std::convert::Infallible>),
        &registry,
        TopologyLimits {
            nodes: 5,
            leaves: 3,
            ram_bytes: 1 << 20,
            operations: 100_000,
        },
    )
    .unwrap();
    assert_eq!(topology.joins.len(), 2);
    assert_eq!(
        topology
            .nodes
            .iter()
            .filter(|n| !n.children.is_empty())
            .map(|n| n.birth.raw())
            .collect::<Vec<_>>(),
        vec![5, 10]
    );
    let mut aggregate = Aggregator::new(
        extent,
        topology.nodes,
        AggregationLimits {
            ram_bytes: 1 << 20,
            bands: 35,
            operations: 100_000,
        },
    )
    .unwrap();
    for raw in 0..extent.cells() {
        let at = CellIndex::new(raw, extent).unwrap();
        let record = routes.read(at).unwrap();
        let terminal = record.owner().unwrap();
        let terminal_record = routes.read(terminal).unwrap();
        let ownership = match terminal_record.receiver(extent, terminal).unwrap() {
            Receiver::Stop(OutletKind::ClosedDepression) => {
                Terminal::Basin(BasinId(extent.anchor_key(terminal)))
            }
            Receiver::Stop(OutletKind::MarineEntry) => Terminal::Sea,
            Receiver::Stop(OutletKind::DomainExport) => Terminal::DomainExport,
            Receiver::Cell(_) => panic!("published owner is not a terminal"),
        };
        let (x, y) = extent.coordinates(at);
        aggregate
            .push(&AnnualCell {
                at: GlobalCell { x, y },
                bed: HeightMm::new(record.height()),
                rain: RainfallMm::new(100),
                evaporation_um: [5000; 12],
                terminal: ownership,
            })
            .unwrap();
        assert_eq!(
            record.height(),
            heights[usize::try_from(raw).unwrap()],
            "routing preserves physical heights"
        );
    }
    let aggregate = aggregate.finish().unwrap();
    assert_eq!(aggregate.catchment_cells.iter().sum::<u32>(), 35);
    assert_eq!(
        (
            aggregate.marine_cells,
            aggregate.sea.cells,
            aggregate.domain.cells
        ),
        (0, 0, 0)
    );
    assert_eq!(aggregate.bands.iter().map(|b| b.cells).sum::<u32>(), 15);
    assert_eq!(
        aggregate
            .nodes
            .iter()
            .map(|n| n.local_precipitation.0)
            .sum::<u128>(),
        35_000_000
    );
    assert_eq!(
        aggregate
            .nodes
            .iter()
            .map(|n| n.local_runoff.0)
            .sum::<u128>(),
        17_500_000
    );
    let solution = annual::solve_annual(
        AnnualInput {
            nodes: &aggregate.nodes,
            children: &children,
            bands: &aggregate.bands,
            joins: &topology.joins,
        },
        AnnualLimits {
            ram_bytes: 1 << 20,
            nodes: 5,
            bands: 35,
            events: 1_000_000,
        },
    )
    .unwrap();
    assert_eq!(solution.lakes.len(), 1);
    let lake = &solution.lakes[0];
    assert_eq!(lake.surface, HeightMm::new(12));
    assert_eq!(lake.submerged_cells, 15);
    // 15×12 − (ten×11 + 1 + 5 + 2 + 10 + 3) = 49 cell-mm.
    assert_eq!(lake.geometric_volume, Litres(490_000));
    assert_eq!(solution.balance.land_precipitation, Litres(20_000_000));
    assert_eq!(solution.balance.land_loss, Litres(10_000_000));
    assert_eq!(solution.balance.lake_precipitation, Litres(15_000_000));
    assert_eq!(solution.balance.lake_evaporation, Litres(9_000_000));
    assert_eq!(solution.balance.marginal_evaporation, Litres(0));
    assert_eq!(solution.balance.sea_outflow, Litres(0));
    assert_eq!(solution.balance.domain_outflow, Litres(16_000_000));
    assert_eq!(
        solution.leaf_net.iter().map(|n| n.net_litres).sum::<i128>(),
        16_000_000
    );
    assert!(solution
        .leaf_net
        .iter()
        .all(|n| n.lake == Some(lake.basin) && n.surface == Some(lake.surface)));
    let leaves: Vec<_> = solution
        .leaf_net
        .iter()
        .map(|n| LeafFlow {
            leaf: n.leaf,
            net_litres: n.net_litres,
            lake: n.lake,
            surface: n.surface,
        })
        .collect();
    let flows = annual_transfers::normalize_transfers(
        extent,
        &leaves,
        &accepted,
        TransferLimits {
            leaves: 3,
            ram_bytes: 1 << 20,
        },
    )
    .unwrap();
    assert_eq!(flows.exterior_outflow, solution.balance.domain_outflow);
    assert_eq!(
        flows.transfers.len(),
        1,
        "submerged internal joins carry no duplicate exported flux"
    );
    let flow = &flows.transfers[0];
    assert_eq!(flow.from, GlobalCell { x: 6, y: 2 });
    assert_eq!(flow.to, None);
    assert_eq!(flow.to_leaf, None);
    assert_eq!(flow.sill, HeightMm::new(12));
    assert_eq!(flow.annual_volume, Litres(16_000_000));
    let outlet = lake.potential_spill.unwrap();
    assert_eq!(outlet.from, flow.from);
    assert_eq!(outlet.to, flow.to);
    assert_eq!(outlet.receiving, ReceivingAccount::DomainExport);
    let from = CellIndex::new(flow.from.y * 7 + flow.from.x, extent).unwrap();
    assert_eq!(
        flow.from_leaf,
        BasinId(extent.anchor_key(routes.read(from).unwrap().owner().unwrap()))
    );
    assert_eq!(
        solution.balance.land_precipitation.0 + solution.balance.lake_precipitation.0,
        solution.balance.land_loss.0
            + solution.balance.lake_evaporation.0
            + solution.balance.marginal_evaporation.0
            + flows.exterior_outflow.0
    );
    assert!(routing_work.cell_reads > 0 && hierarchy_work.nodes > 0 && binding_reads > 0);
    assert!(routes.work().bytes > 0 && hierarchy.work().bytes > 0);
}
