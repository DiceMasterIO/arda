use super::{hierarchy::*, routing::Extent, saddles::Saddle};
use arda_core::{
    formats::hydrology::{decode_record, encode_record, BasinNodeRow, TableSpan},
    hydrology::{BasinId, ReachId, ReceivingAccount, SpillConnection},
};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub(super) struct Directory(pub(super) PathBuf);
impl Directory {
    pub(super) fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "arda-mst-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Default)]
pub(super) struct Memory {
    unions: Vec<Option<UnionRow>>,
    nodes: Vec<Option<NodeRow>>,
    links: Vec<(BasinId, BasinId)>,
    spans: BTreeMap<BasinId, TableSpan>,
    pub(super) output: Vec<BasinNodeRow>,
    pub(super) elders: Vec<ElderLink>,
}
impl Store for Memory {
    type Error = &'static str;
    fn reserve(&mut self, u: u64, n: u64) -> Result<(), Self::Error> {
        self.unions = vec![None; usize::try_from(u).unwrap()];
        self.nodes = vec![None; usize::try_from(n).unwrap()];
        Ok(())
    }
    fn union(&mut self, at: u64) -> Result<UnionRow, Self::Error> {
        self.unions
            .get(usize::try_from(at).unwrap())
            .and_then(|&v| v)
            .ok_or("missing union")
    }
    fn put_union(&mut self, at: u64, row: UnionRow) -> Result<(), Self::Error> {
        *self
            .unions
            .get_mut(usize::try_from(at).unwrap())
            .ok_or("union index")? = Some(row);
        Ok(())
    }
    fn node(&mut self, at: u64) -> Result<NodeRow, Self::Error> {
        self.nodes
            .get(usize::try_from(at).unwrap())
            .and_then(|&v| v)
            .ok_or("missing node")
    }
    fn put_node(&mut self, at: u64, row: NodeRow) -> Result<(), Self::Error> {
        *self
            .nodes
            .get_mut(usize::try_from(at).unwrap())
            .ok_or("node index")? = Some(row);
        Ok(())
    }
    fn link(&mut self, parent: BasinId, child: BasinId) -> Result<(), Self::Error> {
        self.links.push((parent, child));
        Ok(())
    }
    fn finish_links(&mut self, count: u64) -> Result<(), Self::Error> {
        if count != u64::try_from(self.links.len()).unwrap() {
            return Err("link count");
        }
        self.links.sort_unstable_by_key(|&(p, c)| (p.0, c.0));
        for (i, &(parent, _)) in self.links.iter().enumerate() {
            self.spans
                .entry(parent)
                .or_insert(TableSpan {
                    offset: u64::try_from(i).unwrap(),
                    count: 0,
                })
                .count += 1;
        }
        Ok(())
    }
    fn child_span(&mut self, parent: BasinId) -> Result<TableSpan, Self::Error> {
        Ok(self.spans.get(&parent).copied().unwrap_or_default())
    }
    fn emit(&mut self, row: BasinNodeRow) -> Result<(), Self::Error> {
        let bytes = encode_record(&row).unwrap();
        assert_eq!(decode_record::<BasinNodeRow>(&bytes).unwrap(), row);
        self.output.push(row);
        Ok(())
    }
    fn emit_elder(&mut self, row: ElderLink) -> Result<(), Self::Error> {
        self.elders.push(row);
        Ok(())
    }
}
pub(super) fn limits() -> Limits {
    Limits {
        leaves: 10_000,
        store_operations: 10_000_000,
    }
}
// Explicit test-only resolver; production receiving ownership remains root-owned.
pub(super) fn resolver(w: Witness) -> Result<BoundSpill, &'static str> {
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
pub(super) fn run(extent: Extent, minima: &[Minimum], edges: &[Saddle]) -> (Work, Memory) {
    let mut store = Memory::default();
    let work = build(
        extent,
        u64::try_from(minima.len()).unwrap(),
        minima.iter().copied().map(Ok),
        edges.iter().copied().map(Ok),
        &mut store,
        limits(),
        resolver,
    )
    .unwrap();
    (work, store)
}
