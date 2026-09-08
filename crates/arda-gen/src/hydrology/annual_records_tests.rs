use super::annual::{AnnualNode, AnnualSolution, LeafNet, RepresentativeLake};
use super::annual_aggregation::DirectSource;
use super::annual_records::*;
use super::annual_transfers::TreeTransfer;
use super::routing::{Extent, MemoryPages};
use arda_core::hydrology::{
    AnnualWaterBalance, BasinId, Litres, ReceivingAccount, SpillConnection,
};
use arda_core::{GlobalCell, HeightMm};

fn fixture() -> (MemoryPages, Vec<AnnualNode>, AnnualSolution, TreeTransfer) {
    let extent = Extent::new(3, 3).unwrap();
    let store = MemoryPages::new(
        extent,
        &[12, 12, 12, 12, 0, 12, 12, 12, 12],
        &[false; 9],
        1 << 20,
    )
    .unwrap();
    let id = BasinId((1_u64 << 32) | 1);
    let spill = SpillConnection {
        from: GlobalCell { x: 2, y: 0 },
        to: None,
        sill: HeightMm::new(12),
        receiving: ReceivingAccount::DomainExport,
    };
    let node = AnnualNode {
        id,
        parent: None,
        floor: HeightMm::new(0),
        birth: HeightMm::new(0),
        spill: Some(spill),
        destination: Some(super::annual::AnnualDestination::DomainExport),
        children: 0..0,
        bands: 0..0,
        local_runoff: Litres(1),
        local_precipitation: Litres(1),
        local_land_loss: Litres(0),
    };
    let solution = AnnualSolution {
        lakes: vec![RepresentativeLake {
            basin: id,
            surface: HeightMm::new(12),
            submerged_cells: 1,
            geometric_volume: Litres(120_000),
            potential_spill: Some(spill),
        }],
        leaf_net: vec![LeafNet {
            leaf: id,
            account: id,
            lake: Some(id),
            surface: Some(HeightMm::new(12)),
            runoff: Litres(1),
            paid_wet_cost: Litres(0),
            marginal_cost: Litres(0),
            net_litres: 1,
        }],
        balance: AnnualWaterBalance::default(),
        events: 0,
    };
    let transfer = TreeTransfer {
        from_leaf: id,
        to_leaf: None,
        from: GlobalCell { x: 0, y: 0 },
        to: None,
        sill: HeightMm::new(12),
        annual_volume: Litres(1),
    };
    (store, vec![node], solution, transfer)
}
fn limits() -> RecordLimits {
    RecordLimits {
        ram_bytes: 1 << 20,
        source_reads: 100,
    }
}

#[test]
fn positive_subsecond_rounded_flow_keeps_the_actual_active_outlet() {
    let (mut store, nodes, solution, t) = fixture();
    let rows = lake_records(&mut store, &nodes, &solution, &[t], limits()).unwrap();
    assert_eq!(rows[0].annual_outflow, Litres(1));
    assert_eq!(rows[0].mean_outflow.raw(), 0);
    assert_eq!(rows[0].outlet.unwrap().from, GlobalCell { x: 0, y: 0 });
    assert_eq!(rows[0].deepest_bed, HeightMm::new(0));
}

#[test]
fn no_annual_export_preserves_the_potential_spill_without_claiming_supported_flow() {
    let (mut store, nodes, solution, _) = fixture();
    let rows = lake_records(&mut store, &nodes, &solution, &[], limits()).unwrap();
    assert_eq!(rows[0].annual_outflow, Litres(0));
    assert_eq!(rows[0].outlet, solution.lakes[0].potential_spill);
}

#[test]
fn actual_marine_target_is_preserved_and_read_budget_is_enforced() {
    let (_, nodes, solution, mut t) = fixture();
    let extent = Extent::new(3, 3).unwrap();
    let mut sea = [false; 9];
    sea[1] = true;
    let mut store =
        MemoryPages::new(extent, &[12, 0, 12, 12, 0, 12, 12, 12, 12], &sea, 1 << 20).unwrap();
    t.to = Some(GlobalCell { x: 1, y: 0 });
    let rows = lake_records(&mut store, &nodes, &solution, &[t], limits()).unwrap();
    assert_eq!(rows[0].outlet.unwrap().receiving, ReceivingAccount::Sea);
    let mut cap = limits();
    cap.source_reads = 0;
    assert!(matches!(
        lake_records(&mut store, &nodes, &solution, &[t], cap),
        Err(RecordError::Limit)
    ));
}

#[test]
fn routed_nonterminal_and_terminal_targets_use_the_actual_merged_lake() {
    use super::routing::{route_and_own, CellIndex, Limits, RoutingStore};
    let extent = Extent::new(7, 5).unwrap();
    let mut heights = [30; 35];
    heights[15] = -4;
    heights[16] = 12;
    heights[17] = 1;
    heights[24] = -2;
    let mut store = MemoryPages::new(extent, &heights, &[false; 35], 1 << 20).unwrap();
    route_and_own(&mut store, Limits::for_extent(extent)).unwrap();
    let emitter = BasinId((2_u64 << 32) | 1);
    let receiver_leaf = BasinId((3_u64 << 32) | 3);
    let merged = BasinId(1_u64 << 63);
    let (_, mut nodes, mut solution, mut transfer) = fixture();
    nodes[0].id = emitter;
    nodes[0].floor = HeightMm::new(-4);
    let mut receiver = nodes[0].clone();
    receiver.id = merged;
    receiver.floor = HeightMm::new(-2);
    nodes.push(receiver);
    solution.lakes[0].basin = emitter;
    let mut lake = solution.lakes[0].clone();
    lake.basin = merged;
    lake.surface = HeightMm::new(2);
    solution.lakes.push(lake);
    solution.leaf_net[0].leaf = emitter;
    solution.leaf_net[0].lake = Some(emitter);
    let mut leaf = solution.leaf_net[0].clone();
    leaf.leaf = receiver_leaf;
    leaf.account = merged;
    leaf.lake = Some(merged);
    leaf.surface = Some(HeightMm::new(2));
    solution.leaf_net.push(leaf);
    transfer.from_leaf = emitter;
    transfer.from = GlobalCell { x: 2, y: 2 };
    for (raw, to) in [
        (17, GlobalCell { x: 3, y: 2 }),
        (24, GlobalCell { x: 3, y: 3 }),
    ] {
        let at = CellIndex::new(raw, extent).unwrap();
        let owner = store.read(at).unwrap().owner().unwrap();
        assert_eq!(extent.anchor_key(owner), receiver_leaf.0);
        assert_eq!(owner == at, raw == 24);
        transfer.to = Some(to);
        let rows = lake_records(&mut store, &nodes, &solution, &[transfer], limits()).unwrap();
        assert_eq!(
            rows[0].outlet.unwrap().receiving,
            ReceivingAccount::Lake(merged)
        );
    }
    let from = CellIndex::new(16, extent).unwrap();
    assert_eq!(
        extent.anchor_key(store.read(from).unwrap().owner().unwrap()),
        emitter.0
    );
}

#[test]
fn invalid_surface_dry_source_and_ram_fail_without_a_partial_record_set() {
    let (mut store, nodes, mut solution, mut t) = fixture();
    t.sill = HeightMm::new(11);
    assert!(lake_records(&mut store, &nodes, &solution, &[t], limits()).is_err());
    t.sill = HeightMm::new(12);
    solution.leaf_net[0].lake = None;
    assert!(lake_records(&mut store, &nodes, &solution, &[t], limits()).is_err());
    let mut cap = limits();
    cap.ram_bytes = 0;
    assert!(matches!(
        lake_records(&mut store, &nodes, &solution, &[], cap),
        Err(RecordError::Limit)
    ));
}

#[test]
fn source_reclassification_and_actual_fine_export_buckets_close_exactly() {
    let closed = AnnualWaterBalance {
        land_precipitation: Litres(40),
        land_loss: Litres(10),
        lake_precipitation: Litres(60),
        lake_evaporation: Litres(50),
        marginal_evaporation: Litres(5),
        sea_outflow: Litres(35),
        domain_outflow: Litres(0),
    };
    let sea = DirectSource {
        cells: 1,
        precipitation: Litres(20),
        land_loss: Litres(10),
        runoff: Litres(10),
    };
    let domain = DirectSource {
        cells: 1,
        precipitation: Litres(10),
        land_loss: Litres(2),
        runoff: Litres(8),
    };
    let actual = reconcile_balance::<()>(closed, sea, domain, Litres(30), Litres(23)).unwrap();
    assert_eq!(actual.land_precipitation, Litres(70));
    assert_eq!(actual.land_loss, Litres(22));
    assert_eq!(actual.sea_outflow, Litres(30));
    assert_eq!(actual.domain_outflow, Litres(23));
    assert_eq!(actual.lake_precipitation, Litres(60));
}

#[test]
fn ledger_mismatch_and_arithmetic_overflow_are_errors() {
    let z = DirectSource::default();
    assert!(matches!(
        reconcile_balance::<()>(AnnualWaterBalance::default(), z, z, Litres(1), Litres(0)),
        Err(RecordError::Invalid(_))
    ));
    let bad = DirectSource {
        cells: 1,
        precipitation: Litres(2),
        land_loss: Litres(1),
        runoff: Litres(0),
    };
    assert!(
        reconcile_balance::<()>(AnnualWaterBalance::default(), bad, z, Litres(0), Litres(0))
            .is_err()
    );
    let overflow = AnnualWaterBalance {
        sea_outflow: Litres(u128::MAX),
        domain_outflow: Litres(1),
        ..AnnualWaterBalance::default()
    };
    assert!(matches!(
        reconcile_balance::<()>(overflow, z, z, Litres(0), Litres(0)),
        Err(RecordError::Overflow)
    ));
}
