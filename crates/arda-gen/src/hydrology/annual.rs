//! Deterministic representative annual support-cost filling on a witnessed hierarchy.
//! This is a static climate-support convention, not a dated storage simulation.
#![deny(missing_docs)]

use arda_core::hydrology::{AnnualWaterBalance, BasinId, Litres, SpillConnection};
use arda_core::HeightMm;
use std::collections::BTreeSet;
use std::ops::Range;

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
fn validate(input: &AnnualInput<'_>, limits: AnnualLimits, meter: &mut Meter) -> Result<Checked> {
    let n = input.nodes.len();
    let b = input.bands.len();
    let nn = u64::try_from(n).map_err(|_| AnnualError::Overflow)?;
    let bb = u64::try_from(b).map_err(|_| AnnualError::Overflow)?;
    if nn > limits.nodes || bb > limits.bands || required_ram(nn, bb)? > limits.ram_bytes {
        return Err(AnnualError::Limit("admission"));
    }
    if input.children.len() > n || input.joins.len() > n {
        return Err(AnnualError::Invalid("too many child/join entries"));
    }
    if input.nodes.windows(2).any(|w| w[0].id >= w[1].id) {
        return Err(AnnualError::Invalid("node identity order"));
    }
    let mut parent = vec_of(n, None)?;
    let mut children = vec_of(n, Vec::new())?;
    let mut adjacency = vec_of(n, Vec::new())?;
    let mut owner = vec_of(b, 0)?;
    let mut child_seen = vec_of(n, false)?;
    let mut child_slots = vec_of(input.children.len(), false)?;
    let mut band_seen = vec_of(b, false)?;
    let mut own_p = vec_of(n, 0_u128)?;
    let mut own_a = vec_of(n, 0_u128)?;
    let mut source_p = 0_u128;
    let mut source_a = 0_u128;
    let mut total_e = 0_u128;
    for (i, node) in input.nodes.iter().enumerate() {
        meter.tick()?;
        if node.children.start > node.children.end
            || node.children.end > input.children.len()
            || node.bands.start >= node.bands.end
            || node.bands.end > b
        {
            return Err(AnnualError::Invalid("node spans"));
        }
        let spill = node
            .spill
            .ok_or(AnnualError::Invalid("missing physical exit"))?;
        if node.floor.raw() > node.birth.raw() || node.birth.raw() >= spill.sill.raw() {
            return Err(AnnualError::Invalid("nonpositive retained capacity"));
        }
        if let Some(p) = node.parent {
            let p = index(input, p)?;
            if input.nodes[p].birth.raw() != spill.sill.raw() {
                return Err(AnnualError::Invalid("parent sill"));
            }
            parent[i] = Some(p);
        } else if node.destination.is_none() {
            return Err(AnnualError::Invalid("root destination"));
        }
        let p = node.local_precipitation.0;
        let a = node.local_land_loss.0;
        if a > p || node.local_runoff.0 != p - a {
            return Err(AnnualError::Invalid("R=P-A"));
        }
        if !node.children.is_empty() && (p != 0 || a != 0) {
            return Err(AnnualError::Invalid("internal source"));
        }
        if node.children.is_empty() && node.floor != node.birth {
            return Err(AnnualError::Invalid("leaf floor"));
        }
        source_p = add(source_p, p)?;
        source_a = add(source_a, a)?;
        for j in node.children.clone() {
            meter.tick()?;
            if child_slots[j] {
                return Err(AnnualError::Invalid("overlapping child span"));
            }
            child_slots[j] = true;
            let c = index(input, input.children[j])?;
            if child_seen[c] || input.nodes[c].parent != Some(node.id) {
                return Err(AnnualError::Invalid("child relation"));
            }
            child_seen[c] = true;
            children[i]
                .try_reserve(1)
                .map_err(|_| AnnualError::Limit("allocation"))?;
            children[i].push(c);
        }
        if !children[i].is_empty()
            && (children[i].len() < 2
                || children[i]
                    .iter()
                    .map(|&c| input.nodes[c].floor.raw())
                    .min()
                    != Some(node.floor.raw()))
        {
            return Err(AnnualError::Invalid("contracted join/floor"));
        }
        let mut previous = None;
        let mut group_owners = BTreeSet::new();
        for j in node.bands.clone() {
            meter.tick()?;
            let band = &input.bands[j];
            if band_seen[j] {
                return Err(AnnualError::Invalid("overlapping band span"));
            }
            band_seen[j] = true;
            if band.cells == 0
                || band.bed.raw() < node.birth.raw()
                || band.bed.raw() >= spill.sill.raw()
                || previous.is_some_and(|z| z > band.bed.raw())
            {
                return Err(AnnualError::Invalid("band geometry/order"));
            }
            if previous != Some(band.bed.raw()) {
                group_owners.clear();
            }
            if !group_owners.insert(band.owner) {
                return Err(AnnualError::Invalid("duplicate band owner/height"));
            }
            previous = Some(band.bed.raw());
            let o = index(input, band.owner)?;
            if !input.nodes[o].children.is_empty() {
                return Err(AnnualError::Invalid("band nonleaf owner"));
            }
            owner[j] = o;
            let p = band.precipitation.0;
            let a = band.effective_land_loss.0;
            let e = band.open_water_evaporation.0;
            if a > p || a > e {
                return Err(AnnualError::Invalid("band P/A/E"));
            }
            own_p[o] = add(own_p[o], p)?;
            own_a[o] = add(own_a[o], a)?;
            total_e = add(total_e, e)?;
        }
        if input.bands[node.bands.start].bed != node.birth {
            return Err(AnnualError::Invalid("missing birth band"));
        }
    }
    if source_p > MAX_ANNUAL || total_e > MAX_ANNUAL {
        return Err(AnnualError::Limit("annual forcing bound"));
    }
    if child_slots.contains(&false) || band_seen.contains(&false) {
        return Err(AnnualError::Invalid("unowned input row"));
    }
    for i in 0..n {
        meter.tick()?;
        if parent[i].is_some() != child_seen[i] {
            return Err(AnnualError::Invalid("unlisted child"));
        }
        if own_p[i] > input.nodes[i].local_precipitation.0
            || own_a[i] > input.nodes[i].local_land_loss.0
        {
            return Err(AnnualError::Invalid("band exceeds owner source"));
        }
        for j in input.nodes[i].bands.clone() {
            if !descendant(owner[j], i, &parent, meter)? {
                return Err(AnnualError::Invalid("band outside owner subtree"));
            }
        }
    }
    let mut union: Vec<usize> = vec_of(n, 0)?;
    for (i, u) in union.iter_mut().enumerate() {
        *u = i;
    }
    let mut join_count = vec_of(n, 0_usize)?;
    let mut witnesses = BTreeSet::new();
    for (j, edge) in input.joins.iter().enumerate() {
        meter.tick()?;
        if !witnesses.insert(edge.witness) {
            return Err(AnnualError::Invalid("duplicate witness"));
        }
        let p = index(input, edge.parent)?;
        let l = index(input, edge.left_child)?;
        let r = index(input, edge.right_child)?;
        let ll = index(input, edge.left_leaf)?;
        let rr = index(input, edge.right_leaf)?;
        if l == r
            || parent[l] != Some(p)
            || parent[r] != Some(p)
            || !children[ll].is_empty()
            || !children[rr].is_empty()
            || !descendant(ll, l, &parent, meter)?
            || !descendant(rr, r, &parent, meter)?
        {
            return Err(AnnualError::Invalid("join endpoint membership"));
        }
        let ul = find(l, &mut union, meter)?;
        let ur = find(r, &mut union, meter)?;
        if ul == ur {
            return Err(AnnualError::Invalid("join cycle"));
        }
        union[ur] = ul;
        join_count[p] += 1;
        adjacency[l]
            .try_reserve(1)
            .map_err(|_| AnnualError::Limit("allocation"))?;
        adjacency[r]
            .try_reserve(1)
            .map_err(|_| AnnualError::Limit("allocation"))?;
        adjacency[l].push((j, r));
        adjacency[r].push((j, l));
    }
    for i in 0..n {
        if join_count[i] != children[i].len().saturating_sub(1) {
            return Err(AnnualError::Invalid("incomplete join tree"));
        }
    }
    // Parent heights are strictly increasing, so parent chains cannot cycle.
    // Cross-root terminal routes must independently form an acyclic graph.
    let mut marks = vec_of(n, 0_u8)?;
    let mut chain = Vec::new();
    chain
        .try_reserve_exact(n)
        .map_err(|_| AnnualError::Limit("allocation"))?;
    for start in 0..n {
        if parent[start].is_some() || marks[start] == 2 {
            continue;
        }
        let mut at = start;
        loop {
            meter.tick()?;
            if marks[at] == 1 {
                return Err(AnnualError::Invalid("root destination cycle"));
            }
            if marks[at] == 2 {
                break;
            }
            marks[at] = 1;
            chain.push(at);
            match input.nodes[at].destination {
                Some(AnnualDestination::Basin(id)) => {
                    at = index(input, id)?;
                    if !children[at].is_empty() {
                        return Err(AnnualError::Invalid("nonleaf receiving terminal"));
                    }
                    while let Some(p) = parent[at] {
                        meter.tick()?;
                        at = p;
                    }
                }
                Some(AnnualDestination::Sea | AnnualDestination::DomainExport) => break,
                None => return Err(AnnualError::Invalid("missing root destination")),
            }
        }
        for i in chain.drain(..) {
            marks[i] = 2;
        }
    }
    Ok(Checked {
        parent,
        children,
        adjacency,
        owner,
        source_p,
        source_a,
    })
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

fn finish(
    input: &AnnualInput<'_>,
    checked: &Checked,
    state: &[State],
    paid: &[bool],
    sea: u128,
    exterior: u128,
    meter: &mut Meter,
) -> Result<AnnualSolution> {
    let n = input.nodes.len();
    let mut leaf_account = vec_of(n, 0_usize)?;
    let mut wet_cost = vec_of(n, 0_u128)?;
    let mut marginal = vec_of(n, 0_u128)?;
    let mut lake_cells = vec_of(n, 0_u32)?;
    let mut lake_volume = vec_of(n, 0_u128)?;
    for (i, node) in input.nodes.iter().enumerate() {
        if !checked.children[i].is_empty() {
            continue;
        }
        let mut at = i;
        let mut account = i;
        while let Some(p) = checked.parent[at] {
            meter.tick()?;
            if (state[p].active || state[p].redirect.is_some())
                && state[p].height > input.nodes[p].birth.raw()
            {
                account = p;
            }
            at = p;
        }
        leaf_account[i] = account;
        if state[account].height <= node.floor.raw() {
            leaf_account[i] = i;
        }
    }
    let mut balance = AnnualWaterBalance {
        land_precipitation: Litres(checked.source_p),
        land_loss: Litres(checked.source_a),
        sea_outflow: Litres(sea),
        domain_outflow: Litres(exterior),
        ..AnnualWaterBalance::default()
    };
    for (j, band) in input.bands.iter().enumerate() {
        meter.tick()?;
        if !paid[j] {
            continue;
        }
        let o = checked.owner[j];
        let a = leaf_account[o];
        let h = state[a].height;
        if h <= band.bed.raw() {
            return Err(AnnualError::Invalid("paid band not geometrically wet"));
        }
        wet_cost[o] = add(
            wet_cost[o],
            band.open_water_evaporation.0 - band.effective_land_loss.0,
        )?;
        balance.land_precipitation.0 = sub(balance.land_precipitation.0, band.precipitation.0)?;
        balance.land_loss.0 = sub(balance.land_loss.0, band.effective_land_loss.0)?;
        balance.lake_precipitation.0 = add(balance.lake_precipitation.0, band.precipitation.0)?;
        balance.lake_evaporation.0 =
            add(balance.lake_evaporation.0, band.open_water_evaporation.0)?;
        lake_cells[a] = lake_cells[a]
            .checked_add(band.cells)
            .ok_or(AnnualError::Overflow)?;
        let depth = u128::try_from(i64::from(h) - i64::from(band.bed.raw()))
            .map_err(|_| AnnualError::Overflow)?;
        let v = depth
            .checked_mul(u128::from(band.cells))
            .and_then(|v| v.checked_mul(10_000))
            .ok_or(AnnualError::Overflow)?;
        lake_volume[a] = add(lake_volume[a], v)?;
    }
    let mut shares: Vec<(usize, u128, u128)> = Vec::new();
    shares
        .try_reserve_exact(input.bands.len())
        .map_err(|_| AnnualError::Limit("allocation"))?;
    for (i, s) in state.iter().enumerate() {
        if s.partial == 0 {
            continue;
        }
        let end = group_end(input, i, s.next, meter)?;
        let mut total = 0_u128;
        for band in &input.bands[s.next..end] {
            total = add(
                total,
                band.open_water_evaporation.0 - band.effective_land_loss.0,
            )?;
        }
        shares.clear();
        let mut allocated = 0_u128;
        for j in s.next..end {
            meter.tick()?;
            let d = input.bands[j].open_water_evaporation.0 - input.bands[j].effective_land_loss.0;
            let product = s.partial.checked_mul(d).ok_or(AnnualError::Overflow)?;
            let whole = product / total;
            shares.push((j, whole, product % total));
            allocated = add(allocated, whole)?;
        }
        // Counted insertion sort avoids an unmetered comparison path. Group size is
        // independently admitted; the work cap stops pathological same-height groups.
        for j in 1..shares.len() {
            let mut k = j;
            while k > 0 {
                meter.tick()?;
                let l = shares[k - 1];
                let r = shares[k];
                let left = (std::cmp::Reverse(l.2), input.bands[l.0].owner);
                let right = (std::cmp::Reverse(r.2), input.bands[r.0].owner);
                if left <= right {
                    break;
                }
                shares.swap(k - 1, k);
                k -= 1;
            }
        }
        let extra = usize::try_from(s.partial - allocated).map_err(|_| AnnualError::Overflow)?;
        if extra > shares.len() {
            return Err(AnnualError::Invalid("proportional remainder"));
        }
        for (k, &(j, whole, _)) in shares.iter().enumerate() {
            let amount = whole + u128::from(k < extra);
            let o = checked.owner[j];
            marginal[o] = add(marginal[o], amount)?;
        }
        balance.marginal_evaporation.0 = add(balance.marginal_evaporation.0, s.partial)?;
    }
    let mut lakes = Vec::new();
    let mut leaf_net = Vec::new();
    lakes
        .try_reserve_exact(n)
        .map_err(|_| AnnualError::Limit("allocation"))?;
    leaf_net
        .try_reserve_exact(n)
        .map_err(|_| AnnualError::Limit("allocation"))?;
    for (i, node) in input.nodes.iter().enumerate() {
        meter.tick()?;
        if lake_cells[i] > 0 {
            lakes.push(RepresentativeLake {
                basin: node.id,
                surface: HeightMm::new(state[i].height),
                submerged_cells: lake_cells[i],
                geometric_volume: Litres(lake_volume[i]),
                potential_spill: node.spill,
            });
        }
        if checked.children[i].is_empty() {
            let a = leaf_account[i];
            let lake = (lake_cells[a] > 0).then_some(input.nodes[a].id);
            let to_signed = |v| i128::try_from(v).map_err(|_| AnnualError::Overflow);
            leaf_net.push(LeafNet {
                leaf: node.id,
                account: if lake.is_some() {
                    input.nodes[a].id
                } else {
                    node.id
                },
                lake,
                surface: lake.map(|_| HeightMm::new(state[a].height)),
                runoff: node.local_runoff,
                paid_wet_cost: Litres(wet_cost[i]),
                marginal_cost: Litres(marginal[i]),
                net_litres: to_signed(node.local_runoff.0)?
                    .checked_sub(to_signed(wet_cost[i])?)
                    .and_then(|v| v.checked_sub(i128::try_from(marginal[i]).ok()?))
                    .ok_or(AnnualError::Overflow)?,
            });
        }
    }
    let inputs = add(balance.land_precipitation.0, balance.lake_precipitation.0)?;
    let losses = add(
        add(balance.land_loss.0, balance.lake_evaporation.0)?,
        balance.marginal_evaporation.0,
    )?;
    if inputs != add(losses, add(sea, exterior)?)? {
        return Err(AnnualError::Invalid("annual conservation"));
    }
    Ok(AnnualSolution {
        lakes,
        leaf_net,
        balance,
        events: meter.used,
    })
}

#[cfg(test)]
#[path = "annual_support_tests.rs"]
mod annual_support;
