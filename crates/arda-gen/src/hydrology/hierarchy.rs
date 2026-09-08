//! Canonical physical hierarchy from an accepted, sorted witnessed MST.
//!
//! Temporary binary joins are private fixed rows. Equal-height joins are
//! contracted before any public node is emitted; their outward witness is
//! transferred to the component that existed before the event. No event or
//! root-child vector is retained. The caller supplies paged scratch and sorting.
use super::routing::{CellIndex, Extent};
use super::saddles::{Node, Saddle};
use arda_core::formats::hydrology::{BasinNodeRow, TableSpan};
use arda_core::hydrology::{BasinId, ReceivingAccount, SpillConnection};
use arda_core::{GlobalCell, HeightMm};

const INTERNAL: u64 = 1 << 63;

/// A final closed receiver terminal, ordered by its stable packed (y,x) anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Minimum {
    /// Temporary ordinal in the accepted routing extent.
    pub at: CellIndex,
    /// Immutable physical floor, without epsilon or rank.
    pub floor_mm: i32,
}

/// Exact oriented input to the fallible authority/receiving-account resolver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Witness {
    /// Retained source component or dying elder leaf identity, never a temporary join ordinal.
    pub source: BasinId,
    /// Immutable descendant anchor of the pre-spill source component.
    pub source_anchor: GlobalCell,
    /// Actual source-side physical cell.
    pub from: GlobalCell,
    /// Actual adjacent cell, absent only at an actual outer-rim export.
    pub to: Option<GlobalCell>,
    /// Physical sill elevation.
    pub sill: HeightMm,
}

/// Audited routing facts returned by the real indexed witness resolver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundSpill {
    /// Actual adjacent physical spill and immediate receiving identity.
    pub spill: SpillConnection,
    /// Actual closed terminal owning the source endpoint.
    pub source_terminal: GlobalCell,
    /// Actual target owner when closed; None for eventual sea/domain export.
    pub target_terminal: Option<GlobalCell>,
}

/// Fixed logical union row. A disk adapter must encode fields explicitly.
#[derive(Debug, Clone, Copy)]
pub struct UnionRow {
    /// Immutable minimum; absent only in the last, topology-only Exterior row.
    pub minimum: Option<Minimum>,
    /// Union forest parent; roots point to themselves.
    pub parent: u64,
    /// Union-by-rank bound; no recursive traversal or path compression is used.
    pub rank: u8,
    /// Height at which this row ceased being a root.
    pub joined_at: Option<i32>,
    /// Current root's closed component; None means connected to Exterior.
    pub component: Option<u64>,
    /// Last event for which a pre-event component was pinned.
    pub event: Option<i32>,
    /// Root's component immediately before that event.
    pub event_base: Option<u64>,
    /// Current component's elder minimum ordinal; Exterior is the last ordinal.
    pub elder: u64,
    /// Exclusive leaf's elder-family death, distinct from its containment join.
    pub elder_death: Option<ElderDeath>,
}

/// Private leaf-family row retained at that immutable leaf's union ordinal.
#[derive(Debug, Clone, Copy)]
pub struct ElderDeath {
    /// Surviving elder leaf; absent only when joining Exterior.
    pub parent: Option<BasinId>,
    /// Exact witness oriented away from the dying elder component.
    pub spill: PendingSpill,
}

/// Separate temporal physical-leaf authority, not a merged-component store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElderLink {
    /// Immutable physical leaf, using the packed minimum coordinate identity.
    pub leaf: BasinId,
    /// Elder-family containment; this is not necessarily the receiving account.
    pub elder_parent: Option<BasinId>,
    /// Actual oriented physical spill and independently resolved target account.
    pub spill: SpillConnection,
}

/// Private oriented spill with the source component from before its event.
#[derive(Debug, Clone, Copy)]
pub struct PendingSpill {
    /// Actual source endpoint.
    pub from: CellIndex,
    /// Actual adjacent endpoint, or a physical domain exit.
    pub to: Option<CellIndex>,
    /// Physical sill, shared by all joins within one event.
    pub sill_mm: i32,
    /// Positive-capacity component existing strictly before this event.
    pub source_base: u64,
}

/// One private fixed node row; no child vector is present.
#[derive(Debug, Clone, Copy)]
pub struct NodeRow {
    /// Canonical lowest descendant, ties broken by packed (y,x).
    pub anchor: GlobalCell,
    /// Minimum physical descendant elevation.
    pub floor_mm: i32,
    /// Leaf floor, or internal joining height.
    pub birth_mm: i32,
    /// Whether this is an immutable physical terminal.
    pub leaf: bool,
    /// Temporary containing join; always a greater row ordinal.
    pub parent: Option<u64>,
    /// Potential outward witness, present after the MST closes to Exterior.
    pub spill: Option<PendingSpill>,
    /// Final identity, absent on a contracted zero-capacity join.
    pub id: Option<BasinId>,
    /// Nearest retained parent, computed without recursive ancestry walks.
    pub retained_parent: Option<u64>,
}

/// Paged scratch and output interface; errors abandon the private transaction.
///
/// `reserve` admits all declared scratch before mutation. `finish_links` sorts
/// (parent,child) externally, rejects duplicates, writes only child IDs to the
/// global child table and indexes each parent's canonical TableSpan. It must not
/// materialize the largest parent's children. Fixed node rows are emitted later.
/// Every method has its own checked byte/I/O reservation in the disk adapter.
pub trait Store {
    /// Backend failure propagated without publishing a partial hierarchy.
    type Error;
    /// Reserve union rows (closed minima plus Exterior) and binary node rows.
    fn reserve(&mut self, union_rows: u64, node_rows: u64) -> std::result::Result<(), Self::Error>;
    /// Read an initialized fixed union row.
    fn union(&mut self, at: u64) -> std::result::Result<UnionRow, Self::Error>;
    /// Write one fixed union row.
    fn put_union(&mut self, at: u64, row: UnionRow) -> std::result::Result<(), Self::Error>;
    /// Read an initialized fixed temporary node row.
    fn node(&mut self, at: u64) -> std::result::Result<NodeRow, Self::Error>;
    /// Write one fixed temporary node row.
    fn put_node(&mut self, at: u64, row: NodeRow) -> std::result::Result<(), Self::Error>;
    /// Submit one pair to the bounded external child-link sorter.
    fn link(&mut self, parent: BasinId, child: BasinId) -> std::result::Result<(), Self::Error>;
    /// Finish the canonical child table and its fixed span index.
    fn finish_links(&mut self, count: u64) -> std::result::Result<(), Self::Error>;
    /// Read the indexed span; an absent parent has canonical (0,0).
    fn child_span(&mut self, parent: BasinId) -> std::result::Result<TableSpan, Self::Error>;
    /// Stream one public fixed row, in ascending final BasinId order.
    fn emit(&mut self, row: BasinNodeRow) -> std::result::Result<(), Self::Error>;
    /// Stream one exclusive elder-family row, in ascending physical leaf ID order.
    fn emit_elder(&mut self, row: ElderLink) -> std::result::Result<(), Self::Error>;
}

/// Explicit logical-work admission, independent of physical disk I/O limits.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Maximum closed physical leaves admitted before scratch reservation.
    pub leaves: u64,
    /// Total attempted store operations; each is charged before it is called.
    pub store_operations: u64,
}
/// Successful work and emitted table sizes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Work {
    /// Attempted fixed store operations.
    pub store_operations: u64,
    /// Accepted physical MST edges.
    pub edges: u64,
    /// Public positive-capacity node rows.
    pub nodes: u64,
    /// Public child-ID rows.
    pub links: u64,
    /// Removed same-height internal joins.
    pub contracted: u64,
}
/// Invalid authority stream, bounded-resource failure, or backend failure.
#[derive(Debug, thiserror::Error)]
pub enum BuildError<E> {
    /// Scratch, input, output or witness resolver failed.
    #[error("hierarchy backend failed")]
    Backend(#[source] E),
    /// Accepted topology or a private indexed row is inconsistent.
    #[error("invalid hierarchy: {0}")]
    Invalid(&'static str),
    /// The predeclared count or work reservation was exhausted.
    #[error("hierarchy work reservation exhausted")]
    Limit,
}
type Result<T, E> = std::result::Result<T, BuildError<E>>;

struct Pass<'a, S> {
    store: &'a mut S,
    limits: Limits,
    work: Work,
    leaves: u64,
}
impl<S: Store> Pass<'_, S> {
    fn op<T>(
        &mut self,
        f: impl FnOnce(&mut S) -> std::result::Result<T, S::Error>,
    ) -> Result<T, S::Error> {
        if self.work.store_operations >= self.limits.store_operations {
            return Err(BuildError::Limit);
        }
        self.work.store_operations += 1;
        f(self.store).map_err(BuildError::Backend)
    }
    fn u(&mut self, i: u64) -> Result<UnionRow, S::Error> {
        if i > self.leaves {
            return Err(BuildError::Invalid("union index"));
        }
        self.op(|s| s.union(i))
    }
    fn n(&mut self, i: u64) -> Result<NodeRow, S::Error> {
        self.op(|s| s.node(i))
    }
    fn root(&mut self, mut i: u64, before: Option<i32>) -> Result<u64, S::Error> {
        // Union by rank limits valid depth to log2(leaves+1); 64 also terminates corrupt cycles.
        for _ in 0..64 {
            let r = self.u(i)?;
            if r.parent == i {
                return Ok(i);
            }
            let at = r
                .joined_at
                .ok_or(BuildError::Invalid("union parent lacks event"))?;
            if before.is_some_and(|h| at >= h) {
                return Ok(i);
            }
            i = r.parent;
        }
        Err(BuildError::Invalid("cyclic or overdeep union forest"))
    }
    fn bound_minimum(&mut self, extent: Extent, c: GlobalCell) -> Result<u64, S::Error> {
        let last = CellIndex::new(extent.cells() - 1, extent)
            .ok_or(BuildError::Invalid("empty extent"))?;
        let (mx, my) = extent.coordinates(last);
        if c.x > mx || c.y > my {
            return Err(BuildError::Invalid("bound terminal extent"));
        }
        let raw =
            c.y.checked_mul(mx + 1)
                .and_then(|v| v.checked_add(c.x))
                .ok_or(BuildError::Invalid("bound terminal ordinal"))?;
        let at =
            CellIndex::new(raw, extent).ok_or(BuildError::Invalid("bound terminal ordinal"))?;
        self.find_minimum(at)
    }
    fn check_binding(
        &mut self,
        extent: Extent,
        w: Witness,
        bound: BoundSpill,
    ) -> Result<(), S::Error> {
        let expected = self.bound_minimum(extent, w.source_anchor)?;
        let actual = self.bound_minimum(extent, bound.source_terminal)?;
        let expected = self.root(expected, Some(w.sill.raw()))?;
        let actual = self.root(actual, Some(w.sill.raw()))?;
        if expected != actual {
            return Err(BuildError::Invalid(
                "spill source outside pre-event component",
            ));
        }
        if let Some(target) = bound.target_terminal {
            let target = self.bound_minimum(extent, target)?;
            if self.root(target, Some(w.sill.raw()))? == expected {
                return Err(BuildError::Invalid(
                    "spill target inside pre-event component",
                ));
            }
        }
        Ok(())
    }
    fn find_minimum(&mut self, at: CellIndex) -> Result<u64, S::Error> {
        let (mut low, mut high) = (0, self.leaves);
        while low < high {
            let mid = low + (high - low) / 2;
            let m = self
                .u(mid)?
                .minimum
                .ok_or(BuildError::Invalid("missing minimum"))?;
            if m.at < at {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        if low == self.leaves || self.u(low)?.minimum.map(|m| m.at) != Some(at) {
            return Err(BuildError::Invalid("unknown closed terminal"));
        }
        Ok(low)
    }
    fn node_index(&mut self, n: Node) -> Result<u64, S::Error> {
        match n {
            Node::Closed(at) => self.find_minimum(at),
            Node::Exterior => Ok(self.leaves),
        }
    }
    fn pin(&mut self, i: u64, level: i32) -> Result<UnionRow, S::Error> {
        let mut r = self.u(i)?;
        if r.event != Some(level) {
            r.event = Some(level);
            r.event_base = r.component;
            self.op(|s| s.put_union(i, r))?;
        }
        Ok(r)
    }
    fn base(&mut self, leaf: u64, level: i32) -> Result<u64, S::Error> {
        let root = self.root(leaf, Some(level))?;
        self.pin(root, level)?
            .event_base
            .ok_or(BuildError::Invalid("outgoing source was already exterior"))
    }
    fn set_spill(
        &mut self,
        node: u64,
        parent: Option<u64>,
        spill: PendingSpill,
    ) -> Result<(), S::Error> {
        let mut r = self.n(node)?;
        if r.spill.is_some() || spill.sill_mm < r.birth_mm {
            return Err(BuildError::Invalid("invalid repeated node death"));
        }
        r.parent = parent;
        r.spill = Some(spill);
        self.op(|s| s.put_node(node, r))
    }
}
fn point(extent: Extent, at: CellIndex) -> GlobalCell {
    let (x, y) = extent.coordinates(at);
    GlobalCell { x, y }
}
fn anchor(c: GlobalCell) -> u64 {
    (u64::from(c.y) << 32) | u64::from(c.x)
}
fn key(extent: Extent, e: Saddle) -> (i32, u64, u64, u64, u64) {
    let node = |n| match n {
        Node::Closed(at) => extent.anchor_key(at),
        Node::Exterior => u64::MAX,
    };
    let a = extent.anchor_key(e.from);
    let b = e.to.map_or(u64::MAX, |at| extent.anchor_key(at));
    (e.sill_mm, node(e.left), node(e.right), a.min(b), a.max(b))
}

/// Build the immutable physical hierarchy without owning world-sized vectors.
///
/// Inputs contain exactly `leaf_count` sorted final closed terminals and exactly
/// `leaf_count` accepted MST edges connecting them to Exterior. Edges have been
/// physically audited against final receiver ownership and are strictly sorted
/// by `(sill, packed left/right owner, canonical packed witnesses)`.
///
/// The resolver receives the final source ID and exact oriented witness. It
/// checks physical ownership and returns the real receiving account: closed
/// owner -> physical leaf, actual marine target -> Sea, dry transit target ->
/// its preallocated reach breakpoint. This builder never invents ReachIds.
///
/// # Errors
/// Fails on malformed streams, cycles, missing exterior connectivity, zero-depth
/// closed terminals, invalid callback geometry, backend errors or work limits.
pub fn build<S: Store>(
    extent: Extent,
    leaf_count: u64,
    minima: impl IntoIterator<Item = std::result::Result<Minimum, S::Error>>,
    edges: impl IntoIterator<Item = std::result::Result<Saddle, S::Error>>,
    store: &mut S,
    limits: Limits,
    mut resolve: impl FnMut(Witness) -> std::result::Result<BoundSpill, S::Error>,
) -> Result<Work, S::Error> {
    if leaf_count > limits.leaves || leaf_count > u64::from(extent.cells()) {
        return Err(BuildError::Limit);
    }
    let max_nodes = leaf_count.saturating_mul(2).saturating_sub(1);
    let mut p = Pass {
        store,
        limits,
        work: Work::default(),
        leaves: leaf_count,
    };
    p.op(|s| s.reserve(leaf_count + 1, max_nodes))?;
    let mut count = 0;
    let mut last = None;
    for input in minima {
        let m = input.map_err(BuildError::Backend)?;
        if count >= leaf_count
            || CellIndex::new(m.at.raw(), extent) != Some(m.at)
            || last.is_some_and(|v| v >= m.at)
        {
            return Err(BuildError::Invalid("minimum count/order/extent"));
        }
        last = Some(m.at);
        p.op(|s| {
            s.put_union(
                count,
                UnionRow {
                    minimum: Some(m),
                    parent: count,
                    rank: 0,
                    joined_at: None,
                    component: Some(count),
                    event: None,
                    event_base: None,
                    elder: count,
                    elder_death: None,
                },
            )
        })?;
        p.op(|s| {
            s.put_node(
                count,
                NodeRow {
                    anchor: point(extent, m.at),
                    floor_mm: m.floor_mm,
                    birth_mm: m.floor_mm,
                    leaf: true,
                    parent: None,
                    spill: None,
                    id: None,
                    retained_parent: None,
                },
            )
        })?;
        count += 1;
    }
    if count != leaf_count {
        return Err(BuildError::Invalid("missing minima"));
    }
    p.op(|s| {
        s.put_union(
            leaf_count,
            UnionRow {
                minimum: None,
                parent: leaf_count,
                rank: 0,
                joined_at: None,
                component: None,
                event: None,
                event_base: None,
                elder: leaf_count,
                elder_death: None,
            },
        )
    })?;
    let mut next_node = leaf_count;
    let mut previous = None;
    for input in edges {
        let e = input.map_err(BuildError::Backend)?;
        if p.work.edges >= leaf_count {
            return Err(BuildError::Invalid("too many MST edges"));
        }
        let k = key(extent, e);
        if Saddle::decode(e.encode(), extent) != Some(e) || previous.is_some_and(|v| v >= k) {
            return Err(BuildError::Invalid("MST witness/order"));
        }
        previous = Some(k);
        p.work.edges += 1;
        let a = p.node_index(e.left)?;
        let b = p.node_index(e.right)?;
        let ra = p.root(a, None)?;
        let rb = p.root(b, None)?;
        if ra == rb {
            return Err(BuildError::Invalid("cycle in accepted MST"));
        }
        let ua = p.pin(ra, e.sill_mm)?;
        let ub = p.pin(rb, e.sill_mm)?;
        let parent = match (ua.component, ub.component) {
            (Some(ca), Some(cb)) => {
                if next_node >= max_nodes {
                    return Err(BuildError::Invalid("too many joins"));
                }
                let na = p.n(ca)?;
                let nb = p.n(cb)?;
                let best = if (na.floor_mm, anchor(na.anchor)) <= (nb.floor_mm, anchor(nb.anchor)) {
                    na
                } else {
                    nb
                };
                let idx = next_node;
                next_node += 1;
                p.op(|s| {
                    s.put_node(
                        idx,
                        NodeRow {
                            anchor: best.anchor,
                            floor_mm: best.floor_mm,
                            birth_mm: e.sill_mm,
                            leaf: false,
                            parent: None,
                            spill: None,
                            id: None,
                            retained_parent: None,
                        },
                    )
                })?;
                Some(idx)
            }
            _ => None,
        };
        if let Some(c) = ua.component {
            let source_base = p.base(a, e.sill_mm)?;
            p.set_spill(
                c,
                parent,
                PendingSpill {
                    from: e.from,
                    to: e.to,
                    sill_mm: e.sill_mm,
                    source_base,
                },
            )?;
        }
        if let Some(c) = ub.component {
            let source_base = p.base(b, e.sill_mm)?;
            p.set_spill(
                c,
                parent,
                PendingSpill {
                    from: e
                        .to
                        .ok_or(BuildError::Invalid("missing right source endpoint"))?,
                    to: Some(e.from),
                    sill_mm: e.sill_mm,
                    source_base,
                },
            )?;
        }
        // Elder-family death is independent of the binary containment join.
        // Exterior always survives, even for the i32::MIN closed-floor extreme.
        let elder_key = |r: UnionRow| {
            r.minimum
                .map_or((i32::MIN, 0), |m| (m.floor_mm, extent.anchor_key(m.at)))
        };
        let a_elder = p.u(ua.elder)?;
        let b_elder = p.u(ub.elder)?;
        let a_survives = ua.elder == leaf_count
            || (ub.elder != leaf_count && elder_key(a_elder) < elder_key(b_elder));
        let (winner, loser, losing_component, from, to) = if a_survives {
            (
                ua.elder,
                ub.elder,
                b,
                e.to.ok_or(BuildError::Invalid("missing younger source"))?,
                Some(e.from),
            )
        } else {
            (ub.elder, ua.elder, a, e.from, e.to)
        };
        let elder_parent = if winner == leaf_count {
            None
        } else {
            let m = p
                .u(winner)?
                .minimum
                .ok_or(BuildError::Invalid("elder minimum absent"))?;
            Some(BasinId(extent.anchor_key(m.at)))
        };
        let mut dead = p.u(loser)?;
        if dead.elder_death.is_some() {
            return Err(BuildError::Invalid("elder died twice"));
        }
        dead.elder_death = Some(ElderDeath {
            parent: elder_parent,
            spill: PendingSpill {
                from,
                to,
                sill_mm: e.sill_mm,
                source_base: p.base(losing_component, e.sill_mm)?,
            },
        });
        p.op(|s| s.put_union(loser, dead))?;
        // Reload: a leaf-family update may share one of these union-root rows.
        let ua = p.u(ra)?;
        let ub = p.u(rb)?;
        let (root, mut ur, child, mut uc) =
            if (ua.rank, std::cmp::Reverse(ra)) >= (ub.rank, std::cmp::Reverse(rb)) {
                (ra, ua, rb, ub)
            } else {
                (rb, ub, ra, ua)
            };
        ur.elder = winner;
        if ur.rank == uc.rank {
            ur.rank = ur
                .rank
                .checked_add(1)
                .ok_or(BuildError::Invalid("union rank overflow"))?;
        }
        ur.component = parent;
        uc.parent = root;
        uc.joined_at = Some(e.sill_mm);
        p.op(|s| s.put_union(root, ur))?;
        p.op(|s| s.put_union(child, uc))?;
    }
    if p.work.edges != leaf_count {
        return Err(BuildError::Invalid("MST does not reach exterior"));
    }
    let outer = p.root(leaf_count, None)?;
    if p.u(outer)?.component.is_some() {
        return Err(BuildError::Invalid("unclosed physical component"));
    }
    // Mark retention and transfer deleted joins' outward witnesses in bottom-up
    // order. A later enclosing zero join overrides the same pre-event source.
    let mut internal = 0;
    for i in 0..next_node {
        let mut n = p.n(i)?;
        let spill = n
            .spill
            .ok_or(BuildError::Invalid("node lacks physical spill"))?;
        if spill.sill_mm < n.birth_mm {
            return Err(BuildError::Invalid("spill below birth"));
        }
        if n.leaf && spill.sill_mm == n.birth_mm {
            return Err(BuildError::Invalid(
                "zero-capacity closed receiver terminal",
            ));
        }
        if spill.sill_mm == n.birth_mm {
            if spill.source_base >= i {
                return Err(BuildError::Invalid("invalid pre-event source"));
            }
            let mut source = p.n(spill.source_base)?;
            if source.birth_mm >= spill.sill_mm
                || source.spill.map(|v| v.sill_mm) != Some(spill.sill_mm)
            {
                return Err(BuildError::Invalid(
                    "coalesced spill changes physical event",
                ));
            }
            source.spill = Some(spill);
            p.op(|s| s.put_node(spill.source_base, source))?;
            p.work.contracted += 1;
        } else {
            n.id = Some(if n.leaf {
                BasinId(anchor(n.anchor))
            } else {
                let id = BasinId(INTERNAL | internal);
                internal += 1;
                id
            });
            p.work.nodes += 1;
        }
        p.op(|s| s.put_node(i, n))?;
    }
    // Parents have greater temporary ordinals, so one descending pass resolves
    // every contracted chain without an O(depth) walk per leaf.
    for i in (0..next_node).rev() {
        let mut n = p.n(i)?;
        n.retained_parent = if let Some(parent) = n.parent {
            if parent <= i || parent >= next_node {
                return Err(BuildError::Invalid("invalid temporary parent"));
            }
            let r = p.n(parent)?;
            if r.id.is_some() {
                Some(parent)
            } else {
                r.retained_parent
            }
        } else {
            None
        };
        p.op(|s| s.put_node(i, n))?;
        if let (Some(id), Some(parent)) = (n.id, n.retained_parent) {
            let pid = p
                .n(parent)?
                .id
                .ok_or(BuildError::Invalid("missing retained parent identity"))?;
            p.op(|s| s.link(pid, id))?;
            p.work.links += 1;
        }
    }
    let links = p.work.links;
    p.op(|s| s.finish_links(links))?;
    for i in 0..next_node {
        let n = p.n(i)?;
        let Some(id) = n.id else { continue };
        let s = n
            .spill
            .ok_or(BuildError::Invalid("retained node lacks spill"))?;
        let witness = Witness {
            source: id,
            source_anchor: n.anchor,
            from: point(extent, s.from),
            to: s.to.map(|at| point(extent, at)),
            sill: HeightMm::new(s.sill_mm),
        };
        let bound = resolved(extent, witness, &mut resolve)?;
        p.check_binding(extent, witness, bound)?;
        let spill = bound.spill;
        let children = p.op(|s| s.child_span(id))?;
        children
            .validate_total(links)
            .map_err(|_| BuildError::Invalid("child span leaves table"))?;
        if (n.leaf && children.count != 0) || (!n.leaf && children.count < 2) {
            return Err(BuildError::Invalid("invalid coalesced child count"));
        }
        let parent = if let Some(at) = n.retained_parent {
            p.n(at)?.id
        } else {
            None
        };
        p.op(|s| {
            s.emit(BasinNodeRow {
                id,
                parent,
                anchor: n.anchor,
                floor: HeightMm::new(n.floor_mm),
                children,
                spill: Some(spill),
            })
        })?;
    }
    for i in 0..leaf_count {
        let u = p.u(i)?;
        let minimum = u
            .minimum
            .ok_or(BuildError::Invalid("leaf minimum absent"))?;
        let death = u
            .elder_death
            .ok_or(BuildError::Invalid("leaf elder death absent"))?;
        let leaf = BasinId(extent.anchor_key(minimum.at));
        let s = death.spill;
        if s.sill_mm <= minimum.floor_mm {
            return Err(BuildError::Invalid("elder spill not above leaf floor"));
        }
        let witness = Witness {
            source: leaf,
            // An elder can die through a different constituent after another
            // join at this same sill. Validate the actual source's pinned
            // pre-event component, while preserving the dying elder identity.
            source_anchor: p.n(s.source_base)?.anchor,
            from: point(extent, s.from),
            to: s.to.map(|at| point(extent, at)),
            sill: HeightMm::new(s.sill_mm),
        };
        let bound = resolved(extent, witness, &mut resolve)?;
        p.check_binding(extent, witness, bound)?;
        let spill = bound.spill;
        p.op(|s| {
            s.emit_elder(ElderLink {
                leaf,
                elder_parent: death.parent,
                spill,
            })
        })?;
    }
    Ok(p.work)
}

fn resolved<E>(
    extent: Extent,
    witness: Witness,
    resolve: &mut impl FnMut(Witness) -> std::result::Result<BoundSpill, E>,
) -> Result<BoundSpill, E> {
    let bound = resolve(witness).map_err(BuildError::Backend)?;
    let spill = bound.spill;
    if spill.from != witness.from || spill.to != witness.to || spill.sill != witness.sill {
        return Err(BuildError::Invalid("resolver changed physical witness"));
    }
    if spill.to.is_none() {
        let last = CellIndex::new(extent.cells() - 1, extent)
            .ok_or(BuildError::Invalid("empty extent"))?;
        let (mx, my) = extent.coordinates(last);
        let c = spill.from;
        if spill.receiving != ReceivingAccount::DomainExport
            || !(c.x == 0 || c.y == 0 || c.x == mx || c.y == my)
        {
            return Err(BuildError::Invalid("absent spill target is not rim export"));
        }
    }
    if spill.to.is_some() && spill.receiving == ReceivingAccount::DomainExport {
        return Err(BuildError::Invalid("domain export has modeled target"));
    }
    if spill.receiving == ReceivingAccount::Lake(witness.source) {
        return Err(BuildError::Invalid("spill returns to same component"));
    }
    Ok(bound)
}
