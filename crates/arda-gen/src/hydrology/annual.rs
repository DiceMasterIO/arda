//! Deterministic representative annual support-cost filling on a witnessed hierarchy.
//! This is a static climate-support convention, not a dated storage simulation.
#![deny(missing_docs)]

use arda_core::hydrology::{AnnualWaterBalance, BasinId, Litres, SpillConnection};
use arda_core::HeightMm;
use std::collections::BTreeSet;
use std::ops::Range;

mod finish;
mod validate;
use finish::finish;
use validate::validate;

/// Admitted aggregate annual forcing bound; comfortably exceeds the max-domain P/E sums.
/// Bounding both source and support cost also bounds proportional products by 2^124.
pub const MAX_ANNUAL: u128 = 1_u128 << 62;

/// One exact-height band owned by one immutable receiving leaf.
#[derive(Debug, Clone)]
pub struct AnnualBand {
    /// Exact physical bed elevation.
    pub bed: HeightMm,
    /// Immutable contributing leaf, even after geometrical merging.
    pub owner: BasinId,
    /// Number of 100 m square cells.
    pub cells: u32,
    /// Annual precipitation, litres/year.
    pub precipitation: Litres,
    /// Annual baseline effective land loss A, litres/year.
    pub effective_land_loss: Litres,
    /// Annual open-water evaporation E, litres/year.
    pub open_water_evaporation: Litres,
}

/// Eventual receiving terminal beyond an actual physical spill/reach witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnualDestination {
    /// Another immutable closed terminal.
    Basin(BasinId),
    /// Connected fine-grid ocean.
    Sea,
    /// Actual modeled exterior.
    DomainExport,
}

/// A positive-capacity node in the contracted physical hierarchy.
#[derive(Debug, Clone)]
pub struct AnnualNode {
    /// Canonical physical hierarchy identity.
    pub id: BasinId,
    /// Immediate retained parent.
    pub parent: Option<BasinId>,
    /// Minimum physical descendant bed.
    pub floor: HeightMm,
    /// First height at which this node's own bands become available.
    pub birth: HeightMm,
    /// Actual outward witness; every admitted node has a finite physical spill.
    pub spill: Option<SpillConnection>,
    /// Eventual terminal; required only on roots (child transfers use joins).
    pub destination: Option<AnnualDestination>,
    /// Immediate-child IDs in AnnualInput.children.
    pub children: Range<usize>,
    /// Exact-height bands in AnnualInput.bands, nondecreasing by bed.
    pub bands: Range<usize>,
    /// Source R=P-A; nonzero sources occur only on leaves.
    pub local_runoff: Litres,
    /// Full immutable catchment precipitation, including cells above all lake bands.
    pub local_precipitation: Litres,
    /// Full immutable catchment effective land loss.
    pub local_land_loss: Litres,
}

/// An accepted-tree connection between immediate children at a contracted join.
#[derive(Debug, Clone, Copy)]
pub struct AnnualJoin {
    /// Retained parent, not an eliminated zero-capacity intermediate.
    pub parent: BasinId,
    /// Immediate child on the first side.
    pub left_child: BasinId,
    /// Immediate child on the second side.
    pub right_child: BasinId,
    /// Actual endpoint's immutable receiving leaf on the first side.
    pub left_leaf: BasinId,
    /// Actual endpoint's immutable receiving leaf on the second side.
    pub right_leaf: BasinId,
    /// Unique canonical accepted-edge ordinal, used for tied spill ordering.
    pub witness: u64,
}

/// Borrowed canonical input. Nodes are strictly ordered by BasinId.
#[derive(Clone, Copy)]
pub struct AnnualInput<'a> {
    /// Admitted retained hierarchy nodes.
    pub nodes: &'a [AnnualNode],
    /// Exactly one entry for every nonroot node.
    pub children: &'a [BasinId],
    /// Each physical band occurs exactly once.
    pub bands: &'a [AnnualBand],
    /// A tree of n-1 connections for each n-child join.
    pub joins: &'a [AnnualJoin],
}

/// A strictly positive-depth lake; overflow is supplied by the final witnessed flow pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentativeLake {
    /// Actual connected retained identity.
    pub basin: BasinId,
    /// Whole millimetre representative surface.
    pub surface: HeightMm,
    /// Number of strictly submerged physical cells.
    pub submerged_cells: u32,
    /// Derived geometric volume, litres; not an annual source debit.
    pub geometric_volume: Litres,
    /// Potential outward spill, independent of actual annual overflow.
    pub potential_spill: Option<SpillConnection>,
}

/// Final immutable-owner source/sink for the separate witnessed-tree flow normalizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeafNet {
    /// Immutable physical receiving leaf.
    pub leaf: BasinId,
    /// Highest actually connected retained account; a dry owner retains its leaf ID.
    pub account: BasinId,
    /// Positive-depth lake containing this leaf, if any.
    pub lake: Option<BasinId>,
    /// Supported water surface, absent for a representative dry leaf.
    pub surface: Option<HeightMm>,
    /// Original annual source R.
    pub runoff: Litres,
    /// Fully paid additional evaporation cost D on this owner's wet cells.
    pub paid_wet_cost: Litres,
    /// Explicit partially supported band's evaporation adjustment.
    pub marginal_cost: Litres,
    /// Signed annual source minus both costs; may be negative for an absorbing leaf.
    pub net_litres: i128,
}

/// Final static geometry and exact global annual accounting, with no historical pours.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnualSolution {
    /// Actual connected lakes in canonical identity order.
    pub lakes: Vec<RepresentativeLake>,
    /// Immutable leaves in canonical identity order.
    pub leaf_net: Vec<LeafNet>,
    /// Reclassified direct-lake/land ledger, exactly closed.
    pub balance: AnnualWaterBalance,
    /// Counted validation, band, adjacency and queue work used.
    pub events: u64,
}

/// Explicit admission for the in-memory pure kernel, including its borrowed input payload.
#[derive(Debug, Clone, Copy)]
pub struct AnnualLimits {
    /// Conservative payload reservation; allocator bookkeeping is included in the allowance.
    pub ram_bytes: u64,
    /// Maximum retained nodes.
    pub nodes: u64,
    /// Maximum support-band rows.
    pub bands: u64,
    /// Maximum counted work, including topology ancestry and cluster traversal.
    pub events: u64,
}

/// Failure leaves caller-owned input untouched and publishes no partial solution.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AnnualError {
    /// Malformed topology, source, spans, or ownership.
    #[error("invalid annual input: {0}")]
    Invalid(&'static str),
    /// Checked integer arithmetic failed.
    #[error("annual arithmetic overflow")]
    Overflow,
    /// Admission, allocation, or work cap was exceeded.
    #[error("annual resource limit: {0}")]
    Limit(&'static str),
}

type Result<T> = std::result::Result<T, AnnualError>;
fn add(a: u128, b: u128) -> Result<u128> {
    a.checked_add(b).ok_or(AnnualError::Overflow)
}
fn sub(a: u128, b: u128) -> Result<u128> {
    a.checked_sub(b)
        .ok_or(AnnualError::Invalid("negative accounting amount"))
}
fn vec_of<T: Clone>(n: usize, value: T) -> Result<Vec<T>> {
    let mut v = Vec::new();
    v.try_reserve_exact(n)
        .map_err(|_| AnnualError::Limit("allocation"))?;
    v.resize(n, value);
    Ok(v)
}
fn index(input: &AnnualInput<'_>, id: BasinId) -> Result<usize> {
    input
        .nodes
        .binary_search_by_key(&id, |n| n.id)
        .map_err(|_| AnnualError::Invalid("unknown basin"))
}
struct Meter {
    used: u64,
    max: u64,
}
impl Meter {
    fn tick(&mut self) -> Result<()> {
        self.used = self.used.checked_add(1).ok_or(AnnualError::Overflow)?;
        if self.used > self.max {
            return Err(AnnualError::Limit("events"));
        }
        Ok(())
    }
}
#[derive(Clone, Default)]
struct State {
    active: bool,
    redirect: Option<usize>,
    height: i32,
    full: bool,
    next: usize,
    partial: u128,
    budget: u128,
}
struct Checked {
    parent: Vec<Option<usize>>,
    children: Vec<Vec<usize>>,
    adjacency: Vec<Vec<(usize, usize)>>,
    owner: Vec<usize>,
    source_p: u128,
    source_a: u128,
}
fn descendant(
    mut leaf: usize,
    ancestor: usize,
    parent: &[Option<usize>],
    meter: &mut Meter,
) -> Result<bool> {
    loop {
        meter.tick()?;
        if leaf == ancestor {
            return Ok(true);
        }
        match parent[leaf] {
            Some(p) => leaf = p,
            None => return Ok(false),
        }
    }
}
fn find(mut i: usize, links: &mut [usize], meter: &mut Meter) -> Result<usize> {
    while links[i] != i {
        meter.tick()?;
        links[i] = links[links[i]];
        i = links[i];
    }
    Ok(i)
}

/// Conservative reservation for all input/output/working payloads.
/// Two KiB per node and 512 B per band include vectors, queue nodes, join arrays,
/// geometry/output summaries and worst-case proportional-allocation scratch.
pub fn required_ram(nodes: u64, bands: u64) -> Result<u64> {
    nodes
        .checked_mul(2048)
        .and_then(|n| bands.checked_mul(512).and_then(|b| n.checked_add(b)))
        .and_then(|n| n.checked_add(4096))
        .ok_or(AnnualError::Overflow)
}
fn active(mut i: usize, state: &[State], meter: &mut Meter) -> Result<usize> {
    while let Some(p) = state[i].redirect {
        meter.tick()?;
        i = p;
    }
    Ok(i)
}
fn enqueue(
    i: usize,
    input: &AnnualInput<'_>,
    state: &[State],
    queue: &mut BTreeSet<(i32, BasinId, usize)>,
) {
    queue.insert((state[i].height, input.nodes[i].id, i));
}
fn group_end(input: &AnnualInput<'_>, i: usize, start: usize, meter: &mut Meter) -> Result<usize> {
    let mut end = start + 1;
    while end < input.nodes[i].bands.end && input.bands[end].bed == input.bands[start].bed {
        meter.tick()?;
        end += 1;
    }
    Ok(end)
}

/// Solve the first supported representative configuration from empty.
/// All sources are installed before deterministic event processing. Temporary pours
/// are discarded; `leaf_net` is the input to a separate final witnessed flow pass.
pub fn solve_annual(input: AnnualInput<'_>, limits: AnnualLimits) -> Result<AnnualSolution> {
    let mut meter = Meter {
        used: 0,
        max: limits.events,
    };
    let checked = validate(&input, limits, &mut meter)?;
    let n = input.nodes.len();
    let mut state = vec_of(n, State::default())?;
    let mut paid = vec_of(input.bands.len(), false)?;
    let mut queue = BTreeSet::new();
    for (i, node) in input.nodes.iter().enumerate() {
        state[i].height = node.birth.raw();
        state[i].next = node.bands.start;
        state[i].active = checked.children[i].is_empty();
        state[i].budget = node.local_runoff.0;
        if state[i].active && state[i].budget > 0 {
            enqueue(i, &input, &state, &mut queue);
        }
    }
    let mut seen = vec_of(n, 0_u64)?;
    let mut generation = 0_u64;
    let mut cluster = Vec::new();
    cluster
        .try_reserve_exact(n)
        .map_err(|_| AnnualError::Limit("allocation"))?;
    let mut sea = 0_u128;
    let mut exterior = 0_u128;
    while let Some((_, _, raw)) = queue.pop_first() {
        meter.tick()?;
        let i = active(raw, &state, &mut meter)?;
        if i != raw || !state[i].active {
            continue;
        }
        let node = &input.nodes[i];
        if !state[i].full {
            if state[i].budget == 0 {
                continue;
            }
            if state[i].next < node.bands.end {
                let begin = state[i].next;
                let end = group_end(&input, i, begin, &mut meter)?;
                let mut cost = 0_u128;
                for band in &input.bands[begin..end] {
                    meter.tick()?;
                    cost = add(
                        cost,
                        band.open_water_evaporation.0 - band.effective_land_loss.0,
                    )?;
                }
                let remaining = sub(cost, state[i].partial)?;
                if state[i].budget < remaining {
                    state[i].partial = add(state[i].partial, state[i].budget)?;
                    state[i].budget = 0;
                    state[i].height = input.bands[begin].bed.raw();
                    continue;
                }
                state[i].budget -= remaining;
                state[i].partial = 0;
                paid[begin..end].fill(true);
                state[i].next = end;
                state[i].height = input.bands[begin]
                    .bed
                    .raw()
                    .checked_add(1)
                    .ok_or(AnnualError::Overflow)?;
            } else {
                state[i].height = node
                    .spill
                    .ok_or(AnnualError::Invalid("missing spill"))?
                    .sill
                    .raw();
            }
            let sill = node
                .spill
                .ok_or(AnnualError::Invalid("missing spill"))?
                .sill
                .raw();
            state[i].full = state[i].height == sill;
            if !state[i].full {
                if state[i].budget > 0 {
                    enqueue(i, &input, &state, &mut queue);
                }
                continue;
            }
        }
        if let Some(p) = checked.parent[i] {
            generation = generation.checked_add(1).ok_or(AnnualError::Overflow)?;
            cluster.clear();
            cluster.push(i);
            seen[i] = generation;
            let mut cursor = 0;
            let mut boundary: Option<(u64, usize)> = None;
            let mut surplus = 0_u128;
            while cursor < cluster.len() {
                meter.tick()?;
                let at = cluster[cursor];
                cursor += 1;
                surplus = add(surplus, state[at].budget)?;
                state[at].budget = 0;
                for &(j, other) in &checked.adjacency[at] {
                    meter.tick()?;
                    if state[other].full {
                        if seen[other] != generation {
                            seen[other] = generation;
                            cluster.push(other);
                        }
                    } else {
                        let edge = &input.joins[j];
                        let receiving = if edge.left_child == input.nodes[at].id {
                            edge.right_leaf
                        } else {
                            edge.left_leaf
                        };
                        let candidate = (edge.witness, index(&input, receiving)?);
                        if boundary.is_none_or(|old| candidate < old) {
                            boundary = Some(candidate);
                        }
                    }
                }
            }
            if let Some((_, receiving)) = boundary {
                if surplus > 0 {
                    let target = active(receiving, &state, &mut meter)?;
                    state[target].budget = add(state[target].budget, surplus)?;
                    enqueue(target, &input, &state, &mut queue);
                }
            } else {
                if cluster.len() != checked.children[p].len() {
                    return Err(AnnualError::Invalid("disconnected full join"));
                }
                state[p].active = true;
                state[p].budget = add(state[p].budget, surplus)?;
                for &c in &checked.children[p] {
                    state[c].active = false;
                    state[c].redirect = Some(p);
                }
                if state[p].budget > 0 {
                    enqueue(p, &input, &state, &mut queue);
                }
            }
        } else {
            let amount = state[i].budget;
            state[i].budget = 0;
            match node
                .destination
                .ok_or(AnnualError::Invalid("root destination"))?
            {
                AnnualDestination::Basin(id) => {
                    let target = active(index(&input, id)?, &state, &mut meter)?;
                    state[target].budget = add(state[target].budget, amount)?;
                    if amount > 0 {
                        enqueue(target, &input, &state, &mut queue);
                    }
                }
                AnnualDestination::Sea => sea = add(sea, amount)?,
                AnnualDestination::DomainExport => exterior = add(exterior, amount)?,
            }
        }
    }
    finish(&input, &checked, &state, &paid, sea, exterior, &mut meter)
}

#[cfg(test)]
#[path = "annual_support_tests.rs"]
mod annual_support;
