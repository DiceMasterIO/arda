#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]
use crate::hydrology::{
    extraction as extract, fine_flow, flow_metrics, local_rivers, memory_flow, routing, saddles,
};
use arda_core::hydrology::{BasinId, GlobalLake, Litres, ReceivingAccount};
use arda_core::{AreaCoord, DischargeMilli, GlobalCell, HeightMm, Terminus};
use fine_flow::{CellSource, FlowLimits};
use memory_flow::MemoryFlowStore;
use routing::{CellIndex, Extent, MemoryPages};
use saddles::{Node, Saddle};
const YEAR: u128 = 31_536_000;
fn source(extent: Extent) -> Vec<CellSource> {
    (0..extent.cells())
        .map(|raw| CellSource {
            at: CellIndex::new(raw, extent).unwrap(),
            precipitation: Litres(0),
            land_loss: Litres(0),
            evaporation: Litres(0),
            marginal: Litres(0),
            wet: None,
        })
        .collect()
}
fn run(
    extent: Extent,
    heights: &[i32],
    sources: Vec<CellSource>,
    saddles: Vec<Saddle>,
    lakes: &[GlobalLake],
) -> (MemoryPages, MemoryFlowStore) {
    let mut routing =
        MemoryPages::new(extent, heights, &vec![false; heights.len()], 1 << 20).unwrap();
    routing::route_and_own(&mut routing, routing::Limits::for_extent(extent)).unwrap();
    let mut flow = MemoryFlowStore::new(extent, 1 << 20).unwrap();
    let accepted_edges = saddles.len() as u64;
    fine_flow::run(
        &mut routing,
        &mut flow,
        sources.into_iter().map(Ok::<_, ()>),
        saddles.into_iter().map(Ok),
        FlowLimits {
            operations: 1_000_000,
            absolute_litres: 1000 * YEAR,
            accepted_edges,
        },
        |_| Ok(()),
    )
    .unwrap();
    flow_metrics::run(
        &mut routing,
        &mut flow,
        lakes,
        flow_metrics::MetricsLimits {
            ram_bytes: 1 << 20,
            lakes: lakes.len() as u64,
            lake_edges: 100,
            operations: 10_000_000,
        },
    )
    .unwrap();
    (routing, flow)
}
fn lake(id: u64, surface: i32) -> GlobalLake {
    GlobalLake {
        basin: BasinId(id),
        surface: HeightMm::new(surface),
        deepest_bed: HeightMm::new(0),
        submerged_cells: 1,
        outlet: None,
        annual_outflow: Litres(0),
        mean_outflow: DischargeMilli::new(0),
    }
}
#[test]
fn exact_saved_split_crossing_and_absorbing_point() {
    let extent = Extent::new(6, 3).unwrap();
    let at = |raw| CellIndex::new(raw, extent).unwrap();
    let mut heights = vec![20; 18];
    for (raw, h) in [(7, 0), (8, 5), (9, 5), (10, 0), (11, 10)] {
        heights[raw] = h;
    }
    let mut sources = source(extent);
    sources[7].wet = Some((BasinId((1 << 32) | 1), 4));
    sources[10].wet = Some((BasinId((1 << 32) | 4), 5));
    sources[7].evaporation = Litres(40 * YEAR);
    sources[10].evaporation = Litres(65 * YEAR);
    sources[8].precipitation = Litres(2 * YEAR);
    sources[3].precipitation = Litres(103 * YEAR);
    // Actual steepest slope at the high cell is 15/100 toward c, exceeding 20/141.4 toward d.
    let saddles = vec![
        Saddle {
            left: Node::Closed(at(7)),
            right: Node::Closed(at(10)),
            sill_mm: 5,
            from: at(8),
            to: Some(at(9)),
        },
        Saddle {
            left: Node::Closed(at(10)),
            right: Node::Exterior,
            sill_mm: 10,
            from: at(11),
            to: None,
        },
    ];
    let (mut r, mut f) = run(
        extent,
        &heights,
        sources,
        saddles,
        &[lake((1 << 32) | 1, 4), lake((1 << 32) | 4, 5)],
    );
    let mut budget = extract::ExtractionBudget { remaining: 10_000 };
    let split = extract::cell(&mut r, &mut f, at(9), &mut budget).unwrap();
    let branches: Vec<_> = split
        .reaches
        .iter()
        .flatten()
        .map(|e| (e.to.x, e.mean_discharge.raw()))
        .collect();
    assert_eq!(branches, vec![(2, 38), (4, 65)]);
    assert_eq!(split.channels.iter().flatten().count(), 1);
    assert_eq!(split.owned.unwrap().to, GlobalCell { x: 4, y: 1 });
    assert!(matches!(
        split
            .reaches
            .iter()
            .flatten()
            .find(|v| v.to.x == 2)
            .unwrap()
            .receiving,
        ReceivingAccount::Junction(_)
    ));
    let b = extract::cell(&mut r, &mut f, at(8), &mut budget).unwrap();
    assert_eq!(b.channels.iter().flatten().count(), 1);
    let incoming_batch = extract::cell(&mut r, &mut f, at(3), &mut budget).unwrap();
    let incoming = incoming_batch.owned.unwrap();
    assert_eq!(incoming.discharge.raw(), 103);
    assert_eq!(incoming.to, GlobalCell { x: 3, y: 1 });
    assert_eq!(
        incoming.end,
        extract::CellEnd::Junction {
            at: incoming.to,
            branches: 2
        }
    );
    let rows = vec![b.owned.unwrap(), split.owned.unwrap()];
    let local = local_rivers::compose(
        AreaCoord { x: 0, y: 0 },
        rows.clone().into_iter().map(Ok::<_, ()>),
        2,
        1 << 21,
    )
    .unwrap();
    assert_eq!(local.len(), 2);
    assert!(local
        .iter()
        .all(|r| r.course.len() == 1 && r.ends == Terminus::Lake && r.feeds.is_none()));
    assert!(local_rivers::compose(
        AreaCoord { x: 0, y: 0 },
        [Ok::<_, ()>(rows[0]), Ok(rows[0])],
        2,
        1 << 21
    )
    .is_err());
    assert!(extract::cell(
        &mut r,
        &mut f,
        at(9),
        &mut extract::ExtractionBudget { remaining: 1 }
    )
    .is_err());
    // The real incoming reach owns its source, independently from the split target.
    let divergent = local_rivers::compose(
        AreaCoord { x: 0, y: 0 },
        [Ok::<_, ()>(incoming)],
        1,
        1 << 21,
    )
    .unwrap();
    assert_eq!(divergent[0].ends, Terminus::Divergence);
    assert_eq!(divergent[0].feeds, None);
    assert_eq!(divergent[0].course[0].y(), 0);

    let extent = Extent::new(514, 1).unwrap();
    let mut sources = source(extent);
    sources[511].precipitation = Litres(60 * YEAR);
    let heights: Vec<_> = (0..514).rev().collect();
    let (mut r, mut f) = run(extent, &heights, sources, vec![], &[]);
    let mut budget = extract::ExtractionBudget { remaining: 10000 };
    let cross = extract::cell(
        &mut r,
        &mut f,
        CellIndex::new(511, extent).unwrap(),
        &mut budget,
    )
    .unwrap();
    let c = cross.crossings.into_iter().flatten().next().unwrap();
    assert_eq!((c.from.x, c.to.x, c.mean_discharge.raw()), (511, 512, 60));
    assert_eq!(c.annual_volume, Litres(60 * YEAR));
    let terminal = extract::cell(
        &mut r,
        &mut f,
        CellIndex::new(513, extent).unwrap(),
        &mut budget,
    )
    .unwrap();
    let point = terminal.reaches.into_iter().flatten().next().unwrap();
    assert!(point.id.is_point());
    assert_eq!(point.from, point.to);
    assert_eq!(point.receiving, ReceivingAccount::DomainExport);

    let extent = Extent::new(3, 3).unwrap();
    let at = |raw| CellIndex::new(raw, extent).unwrap();
    let mut heights = vec![10; 9];
    heights[4] = 0;
    let mut sources = source(extent);
    sources[3].precipitation = Litres(30 * YEAR);
    sources[5].precipitation = Litres(30 * YEAR);
    sources[4].marginal = Litres(60 * YEAR);
    let (mut r, mut f) = run(
        extent,
        &heights,
        sources,
        vec![Saddle {
            left: Node::Closed(at(4)),
            right: Node::Exterior,
            sill_mm: 10,
            from: at(0),
            to: None,
        }],
        &[],
    );
    let point = extract::cell(
        &mut r,
        &mut f,
        at(4),
        &mut extract::ExtractionBudget { remaining: 1000 },
    )
    .unwrap();
    assert_eq!(point.channels.iter().flatten().count(), 0);
    assert_eq!(point.reaches.iter().flatten().count(), 1);
    let row = point.reaches.iter().flatten().next().unwrap();
    assert!(row.id.is_point());
    assert_eq!(row.mean_discharge.raw(), 60);
    assert_eq!(
        row.receiving,
        ReceivingAccount::Lake(BasinId((1 << 32) | 1))
    );
    assert_eq!(point.owned.unwrap().end, extract::CellEnd::Basin);
}
