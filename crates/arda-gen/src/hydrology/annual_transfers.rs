//! Final annual fluxes across the witnessed physical basin tree.
//!
//! Computational filling transfers are discarded: each edge carries the signed
//! sum of final leaf balances on its side of the tree. This prevents a merged
//! lake from exporting the same water twice through historical child pours.

use super::routing::Extent;
use super::saddles::{Node, Saddle};
use arda_core::hydrology::{BasinId, Litres};
use arda_core::{GlobalCell, HeightMm};

/// One immutable physical leaf after the annual support solve.
#[derive(Debug, Clone, Copy)]
pub struct LeafFlow {
    /// Packed physical terminal identity, in ascending order in the input.
    pub leaf: BasinId,
    /// Runoff minus this leaf's paid wet and marginal annual costs.
    pub net_litres: i128,
    /// Actual positive-depth connected lake, never a computational fill group.
    pub lake: Option<BasinId>,
    /// Representative surface of that actual lake, absent on dry leaves.
    pub surface: Option<HeightMm>,
}

/// Nonzero final transfer over one actual spill witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeTransfer {
    /// Immutable leaf owning the emitting side of the witness.
    pub from_leaf: BasinId,
    /// Receiving closed owner; None is the checked exterior topology node.
    pub to_leaf: Option<BasinId>,
    /// Actual source cell, including a zero-depth spill cell at the lake rim.
    pub from: GlobalCell,
    /// Actual neighboring target, absent only at the modeled outer boundary.
    pub to: Option<GlobalCell>,
    /// Physical sill; the emitting lake must support this elevation.
    pub sill: HeightMm,
    /// Whole litres in a representative 365-day year.
    pub annual_volume: Litres,
}

/// Admission for the in-memory basin tree, independently of the fine grid.
#[derive(Debug, Clone, Copy)]
pub struct TransferLimits {
    /// Maximum physical closed leaves.
    pub leaves: u64,
    /// Conservative owned-buffer allowance; input arrays are caller-owned.
    pub ram_bytes: u64,
}

/// Canonical final physical transfers and their exterior balance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedTransfers {
    /// Ordered by physical source/target and immutable leaf identities.
    pub transfers: Vec<TreeTransfer>,
    /// Closed-basin water entering the exterior; direct open catchments add separately.
    pub exterior_outflow: Litres,
}

/// Invalid topology/state, arithmetic overflow, or an unadmitted working set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TransferError {
    /// Caller inputs do not describe one valid physical basin tree.
    #[error("invalid annual transfer input: {0}")]
    Invalid(&'static str),
    /// Exact signed accounting cannot be represented.
    #[error("annual transfer arithmetic overflow")]
    Overflow,
    /// Required buffers or leaf count exceed the caller's allowance.
    #[error("annual transfer resource limit")]
    Limit,
}

/// Conservative live owned payload; allocator overhead remains separately measured.
#[must_use]
pub fn required_ram(leaves: u64) -> Option<u64> {
    // Seven ordinal arrays, signed subtree sums, CSR edges and worst-case output.
    let word = std::mem::size_of::<usize>() as u128;
    let count = u128::from(leaves).checked_add(1)?;
    let arrays = count.checked_mul(7 * word + 16)?;
    let edges = u128::from(leaves).checked_mul(4 * word)?;
    let out = u128::from(leaves).checked_mul(std::mem::size_of::<TreeTransfer>() as u128)?;
    u64::try_from(
        arrays
            .checked_add(edges)?
            .checked_add(out)?
            .checked_add(4096)?,
    )
    .ok()
}

fn global(extent: Extent, at: super::routing::CellIndex) -> GlobalCell {
    let (x, y) = extent.coordinates(at);
    GlobalCell { x, y }
}

fn slot(node: Node, extent: Extent, leaves: &[LeafFlow]) -> Result<usize, TransferError> {
    match node {
        Node::Exterior => Ok(leaves.len()),
        Node::Closed(at) => leaves
            .binary_search_by_key(&extent.anchor_key(at), |v| v.leaf.0)
            .map_err(|_| TransferError::Invalid("missing physical leaf")),
    }
}

/// Normalizes final leaf balances over an already physically bound witnessed MST.
///
/// `Saddle::decode` validates local geometry here; the earlier routing/MST binding
/// validates endpoint ownership and physical heights. Every source surface is
/// checked again before a nonzero transfer is published.
///
/// # Errors
/// Rejects malformed/duplicate/disconnected/cyclic trees, unsupported spills,
/// invalid wet identities, exterior water imports and arithmetic/resource overrun.
pub fn normalize_transfers(
    extent: Extent,
    leaves: &[LeafFlow],
    edges: &[Saddle],
    limits: TransferLimits,
) -> Result<NormalizedTransfers, TransferError> {
    let count = u64::try_from(leaves.len()).map_err(|_| TransferError::Limit)?;
    if count > limits.leaves || required_ram(count).ok_or(TransferError::Limit)? > limits.ram_bytes
    {
        return Err(TransferError::Limit);
    }
    if edges.len() != leaves.len() {
        return Err(TransferError::Invalid("tree edge count"));
    }
    let last_cell = super::routing::CellIndex::new(extent.cells() - 1, extent)
        .ok_or(TransferError::Invalid("empty extent"))?;
    let (last_x, last_y) = extent.coordinates(last_cell);
    let mut absolute = 0u128;
    let mut last = None;
    for leaf in leaves {
        let x = leaf.leaf.0 & 0xffff_ffff;
        let y = leaf.leaf.0 >> 32;
        if x > u64::from(last_x)
            || y > u64::from(last_y)
            || last.is_some_and(|id| id >= leaf.leaf)
            || leaf.lake.is_some() != leaf.surface.is_some()
        {
            return Err(TransferError::Invalid("leaf identity/order/state"));
        }
        absolute = absolute
            .checked_add(leaf.net_litres.unsigned_abs())
            .ok_or(TransferError::Overflow)?;
        if absolute > i128::MAX as u128 {
            return Err(TransferError::Overflow);
        }
        last = Some(leaf.leaf);
    }
    let n = leaves.len().checked_add(1).ok_or(TransferError::Limit)?;
    let exterior = leaves.len();
    let mut degrees = vec![0usize; n];
    for edge in edges {
        if Saddle::decode(edge.encode(), extent) != Some(*edge) {
            return Err(TransferError::Invalid("spill geometry"));
        }
        let a = slot(edge.left, extent, leaves)?;
        let b = slot(edge.right, extent, leaves)?;
        degrees[a] = degrees[a].checked_add(1).ok_or(TransferError::Limit)?;
        degrees[b] = degrees[b].checked_add(1).ok_or(TransferError::Limit)?;
    }
    let mut offsets = vec![0usize; n + 1];
    for i in 0..n {
        offsets[i + 1] = offsets[i]
            .checked_add(degrees[i])
            .ok_or(TransferError::Limit)?;
    }
    let mut positions = offsets[..n].to_vec();
    let mut adjacency = vec![(0usize, 0usize); offsets[n]];
    for (index, edge) in edges.iter().enumerate() {
        let a = slot(edge.left, extent, leaves)?;
        let b = slot(edge.right, extent, leaves)?;
        adjacency[positions[a]] = (b, index);
        positions[a] += 1;
        adjacency[positions[b]] = (a, index);
        positions[b] += 1;
    }
    drop(positions);
    drop(degrees);
    let mut parent = vec![usize::MAX; n];
    let mut parent_edge = vec![usize::MAX; n];
    let mut order = Vec::with_capacity(n);
    parent[exterior] = exterior;
    order.push(exterior);
    let mut cursor = 0;
    while cursor < order.len() {
        let at = order[cursor];
        for &(next, edge) in &adjacency[offsets[at]..offsets[at + 1]] {
            if edge == parent_edge[at] {
                continue;
            }
            if parent[next] != usize::MAX {
                return Err(TransferError::Invalid("cycle or duplicate edge"));
            }
            parent[next] = at;
            parent_edge[next] = edge;
            order.push(next);
        }
        cursor += 1;
    }
    if order.len() != n {
        return Err(TransferError::Invalid("disconnected tree"));
    }
    let mut sums = vec![0i128; n];
    for (i, leaf) in leaves.iter().enumerate() {
        sums[i] = leaf.net_litres;
    }
    let mut transfers = Vec::with_capacity(edges.len());
    for &child in order[1..].iter().rev() {
        let up = parent[child];
        let amount = sums[child];
        sums[up] = sums[up]
            .checked_add(amount)
            .ok_or(TransferError::Overflow)?;
        let edge = edges[parent_edge[child]];
        let child_state = leaves[child];
        let parent_state = leaves.get(up);
        let same_lake =
            child_state.lake.is_some() && parent_state.is_some_and(|p| p.lake == child_state.lake);
        if same_lake {
            if parent_state.and_then(|p| p.surface) != child_state.surface
                || child_state.surface.is_none_or(|h| h.raw() <= edge.sill_mm)
            {
                return Err(TransferError::Invalid("disconnected shared lake identity"));
            }
        } else if child_state.surface.is_some_and(|h| h.raw() > edge.sill_mm)
            && parent_state
                .and_then(|p| p.surface)
                .is_some_and(|h| h.raw() > edge.sill_mm)
        {
            return Err(TransferError::Invalid(
                "different lake identities submerge shared sill",
            ));
        }
        // Wet identity is independent of whether this edge transports any net water.
        if amount == 0 {
            continue;
        }
        let (source, target) = if amount > 0 { (child, up) } else { (up, child) };
        if source == exterior {
            return Err(TransferError::Invalid("unfunded exterior import"));
        }
        let emitting = leaves[source];
        let receiving = leaves.get(target);
        if same_lake {
            continue;
        }
        if emitting.surface.is_none_or(|h| h.raw() != edge.sill_mm) {
            return Err(TransferError::Invalid("unsupported source spill"));
        }
        let source_is_left = source == slot(edge.left, extent, leaves)?;
        let (from, to) = if source_is_left {
            (edge.from, edge.to)
        } else {
            (
                edge.to
                    .ok_or(TransferError::Invalid("reversed domain export"))?,
                Some(edge.from),
            )
        };
        transfers.push(TreeTransfer {
            from_leaf: emitting.leaf,
            to_leaf: receiving.map(|r| r.leaf),
            from: global(extent, from),
            to: to.map(|v| global(extent, v)),
            sill: HeightMm::new(edge.sill_mm),
            annual_volume: Litres(amount.unsigned_abs()),
        });
    }
    let exterior_outflow = u128::try_from(sums[exterior])
        .map_err(|_| TransferError::Invalid("negative exterior balance"))?;
    // The full key is unique on a validated tree, so unstable sorting is deterministic
    // and requires no additional heap buffer beyond the admitted output allocation.
    transfers.sort_unstable_by_key(|t| {
        (
            t.from.y,
            t.from.x,
            t.to.map(|c| (c.y, c.x)),
            t.from_leaf,
            t.to_leaf,
        )
    });
    Ok(NormalizedTransfers {
        transfers,
        exterior_outflow: Litres(exterior_outflow),
    })
}
