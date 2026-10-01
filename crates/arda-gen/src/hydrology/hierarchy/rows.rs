//! The fixed rows a hierarchy build reads and writes, and the paged
//! scratch [`Store`] the caller supplies for them.

use super::*;

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
