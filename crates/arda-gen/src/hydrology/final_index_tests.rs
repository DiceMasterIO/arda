//! Sparse completed authority controls; no routing/flow mutation is permitted.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::hydrology::annual::{AnnualDestination, AnnualNode, LeafNet};
use crate::hydrology::final_index::{self, FinalIndex, IndexError, IndexLimits};
use crate::hydrology::fine_flow::{FlowRecord, FlowStore};
use crate::hydrology::routing::{CellIndex, CellRecord, Extent, RoutingStore, Tape};
use arda_core::hydrology::{
    BasinId, CatchmentId, HydrologyDomain, Litres, ReachId, ReceivingAccount, SpillConnection,
};
use arda_core::{AreaCoord, GlobalCell, HeightMm};
use std::collections::BTreeMap;
const YEAR: u64 = 31_536_000;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceError {
    Read,
    Mutation,
}
#[derive(Clone)]
struct Terrain {
    extent: Extent,
    rows: BTreeMap<u32, CellRecord>,
    fail: Option<u32>,
    reads: u64,
}
#[derive(Clone)]
struct Flow {
    extent: Extent,
    rows: BTreeMap<u32, FlowRecord>,
    fail: Option<u32>,
    reads: u64,
}
fn cell(e: Extent, x: u32, y: u32) -> CellIndex {
    let (w, _) = e.coordinates(CellIndex::new(e.cells() - 1, e).unwrap());
    CellIndex::new(y * (w + 1) + x, e).unwrap()
}
fn record(
    e: Extent,
    at: CellIndex,
    height: i32,
    owner: CellIndex,
    receiver: u8,
    marine: bool,
) -> CellRecord {
    let mut bytes = CellRecord::prepared(height, marine).encode();
    bytes[4..8].copy_from_slice(&0_u32.to_le_bytes());
    bytes[8..12].copy_from_slice(&owner.raw().to_le_bytes());
    bytes[12] = receiver;
    CellRecord::decode(bytes, e, at).unwrap()
}
impl RoutingStore for Terrain {
    type Error = SourceError;
    fn extent(&self) -> Extent {
        self.extent
    }
    fn read(&mut self, at: CellIndex) -> Result<CellRecord, Self::Error> {
        self.reads += 1;
        if self.fail == Some(at.raw()) {
            return Err(SourceError::Read);
        }
        Ok(self
            .rows
            .get(&at.raw())
            .copied()
            .unwrap_or_else(|| record(self.extent, at, 0, at, 9, true)))
    }
    fn write(&mut self, _: CellIndex, _: CellRecord) -> Result<(), Self::Error> {
        Err(SourceError::Mutation)
    }
    fn mark_marine(&mut self, _: CellIndex) -> Result<(), Self::Error> {
        Err(SourceError::Mutation)
    }
    fn clear(&mut self, _: Tape) -> Result<(), Self::Error> {
        Err(SourceError::Mutation)
    }
    fn len(&self, _: Tape) -> u32 {
        0
    }
    fn push(&mut self, _: Tape, _: CellIndex) -> Result<(), Self::Error> {
        Err(SourceError::Mutation)
    }
    fn get(&mut self, _: Tape, _: u32) -> Result<CellIndex, Self::Error> {
        Err(SourceError::Mutation)
    }
}
impl FlowStore for Flow {
    type Error = SourceError;
    fn extent(&self) -> Extent {
        self.extent
    }
    fn read(&mut self, at: CellIndex) -> Result<FlowRecord, Self::Error> {
        self.reads += 1;
        if self.fail == Some(at.raw()) {
            return Err(SourceError::Read);
        }
        Ok(self.rows.get(&at.raw()).copied().unwrap_or(FlowRecord {
            visited: true,
            ..FlowRecord::default()
        }))
    }
    fn write(&mut self, _: CellIndex, _: FlowRecord) -> Result<(), Self::Error> {
        Err(SourceError::Mutation)
    }
}
struct Fixture {
    terrain: Terrain,
    flow: Flow,
    domain: HydrologyDomain,
    nodes: Vec<AnnualNode>,
    leaves: Vec<LeafNet>,
}
impl Fixture {
    fn empty(width: u32, height: u32, wide: u32, high: u32) -> Self {
        let extent = Extent::new(width, height).unwrap();
        Self {
            terrain: Terrain {
                extent,
                rows: BTreeMap::new(),
                fail: None,
                reads: 0,
            },
            flow: Flow {
                extent,
                rows: BTreeMap::new(),
                fail: None,
                reads: 0,
            },
            domain: HydrologyDomain {
                width_cells: width,
                height_cells: height,
                exported_areas_wide: wide,
                exported_areas_high: high,
            },
            nodes: vec![],
            leaves: vec![],
        }
    }
    fn sea_edge(&mut self, x: u32, y: u32, dx: i32, q: u64) {
        let e = self.terrain.extent;
        let at = cell(e, x, y);
        let to = cell(e, u32::try_from(i64::from(x) + i64::from(dx)).unwrap(), y);
        self.terrain.rows.insert(
            at.raw(),
            record(e, at, 100, to, if dx == 1 { 4 } else { 3 }, false),
        );
        let annual = q * YEAR;
        let mut row = FlowRecord {
            net: i128::from(annual),
            visited: true,
            ..FlowRecord::default()
        };
        row.metrics.drainage_cells = 1;
        row.metrics.order = 1;
        row.metrics.scalar_annual = annual;
        self.flow.rows.insert(at.raw(), row);
        self.flow
            .rows
            .entry(to.raw())
            .or_insert(FlowRecord {
                visited: true,
                ..FlowRecord::default()
            })
            .metrics
            .catchment_cells += 1;
    }
    fn export_point(&mut self, x: u32, y: u32, q: u64) {
        let e = self.terrain.extent;
        let at = cell(e, x, y);
        self.terrain
            .rows
            .insert(at.raw(), record(e, at, 100, at, 10, false));
        let mut row = FlowRecord {
            net: i128::from(q * YEAR),
            visited: true,
            ..FlowRecord::default()
        };
        row.metrics.catchment_cells = 1;
        row.metrics.drainage_cells = 1;
        row.metrics.order = 1;
        row.metrics.scalar_annual = q * YEAR;
        self.flow.rows.insert(at.raw(), row);
    }
    fn run(
        &mut self,
        limits: IndexLimits,
    ) -> Result<FinalIndex, IndexError<SourceError, SourceError>> {
        final_index::build(
            &mut self.terrain,
            &mut self.flow,
            self.domain,
            &self.nodes,
            &self.leaves,
            limits,
        )
    }
}
fn limits() -> IndexLimits {
    IndexLimits {
        ram_bytes: 1 << 24,
        records: 100,
        area_references: 100,
        operations: 100_000_000,
    }
}
fn ac(x: i32, y: i32) -> AreaCoord {
    AreaCoord { x, y }
}
fn featured() -> Fixture {
    let mut f = Fixture::empty(1032, 512, 2, 1);
    // Two dry contributors share one actual marine terminal; the marine cell is not a source.
    f.sea_edge(510, 100, 1, 60);
    f.sea_edge(512, 100, -1, 40);
    // Half-width is >88 cells: this centreline is outside area0 but its footprint overlaps.
    f.sea_edge(600, 200, 1, 33_249_619_483);
    f.export_point(0, 300, 50);
    // Saved fringe channel is included only in the last exported area's width halo.
    f.sea_edge(1028, 400, 1, 33_249_619_483);
    f
}
#[test]
fn sorted_authorities_preserve_marine_source_count_and_crossing_direction() {
    let mut f = featured();
    let terrain = f.terrain.rows.clone();
    let flow = f.flow.rows.clone();
    let index = f.run(limits()).unwrap();
    assert_eq!(f.terrain.rows, terrain);
    assert_eq!(f.flow.rows, flow);
    assert_eq!(index.reaches.len(), 5);
    assert_eq!(index.crossings.len(), 1);
    assert_eq!(index.catchments.len(), 4);
    assert!(index.reaches.windows(2).all(|v| v[0].id < v[1].id));
    assert!(index
        .catchments
        .windows(2)
        .all(|v| v[0].catchment < v[1].catchment));
    assert_eq!(
        index
            .catchments
            .iter()
            .map(|v| v.contributing_cells)
            .sum::<u32>(),
        5
    );
    let owner = CatchmentId((100_u64 << 32) | 511);
    let marine = index
        .catchments
        .iter()
        .find(|c| c.catchment == owner)
        .unwrap();
    assert_eq!(marine.contributing_cells, 2);
    assert_eq!(marine.receiving, ReceivingAccount::Sea);
    let crossing = &index.crossings[0];
    assert_eq!(crossing.from, GlobalCell { x: 512, y: 100 });
    assert_eq!(crossing.to, GlobalCell { x: 511, y: 100 });
    assert_eq!(crossing.id.low, crossing.to);
    assert_eq!(crossing.id.high, crossing.from);
    assert_eq!(crossing.annual_volume, Litres(u128::from(40 * YEAR)));
    let reach = index
        .reaches
        .iter()
        .find(|r| r.id == crossing.reach)
        .unwrap();
    assert_eq!(crossing.catchment, reach.catchment);
    assert_eq!(crossing.mean_discharge, reach.mean_discharge);
    for r in &index.reaches {
        arda_core::formats::hydrology::encode_record(r).unwrap();
    }
    for c in &index.catchments {
        arda_core::formats::hydrology::encode_record(c).unwrap();
    }
    for c in &index.crossings {
        arda_core::formats::hydrology::encode_record(c).unwrap();
    }
}
#[test]
fn physical_width_halo_points_fringe_and_area_lookup_are_exact_and_unique() {
    let mut f = featured();
    let index = f.run(limits()).unwrap();
    let a = index.area(ac(0, 0)).unwrap();
    let b = index.area(ac(1, 0)).unwrap();
    let ordinal = |x, y| {
        index
            .reaches
            .iter()
            .position(|r| r.from == GlobalCell { x, y })
            .unwrap()
    };
    assert_eq!(
        a,
        &[
            ordinal(510, 100),
            ordinal(512, 100),
            ordinal(600, 200),
            ordinal(0, 300)
        ]
    );
    assert_eq!(
        b,
        &[
            ordinal(510, 100),
            ordinal(512, 100),
            ordinal(600, 200),
            ordinal(1028, 400)
        ]
    );
    assert!(a.windows(2).all(|p| p[0] < p[1]));
    assert!(b.windows(2).all(|p| p[0] < p[1]));
    assert!(index.reaches[ordinal(0, 300)].id.is_point());
    assert_eq!(
        index.payload_bytes,
        final_index::required_ram(10, 8, 2).unwrap()
    );
    assert!(index.area(ac(-1, 0)).is_none());
    assert!(index.area(ac(0, -1)).is_none());
    assert!(index.area(ac(2, 0)).is_none());
    assert!(index.area(ac(0, 1)).is_none());
}
#[test]
fn empty_multiple_area_lists_and_exact_work_budget_are_admitted_without_features() {
    let mut f = Fixture::empty(512, 1024, 1, 2);
    let out = f.run(limits()).unwrap();
    assert_eq!(out.area_reaches, vec![Vec::<usize>::new(), vec![]]);
    assert_eq!(out.operations, 3 * u64::from(f.terrain.extent.cells()) + 4);
    assert_eq!(
        out.payload_bytes,
        final_index::required_ram(0, 0, 2).unwrap()
    );
    let cap = IndexLimits {
        operations: out.operations,
        ram_bytes: out.payload_bytes,
        records: 0,
        area_references: 0,
    };
    assert!(f.run(cap).is_ok());
    assert!(matches!(
        f.run(IndexLimits {
            operations: out.operations - 1,
            ..cap
        }),
        Err(IndexError::Limit)
    ));
}
#[test]
fn row_reference_ram_and_work_caps_fail_before_publication() {
    let mut f = featured();
    let out = f.run(limits()).unwrap();
    let exact = IndexLimits {
        ram_bytes: out.payload_bytes,
        records: 10,
        area_references: 8,
        operations: out.operations,
    };
    assert!(f.run(exact).is_ok());
    for cap in [
        IndexLimits {
            records: 9,
            ..exact
        },
        IndexLimits {
            area_references: 7,
            ..exact
        },
        IndexLimits {
            ram_bytes: out.payload_bytes - 1,
            ..exact
        },
        IndexLimits {
            operations: out.operations - 1,
            ..exact
        },
    ] {
        assert!(matches!(
            f.run(cap),
            Err(IndexError::Limit) | Err(IndexError::Extract(_))
        ));
    }
    assert_eq!(final_index::required_ram(u64::MAX, 0, 0), None);
    assert_eq!(final_index::required_ram(0, u64::MAX, 0), None);
    assert_eq!(final_index::required_ram(0, 0, u64::MAX), None);
    assert!(std::mem::size_of::<arda_core::hydrology::GlobalReach>() * 2 + 128 < 512);
}
#[test]
fn completed_count_corruption_and_original_source_errors_remain_typed() {
    let mut f = Fixture::empty(512, 512, 1, 1);
    f.sea_edge(1, 0, -1, 40);
    let owner = cell(f.terrain.extent, 0, 0);
    let source = cell(f.terrain.extent, 1, 0);
    f.flow
        .rows
        .get_mut(&owner.raw())
        .unwrap()
        .metrics
        .catchment_cells = 2;
    assert!(matches!(
        f.run(limits()),
        Err(IndexError::Invalid(
            "catchment count differs from nonmarine sources"
        ))
    ));
    f.flow
        .rows
        .get_mut(&owner.raw())
        .unwrap()
        .metrics
        .catchment_cells = 1;
    f.flow
        .rows
        .get_mut(&source.raw())
        .unwrap()
        .metrics
        .catchment_cells = 1;
    assert!(matches!(
        f.run(limits()),
        Err(IndexError::Invalid("nonterminal catchment count"))
    ));
    f.flow
        .rows
        .get_mut(&source.raw())
        .unwrap()
        .metrics
        .catchment_cells = 0;
    f.terrain.fail = Some(0);
    assert!(matches!(
        f.run(limits()),
        Err(IndexError::Routing(SourceError::Read))
    ));
    f.terrain.fail = None;
    f.flow.fail = Some(0);
    assert!(matches!(
        f.run(limits()),
        Err(IndexError::Flow(SourceError::Read))
    ));
    f.flow.fail = None;
    f.flow.rows.get_mut(&source.raw()).unwrap().visited = false;
    assert!(matches!(f.run(limits()), Err(IndexError::Extract(_))));
}
#[test]
fn domain_and_early_ram_failures_do_not_read_sources() {
    let mut f = Fixture::empty(512, 512, 1, 1);
    f.domain.exported_areas_wide = 2;
    assert!(matches!(f.run(limits()), Err(IndexError::Invalid(_))));
    f.domain.exported_areas_wide = 1;
    f.domain.height_cells = 513;
    assert!(matches!(f.run(limits()), Err(IndexError::Invalid(_))));
    f.domain.height_cells = 512;
    assert!(matches!(
        f.run(IndexLimits {
            ram_bytes: 0,
            ..limits()
        }),
        Err(IndexError::Limit)
    ));
    assert_eq!(f.terrain.reads, 0);
    assert_eq!(f.flow.reads, 0);
    f.flow.extent = Extent::new(513, 512).unwrap();
    assert!(matches!(f.run(limits()), Err(IndexError::Invalid(_))));
}
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

#[test]
fn public_area_lookup_rejects_malformed_export_arithmetic_without_panicking() {
    let index = FinalIndex {
        domain: HydrologyDomain {
            width_cells: 512,
            height_cells: 512,
            exported_areas_wide: u32::MAX,
            exported_areas_high: u32::MAX,
        },
        reaches: vec![],
        crossings: vec![],
        catchments: vec![],
        area_reaches: vec![vec![]],
        operations: 0,
        payload_bytes: 0,
    };
    assert!(index.area(ac(1, 2)).is_none());
    assert!(index.area(ac(1, 1)).is_none());
}
