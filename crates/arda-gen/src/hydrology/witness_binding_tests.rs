//! One focused actual-grid hierarchy binding integration control.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::hydrology::{
    handoff::build_from_mst,
    hierarchy::{self, Minimum, Witness},
    hierarchy_disk::{DiskError, DiskHierarchyStore, DiskLimits},
    mst::{self, MstLimits},
    routing::{self, CellIndex, Extent, MemoryPages, RoutingStore},
    witness_binding::{BindingError, BreakpointRegistry, Destination, WitnessResolver},
};
use crate::orchestrator::child_links::{ChildLimits, DiskChildLinks};
use arda_core::{
    hydrology::{BasinId, ReceivingAccount},
    GlobalCell, HeightMm,
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
            "arda-witness-{}-{}",
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
fn terrain(tied: bool) -> (Extent, MemoryPages) {
    terrain_with_floors(tied, [1, 2, 3])
}
fn terrain_with_floors(tied: bool, floors: [i32; 3]) -> (Extent, MemoryPages) {
    let e = Extent::new(7, 5).unwrap();
    let mut h = vec![20; 35];
    for y in 1..4 {
        for x in 1..6 {
            h[y * 7 + x] = 11;
        }
    }
    h[15] = floors[0];
    h[17] = floors[1];
    h[19] = floors[2];
    h[16] = 5;
    h[18] = if tied { 5 } else { 10 };
    h[20] = 12;
    let mut s = MemoryPages::new(e, &h, &[false; 35], u64::MAX).unwrap();
    routing::route_and_own(&mut s, routing::Limits::for_extent(e)).unwrap();
    (e, s)
}
#[test]
fn tied_elder_death_binds_actual_source_component_before_the_sill() {
    // Minimized default42 topology: A joins B, then AB joins older C at the
    // same sill. A is the dying elder, but the second physical witness starts
    // in B. Before that sill, A and B are distinct source components.
    let (e, mut routing) = terrain_with_floors(true, [2, 3, 1]);
    let mst_dir = Directory::new();
    let hierarchy_dir = Directory::new();
    let product = mst::produce(&mut routing, &mst_dir.0, cap()).unwrap();
    assert_eq!(product.work.leaves, 3);
    let mut store = disk(&hierarchy_dir, e);
    let mut registry = BreakpointRegistry::new::<routing::MemoryError>(20, 1 << 16).unwrap();
    let (work, _) = build_from_mst(
        product,
        &mut routing,
        &mut store,
        &mut registry,
        hierarchy::Limits {
            leaves: 3,
            store_operations: 100_000,
        },
        1000,
    )
    .unwrap();
    store.finish().unwrap();
    assert_eq!(work.contracted, 1);
    assert_eq!(store.output_counts().unwrap(), (4, 3));
    let elder = store.output_elder(0).unwrap();
    assert_eq!(elder.leaf, BasinId((2 << 32) | 1));
    assert_eq!(elder.elder_parent, Some(BasinId((2 << 32) | 5)));
    assert_eq!(elder.spill.sill.raw(), 5);
    let from = CellIndex::new(elder.spill.from.y * 7 + elder.spill.from.x, e).unwrap();
    assert_eq!(routing.read(from).unwrap().owner().unwrap().raw(), 17);
    let target = elder.spill.to.unwrap();
    let to = CellIndex::new(target.y * 7 + target.x, e).unwrap();
    assert_eq!(routing.read(to).unwrap().owner().unwrap().raw(), 19);
    assert_eq!(
        registry
            .destination::<routing::MemoryError>(elder.spill.receiving)
            .unwrap(),
        Destination::Basin(BasinId((2 << 32) | 5)),
    );
}
fn cap() -> MstLimits {
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
fn disk(d: &Directory, e: Extent) -> DiskHierarchyStore {
    let mut c = ChildLimits::required(&d.0, 2, 20).unwrap();
    c.io_bytes = 1 << 30;
    c.io_operations = 1_000_000;
    c.comparisons = 1_000_000;
    let children = DiskChildLinks::create(&d.0, c).unwrap();
    DiskHierarchyStore::new(
        &d.0,
        e,
        DiskLimits {
            cache_bytes: DiskHierarchyStore::cache_required(&d.0, 1).unwrap(),
            scratch_bytes: 1 << 24,
            io_bytes: 1 << 30,
            io_operations: 1_000_000,
        },
        children,
    )
    .unwrap()
}
#[test]
fn actual_tiny_grid_handoff_binds_components_and_real_transit_breakpoints() {
    for tied in [false, true] {
        let (e, mut routing) = terrain(tied);
        let mst_dir = Directory::new();
        let hierarchy_dir = Directory::new();
        let product = mst::produce(&mut routing, &mst_dir.0, cap()).unwrap();
        assert_eq!(product.work.leaves, 3);
        let mut store = disk(&hierarchy_dir, e);
        let mut registry = BreakpointRegistry::new::<routing::MemoryError>(20, 1 << 16).unwrap();
        let (work, reads) = build_from_mst(
            product,
            &mut routing,
            &mut store,
            &mut registry,
            hierarchy::Limits {
                leaves: 3,
                store_operations: 100_000,
            },
            1000,
        )
        .unwrap();
        store.finish().unwrap();
        assert!(reads > 0);
        assert!(work.nodes >= 4);
        if tied {
            assert_eq!(work.contracted, 1);
        }
        let (nodes, elders) = store.output_counts().unwrap();
        assert_eq!(elders, 3);
        let records: Vec<_> = (0..nodes).map(|i| store.output_node(i).unwrap()).collect();
        assert!(records.iter().any(|n| n.id.0 >> 63 == 1));
        let elder: Vec<_> = (0..elders)
            .map(|i| store.output_elder(i).unwrap())
            .collect();
        let oldest = elder
            .iter()
            .find(|x| x.leaf == BasinId((2 << 32) | 1))
            .unwrap();
        assert_eq!(oldest.spill.receiving, ReceivingAccount::DomainExport);
        let from = CellIndex::new(oldest.spill.from.y * 7 + oldest.spill.from.x, e).unwrap();
        let actual_owner = routing.read(from).unwrap().owner().unwrap();
        assert_ne!(
            e.anchor_key(actual_owner),
            oldest.leaf.0,
            "eldest spills from another constituent of the pre-event component"
        );
        assert!(registry.records().next().is_some());
        for start in registry.records() {
            assert_eq!(
                start.id.0,
                (u64::from(start.at.y) << 32) | u64::from(start.at.x)
            );
            assert!(start.original_next.is_some());
            assert!(matches!(start.destination, Destination::Basin(_)));
            assert_eq!(
                registry
                    .destination::<routing::MemoryError>(ReceivingAccount::Junction(start.id))
                    .unwrap(),
                start.destination
            );
        }
    }
    // Same accepted physical graph: lying about source membership is rejected.
    let (e, mut routing) = terrain(false);
    let mdir = Directory::new();
    let mut product = mst::produce(&mut routing, &mdir.0, cap()).unwrap();
    let minima: Vec<_> = product
        .minima
        .by_ref()
        .map(|m| {
            let m = m.unwrap();
            Minimum {
                at: m.at,
                floor_mm: m.floor_mm,
            }
        })
        .collect();
    let edges: Vec<_> = product.edges.by_ref().collect::<Result<_, _>>().unwrap();
    let d = Directory::new();
    let mut store = disk(&d, e);
    let mut registry = BreakpointRegistry::new::<routing::MemoryError>(20, 1 << 16).unwrap();
    let mut resolver = WitnessResolver::new(&mut routing, &mut registry, 1000);
    let failure = hierarchy::build(
        e,
        3,
        minima.into_iter().map(Ok),
        edges.into_iter().map(Ok),
        &mut store,
        hierarchy::Limits {
            leaves: 3,
            store_operations: 100_000,
        },
        |w| {
            let mut bound = resolver
                .resolve(w)
                .map_err(|_| DiskError::Invalid("test binding"))?;
            bound.source_terminal = GlobalCell { x: 5, y: 2 };
            Ok(bound)
        },
    );
    assert!(matches!(
        failure,
        Err(hierarchy::BuildError::Invalid(
            "spill source outside pre-event component"
        ))
    ));
    let (_, mut routing) = terrain(false);
    let mut registry = BreakpointRegistry::new::<routing::MemoryError>(0, 4096).unwrap();
    let w = Witness {
        source: BasinId((2 << 32) | 3),
        source_anchor: GlobalCell { x: 3, y: 2 },
        from: GlobalCell { x: 3, y: 2 },
        to: Some(GlobalCell { x: 2, y: 2 }),
        sill: HeightMm::new(5),
    };
    assert!(matches!(
        WitnessResolver::new(&mut routing, &mut registry, 0).resolve(w),
        Err(BindingError::Limit("routing reads"))
    ));
    assert!(matches!(
        WitnessResolver::new(&mut routing, &mut registry, 4).resolve(w),
        Err(BindingError::Limit("breakpoint count"))
    ));
    let mut registry = BreakpointRegistry::new::<routing::MemoryError>(2, 8192).unwrap();
    let bad = Witness {
        sill: HeightMm::new(6),
        ..w
    };
    assert!(matches!(
        WitnessResolver::new(&mut routing, &mut registry, 4).resolve(bad),
        Err(BindingError::Invalid("physical sill"))
    ));
}
