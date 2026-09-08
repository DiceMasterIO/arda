//! Synthetic prepared-file fixture; register inside continent for private grid fields.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "../hydrology/area_output_tests.rs"]
mod area_output_tests;
#[path = "global_output_tests.rs"]
mod global_output_tests;
use super::{climate::ContinentClimate, ContinentGrid};
use crate::hydrology::{
    annual, annual_aggregation, annual_records, annual_topology, annual_transfers,
    fine_flow::{self, FlowStore},
    flow_metrics, hierarchy,
    hierarchy_disk::DiskHierarchyStore,
    mst, ocean,
    prepared_domain::PreparedDomain,
    routing::{self, CellIndex, Extent, RoutingStore},
    types::PreparedTerrain,
};
use crate::orchestrator::{
    annual_source,
    child_links::ChildLimits,
    flow_disk::{self},
    prepared_files::{self, PreparedWriter},
    routing_disk::{self, DiskRoutingStore},
    shared_solve::*,
};
use arda_core::{
    hydrology::Litres, ClimateRegime, GenerateConfig, HeightMm, LatitudeBand, RainfallMm, SizeKm,
};
use std::path::{Path, PathBuf};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!(
            "arda-shared-solve-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn config() -> GenerateConfig {
    GenerateConfig::new(SizeKm::new(64, 64), LatitudeBand::new(35, 55), 15).unwrap()
}
fn height(x: u32, y: u32) -> i32 {
    if !(10..=14).contains(&x) || !(10..=14).contains(&y) {
        return -1000;
    }
    if (x, y) == (12, 12) {
        1
    } else if (x, y) == (13, 12) {
        5
    } else if (x, y) == (14, 12) {
        10
    } else {
        20
    }
}
fn limits(path: &Path, extent: Extent) -> SharedLimits {
    let h = path.join("hierarchy");
    let n = u64::from(extent.cells());
    let mut child = ChildLimits::required(&h, 2, 63).unwrap();
    child.io_bytes = 1 << 30;
    child.io_operations = 1_000_000;
    child.comparisons = 1_000_000;
    let domain = PreparedDomain::for_config(config()).unwrap();
    SharedLimits {
        routing_storage: routing_disk::DiskLimits {
            cache_bytes: 8192 * 128,
            scratch_bytes: DiskRoutingStore::scratch_required(extent),
            io_bytes: 1 << 38,
            io_operations: 100_000_000,
        },
        ocean: ocean::Limits::for_extent(extent),
        routing: routing::Limits::for_extent(extent),
        mst: mst::MstLimits {
            ram_bytes: 1 << 24,
            scratch_bytes: 1 << 24,
            io_bytes: 1 << 30,
            io_operations: 1_000_000,
            cache_pages: 2,
            sort_buffer_records: 2,
            slot_reads: 1_000_000,
            slot_writes: 1_000_000,
            comparisons: 1_000_000,
            routing_reads: 30 * n,
            scan_candidates: 10 * n,
        },
        replay: mst::ReadLimits {
            ram_bytes: 1 << 16,
            io_bytes: 1 << 20,
            io_operations: 10000,
            comparisons: 1000,
        },
        child,
        hierarchy_storage: crate::hydrology::hierarchy_disk::DiskLimits {
            cache_bytes: DiskHierarchyStore::cache_required(&h, 2).unwrap(),
            scratch_bytes: 1 << 24,
            io_bytes: 1 << 30,
            io_operations: 1_000_000,
        },
        hierarchy: hierarchy::Limits {
            leaves: 32,
            store_operations: 1_000_000,
        },
        binding_records: 32,
        binding_ram: 1 << 20,
        binding_reads: 10000,
        topology: annual_topology::TopologyLimits {
            nodes: 63,
            leaves: 32,
            ram_bytes: 1 << 20,
            operations: 100000,
        },
        aggregation: annual_aggregation::AggregationLimits {
            ram_bytes: 1 << 20,
            bands: 128,
            operations: 20 * n,
        },
        annual: annual::AnnualLimits {
            ram_bytes: 1 << 20,
            nodes: 63,
            bands: 128,
            events: 1_000_000,
        },
        transfers: annual_transfers::TransferLimits {
            ram_bytes: 1 << 20,
            leaves: 32,
        },
        records: annual_records::RecordLimits {
            ram_bytes: 1 << 20,
            source_reads: 10000,
        },
        source: annual_source::SourceLimits {
            ram_bytes: annual_source::required_ram(domain).unwrap(),
            operations: annual_source::required_operations(domain).unwrap(),
        },
        flow_storage: flow_disk::FlowLimits {
            cache_pages: 128,
            ram_bytes: flow_disk::ram_required(&path.join("flow.bin"), 128).unwrap(),
            scratch_bytes: flow_disk::scratch_required(extent),
            io_bytes: 1 << 38,
            io_operations: 100_000_000,
        },
        flow: fine_flow::FlowLimits {
            operations: 100 * n,
            absolute_litres: 1 << 62,
            accepted_edges: 32,
        },
        metrics: flow_metrics::MetricsLimits {
            ram_bytes: 1 << 20,
            lakes: 32,
            lake_edges: 256,
            operations: 200 * n,
        },
        bridge: BridgeLimits {
            ram_bytes: bridge_ram(path, 32, 63).unwrap(),
            io_bytes: 1 << 20,
            io_operations: 10000,
        },
    }
}
#[test]
fn real_prepared_private_stages_produce_supported_water_metrics_and_closed_world_ledger() {
    let temp = Temp::new();
    let output = crate::orchestrator::publication::WorldOutput::begin(&temp.0).unwrap();
    let prepared_dir = output.scratch().join("prepared");
    let solve_dir = output.scratch().join("solve");
    std::fs::create_dir(&prepared_dir).unwrap();
    std::fs::create_dir(&solve_dir).unwrap();
    let config = config();
    let domain = PreparedDomain::for_config(config).unwrap();
    let extent = Extent::new(domain.width(), domain.height()).unwrap();
    let prepared_limits = prepared_files::Limits {
        ram_bytes: prepared_files::ram_required(&prepared_dir, domain, 2).unwrap(),
        scratch_bytes: prepared_files::scratch_required(domain),
        cache_tiles: 2,
        io_bytes: 1 << 32,
        io_operations: 1_000_000,
        tile_queries: 4 * u64::from(extent.cells()),
    };
    let mut writer = PreparedWriter::new(&prepared_dir, domain, prepared_limits).unwrap();
    for i in [2, 3, 1, 0] {
        let e = domain.entry(i).unwrap();
        let heights = (0..512u32)
            .flat_map(|y| {
                (0..512u32).map(move |x| {
                    HeightMm::new(height(
                        u32::try_from(e.area.x).unwrap() * 512 + x,
                        u32::try_from(e.area.y).unwrap() * 512 + y,
                    ))
                })
            })
            .collect();
        writer
            .write(&PreparedTerrain {
                area: e.area,
                valid: e.valid,
                heights,
                annual_rain: vec![RainfallMm::new(60000); 262144],
                temperature_base_centi: vec![1000; 262144],
            })
            .unwrap();
    }
    let mut prepared = writer.finish().unwrap();
    let grid = ContinentGrid {
        width: 64,
        height: 64,
        height_mm: vec![0; 4096],
    };
    let climate = ContinentClimate {
        temperature: vec![1000; 4096],
        rainfall: vec![100; 4096],
        regime: vec![ClimateRegime::Temperate; 4096],
        moisture: vec![0; 4096],
        ocean: vec![true; 4096],
        ocean_distance_km: vec![0; 4096],
    };
    let l = limits(&solve_dir, extent);
    let reservation = l.reservations().unwrap();
    assert!(
        reservation.ram_bytes >= l.bridge.ram_bytes + l.source.ram_bytes + l.flow_storage.ram_bytes
    );
    let mut refused = l;
    refused.bridge.ram_bytes = 0;
    assert!(matches!(
        solve(&mut prepared, &grid, &climate, config, &solve_dir, refused),
        Err(SharedError::Limit("bridge/source preflight"))
    ));
    assert_eq!(std::fs::read_dir(&solve_dir).unwrap().count(), 0);
    let mut artifacts = solve(&mut prepared, &grid, &climate, config, &solve_dir, l).unwrap();
    assert_eq!(artifacts.work.mst.leaves, 1);
    assert_eq!(artifacts.work.flow.cells, 25);
    assert_eq!(artifacts.lakes.len(), 1);
    let lake = &artifacts.lakes[0];
    assert_eq!(lake.surface, HeightMm::new(10));
    assert_eq!(lake.submerged_cells, 2);
    assert!(lake.annual_outflow.0 > 0);
    assert_eq!(
        artifacts.balance.land_precipitation.0 + artifacts.balance.lake_precipitation.0,
        15_000_000_000
    );
    assert_eq!(artifacts.balance.domain_outflow, Litres(0));
    assert_eq!(
        artifacts.balance.sea_outflow.0,
        artifacts.work.flow.exterior_litres
    );
    let b = artifacts.balance;
    assert_eq!(
        b.land_precipitation.0 + b.lake_precipitation.0,
        b.land_loss.0 + b.lake_evaporation.0 + b.marginal_evaporation.0 + b.sea_outflow.0
    );
    assert_eq!(artifacts.work.sources[0].samples, 25);
    assert_eq!(artifacts.work.sources[1].samples, 25);
    assert_eq!(artifacts.work.sources[0].windows, 1);
    let center = CellIndex::new(12 * 640 + 12, extent).unwrap();
    let wet = artifacts.flow.read(center).unwrap();
    assert_eq!(wet.lake, Some(lake.basin));
    assert_eq!(wet.surface_mm, 10);
    assert!(wet.metrics.drainage_cells >= 2);
    assert_eq!(artifacts.routing.read(center).unwrap().height(), 1);
    assert!(artifacts
        .routing
        .read(CellIndex::new(0, extent).unwrap())
        .unwrap()
        .is_marine());
    assert_eq!(artifacts.hierarchy.output_counts().unwrap(), (1, 1));
    assert_eq!(artifacts.nodes.len(), 1);
    assert!(solve_dir
        .join("hierarchy")
        .join(crate::orchestrator::child_links::CHILD_FILE)
        .is_file());
    let mut owner_counts = std::collections::BTreeMap::new();
    for y in 10..=14 {
        for x in 10..=14 {
            let at = CellIndex::new(y * 640 + x, extent).unwrap();
            let owner = artifacts.routing.read(at).unwrap().owner().unwrap();
            let (x, y) = extent.coordinates(owner);
            *owner_counts
                .entry(arda_core::GlobalCell { x, y })
                .or_insert(0u32) += 1;
        }
    }
    let index = crate::hydrology::final_index::build(
        &mut artifacts.routing,
        &mut artifacts.flow,
        arda_core::hydrology::HydrologyDomain {
            width_cells: 640,
            height_cells: 640,
            exported_areas_wide: 1,
            exported_areas_high: 1,
        },
        &artifacts.nodes,
        &artifacts.solution.leaf_net,
        crate::hydrology::final_index::IndexLimits {
            ram_bytes: 1 << 28,
            records: 100_000,
            area_references: 1_000_000,
            operations: 1_000_000_000,
        },
    )
    .unwrap();
    assert_eq!(
        index
            .catchments
            .iter()
            .map(|c| c.contributing_cells)
            .sum::<u32>(),
        25
    );
    let actual: std::collections::BTreeMap<_, _> = index
        .catchments
        .iter()
        .map(|c| (c.terminal, c.contributing_cells))
        .collect();
    assert_eq!(actual, owner_counts);
    let closed = index
        .catchments
        .iter()
        .find(|c| c.basin == Some(lake.basin))
        .unwrap();
    assert_eq!(closed.representative_lake, Some(lake.basin));
    assert_eq!(closed.terminal, arda_core::GlobalCell { x: 12, y: 12 });
    assert_eq!(index.area_reaches.len(), 1);
    assert!(!index.reaches.is_empty());
    area_output_tests::check(&mut prepared, &mut artifacts, &index);
    // A second attempted private solve must refuse existing paths, preserving success.
    assert!(matches!(
        solve(&mut prepared, &grid, &climate, config, &solve_dir, l),
        Err(SharedError::Io(_))
    ));
    global_output_tests::check(output, &temp.0, artifacts, prepared, index, config);
}
#[test]
fn bridge_and_reservation_arithmetic_refuse_before_any_stage_file_creation() {
    let t = Temp::new();
    let e = Extent::new(640, 640).unwrap();
    let mut l = limits(&t.0, e);
    l.bridge.ram_bytes = u64::MAX;
    assert!(l.reservations().is_err());
    assert!(bridge_ram(&t.0, u64::MAX, u64::MAX).is_none());
    assert_eq!(std::fs::read_dir(&t.0).unwrap().count(), 0);
}
