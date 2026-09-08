use crate::hydrology::hierarchy::*;
use crate::hydrology::routing::{CellIndex, Extent};
use crate::hydrology::saddles::{Node, Saddle};
use arda_core::{
    formats::hydrology::{decode_record, encode_record, BasinNodeRow, TableSpan},
    hydrology::{BasinId, ReachId, ReceivingAccount, SpillConnection},
    GlobalCell,
};
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct Memory {
    pub(crate) unions: Vec<Option<UnionRow>>,
    pub(crate) nodes: Vec<Option<NodeRow>>,
    links: Vec<(BasinId, BasinId)>,
    child_ids: Vec<BasinId>,
    spans: BTreeMap<BasinId, TableSpan>,
    pub(crate) output: Vec<BasinNodeRow>,
    pub(crate) elders: Vec<ElderLink>,
    fail_after: Option<usize>,
    calls: usize,
    reserved: bool,
    corrupt_span: bool,
}
impl Memory {
    fn call(&mut self) -> Result<(), &'static str> {
        self.calls += 1;
        if self.fail_after == Some(self.calls) {
            Err("injected I/O failure")
        } else {
            Ok(())
        }
    }
}
impl Store for Memory {
    type Error = &'static str;
    fn reserve(&mut self, u: u64, n: u64) -> Result<(), Self::Error> {
        self.call()?;
        self.reserved = true;
        self.unions = vec![None; usize::try_from(u).unwrap()];
        self.nodes = vec![None; usize::try_from(n).unwrap()];
        Ok(())
    }
    fn union(&mut self, i: u64) -> Result<UnionRow, Self::Error> {
        self.call()?;
        self.unions
            .get(usize::try_from(i).unwrap())
            .and_then(|x| *x)
            .ok_or("missing union")
    }
    fn put_union(&mut self, i: u64, r: UnionRow) -> Result<(), Self::Error> {
        self.call()?;
        *self
            .unions
            .get_mut(usize::try_from(i).unwrap())
            .ok_or("union outside reservation")? = Some(r);
        Ok(())
    }
    fn node(&mut self, i: u64) -> Result<NodeRow, Self::Error> {
        self.call()?;
        self.nodes
            .get(usize::try_from(i).unwrap())
            .and_then(|x| *x)
            .ok_or("missing node")
    }
    fn put_node(&mut self, i: u64, r: NodeRow) -> Result<(), Self::Error> {
        self.call()?;
        *self
            .nodes
            .get_mut(usize::try_from(i).unwrap())
            .ok_or("node outside reservation")? = Some(r);
        Ok(())
    }
    fn link(&mut self, p: BasinId, c: BasinId) -> Result<(), Self::Error> {
        self.call()?;
        self.links.push((p, c));
        Ok(())
    }
    fn finish_links(&mut self, n: u64) -> Result<(), Self::Error> {
        self.call()?;
        if n != u64::try_from(self.links.len()).unwrap() {
            return Err("link count");
        }
        self.links.sort_unstable_by_key(|(p, c)| (p.0, c.0));
        for &(p, c) in &self.links {
            let i = u64::try_from(self.child_ids.len()).unwrap();
            let s = self.spans.entry(p).or_insert(TableSpan {
                offset: i,
                count: 0,
            });
            s.count += 1;
            self.child_ids.push(c);
        }
        Ok(())
    }
    fn child_span(&mut self, p: BasinId) -> Result<TableSpan, Self::Error> {
        self.call()?;
        if self.corrupt_span {
            return Ok(TableSpan {
                offset: u64::MAX,
                count: 2,
            });
        }
        Ok(self.spans.get(&p).copied().unwrap_or_default())
    }
    fn emit_elder(&mut self, row: ElderLink) -> Result<(), Self::Error> {
        self.call()?;
        self.elders.push(row);
        Ok(())
    }
    fn emit(&mut self, row: BasinNodeRow) -> Result<(), Self::Error> {
        self.call()?;
        let bytes = encode_record(&row).unwrap();
        let decoded = decode_record::<BasinNodeRow>(&bytes).unwrap();
        assert_eq!(decoded, row);
        self.output.push(row);
        Ok(())
    }
}
pub(crate) fn limits() -> Limits {
    Limits {
        leaves: 10_000,
        store_operations: 10_000_000,
    }
}
pub(crate) fn at(e: Extent, x: u32, y: u32) -> CellIndex {
    let last = e.coordinates(CellIndex::new(e.cells() - 1, e).unwrap());
    CellIndex::new(y * (last.0 + 1) + x, e).unwrap()
}
pub(crate) fn m(e: Extent, x: u32, floor: i32) -> Minimum {
    Minimum {
        at: at(e, x, 1),
        floor_mm: floor,
    }
}
pub(crate) fn between(e: Extent, left: Minimum, right: Minimum, h: i32) -> Saddle {
    Saddle {
        left: Node::Closed(left.at),
        right: Node::Closed(right.at),
        sill_mm: h,
        from: left.at,
        to: Some(at(e, e.coordinates(left.at).0 + 1, 1)),
    }
}
pub(crate) fn outside(e: Extent, left: Minimum, h: i32) -> Saddle {
    let x = e.coordinates(left.at).0;
    Saddle {
        left: Node::Closed(left.at),
        right: Node::Exterior,
        sill_mm: h,
        from: at(e, x, 0),
        to: None,
    }
}
pub(crate) fn resolver(w: Witness) -> Result<BoundSpill, &'static str> {
    Ok(BoundSpill {
        source_terminal: w.source_anchor,
        target_terminal: None,
        spill: SpillConnection {
            from: w.from,
            to: w.to,
            sill: w.sill,
            receiving: if let Some(c) = w.to {
                ReceivingAccount::Reach(ReachId((u64::from(c.y) << 32) | u64::from(c.x)))
            } else {
                ReceivingAccount::DomainExport
            },
        },
    })
}
pub(crate) fn run(e: Extent, ms: &[Minimum], es: &[Saddle]) -> (Work, Memory) {
    let mut s = Memory::default();
    let w = build(
        e,
        u64::try_from(ms.len()).unwrap(),
        ms.iter().copied().map(Ok),
        es.iter().copied().map(Ok),
        &mut s,
        limits(),
        resolver,
    )
    .unwrap();
    (w, s)
}
#[test]
fn nested_physical_nodes_have_fixed_rows_and_streamed_sorted_children() {
    let e = Extent::new(8, 3).unwrap();
    let ms = [m(e, 1, 0), m(e, 3, 2), m(e, 5, 3)];
    let es = [
        between(e, ms[0], ms[1], 5),
        between(e, ms[1], ms[2], 8),
        outside(e, ms[2], 12),
    ];
    let (w, s) = run(e, &ms, &es);
    assert_eq!((w.nodes, w.links, w.contracted), (5, 4, 0));
    let p = &s.output[3];
    assert_eq!(
        p.children,
        TableSpan {
            offset: 0,
            count: 2
        }
    );
    assert_eq!(
        s.child_ids[..2],
        [
            BasinId(e.anchor_key(ms[0].at)),
            BasinId(e.anchor_key(ms[1].at))
        ]
    );
    assert_eq!(p.parent, Some(s.output[4].id));
    assert_eq!(s.output[4].children.count, 2);
    assert_eq!(s.output[4].parent, None);
}
#[test]
fn a_closed_equal_height_event_has_one_parent_and_no_binary_join_chain() {
    let e = Extent::new(8, 3).unwrap();
    let ms = [m(e, 1, 0), m(e, 3, 1), m(e, 5, 2)];
    let es = [
        between(e, ms[0], ms[1], 5),
        between(e, ms[1], ms[2], 5),
        outside(e, ms[2], 9),
    ];
    let (w, s) = run(e, &ms, &es);
    assert_eq!((w.nodes, w.links, w.contracted), (4, 3, 1));
    assert_eq!(s.output[3].children.count, 3);
    assert!(s.output[..3]
        .iter()
        .all(|n| n.parent == Some(s.output[3].id)));
    // Removed AB's outward B->C witness replaces B->A; the group still has its true spanning boundary.
    assert_eq!(s.output[1].spill.unwrap().from, GlobalCell { x: 3, y: 1 });
}
#[test]
fn equal_height_exterior_event_keeps_actual_exit_and_has_no_mutual_root_flow() {
    let e = Extent::new(6, 3).unwrap();
    let ms = [m(e, 1, 0), m(e, 3, 1)];
    let es = [between(e, ms[0], ms[1], 5), outside(e, ms[1], 5)];
    let (w, s) = run(e, &ms, &es);
    assert_eq!((w.nodes, w.links, w.contracted), (2, 0, 1));
    assert!(s.output.iter().all(|n| n.parent.is_none()));
    assert_eq!(
        s.output[0].spill.unwrap().to,
        Some(GlobalCell { x: 2, y: 1 })
    );
    assert_eq!(s.output[1].spill.unwrap().from, GlobalCell { x: 3, y: 0 });
    assert_eq!(s.output[1].spill.unwrap().to, None);
    assert_eq!(
        s.output[1].spill.unwrap().receiving,
        ReceivingAccount::DomainExport
    );
}
#[test]
fn giant_tied_event_streams_children_without_event_vector() {
    let e = Extent::new(4096, 3).unwrap();
    let ms: Vec<_> = (0..1024).map(|i| m(e, 2 * i + 1, 0)).collect();
    let mut es: Vec<_> = ms.windows(2).map(|w| between(e, w[0], w[1], 5)).collect();
    es.push(outside(e, *ms.last().unwrap(), 10));
    let (w, s) = run(e, &ms, &es);
    assert_eq!((w.nodes, w.links, w.contracted), (1025, 1024, 1022));
    assert_eq!(s.output.last().unwrap().children.count, 1024);
    assert!(w.store_operations < 250_000);
}
#[test]
fn identities_and_public_bytes_ignore_temporary_domain_width() {
    let mut outputs = Vec::new();
    for width in [8, 12] {
        let e = Extent::new(width, 3).unwrap();
        let ms = [m(e, 1, -100), m(e, 3, -90)];
        let es = [between(e, ms[0], ms[1], -80), outside(e, ms[1], 10)];
        outputs.push(run(e, &ms, &es).1.output);
    }
    assert_eq!(outputs[0], outputs[1]);
    assert_eq!(outputs[0][2].id, BasinId(1 << 63));
}
#[test]
fn malformed_streams_and_zero_depth_terminals_fail() {
    let e = Extent::new(6, 3).unwrap();
    let ms = [m(e, 1, 0), m(e, 3, 1)];
    let a = between(e, ms[0], ms[1], 5);
    let b = outside(e, ms[1], 10);
    for es in [vec![b, a], vec![a, a], vec![a], vec![a, b, b]] {
        assert!(build(
            e,
            2,
            ms.into_iter().map(Ok),
            es.into_iter().map(Ok),
            &mut Memory::default(),
            limits(),
            resolver
        )
        .is_err());
    }
    assert!(build(
        e,
        1,
        [Ok(ms[0])],
        [Ok(outside(e, ms[0], 0))],
        &mut Memory::default(),
        limits(),
        resolver
    )
    .is_err());
    assert!(build(
        e,
        2,
        [Ok(ms[1]), Ok(ms[0])],
        [Ok(a), Ok(b)],
        &mut Memory::default(),
        limits(),
        resolver
    )
    .is_err());
}
#[test]
fn witness_resolver_cannot_change_geometry_or_fake_marine_export() {
    let e = Extent::new(6, 3).unwrap();
    let leaf = m(e, 1, 0);
    let edge = outside(e, leaf, 5);
    for change in [0, 1, 2] {
        let result = build(
            e,
            1,
            [Ok(leaf)],
            [Ok(edge)],
            &mut Memory::default(),
            limits(),
            |w| {
                let mut s = resolver(w)?;
                match change {
                    0 => s.spill.from.x += 1,
                    1 => s.spill.receiving = ReceivingAccount::Sea,
                    _ => s.spill.receiving = ReceivingAccount::Lake(w.source),
                }
                Ok(s)
            },
        );
        assert!(result.is_err());
    }
}
#[test]
fn admission_and_backend_errors_never_silently_publish_success() {
    let e = Extent::new(6, 3).unwrap();
    let leaf = m(e, 1, 0);
    let edge = outside(e, leaf, 5);
    let mut s = Memory::default();
    assert!(build(
        e,
        1,
        [Ok(leaf)],
        [Ok(edge)],
        &mut s,
        Limits {
            leaves: 0,
            store_operations: 10
        },
        resolver
    )
    .is_err());
    assert!(!s.reserved);
    let (w, baseline) = run(e, &[leaf], &[edge]);
    for count in 1..=baseline.calls {
        let mut s = Memory {
            fail_after: Some(count),
            ..Default::default()
        };
        assert!(build(e, 1, [Ok(leaf)], [Ok(edge)], &mut s, limits(), resolver).is_err());
    }
    let mut s = Memory::default();
    assert!(build(
        e,
        1,
        [Ok(leaf)],
        [Ok(edge)],
        &mut s,
        Limits {
            leaves: 1,
            store_operations: w.store_operations - 1
        },
        resolver
    )
    .is_err());
    let mut s = Memory {
        corrupt_span: true,
        ..Default::default()
    };
    assert!(build(e, 1, [Ok(leaf)], [Ok(edge)], &mut s, limits(), resolver).is_err());
}
#[test]
fn empty_closed_domain_has_empty_public_tables() {
    let e = Extent::new(2, 2).unwrap();
    let (w, s) = run(e, &[], &[]);
    assert_eq!((w.nodes, w.links), (0, 0));
    assert!(s.output.is_empty());
}

#[test]
fn same_tied_event_preserves_distinct_containment_and_elder_transfer_roles() {
    let e = Extent::new(8, 3).unwrap();
    let ms = [m(e, 1, 0), m(e, 3, 1), m(e, 5, 2)];
    let es = [
        between(e, ms[0], ms[1], 5),
        between(e, ms[1], ms[2], 5),
        outside(e, ms[2], 9),
    ];
    let (_, s) = run(e, &ms, &es);
    assert_eq!(s.output.len(), 4);
    assert_eq!(s.elders.len(), 3);
    let a = s.elders[0];
    let b = s.elders[1];
    let c = s.elders[2];
    assert_eq!(a.elder_parent, None);
    assert_eq!(a.spill.sill.raw(), 9);
    assert_eq!(b.elder_parent, Some(a.leaf));
    assert_eq!(c.elder_parent, Some(a.leaf));
    assert_eq!(b.spill.sill.raw(), 5);
    assert_eq!(c.spill.sill.raw(), 5);
    assert_eq!(c.spill.to, Some(GlobalCell { x: 3, y: 1 }));
    // Actual target is B's side, while family elder is A. The callback result is not overwritten.
    assert_eq!(
        c.spill.receiving,
        ReceivingAccount::Reach(ReachId((1_u64 << 32) | 3))
    );
    assert_eq!(s.output[0].spill.unwrap().sill.raw(), 5);
    assert_eq!(s.output[0].parent, Some(s.output[3].id));
}

#[test]
fn contained_join_levels_equal_independent_path_maxima_across_tied_events() {
    let e = Extent::new(40, 3).unwrap();
    let ms: Vec<_> = (0..16).map(|i| m(e, 2 * i + 1, 0)).collect();
    for seed in 0..24_u32 {
        let heights: Vec<_> = (0..15_u32)
            .map(|i| 1 + i32::try_from((i * 17 + seed * 13 + i * i * (seed + 3)) % 9).unwrap())
            .collect();
        let mut es: Vec<_> = ms
            .windows(2)
            .zip(&heights)
            .map(|(w, &h)| between(e, w[0], w[1], h))
            .collect();
        es.sort_unstable_by_key(|s| (s.sill_mm, s.left, s.right, s.from, s.to));
        es.push(outside(e, *ms.last().unwrap(), 12));
        let (_, s) = run(e, &ms, &es);
        let rows: BTreeMap<_, _> = s.output.iter().map(|n| (n.id.0, n)).collect();
        for a in 0..ms.len() {
            for b in a + 1..ms.len() {
                let mut ancestry = Vec::new();
                let mut current = Some(BasinId(e.anchor_key(ms[a].at)));
                while let Some(id) = current {
                    ancestry.push(id);
                    current = rows[&id.0].parent;
                }
                let mut other = BasinId(e.anchor_key(ms[b].at));
                while !ancestry.contains(&other) {
                    other = rows[&other.0].parent.unwrap();
                }
                let parent = rows[&other.0];
                let child = s.child_ids[usize::try_from(parent.children.offset).unwrap()];
                let actual = rows[&child.0].spill.unwrap().sill.raw();
                assert_eq!(actual, *heights[a..b].iter().max().unwrap());
            }
        }
        // Same minimum floors use the immutable physical anchor as the elder tie.
        assert!(s
            .elders
            .iter()
            .all(|r| r.elder_parent.is_none_or(|p| p.0 < r.leaf.0)));
    }
}

#[test]
fn tied_external_chain_overrides_only_the_true_outward_source() {
    let e = Extent::new(8, 3).unwrap();
    let ms = [m(e, 1, i32::MIN), m(e, 3, -10), m(e, 5, -5)];
    let es = [
        between(e, ms[0], ms[1], 5),
        between(e, ms[1], ms[2], 5),
        outside(e, ms[2], 5),
    ];
    let (w, s) = run(e, &ms, &es);
    assert_eq!((w.nodes, w.links, w.contracted), (3, 0, 2));
    assert_eq!(
        s.output[0].spill.unwrap().to,
        Some(GlobalCell { x: 2, y: 1 })
    );
    assert_eq!(
        s.output[1].spill.unwrap().to,
        Some(GlobalCell { x: 4, y: 1 })
    );
    assert_eq!(s.output[2].spill.unwrap().to, None);
    assert_eq!(s.elders[0].elder_parent, None);
    assert_eq!(s.elders[0].spill.from, GlobalCell { x: 5, y: 0 });
}
