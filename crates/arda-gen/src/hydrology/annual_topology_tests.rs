//! Hand-calculated retained-hierarchy controls; no natural terrain runs.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]
use crate::hydrology::{
    annual_topology::{adapt, required_ram, TopologyError, TopologyLimits},
    routing::{CellIndex, Extent},
    saddles::{Node, Saddle},
    witness_binding::BreakpointRegistry,
};
use arda_core::{
    formats::hydrology::{BasinNodeRow, TableSpan},
    hydrology::{BasinId, ReceivingAccount, SpillConnection},
    GlobalCell, HeightMm,
};
const I: u64 = 1 << 63;
fn leaf(x: u32) -> BasinId {
    BasinId((1_u64 << 32) | u64::from(x))
}
fn spill(height: i32) -> SpillConnection {
    SpillConnection {
        from: GlobalCell { x: 6, y: 1 },
        to: None,
        sill: HeightMm::new(height),
        receiving: ReceivingAccount::DomainExport,
    }
}
fn row(
    id: BasinId,
    parent: Option<BasinId>,
    floor: i32,
    children: TableSpan,
    height: i32,
) -> BasinNodeRow {
    BasinNodeRow {
        id,
        parent,
        anchor: GlobalCell {
            x: if id.0 < I {
                u32::try_from(id.0 & 0xffff_ffff).unwrap()
            } else {
                1
            },
            y: 1,
        },
        floor: HeightMm::new(floor),
        children,
        spill: Some(spill(height)),
    }
}
fn fixture(kind: u8) -> (Extent, Vec<BasinNodeRow>, Vec<BasinId>, Vec<Saddle>) {
    let e = Extent::new(7, 3).unwrap();
    let at = |x: u32| CellIndex::new(7 + x, e).unwrap();
    let a = leaf(1);
    let b = leaf(3);
    let c = leaf(5);
    let p = BasinId(I);
    let q = BasinId(I + 1);
    let empty = TableSpan::default();
    let (rows, children) = match kind {
        0 => (
            vec![
                row(a, Some(p), 0, empty, 5),
                row(b, Some(p), 1, empty, 5),
                row(c, Some(q), 2, empty, 10),
                row(
                    p,
                    Some(q),
                    0,
                    TableSpan {
                        offset: 0,
                        count: 2,
                    },
                    10,
                ),
                row(
                    q,
                    None,
                    0,
                    TableSpan {
                        offset: 2,
                        count: 2,
                    },
                    12,
                ),
            ],
            vec![a, b, c, p],
        ),
        1 => (
            vec![
                row(a, Some(p), 0, empty, 5),
                row(b, Some(p), 1, empty, 5),
                row(c, Some(p), 2, empty, 5),
                row(
                    p,
                    None,
                    0,
                    TableSpan {
                        offset: 0,
                        count: 3,
                    },
                    12,
                ),
            ],
            vec![a, b, c],
        ),
        _ => (
            vec![
                row(a, None, 0, empty, 5),
                row(b, None, 1, empty, 5),
                row(c, None, 2, empty, 5),
            ],
            vec![],
        ),
    };
    let edges = vec![
        Saddle {
            left: Node::Closed(at(1)),
            right: Node::Closed(at(3)),
            sill_mm: 5,
            from: at(2),
            to: Some(at(3)),
        },
        Saddle {
            left: Node::Closed(at(3)),
            right: Node::Closed(at(5)),
            sill_mm: if kind == 0 { 10 } else { 5 },
            from: at(4),
            to: Some(at(5)),
        },
        Saddle {
            left: Node::Closed(at(5)),
            right: Node::Exterior,
            sill_mm: if kind == 2 { 5 } else { 12 },
            from: at(6),
            to: None,
        },
    ];
    (e, rows, children, edges)
}
fn limits(n: usize) -> TopologyLimits {
    TopologyLimits {
        nodes: n as u64,
        leaves: 3,
        ram_bytes: required_ram(n as u64).unwrap(),
        operations: 10000,
    }
}
#[test]
fn nested_tied_and_exterior_joins_keep_real_children_and_leaf_owners() {
    let registry = BreakpointRegistry::new::<()>(0, 4096).unwrap();
    for kind in 0..3 {
        let (e, rows, children, edges) = fixture(kind);
        let cap = limits(rows.len());
        let output = adapt(
            e,
            rows.clone().into_iter().map(Ok::<_, ()>),
            &children,
            edges.clone().into_iter().map(Ok),
            &registry,
            cap,
        )
        .unwrap();
        assert!(output.nodes.iter().all(|n| n.bands.is_empty()
            && n.local_runoff.0 == 0
            && n.local_precipitation.0 == 0
            && n.local_land_loss.0 == 0));
        let got: Vec<_> = output
            .joins
            .iter()
            .map(|j| {
                (
                    j.parent.0,
                    j.left_child.0,
                    j.right_child.0,
                    j.left_leaf.0,
                    j.right_leaf.0,
                    j.witness,
                )
            })
            .collect();
        let a = leaf(1).0;
        let b = leaf(3).0;
        let c = leaf(5).0;
        assert_eq!(
            got,
            match kind {
                0 => vec![(I, a, b, a, b, 0), (I + 1, I, c, b, c, 1)],
                1 => vec![(I, a, b, a, b, 0), (I, b, c, b, c, 1)],
                _ => vec![],
            }
        );
        assert_eq!(
            output
                .nodes
                .iter()
                .map(|n| n.birth.raw())
                .collect::<Vec<_>>(),
            match kind {
                0 => vec![0, 1, 2, 5, 10],
                1 => vec![0, 1, 2, 5],
                _ => vec![0, 1, 2],
            }
        );
        let mut short = cap;
        short.operations = output.operations - 1;
        assert!(matches!(
            adapt(
                e,
                rows.clone().into_iter().map(Ok::<_, ()>),
                &children,
                edges.clone().into_iter().map(Ok),
                &registry,
                short
            ),
            Err(TopologyError::Limit)
        ));
        let mut exact = cap;
        exact.operations = output.operations;
        assert!(adapt(
            e,
            rows.clone().into_iter().map(Ok::<_, ()>),
            &children,
            edges.clone().into_iter().map(Ok),
            &registry,
            exact
        )
        .is_ok());
        let mut ram = cap;
        ram.ram_bytes -= 1;
        assert!(matches!(
            adapt(
                e,
                rows.clone().into_iter().map(Ok::<_, ()>),
                &children,
                edges.clone().into_iter().map(Ok),
                &registry,
                ram
            ),
            Err(TopologyError::Limit)
        ));
        let mut wrong = edges.clone();
        wrong[0].sill_mm = 4;
        assert!(adapt(
            e,
            rows.clone().into_iter().map(Ok::<_, ()>),
            &children,
            wrong.into_iter().map(Ok),
            &registry,
            cap
        )
        .is_err());
        assert!(adapt(
            e,
            rows.clone().into_iter().map(Ok::<_, ()>),
            &children,
            edges[..2].iter().copied().map(Ok),
            &registry,
            cap
        )
        .is_err());
        if kind != 2 {
            let mut bad = rows.clone();
            bad[0].parent = None;
            assert!(adapt(
                e,
                bad.into_iter().map(Ok::<_, ()>),
                &children,
                edges.into_iter().map(Ok),
                &registry,
                cap
            )
            .is_err());
        }
    }
    let (e, rows, children, edges) = fixture(0);
    assert!(matches!(
        adapt(
            e,
            rows.clone()
                .into_iter()
                .map(|_| Err::<BasinNodeRow, _>("reader")),
            &children,
            edges.into_iter().map(Ok),
            &registry,
            limits(rows.len())
        ),
        Err(TopologyError::Input("reader"))
    ));
    let empty = adapt(
        Extent::new(1, 1).unwrap(),
        std::iter::empty::<Result<BasinNodeRow, ()>>(),
        &[],
        std::iter::empty::<Result<Saddle, ()>>(),
        &registry,
        TopologyLimits {
            nodes: 0,
            leaves: 0,
            ram_bytes: 4096,
            operations: 10,
        },
    )
    .unwrap();
    assert!(empty.nodes.is_empty() && empty.joins.is_empty());
}
