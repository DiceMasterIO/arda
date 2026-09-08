//! Hand-calculated metrics on actual routed and signed fine-flow fixtures.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::hydrology::{
    fine_flow::{self, CellSource, FlowLimits, FlowStore},
    flow_adjacency::{self, AdjacencyError},
    flow_metrics::{self, MetricsError, MetricsLimits, CHANNEL_ANNUAL},
    memory_flow::MemoryFlowStore,
    routing::{self, CellIndex, Extent, MemoryPages, RoutingStore},
    saddles::{Node, Saddle},
};
use arda_core::{
    hydrology::{BasinId, GlobalLake, Litres},
    DischargeMilli, HeightMm,
};
use std::collections::BTreeMap;
const YEAR: u128 = 31_536_000;
struct Fixture {
    routing: MemoryPages,
    flow: MemoryFlowStore,
    lakes: Vec<GlobalLake>,
    extent: Extent,
}
impl Fixture {
    fn at(&self, raw: u32) -> CellIndex {
        CellIndex::new(raw, self.extent).unwrap()
    }
    fn row(&mut self, raw: u32) -> fine_flow::FlowRecord {
        self.flow.read(self.at(raw)).unwrap()
    }
    fn run(&mut self) -> flow_metrics::MetricsWork {
        flow_metrics::run(&mut self.routing, &mut self.flow, &self.lakes, limits()).unwrap()
    }
}
fn limits() -> MetricsLimits {
    MetricsLimits {
        ram_bytes: 1 << 20,
        lakes: 100,
        lake_edges: 1000,
        operations: 1_000_000,
    }
}
type Wet = (u32, u64, i32);
type Supply = (u32, u128, u128, u128);
type Edge = (u32, Option<u32>, u32, Option<u32>, i32);
fn fixture(
    width: u32,
    heights: &[i32],
    wet: &[Wet],
    supplies: &[Supply],
    edges: &[Edge],
) -> Fixture {
    fixture_with_marine(
        width,
        heights,
        &vec![false; heights.len()],
        wet,
        supplies,
        edges,
    )
}
fn fixture_with_marine(
    width: u32,
    heights: &[i32],
    marine: &[bool],
    wet: &[Wet],
    supplies: &[Supply],
    edges: &[Edge],
) -> Fixture {
    let height = u32::try_from(heights.len()).unwrap() / width;
    let extent = Extent::new(width, height).unwrap();
    let at = |raw| CellIndex::new(raw, extent).unwrap();
    let mut routing = MemoryPages::new(extent, heights, marine, 1 << 20).unwrap();
    routing::route_and_own(&mut routing, routing::Limits::for_extent(extent)).unwrap();
    let mut flow = MemoryFlowStore::new(extent, 1 << 20).unwrap();
    let mut sources: Vec<_> = (0..extent.cells())
        .map(|raw| CellSource {
            at: at(raw),
            precipitation: Litres(0),
            land_loss: Litres(0),
            evaporation: Litres(0),
            marginal: Litres(0),
            wet: None,
        })
        .collect();
    let mut lakes: BTreeMap<BasinId, GlobalLake> = BTreeMap::new();
    for &(raw, id, surface) in wet {
        sources[usize::try_from(raw).unwrap()].wet = Some((BasinId(id), surface));
        let bed = HeightMm::new(heights[usize::try_from(raw).unwrap()]);
        let lake = lakes.entry(BasinId(id)).or_insert(GlobalLake {
            basin: BasinId(id),
            surface: HeightMm::new(surface),
            deepest_bed: bed,
            submerged_cells: 0,
            outlet: None,
            annual_outflow: Litres(0),
            mean_outflow: DischargeMilli::new(0),
        });
        lake.submerged_cells += 1;
        lake.deepest_bed = lake.deepest_bed.min(bed);
    }
    for &(raw, p, e, m) in supplies {
        let row = &mut sources[usize::try_from(raw).unwrap()];
        row.precipitation = Litres(p);
        row.evaporation = Litres(e);
        row.marginal = Litres(m);
    }
    let accepted: Vec<_> = edges
        .iter()
        .map(|&(left, right, from, to, sill)| Saddle {
            left: Node::Closed(at(left)),
            right: right.map_or(Node::Exterior, |r| Node::Closed(at(r))),
            from: at(from),
            to: to.map(at),
            sill_mm: sill,
        })
        .collect();
    fine_flow::run(
        &mut routing,
        &mut flow,
        sources.into_iter().map(Ok::<_, ()>),
        accepted.into_iter().map(Ok),
        FlowLimits {
            operations: 1_000_000,
            absolute_litres: 1 << 62,
            accepted_edges: u64::try_from(edges.len()).unwrap(),
        },
        |_| Ok(()),
    )
    .unwrap();
    Fixture {
        routing,
        flow,
        lakes: lakes.into_values().collect(),
        extent,
    }
}
fn bowl() -> Fixture {
    let mut heights = [20; 25];
    let mut wet = Vec::new();
    let id = (2_u64 << 32) | 2;
    for y in 1..4 {
        for x in 1..4 {
            let raw = y * 5 + x;
            heights[usize::try_from(raw).unwrap()] = 1;
            wet.push((raw, id, 5));
        }
    }
    heights[12] = 0;
    heights[14] = 5;
    fixture(
        5,
        &heights,
        &wet,
        &[(0, 60 * YEAR, 0, 0), (4, 60 * YEAR, 0, 0)],
        &[(12, None, 14, None, 5)],
    )
}
fn dry_hand() -> Fixture {
    fixture(
        3,
        &[4, 3, 2, 3, 2, 1, 2, 1, 0],
        &[],
        &[(8, 40 * YEAR, 0, 0)],
        &[],
    )
}
fn fork() -> Fixture {
    let mut h = [20; 21];
    h[8] = 0;
    h[9] = 5;
    h[10] = 5;
    h[11] = 5;
    h[12] = 0;
    h[13] = 10;
    fixture(
        7,
        &h,
        &[(8, (1 << 32) | 1, 5), (12, (1 << 32) | 5, 5)],
        &[
            (8, 0, 50 * YEAR, 0),
            (12, 0, 50 * YEAR, 0),
            (9, 40 * YEAR, 0, 0),
            (10, 20 * YEAR, 0, 0),
            (11, 40 * YEAR, 0, 0),
        ],
        &[(8, Some(12), 10, Some(11), 5), (12, None, 13, None, 10)],
    )
}
#[test]
fn eight_equal_tributaries_keep_their_metrics_in_real_disk_storage() {
    use crate::orchestrator::flow_disk::{self, DiskFlowStore, FlowDiskError};

    // Every neighbor drains into the centre, which absorbs the complete annual
    // supply. Eight first-order tributaries produce one second-order channel.
    let mut supplies: Vec<_> = (0..9)
        .filter(|&raw| raw != 4)
        .map(|raw| (raw, 40 * YEAR, 0, 0))
        .collect();
    supplies.push((4, 0, 0, 320 * YEAR));
    let mut f = fixture(
        3,
        &[10, 10, 10, 10, 0, 5, 10, 10, 10],
        &[],
        &supplies,
        &[(4, None, 5, None, 5)],
    );
    let path =
        std::env::temp_dir().join(format!("arda-eight-tributaries-{}.bin", std::process::id()));
    let mut disk = DiskFlowStore::create(
        &path,
        f.extent,
        (0..9).map(|_| Ok::<_, std::convert::Infallible>(fine_flow::FlowRecord::default())),
        flow_disk::FlowLimits {
            cache_pages: 1,
            ram_bytes: flow_disk::ram_required(&path, 1).unwrap(),
            scratch_bytes: flow_disk::scratch_required(f.extent),
            io_bytes: 1 << 20,
            io_operations: 10_000,
        },
    )
    .unwrap();
    for raw in 0..9 {
        disk.write(f.at(raw), f.row(raw)).unwrap();
    }
    flow_metrics::run(&mut f.routing, &mut disk, &f.lakes, limits()).unwrap();
    disk.flush().unwrap();
    let row = disk.read(f.at(4)).unwrap();
    assert_eq!(row.metrics.max_in_ties, 8);
    assert_eq!(row.metrics.order, 2);
    assert_eq!(row.metrics.drainage_cells, 9);
    assert_eq!(row.metrics.scalar_annual, 320 * 31_536_000);
    let mut invalid = row;
    invalid.metrics.max_in_ties = 9;
    assert!(matches!(
        disk.write(f.at(4), invalid),
        Err(FlowDiskError::Invalid("metrics state"))
    ));
    drop(disk);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn collapsed_lake_retains_two_incoming_orders_and_drains_into_real_outlet() {
    let mut f = bowl();
    let before: Vec<_> = (0..25)
        .map(|raw| {
            let at = f.at(raw);
            f.routing.read(at).unwrap()
        })
        .collect();
    let work = f.run();
    assert_eq!(work.nodes, 17);
    assert_eq!(work.lake_edges, 1);
    assert_eq!(work.max_order, 2);
    assert_eq!(f.row(0).metrics.order, 1);
    assert_eq!(f.row(4).metrics.order, 1);
    for raw in [6, 7, 8, 11, 12, 13, 16, 17, 18] {
        let row = f.row(raw);
        assert_eq!(row.metrics.order, 2);
        assert_eq!(row.metrics.drainage_cells, 24);
        assert_eq!(row.metrics.hand_at, None);
        assert_eq!(row.metrics.hand_distance_mm, u64::MAX);
        assert_eq!(row.metrics.scalar_annual, 0);
    }
    let outlet = f.row(14);
    assert_eq!(outlet.metrics.scalar_annual, 120 * 31_536_000_u64);
    assert_eq!(outlet.metrics.order, 2);
    assert_eq!(outlet.metrics.drainage_cells, 25);
    assert_eq!(f.row(12).metrics.catchment_cells, 25);
    assert_eq!(outlet.metrics.hand_at, Some(f.at(14)));
    assert_eq!(outlet.metrics.hand_distance_mm, 0);
    assert_eq!(
        f.row(2).metrics.hand_at,
        None,
        "HAND stops at standing water despite its downstream outlet"
    );
    for raw in 0..25 {
        let at = f.at(raw);
        assert_eq!(
            f.routing.read(at).unwrap(),
            before[usize::try_from(raw).unwrap()]
        );
    }
}
#[test]
fn original_zero_edges_carry_drainage_and_physical_diagonal_hand_distance() {
    let mut f = dry_hand();
    let at = f.at(4);
    assert!(flow_adjacency::incident(&mut f.routing, &mut f.flow, at)
        .unwrap()
        .into_iter()
        .flatten()
        .any(|e| e.from == at && e.annual.0 == 0));
    let work = f.run();
    assert_eq!(work.nodes, 9);
    assert_eq!(work.channel_cells, 1);
    assert_eq!(f.row(8).metrics.drainage_cells, 9);
    assert_eq!(f.row(8).metrics.catchment_cells, 9);
    assert_eq!(f.row(8).metrics.order, 1);
    assert_eq!(f.row(4).metrics.order, 0);
    assert_eq!(f.row(4).metrics.hand_at, Some(f.at(8)));
    assert_eq!(f.row(4).metrics.hand_distance_mm, 141_400);
    assert_eq!(f.row(0).metrics.hand_distance_mm, 282_800);
    assert_eq!(f.row(3).metrics.hand_distance_mm, 241_400);
}
#[test]
fn real_dry_divergence_uses_max_scalar_edge_and_tied_shortest_hand_target() {
    let mut f = fork();
    f.run();
    let middle = f.row(10);
    assert_eq!(
        middle.metrics.scalar_annual,
        10 * 31_536_000_u64,
        "scalar Q is max branch, not branch sum"
    );
    assert_eq!(
        middle.metrics.order, 0,
        "both 10 L/s branches are below channel threshold"
    );
    assert_eq!(middle.metrics.hand_distance_mm, 100_000);
    assert_eq!(
        middle.metrics.hand_at,
        Some(f.at(9)),
        "equal distance chooses packed channel coordinate"
    );
    assert_eq!(f.row(9).metrics.scalar_annual, 50 * 31_536_000_u64);
    assert_eq!(f.row(11).metrics.scalar_annual, 50 * 31_536_000_u64);
    assert_eq!(f.row(9).metrics.order, 1);
    assert_eq!(f.row(11).metrics.order, 1);
    let count = f.row(10).metrics.drainage_cells;
    assert!(
        f.row(9).metrics.drainage_cells > count && f.row(11).metrics.drainage_cells > count,
        "both nonreconverging branches inherit upstream drainage"
    );
}
#[test]
fn absorbing_terminal_may_be_an_isolated_scalar_channel_without_a_reach_step() {
    let mut f = fixture(
        3,
        &[10, 10, 10, 10, 0, 5, 10, 10, 10],
        &[],
        &[
            (0, 30 * YEAR, 0, 0),
            (2, 30 * YEAR, 0, 0),
            (4, 0, 0, 60 * YEAR),
        ],
        &[(4, None, 5, None, 5)],
    );
    let at = f.at(4);
    let edges = flow_adjacency::incident(&mut f.routing, &mut f.flow, at).unwrap();
    assert!(edges.into_iter().flatten().all(|e| e.from != at));
    f.run();
    let terminal = f.row(4);
    assert_eq!(terminal.metrics.scalar_annual, 60 * 31_536_000_u64);
    assert_eq!(terminal.metrics.order, 1);
    assert_eq!(terminal.metrics.hand_at, Some(at));
    assert_eq!(terminal.metrics.drainage_cells, 9);
    assert_eq!(terminal.metrics.catchment_cells, 9);
}
#[test]
fn zero_selected_exterior_is_omitted_but_original_zero_export_is_retained() {
    let mut f = fork();
    let at = f.at(13);
    assert!(!flow_adjacency::incident(&mut f.routing, &mut f.flow, at)
        .unwrap()
        .into_iter()
        .flatten()
        .any(|e| e.from == at && e.to.is_none()));
    let mut f = fixture(2, &[3, 2, 2, 1], &[], &[], &[]);
    let at = f.at(3);
    assert!(flow_adjacency::incident(&mut f.routing, &mut f.flow, at)
        .unwrap()
        .into_iter()
        .flatten()
        .any(|e| e.from == at && e.to.is_none() && e.annual.0 == 0));
}
#[test]
fn known_limits_reject_before_publication_and_exact_work_cap_is_sufficient() {
    let mut f = bowl();
    let mut short = limits();
    short.lakes = 0;
    assert!(matches!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, short),
        Err(MetricsError::Limit(_))
    ));
    let mut f = bowl();
    let mut short = limits();
    short.ram_bytes = flow_metrics::required_ram(1, 0).unwrap() - 1;
    assert!(matches!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, short),
        Err(MetricsError::Limit(_))
    ));
    let mut f = bowl();
    let mut short = limits();
    short.lake_edges = 0;
    assert!(matches!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, short),
        Err(MetricsError::Limit(_))
    ));
    let mut f = bowl();
    let mut short = limits();
    short.ram_bytes = flow_metrics::required_ram(1, 0).unwrap();
    assert!(matches!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, short),
        Err(MetricsError::Limit(_))
    ));
    let mut f = dry_hand();
    let work = f.run();
    let mut f = dry_hand();
    let mut short = limits();
    short.operations = work.operations - 1;
    assert!(matches!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, short),
        Err(MetricsError::Limit("operations"))
    ));
    let mut f = dry_hand();
    short.operations = work.operations;
    assert_eq!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, short).unwrap(),
        work
    );
    assert!(flow_metrics::required_ram(u64::MAX, 0).is_none());
    assert!(flow_metrics::required_ram(0, u64::MAX).is_none());
}
#[test]
fn bad_lake_membership_nonfresh_scratch_and_huge_scalar_are_typed_errors() {
    let mut f = bowl();
    f.lakes[0].submerged_cells -= 1;
    assert!(matches!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, limits()),
        Err(MetricsError::Invalid("lake membership count"))
    ));
    let mut f = bowl();
    f.lakes.clear();
    assert!(matches!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, limits()),
        Err(MetricsError::Invalid("unknown wet lake"))
    ));
    let mut f = dry_hand();
    f.run();
    assert!(matches!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, limits()),
        Err(MetricsError::Invalid("metrics scratch is not fresh"))
    ));
    let mut f = dry_hand();
    let at = f.at(8);
    let mut row = f.row(8);
    row.net = 1_i128 << 65;
    f.flow.write(at, row).unwrap();
    assert!(matches!(
        flow_metrics::run(&mut f.routing, &mut f.flow, &f.lakes, limits()),
        Err(MetricsError::Overflow)
    ));
}
#[test]
fn adjacency_rejects_unfinished_asymmetric_and_unequal_lake_state() {
    let mut f = fork();
    let at = f.at(10);
    let mut row = f.row(10);
    row.visited = false;
    f.flow.write(at, row).unwrap();
    assert!(matches!(
        flow_adjacency::incident(&mut f.routing, &mut f.flow, at),
        Err(AdjacencyError::Invalid("unfinished fine flow"))
    ));
    let mut f = fork();
    let at = f.at(10);
    let mut row = f.row(10);
    row.selected = 0;
    f.flow.write(at, row).unwrap();
    assert!(matches!(
        flow_adjacency::incident(&mut f.routing, &mut f.flow, at),
        Err(AdjacencyError::Invalid("asymmetric saddle"))
    ));
    let mut f = bowl();
    let at = f.at(12);
    let mut row = f.row(12);
    row.surface_mm += 1;
    f.flow.write(at, row).unwrap();
    assert!(matches!(
        flow_adjacency::incident(&mut f.routing, &mut f.flow, at),
        Err(AdjacencyError::Invalid("disconnected lake identity"))
    ));
}
#[test]
fn threshold_uses_exact_annual_amount_before_integer_mean_conversion() {
    let mut f = fixture(
        2,
        &[3, 2, 2, 1],
        &[],
        &[(3, u128::from(CHANNEL_ANNUAL) - 1, 0, 0)],
        &[],
    );
    f.run();
    assert_eq!(f.row(3).metrics.order, 0);
    assert_eq!(f.row(3).metrics.hand_at, None);
    let mut f = fixture(
        2,
        &[3, 2, 2, 1],
        &[],
        &[(3, u128::from(CHANNEL_ANNUAL), 0, 0)],
        &[],
    );
    f.run();
    assert_eq!(f.row(3).metrics.order, 1);
    assert_eq!(f.row(3).metrics.hand_at, Some(f.at(3)));
}

#[test]
fn marine_terminals_count_only_original_nonmarine_catchment_cells() {
    let mut f = fixture_with_marine(
        2,
        &[5, 4, 2, 0],
        &[false, false, false, true],
        &[],
        &[(0, 40 * YEAR, 0, 0)],
        &[],
    );
    let work = f.run();
    assert_eq!(work.nodes, 3);
    assert_eq!(
        f.row(3).metrics.catchment_cells,
        3,
        "ocean cell itself is excluded"
    );
    assert_eq!(f.row(3).metrics.hand_at, None);
    assert_eq!(f.row(3).metrics.drainage_cells, 0);
    assert_eq!(f.row(0).metrics.drainage_cells, 1);
    assert_eq!(f.row(0).metrics.order, 1);
    assert_eq!(
        f.row(1).metrics.hand_at,
        None,
        "HAND never crosses the common marine receiver"
    );
}
#[test]
fn zero_selected_marine_spill_is_omitted_from_metric_graph() {
    let mut h = [20; 12];
    h[3] = 0;
    h[5] = 1;
    h[6] = 5;
    let mut marine = [false; 12];
    marine[3] = true;
    let mut f = fixture_with_marine(4, &h, &marine, &[], &[], &[(5, None, 6, Some(3), 5)]);
    let at = f.at(6);
    assert!(!flow_adjacency::incident(&mut f.routing, &mut f.flow, at)
        .unwrap()
        .into_iter()
        .flatten()
        .any(|e| e.from == at && e.to == Some(f.at(3))));
    f.run();
}
#[test]
fn subthreshold_branch_does_not_carry_its_donors_qualifying_order() {
    let mut h = [20; 21];
    h[8] = 0;
    h[9] = 5;
    h[10] = 5;
    h[11] = 5;
    h[12] = 0;
    h[13] = 10;
    let mut f = fixture(
        7,
        &h,
        &[(8, (1 << 32) | 1, 5), (12, (1 << 32) | 5, 5)],
        &[
            (8, 0, 50 * YEAR, 0),
            (12, 0, 50 * YEAR, 0),
            (2, 40 * YEAR, 0, 0),
            (10, 60 * YEAR, 0, 0),
        ],
        &[(8, Some(12), 10, Some(11), 5), (12, None, 13, None, 10)],
    );
    f.run();
    assert_eq!(f.row(10).metrics.scalar_annual, 50 * 31_536_000_u64);
    assert_eq!(f.row(10).metrics.order, 1);
    assert_eq!(f.row(9).metrics.scalar_annual, 50 * 31_536_000_u64);
    assert_eq!(
        f.row(9).metrics.order,
        1,
        "40 L/s input plus a 10 L/s branch is one qualifying incoming order"
    );
}
