//! Exact tiny controls using real physical D8 routing and the planned memory backend.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]
use crate::hydrology::{
    fine_flow::{
        run, run_with_source, CellSource, DirectedFlow, FlowError, FlowLimits, FlowStore,
        FlowTarget,
    },
    memory_flow::MemoryFlowStore,
    routing::{self, CellIndex, Extent, MemoryPages, RoutingStore},
    saddles::{Node, Saddle},
};
use arda_core::hydrology::{BasinId, Litres};
fn fixture(split: bool) -> (MemoryPages, Vec<CellSource>, Vec<Saddle>) {
    let extent = Extent::new(6, 3).unwrap();
    let at = |raw| CellIndex::new(raw, extent).unwrap();
    let mut heights = vec![20; 18];
    heights[7] = 0;
    heights[8] = 5;
    heights[9] = 5;
    heights[10] = 0;
    heights[11] = 10;
    let mut terrain = MemoryPages::new(extent, &heights, &[false; 18], 1 << 20).unwrap();
    routing::route_and_own(&mut terrain, routing::Limits::for_extent(extent)).unwrap();
    let mut sources: Vec<_> = (0..18)
        .map(|raw| CellSource {
            at: at(raw),
            precipitation: Litres(0),
            land_loss: Litres(0),
            evaporation: Litres(0),
            marginal: Litres(0),
            wet: None,
        })
        .collect();
    sources[7].wet = Some((BasinId((1 << 32) | 1), if split { 4 } else { 5 }));
    sources[10].wet = Some((BasinId((1 << 32) | 4), if split { 5 } else { 4 }));
    sources[8].precipitation = Litres(2);
    sources[9].precipitation = Litres(if split { 103 } else { 3 });
    if split {
        sources[7].evaporation = Litres(40);
        sources[10].evaporation = Litres(65);
    } else {
        sources[7].precipitation = Litres(100);
        sources[10].evaporation = Litres(100);
        sources[10].marginal = Litres(5);
    }
    let edges = vec![
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
    (terrain, sources, edges)
}
fn cap() -> FlowLimits {
    FlowLimits {
        operations: 100_000,
        absolute_litres: 1000,
        accepted_edges: 2,
    }
}
fn key(flow: &DirectedFlow) -> (u32, u32, u128) {
    let FlowTarget::Cell(to) = flow.to else {
        panic!("unexpected boundary flux")
    };
    (flow.from.x, to.x, flow.annual.0)
}
#[test]
fn actual_receiver_forest_reverses_sill_arm_and_keeps_a_real_dry_split() {
    for split in [false, true] {
        let (mut terrain, sources, edges) = fixture(split);
        let extent = Extent::new(6, 3).unwrap();
        let mut store = MemoryFlowStore::new(extent, 1 << 16).unwrap();
        let mut output = Vec::new();
        let work = run(
            &mut terrain,
            &mut store,
            sources.into_iter().map(Ok::<_, ()>),
            edges.into_iter().map(Ok),
            cap(),
            |v| {
                output.push(v);
                Ok(())
            },
        )
        .unwrap();
        let mut got: Vec<_> = output.iter().map(key).collect();
        got.sort_unstable();
        assert_eq!(
            got,
            if split {
                vec![(2, 1, 40), (3, 2, 38), (3, 4, 65)]
            } else {
                vec![(1, 2, 100), (2, 3, 102), (3, 4, 105)]
            }
        );
        assert_eq!(work.cells, 18);
        assert_eq!(work.exterior_litres, 0);
        assert_eq!(work.emitted, 3);
        for (raw, expected) in [(7, 0), (8, 2), (9, if split { 103 } else { 3 }), (10, 0)] {
            let row = store.read(CellIndex::new(raw, extent).unwrap()).unwrap();
            assert_eq!(row.metrics.scalar_annual, expected);
            assert_eq!(row.metrics.hand_distance_mm, u64::MAX);
            assert_eq!(row.metrics.hand_at, None);
            assert_eq!(row.metrics.order, 0);
        }
        let (mut terrain, sources, edges) = fixture(split);
        let mut store = MemoryFlowStore::new(extent, 1 << 16).unwrap();
        let mut short = cap();
        short.operations = work.operations - 1;
        assert!(matches!(
            run(
                &mut terrain,
                &mut store,
                sources.into_iter().map(Ok::<_, ()>),
                edges.into_iter().map(Ok),
                short,
                |_| Ok(())
            ),
            Err(FlowError::Limit)
        ));
    }
    let extent = Extent::new(6, 3).unwrap();
    let (mut terrain, mut sources, edges) = fixture(false);
    for raw in [7, 8, 9, 10] {
        sources[raw].wet = Some((BasinId(9), 6));
    }
    let mut store = MemoryFlowStore::new(extent, 1 << 16).unwrap();
    let work = run(
        &mut terrain,
        &mut store,
        sources.into_iter().map(Ok::<_, ()>),
        edges.into_iter().map(Ok),
        cap(),
        |_| panic!("same-lake internal edge visible"),
    )
    .unwrap();
    assert_eq!(work.emitted, 0);
    for fault in 0..4 {
        let (mut terrain, mut sources, edges) = fixture(false);
        let mut store = MemoryFlowStore::new(extent, 1 << 16).unwrap();
        match fault {
            0 => sources[7].wet.as_mut().unwrap().1 = 4,
            1 => sources[10].evaporation.0 += 1,
            2 => sources[8].marginal = Litres(1),
            _ => sources.swap(0, 1),
        }
        let error = run(
            &mut terrain,
            &mut store,
            sources.into_iter().map(Ok::<_, ()>),
            edges.into_iter().map(Ok),
            cap(),
            |_| Ok(()),
        )
        .unwrap_err();
        assert!(
            matches!(error,FlowError::Invalid(message) if message==match fault{0=>"unsupported uphill visible flow",1=>"external import",2=>"nonterminal marginal debit",_=>"source order/count"})
        );
    }
    let (mut terrain, sources, edges) = fixture(false);
    let mut store = MemoryFlowStore::new(extent, 1 << 16).unwrap();
    assert!(matches!(
        run(
            &mut terrain,
            &mut store,
            sources.into_iter().map(Ok),
            edges.into_iter().map(Ok),
            cap(),
            |_| Err("output")
        ),
        Err(FlowError::Stream("output"))
    ));
    let sea = Extent::new(1, 1).unwrap();
    let at = CellIndex::new(0, sea).unwrap();
    let mut terrain = MemoryPages::new(sea, &[0], &[true], 1 << 16).unwrap();
    routing::route_and_own(&mut terrain, routing::Limits::for_extent(sea)).unwrap();
    let mut store = MemoryFlowStore::new(sea, 1 << 16).unwrap();
    let source = CellSource {
        at,
        precipitation: Litres(0),
        land_loss: Litres(0),
        evaporation: Litres(0),
        marginal: Litres(0),
        wet: None,
    };
    let work = run(
        &mut terrain,
        &mut store,
        [Ok::<_, ()>(source)],
        [],
        FlowLimits {
            accepted_edges: 0,
            ..cap()
        },
        |_| Ok(()),
    )
    .unwrap();
    assert_eq!(work.cells, 0);
}

#[test]
fn source_callback_borrows_the_same_routing_store_and_iterator_bounds_remain_exact() {
    let extent = Extent::new(6, 3).unwrap();
    let (mut terrain, sources, edges) = fixture(false);
    let mut store = MemoryFlowStore::new(extent, 1 << 16).unwrap();
    let mut calls = 0;
    let mut output = Vec::new();
    run_with_source(
        &mut terrain,
        &mut store,
        |routing, at| {
            let row = routing.read(at).map_err(|_| "routing")?;
            assert!(row.owner().is_some());
            calls += 1;
            Ok::<_, &str>(sources[at.raw() as usize])
        },
        edges.into_iter().map(Ok),
        cap(),
        |edge| {
            output.push(edge);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(calls, 18);
    let mut got: Vec<_> = output.iter().map(key).collect();
    got.sort_unstable();
    assert_eq!(got, vec![(1, 2, 100), (2, 3, 102), (3, 4, 105)]);
    for extra in [false, true] {
        let (mut terrain, mut sources, edges) = fixture(false);
        if extra {
            sources.push(sources[0]);
        } else {
            sources.pop();
        }
        let mut store = MemoryFlowStore::new(extent, 1 << 16).unwrap();
        assert!(matches!(
            run(
                &mut terrain,
                &mut store,
                sources.into_iter().map(Ok::<_, ()>),
                edges.into_iter().map(Ok),
                cap(),
                |_| Ok(())
            ),
            Err(FlowError::Invalid(_))
        ));
    }
    let (mut terrain, _, edges) = fixture(false);
    let mut store = MemoryFlowStore::new(extent, 1 << 16).unwrap();
    assert!(matches!(
        run_with_source(
            &mut terrain,
            &mut store,
            |_, _| Err("source"),
            edges.into_iter().map(Ok),
            cap(),
            |_| Ok(())
        ),
        Err(FlowError::Stream("source"))
    ));
}
