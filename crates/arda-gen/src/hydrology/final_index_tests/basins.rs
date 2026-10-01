//! Closed-basin authorities on a bowl fixture: dry-basin spill points and
//! wet links, duplicate and mismatched authorities, seam halos and branch
//! copies from wide seeds.

use super::*;

fn closed_bowl() -> Fixture {
    let mut f = Fixture::empty(512, 512, 1, 1);
    let e = f.terrain.extent;
    let center = cell(e, 20, 20);
    let north = cell(e, 20, 19);
    let id = BasinId(e.anchor_key(center));
    for (direction, (dx, dy)) in [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ]
    .into_iter()
    .enumerate()
    {
        let x = u32::try_from(20 + dx).unwrap();
        let y = u32::try_from(20 + dy).unwrap();
        let at = cell(e, x, y);
        let to = cell(
            e,
            u32::try_from(20 + 2 * dx).unwrap(),
            u32::try_from(20 + 2 * dy).unwrap(),
        );
        f.terrain.rows.insert(
            at.raw(),
            record(e, at, 20, to, u8::try_from(direction).unwrap(), false),
        );
        let row = FlowRecord {
            visited: true,
            selected: if at == north { 1 << 6 } else { 0 },
            ..FlowRecord::default()
        };
        f.flow.rows.insert(at.raw(), row);
        let mut marine = FlowRecord {
            visited: true,
            ..FlowRecord::default()
        };
        marine.metrics.catchment_cells = 1;
        f.flow.rows.insert(to.raw(), marine);
    }
    f.terrain
        .rows
        .insert(center.raw(), record(e, center, 10, center, 8, false));
    let mut row = FlowRecord {
        visited: true,
        parent: Some(north),
        selected: 1 << 1,
        ..FlowRecord::default()
    };
    row.metrics.catchment_cells = 1;
    row.metrics.drainage_cells = 1;
    row.metrics.order = 1;
    row.metrics.scalar_annual = 60 * YEAR;
    f.flow.rows.insert(center.raw(), row);
    let spill = SpillConnection {
        from: GlobalCell { x: 20, y: 20 },
        to: Some(GlobalCell { x: 20, y: 19 }),
        sill: HeightMm::new(20),
        receiving: ReceivingAccount::Junction(arda_core::hydrology::JunctionId::at(GlobalCell {
            x: 20,
            y: 19,
        })),
    };
    f.nodes.push(AnnualNode {
        id,
        parent: None,
        floor: HeightMm::new(10),
        birth: HeightMm::new(10),
        spill: Some(spill),
        destination: Some(AnnualDestination::Sea),
        children: 0..0,
        bands: 0..0,
        local_runoff: Litres(u128::from(60 * YEAR)),
        local_precipitation: Litres(u128::from(120 * YEAR)),
        local_land_loss: Litres(u128::from(60 * YEAR)),
    });
    f.leaves.push(LeafNet {
        leaf: id,
        account: id,
        lake: None,
        surface: None,
        runoff: Litres(u128::from(60 * YEAR)),
        paid_wet_cost: Litres(0),
        marginal_cost: Litres(u128::from(60 * YEAR)),
        net_litres: 0,
    });
    f
}
#[test]
fn physical_dry_basin_point_retains_potential_spill_and_actual_wet_link() {
    let mut f = closed_bowl();
    let id = f.nodes[0].id;
    let center = cell(f.terrain.extent, 20, 20);
    let index = f.run(limits()).unwrap();
    assert_eq!(index.reaches.len(), 1);
    assert_eq!(index.catchments.len(), 9);
    let p = &index.reaches[0];
    assert_eq!(p.id, ReachId::point(GlobalCell { x: 20, y: 20 }).unwrap());
    assert_eq!(p.from, p.to);
    assert_eq!(p.receiving, ReceivingAccount::Lake(id));
    assert_eq!(p.mean_discharge.raw(), 60);
    let c = index
        .catchments
        .iter()
        .find(|c| c.basin == Some(id))
        .unwrap();
    assert_eq!(c.representative_lake, None);
    assert_eq!(c.potential_spill, f.nodes[0].spill);
    for c in &index.catchments {
        arda_core::formats::hydrology::encode_record(c).unwrap();
    }
    // The annual representative link is copied from the completed wet authority.
    let wet = BasinId(1_u64 << 63 | 1);
    f.leaves[0].lake = Some(wet);
    f.leaves[0].surface = Some(HeightMm::new(15));
    let row = f.flow.rows.get_mut(&center.raw()).unwrap();
    row.lake = Some(wet);
    row.surface_mm = 15;
    row.metrics.scalar_annual = 0;
    let index = f.run(limits()).unwrap();
    assert!(index.reaches.is_empty());
    let c = index
        .catchments
        .iter()
        .find(|c| c.basin == Some(id))
        .unwrap();
    assert_eq!(c.representative_lake, Some(wet));
    assert_eq!(c.receiving, f.nodes[0].spill.unwrap().receiving);
    for c in &index.catchments {
        arda_core::formats::hydrology::encode_record(c).unwrap();
    }
}
#[test]
fn missing_leaf_mismatched_wet_state_and_duplicate_authorities_fail() {
    let mut f = closed_bowl();
    let node = f.nodes.pop().unwrap();
    assert!(matches!(
        f.run(limits()),
        Err(IndexError::Invalid("missing physical leaf"))
    ));
    f.nodes.push(node.clone());
    let leaf = f.leaves.pop().unwrap();
    assert!(matches!(
        f.run(limits()),
        Err(IndexError::Invalid("missing final leaf state"))
    ));
    f.leaves.push(leaf.clone());
    f.leaves[0].lake = Some(BasinId(99));
    assert!(matches!(
        f.run(limits()),
        Err(IndexError::Invalid("terminal leaf state"))
    ));
    f.leaves[0] = leaf;
    f.nodes.push(node);
    let reads = f.terrain.reads;
    assert!(matches!(
        f.run(limits()),
        Err(IndexError::Invalid("domain or authority order"))
    ));
    assert_eq!(f.terrain.reads, reads);
    f.nodes.pop();
    f.leaves.push(f.leaves[0].clone());
    assert!(matches!(
        f.run(limits()),
        Err(IndexError::Invalid("domain or authority order"))
    ));
    assert!(matches!(
        f.run(IndexLimits {
            operations: 0,
            ..limits()
        }),
        Err(IndexError::Limit)
    ));
}

#[test]
fn outer_point_halo_uses_row_major_area_indices_across_a_vertical_seam() {
    let mut f = Fixture::empty(1024, 1024, 2, 2);
    f.export_point(1023, 511, 50);
    let index = f.run(limits()).unwrap();
    assert_eq!(index.area_reaches, vec![vec![], vec![0], vec![], vec![0]]);
    assert_eq!(index.area(ac(1, 0)), Some([0_usize].as_slice()));
    assert_eq!(index.area(ac(1, 1)), Some([0_usize].as_slice()));
    assert_eq!(index.reaches[0].from, GlobalCell { x: 1023, y: 511 });
    assert_eq!(index.reaches[0].to, index.reaches[0].from);
}
#[test]
fn wide_seed_copies_narrow_incident_branch_once_without_recursive_continuation() {
    let mut f = Fixture::empty(1024, 512, 2, 1);
    let e = f.terrain.extent;
    let a = cell(e, 600, 200);
    let junction = cell(e, 601, 200);
    let narrow = cell(e, 602, 200);
    let sink = cell(e, 603, 200);
    let basin = BasinId(e.anchor_key(sink));
    let high = 33_249_619_483_u64;
    for (at, height, receiver) in [
        (a, 40, 4),
        (junction, 30, 4),
        (narrow, 20, 4),
        (sink, 10, 8),
    ] {
        f.terrain
            .rows
            .insert(at.raw(), record(e, at, height, sink, receiver, false));
    }
    for (at, parent, q, net, selected) in [
        (a, Some(junction), high, i128::from(high * YEAR), 0),
        (
            junction,
            None,
            high - 1,
            i128::from((high - 1) * YEAR),
            1 << 6,
        ),
        (narrow, Some(junction), 40, -i128::from(YEAR), 0),
        (sink, Some(narrow), 40, -i128::from(40 * YEAR), 0),
    ] {
        let mut row = FlowRecord {
            parent,
            net,
            selected,
            visited: true,
            ..FlowRecord::default()
        };
        row.metrics.drainage_cells = 4;
        row.metrics.order = 1;
        row.metrics.scalar_annual = q * YEAR;
        row.metrics.catchment_cells = if at == sink { 4 } else { 0 };
        f.flow.rows.insert(at.raw(), row);
    }
    let spill = SpillConnection {
        from: GlobalCell { x: 601, y: 200 },
        to: Some(GlobalCell { x: 601, y: 201 }),
        sill: HeightMm::new(30),
        receiving: ReceivingAccount::Sea,
    };
    f.nodes.push(AnnualNode {
        id: basin,
        parent: None,
        floor: HeightMm::new(10),
        birth: HeightMm::new(10),
        spill: Some(spill),
        destination: Some(AnnualDestination::Sea),
        children: 0..0,
        bands: 0..0,
        local_runoff: Litres(0),
        local_precipitation: Litres(0),
        local_land_loss: Litres(0),
    });
    f.leaves.push(LeafNet {
        leaf: basin,
        account: basin,
        lake: None,
        surface: None,
        runoff: Litres(0),
        paid_wet_cost: Litres(0),
        marginal_cost: Litres(0),
        net_litres: 0,
    });
    let index = f.run(limits()).unwrap();
    assert_eq!(index.reaches.len(), 5);
    let ids = |area| {
        index
            .area(area)
            .unwrap()
            .iter()
            .map(|&i| index.reaches[i].id)
            .collect::<Vec<_>>()
    };
    let point = |x, y| GlobalCell { x, y };
    let branch = ReachId::from_step(point(601, 200), point(602, 200)).unwrap();
    let continuation = ReachId::from_step(point(602, 200), point(603, 200)).unwrap();
    let left = ids(ac(0, 0));
    assert_eq!(left.len(), 3);
    assert!(left.contains(&branch));
    assert!(!left.contains(&continuation));
    assert!(!left.contains(&ReachId::point(point(603, 200)).unwrap()));
    assert_eq!(ids(ac(1, 0)).len(), 5);
    assert_eq!(
        index.payload_bytes,
        final_index::required_ram(6, 8, 2).unwrap()
    );
    assert!(matches!(
        f.run(IndexLimits {
            area_references: 7,
            ..limits()
        }),
        Err(IndexError::Limit)
    ));
    assert!(f
        .run(IndexLimits {
            area_references: 8,
            ..limits()
        })
        .is_ok());
}
