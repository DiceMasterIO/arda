//! Bounded annual adjacency from retained physical rows and the accepted witnessed MST.
use super::annual::{AnnualDestination, AnnualJoin, AnnualNode};
use super::routing::{CellIndex, Extent};
use super::saddles::{Node, Saddle};
use super::witness_binding::{BindingError, BreakpointRegistry, Destination};
use arda_core::formats::hydrology::BasinNodeRow;
use arda_core::hydrology::{BasinId, Litres};

/// Exact table counts and live adapter reservation; input-reader I/O is separate.
#[derive(Debug, Clone, Copy)]
pub struct TopologyLimits {
    /// Exact retained row count.
    pub nodes: u64,
    /// Exact immutable terminal count, also the accepted MST edge count.
    pub leaves: u64,
    /// Includes nodes, joins, borrowed children and transient indexed working arrays.
    pub ram_bytes: u64,
    /// Attempted iterator reads, row visits, key comparisons and ancestry/union steps.
    pub operations: u64,
}
/// Complete topology for the caller's subsequent physical-cell source/band aggregation.
pub struct AnnualTopology {
    /// Canonical IDs, existing child spans; all band/source fields start at zero.
    pub nodes: Vec<AnnualNode>,
    /// Accepted connections, ordered by the immutable canonical MST ordinal.
    pub joins: Vec<AnnualJoin>,
    /// Attempted counted work.
    pub operations: u64,
}
/// No partial output is published after failure.
#[derive(Debug, thiserror::Error)]
pub enum TopologyError<E> {
    /// An admitted input reader failed.
    #[error("annual topology input failed")]
    Input(#[source] E),
    /// A physical receiving account is missing from the checked witness registry.
    #[error("annual topology destination failed")]
    Binding(#[source] BindingError<E>),
    /// The two immutable authority tables disagree.
    #[error("invalid annual topology: {0}")]
    Invalid(&'static str),
    /// Count, allocation, arithmetic or logical-work reservation failed.
    #[error("annual topology resource limit")]
    Limit,
}
type Result<T, E> = std::result::Result<T, TopologyError<E>>;
/// Conservative live payload bound, including transient working arrays and allocator allowance.
#[must_use]
pub fn required_ram(nodes: u64) -> Option<u64> {
    nodes.checked_mul(1024)?.checked_add(4096)
}
fn edge_key(extent: Extent, edge: Saddle) -> (i32, u64, u64, u64, u64) {
    let owner = |n| match n {
        Node::Closed(at) => extent.anchor_key(at),
        Node::Exterior => u64::MAX,
    };
    let from = extent.anchor_key(edge.from);
    let to = edge.to.map_or(u64::MAX, |at| extent.anchor_key(at));
    (
        edge.sill_mm,
        owner(edge.left),
        owner(edge.right),
        from.min(to),
        from.max(to),
    )
}
struct Meter {
    used: u64,
    maximum: u64,
}
impl Meter {
    fn tick<E>(&mut self) -> Result<(), E> {
        if self.used >= self.maximum {
            return Err(TopologyError::Limit);
        }
        self.used += 1;
        Ok(())
    }
}
fn reserve<T, E>(n: usize) -> Result<Vec<T>, E> {
    let mut v = Vec::new();
    v.try_reserve_exact(n).map_err(|_| TopologyError::Limit)?;
    Ok(v)
}
fn index<E>(nodes: &[AnnualNode], id: BasinId, meter: &mut Meter) -> Result<usize, E> {
    let (mut lo, mut hi) = (0, nodes.len());
    while lo < hi {
        meter.tick()?;
        let mid = lo + (hi - lo) / 2;
        match nodes[mid].id.cmp(&id) {
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => hi = mid,
            std::cmp::Ordering::Equal => return Ok(mid),
        }
    }
    Err(TopologyError::Invalid("unknown physical basin"))
}
fn union_root<E>(mut at: usize, union: &mut [usize], meter: &mut Meter) -> Result<usize, E> {
    loop {
        meter.tick()?;
        if union[at] == at {
            return Ok(at);
        }
        union[at] = union[union[at]];
        at = union[at];
    }
}
fn before<E>(
    mut at: usize,
    sill: i32,
    nodes: &[AnnualNode],
    parents: &[Option<usize>],
    meter: &mut Meter,
) -> Result<usize, E> {
    loop {
        meter.tick()?;
        match parents[at] {
            Some(p) if nodes[p].birth.raw() < sill => at = p,
            _ => return Ok(at),
        }
    }
}

/// Convert canonical retained rows and replay the exact physically audited accepted edges.
///
/// `children` is the already checked canonical global child table. Readers must
/// share the supplied error type and retain their own byte/I/O reservations.
/// Accepted edges are the unchanged sorter output, not a new candidate scan.
/// Root destinations preserve real physical dry junctions through the binder.
/// Source and band aggregation runs only after this function succeeds.
///
/// # Errors
/// Rejects malformed counts, identities, spans, capacity, parents, join cycles,
/// unknown endpoints, incomplete join trees, failed inputs and exhausted caps.
#[allow(clippy::too_many_lines)]
pub fn adapt<E>(
    extent: Extent,
    rows: impl IntoIterator<Item = std::result::Result<BasinNodeRow, E>>,
    children: &[BasinId],
    edges: impl IntoIterator<Item = std::result::Result<Saddle, E>>,
    registry: &BreakpointRegistry,
    limits: TopologyLimits,
) -> Result<AnnualTopology, E> {
    if limits.leaves > limits.nodes
        || limits.leaves > u64::from(extent.cells())
        || limits.nodes > limits.leaves.saturating_mul(2).saturating_sub(1)
        || required_ram(limits.nodes).is_none_or(|n| n > limits.ram_bytes)
    {
        return Err(TopologyError::Limit);
    }
    let last =
        CellIndex::new(extent.cells() - 1, extent).ok_or(TopologyError::Invalid("empty extent"))?;
    let (max_x, max_y) = extent.coordinates(last);
    let n = usize::try_from(limits.nodes).map_err(|_| TopologyError::Limit)?;
    if children.len() > n {
        return Err(TopologyError::Invalid("child count"));
    }
    let mut meter = Meter {
        used: 0,
        maximum: limits.operations,
    };
    let mut nodes: Vec<AnnualNode> = reserve(n)?;
    let mut rows = rows.into_iter();
    loop {
        meter.tick()?;
        let Some(row) = rows.next() else { break };
        let r = row.map_err(TopologyError::Input)?;
        if nodes.len() == n || nodes.last().is_some_and(|v| v.id >= r.id) {
            return Err(TopologyError::Invalid("node count/order"));
        }
        let start = usize::try_from(r.children.offset).map_err(|_| TopologyError::Limit)?;
        let count = usize::try_from(r.children.count).map_err(|_| TopologyError::Limit)?;
        let end = start.checked_add(count).ok_or(TopologyError::Limit)?;
        if end > children.len() || (count == 0 && start != 0) || count == 1 {
            return Err(TopologyError::Invalid("child span"));
        }
        let spill = r
            .spill
            .ok_or(TopologyError::Invalid("missing physical spill"))?;
        if r.anchor.x > max_x
            || r.anchor.y > max_y
            || (count == 0 && r.id.0 != ((u64::from(r.anchor.y) << 32) | u64::from(r.anchor.x)))
            || (count != 0 && r.id.0 >> 63 != 1)
        {
            return Err(TopologyError::Invalid("node identity/anchor"));
        }
        let destination = if r.parent.is_none() {
            Some(
                match registry
                    .destination(spill.receiving)
                    .map_err(TopologyError::Binding)?
                {
                    Destination::Basin(id) => AnnualDestination::Basin(id),
                    Destination::Sea => AnnualDestination::Sea,
                    Destination::DomainExport => AnnualDestination::DomainExport,
                },
            )
        } else {
            None
        };
        nodes.push(AnnualNode {
            id: r.id,
            parent: r.parent,
            floor: r.floor,
            birth: r.floor,
            spill: Some(spill),
            destination,
            children: start..end,
            bands: 0..0,
            local_runoff: Litres(0),
            local_precipitation: Litres(0),
            local_land_loss: Litres(0),
        });
    }
    if nodes.len() != n {
        return Err(TopologyError::Invalid("missing rows"));
    }
    let mut parents = reserve(n)?;
    let mut seen = reserve(n)?;
    seen.resize(n, false);
    let mut slots = reserve(children.len())?;
    slots.resize(children.len(), false);
    let mut leaf_count = 0_u64;
    for node in &nodes {
        meter.tick()?;
        parents.push(
            node.parent
                .map(|p| index(&nodes, p, &mut meter))
                .transpose()?,
        );
    }
    for i in 0..n {
        meter.tick()?;
        if nodes[i].children.is_empty() {
            leaf_count += 1;
            continue;
        }
        let mut birth = None;
        let mut floor = i32::MAX;
        let mut last = None;
        for slot in nodes[i].children.clone() {
            meter.tick()?;
            if slots[slot] || last.is_some_and(|v| v >= children[slot]) {
                return Err(TopologyError::Invalid("child order/overlap"));
            }
            slots[slot] = true;
            last = Some(children[slot]);
            let c = index(&nodes, children[slot], &mut meter)?;
            if seen[c] || parents[c] != Some(i) {
                return Err(TopologyError::Invalid("child relation"));
            }
            seen[c] = true;
            let sill = nodes[c]
                .spill
                .ok_or(TopologyError::Invalid("missing child spill"))?
                .sill;
            if birth.is_some_and(|b| b != sill) {
                return Err(TopologyError::Invalid("unequal child sills"));
            }
            birth = Some(sill);
            floor = floor.min(nodes[c].floor.raw());
        }
        if floor != nodes[i].floor.raw() {
            return Err(TopologyError::Invalid("internal floor"));
        }
        nodes[i].birth = birth.ok_or(TopologyError::Invalid("empty join"))?;
    }
    if leaf_count != limits.leaves {
        return Err(TopologyError::Invalid("leaf count"));
    }
    for used in slots {
        meter.tick()?;
        if !used {
            return Err(TopologyError::Invalid("unowned child slot"));
        }
    }
    for i in 0..n {
        meter.tick()?;
        let v = &nodes[i];
        let spill = v
            .spill
            .ok_or(TopologyError::Invalid("missing spill"))?
            .sill
            .raw();
        if seen[i] != parents[i].is_some()
            || v.floor.raw() > v.birth.raw()
            || v.birth.raw() >= spill
            || parents[i].is_some_and(|p| nodes[p].birth.raw() != spill)
        {
            return Err(TopologyError::Invalid("retained capacity/parent"));
        }
        if let Some(AnnualDestination::Basin(id)) = v.destination {
            let target = index(&nodes, id, &mut meter)?;
            if !nodes[target].children.is_empty() {
                return Err(TopologyError::Invalid("nonleaf destination"));
            }
        }
    }
    let mut union = reserve(n)?;
    union.extend(0..n);
    let mut counts = reserve(n)?;
    counts.resize(n, 0_usize);
    let mut joins = reserve(usize::try_from(limits.leaves).map_err(|_| TopologyError::Limit)?)?;
    let mut previous = None;
    let mut witness = 0_u64;
    let mut edges = edges.into_iter();
    loop {
        meter.tick()?;
        let Some(edge) = edges.next() else { break };
        let edge = edge.map_err(TopologyError::Input)?;
        if witness >= limits.leaves || Saddle::decode(edge.encode(), extent) != Some(edge) {
            return Err(TopologyError::Invalid("edge count/codec"));
        }
        let key = edge_key(extent, edge);
        meter.tick()?;
        if previous.is_some_and(|v| v >= key) {
            return Err(TopologyError::Invalid("accepted edge order"));
        }
        previous = Some(key);
        let ordinal = witness;
        witness += 1;
        let Node::Closed(left) = edge.left else {
            return Err(TopologyError::Invalid("exterior source"));
        };
        let left_leaf = BasinId(extent.anchor_key(left));
        let li = index(&nodes, left_leaf, &mut meter)?;
        if !nodes[li].children.is_empty() {
            return Err(TopologyError::Invalid("nonleaf endpoint"));
        }
        let lc = before(li, edge.sill_mm, &nodes, &parents, &mut meter)?;
        let Node::Closed(right) = edge.right else {
            if parents[lc].is_some()
                || nodes[lc]
                    .spill
                    .ok_or(TopologyError::Invalid("root spill"))?
                    .sill
                    .raw()
                    > edge.sill_mm
            {
                return Err(TopologyError::Invalid("exterior edge below root spill"));
            }
            continue;
        };
        let right_leaf = BasinId(extent.anchor_key(right));
        let ri = index(&nodes, right_leaf, &mut meter)?;
        if !nodes[ri].children.is_empty() {
            return Err(TopologyError::Invalid("nonleaf endpoint"));
        }
        let rc = before(ri, edge.sill_mm, &nodes, &parents, &mut meter)?;
        if lc == rc {
            return Err(TopologyError::Invalid("edge internal below sill"));
        }
        if let Some(p) = parents[lc].filter(|p| parents[rc] == Some(*p)) {
            if nodes[p].birth.raw() != edge.sill_mm {
                return Err(TopologyError::Invalid("join sill"));
            }
            let a = union_root(lc, &mut union, &mut meter)?;
            let b = union_root(rc, &mut union, &mut meter)?;
            if a == b {
                return Err(TopologyError::Invalid("join cycle"));
            }
            union[b] = a;
            counts[p] += 1;
            joins.push(AnnualJoin {
                parent: nodes[p].id,
                left_child: nodes[lc].id,
                right_child: nodes[rc].id,
                left_leaf,
                right_leaf,
                witness: ordinal,
            });
        } else if parents[lc].is_some()
            || parents[rc].is_some()
            || nodes[lc]
                .spill
                .ok_or(TopologyError::Invalid("root spill"))?
                .sill
                .raw()
                > edge.sill_mm
            || nodes[rc]
                .spill
                .ok_or(TopologyError::Invalid("root spill"))?
                .sill
                .raw()
                > edge.sill_mm
        {
            return Err(TopologyError::Invalid(
                "edge lacks retained join or exterior roots",
            ));
        }
    }
    if witness != limits.leaves {
        return Err(TopologyError::Invalid("missing accepted edges"));
    }
    for (i, node) in nodes.iter().enumerate() {
        meter.tick()?;
        if counts[i] != node.children.len().saturating_sub(1) {
            return Err(TopologyError::Invalid("incomplete join tree"));
        }
    }
    Ok(AnnualTopology {
        nodes,
        joins,
        operations: meter.used,
    })
}
