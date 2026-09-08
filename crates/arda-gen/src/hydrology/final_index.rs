//! Bounded saved-feature tables and area lookup after the physical solve completes.
#![deny(missing_docs)]

use super::annual::{AnnualNode, LeafNet};
use super::extraction::{self, ExtractError, ExtractionBudget};
use super::fine_flow::FlowStore;
use super::routing::{CellIndex, OutletKind, Receiver, RoutingStore};
use arda_core::hydrology::{
    self, AnnualCatchment, BasinId, CatchmentId, CrossingId, GlobalReach, HydrologyDomain, ReachId,
    ReceivingAccount, SharedCrossing,
};
use arda_core::{AreaCoord, GlobalCell};
use std::collections::{BTreeMap, BTreeSet};

/// Explicit feature-index reservation; no full fine-cell array is allocated here.
#[derive(Debug, Clone, Copy)]
pub struct IndexLimits {
    /// Own maps, final vectors and spatial-reference overlap in bytes.
    pub ram_bytes: u64,
    /// Maximum sum of actual reach, crossing and catchment rows.
    pub records: u64,
    /// Maximum copied area-to-reach references, including physical width halo.
    pub area_references: u64,
    /// Source reads, extraction work, retained rows and spatial candidate visits.
    pub operations: u64,
}

/// Completed immutable tables, with canonical row ordering for fixed-record writers.
pub struct FinalIndex {
    /// Exact modeled and exported rectangle.
    pub domain: HydrologyDomain,
    /// Sorted unique actual saved edges and explicit terminal points.
    pub reaches: Vec<GlobalReach>,
    /// Sorted unique real boundary adjacencies.
    pub crossings: Vec<SharedCrossing>,
    /// Sorted nonempty original contributing owners, including direct marine terminals.
    pub catchments: Vec<AnnualCatchment>,
    /// Row-major exported area lists: width seeds plus one-hop incident endpoint rows.
    /// Each list is sorted/unique; added incident rows are not recursively expanded.
    pub area_reaches: Vec<Vec<usize>>,
    /// Counted work used by the completed construction.
    pub operations: u64,
    /// Conservative admitted peak owned payload for the actual record counts.
    pub payload_bytes: u64,
}

/// Original backend failures, invalid completed state or exceeded explicit admission.
#[derive(Debug, thiserror::Error)]
pub enum IndexError<R, F> {
    /// Physical terrain/owner read failed.
    #[error("final feature-index terrain read failed")]
    Routing(#[source] R),
    /// Completed metrics read failed.
    #[error("final feature-index flow read failed")]
    Flow(#[source] F),
    /// Physical saved-edge extraction failed.
    #[error("final feature-index extraction failed")]
    Extract(#[source] ExtractError<R, F>),
    /// The supplied completed authorities disagree.
    #[error("invalid final feature index: {0}")]
    Invalid(&'static str),
    /// Checked count, allocation, RAM or work admission failed.
    #[error("final feature-index resource limit")]
    Limit,
}

/// Conservative owned payload, including map-to-vector and area-vector growth overlap.
#[must_use]
pub fn required_ram(records: u64, references: u64, areas: u64) -> Option<u64> {
    records
        .checked_mul(512)?
        .checked_add(references.checked_mul(96)?)?
        .checked_add(areas.checked_mul(4096)?)?
        .checked_add(65536)
}

fn charge<R, F>(work: &mut ExtractionBudget, n: u64) -> Result<(), IndexError<R, F>> {
    work.remaining = work.remaining.checked_sub(n).ok_or(IndexError::Limit)?;
    Ok(())
}
fn admit<R, F>(
    limits: IndexLimits,
    records: u64,
    references: u64,
    areas: u64,
) -> Result<u64, IndexError<R, F>> {
    let bytes = required_ram(records, references, areas).ok_or(IndexError::Limit)?;
    if records > limits.records || references > limits.area_references || bytes > limits.ram_bytes {
        return Err(IndexError::Limit);
    }
    Ok(bytes)
}
fn collect<K, V, R, F>(map: BTreeMap<K, V>) -> Result<Vec<V>, IndexError<R, F>> {
    let mut rows = Vec::new();
    rows.try_reserve_exact(map.len())
        .map_err(|_| IndexError::Limit)?;
    rows.extend(map.into_values());
    Ok(rows)
}

#[allow(clippy::too_many_arguments)]
fn incident_reference<R, F>(
    id: ReachId,
    reaches: &[GlobalReach],
    selected: &mut BTreeSet<usize>,
    references: &mut u64,
    work: &mut ExtractionBudget,
    limits: IndexLimits,
    records: u64,
    areas: u64,
) -> Result<(), IndexError<R, F>> {
    charge(work, 1)?;
    if let Ok(ordinal) = reaches.binary_search_by_key(&id, |r| r.id) {
        if !selected.contains(&ordinal) {
            *references = references.checked_add(1).ok_or(IndexError::Limit)?;
            admit(limits, records, *references, areas)?;
            charge(work, 1)?;
            selected.insert(ordinal);
        }
    }
    Ok(())
}

/// Index only saved water features after fine-flow and metric success.
/// Borrowed terrain, flow pages, hierarchy nodes and leaf state have separate reservations.
/// Ordered-map lookup cost is bounded by log(record cap); library comparisons are not
/// individual operation-counter events. All fine record reads and spatial candidates are.
#[allow(clippy::too_many_arguments)]
pub fn build<R: RoutingStore, F: FlowStore>(
    routing: &mut R,
    flow: &mut F,
    domain: HydrologyDomain,
    nodes: &[AnnualNode],
    leaves: &[LeafNet],
    limits: IndexLimits,
) -> Result<FinalIndex, IndexError<R::Error, F::Error>> {
    let mut work = ExtractionBudget {
        remaining: limits.operations,
    };
    charge(
        &mut work,
        u64::try_from(nodes.len())
            .map_err(|_| IndexError::Limit)?
            .checked_add(u64::try_from(leaves.len()).map_err(|_| IndexError::Limit)?)
            .ok_or(IndexError::Limit)?,
    )?;
    let extent = routing.extent();
    let last =
        CellIndex::new(extent.cells() - 1, extent).ok_or(IndexError::Invalid("empty extent"))?;
    let (mx, my) = extent.coordinates(last);
    if flow.extent() != extent
        || domain.width_cells != mx + 1
        || domain.height_cells != my + 1
        || domain.exported_areas_wide == 0
        || domain.exported_areas_high == 0
        || domain.exported_areas_wide > domain.width_cells / 512
        || domain.exported_areas_high > domain.height_cells / 512
        || nodes.windows(2).any(|v| v[0].id >= v[1].id)
        || leaves.windows(2).any(|v| v[0].leaf >= v[1].leaf)
    {
        return Err(IndexError::Invalid("domain or authority order"));
    }
    let areas = u64::from(domain.exported_areas_wide) * u64::from(domain.exported_areas_high);
    admit(limits, 0, 0, areas)?;
    let mut reaches = BTreeMap::<ReachId, GlobalReach>::new();
    let mut crossings = BTreeMap::<CrossingId, SharedCrossing>::new();
    let mut catchments = BTreeMap::<CatchmentId, AnnualCatchment>::new();
    let mut records = 0_u64;
    let mut source_cells = 0_u64;
    let mut owner_cells = 0_u64;
    for raw in 0..extent.cells() {
        charge(&mut work, 2)?;
        let at = CellIndex::new(raw, extent).ok_or(IndexError::Invalid("cell coordinate"))?;
        let terrain = routing.read(at).map_err(IndexError::Routing)?;
        let water = flow.read(at).map_err(IndexError::Flow)?;
        if !terrain.is_marine() {
            source_cells += 1;
        }
        let receiver = terrain
            .receiver(extent, at)
            .ok_or(IndexError::Invalid("unfinished routing"))?;
        if terrain.is_marine() && receiver != Receiver::Stop(OutletKind::MarineEntry) {
            return Err(IndexError::Invalid("marine routing terminal"));
        }
        if let Receiver::Stop(kind) = receiver {
            if terrain.owner() != Some(at)
                || (!terrain.is_marine() && water.metrics.catchment_cells == 0)
            {
                return Err(IndexError::Invalid("terminal catchment ownership"));
            }
            let count = water.metrics.catchment_cells;
            if count > 0 {
                owner_cells = owner_cells
                    .checked_add(u64::from(count))
                    .ok_or(IndexError::Limit)?;
                if owner_cells > u64::from(extent.cells()) {
                    return Err(IndexError::Invalid("catchment counts exceed domain"));
                }
                let (x, y) = extent.coordinates(at);
                let terminal = GlobalCell { x, y };
                let catchment = CatchmentId(extent.anchor_key(at));
                let (basin, representative_lake, potential_spill, receiving) = match kind {
                    OutletKind::MarineEntry => {
                        if !terrain.is_marine() {
                            return Err(IndexError::Invalid("nonmarine sea terminal"));
                        }
                        (None, None, None, ReceivingAccount::Sea)
                    }
                    OutletKind::DomainExport => (None, None, None, ReceivingAccount::DomainExport),
                    OutletKind::ClosedDepression => {
                        let id = BasinId(catchment.0);
                        let i = nodes
                            .binary_search_by_key(&id, |n| n.id)
                            .map_err(|_| IndexError::Invalid("missing physical leaf"))?;
                        let j = leaves
                            .binary_search_by_key(&id, |n| n.leaf)
                            .map_err(|_| IndexError::Invalid("missing final leaf state"))?;
                        if !nodes[i].children.is_empty() || water.lake != leaves[j].lake {
                            return Err(IndexError::Invalid("terminal leaf state"));
                        }
                        let spill = nodes[i]
                            .spill
                            .ok_or(IndexError::Invalid("physical leaf lacks spill"))?;
                        (Some(id), water.lake, Some(spill), spill.receiving)
                    }
                };
                records = records.checked_add(1).ok_or(IndexError::Limit)?;
                admit(limits, records, 0, areas)?;
                charge(&mut work, 1)?;
                if catchments
                    .insert(
                        catchment,
                        AnnualCatchment {
                            catchment,
                            terminal,
                            contributing_cells: count,
                            basin,
                            representative_lake,
                            potential_spill,
                            receiving,
                        },
                    )
                    .is_some()
                {
                    return Err(IndexError::Invalid("duplicate catchment"));
                }
            }
        }
        if !matches!(receiver, Receiver::Stop(_)) && water.metrics.catchment_cells != 0 {
            return Err(IndexError::Invalid("nonterminal catchment count"));
        }
        let batch = extraction::cell(routing, flow, at, &mut work).map_err(IndexError::Extract)?;
        for reach in batch.reaches.into_iter().flatten() {
            records = records.checked_add(1).ok_or(IndexError::Limit)?;
            admit(limits, records, 0, areas)?;
            charge(&mut work, 1)?;
            if reaches.insert(reach.id, reach).is_some() {
                return Err(IndexError::Invalid("duplicate reach"));
            }
        }
        for crossing in batch.crossings.into_iter().flatten() {
            records = records.checked_add(1).ok_or(IndexError::Limit)?;
            admit(limits, records, 0, areas)?;
            charge(&mut work, 1)?;
            if crossings.insert(crossing.id, crossing).is_some() {
                return Err(IndexError::Invalid("duplicate crossing"));
            }
        }
    }
    if owner_cells != source_cells {
        return Err(IndexError::Invalid(
            "catchment count differs from nonmarine sources",
        ));
    }
    charge(&mut work, records)?;
    let reaches = collect(reaches)?;
    let crossings = collect(crossings)?;
    let catchments: Vec<AnnualCatchment> = collect(catchments)?;
    let mut area_reaches = Vec::new();
    let area_count = usize::try_from(areas).map_err(|_| IndexError::Limit)?;
    area_reaches
        .try_reserve_exact(area_count)
        .map_err(|_| IndexError::Limit)?;
    charge(&mut work, areas)?;
    area_reaches.resize_with(area_count, Vec::new);
    let mut references = 0_u64;
    for (i, reach) in reaches.iter().enumerate() {
        charge(&mut work, 1)?;
        if catchments
            .binary_search_by_key(&reach.catchment, |c| c.catchment)
            .is_err()
        {
            return Err(IndexError::Invalid(
                "reach owner has no contributing catchment",
            ));
        }
        let radius = if reach.mean_discharge.raw() >= 40 {
            let width =
                hydrology::channel_width_dm(reach.mean_discharge).ok_or(IndexError::Limit)?;
            width
                .div_ceil(2000)
                .checked_add(1)
                .ok_or(IndexError::Limit)?
        } else {
            0
        };
        // Include a conservative physical half-width plus a cell for preview/cap rounding.
        // Clip against exported tiles; hydrology-only fringe rows remain globally saved.
        let min_x = reach.from.x.min(reach.to.x).saturating_sub(radius) / 512;
        let min_y = reach.from.y.min(reach.to.y).saturating_sub(radius) / 512;
        let max_x = reach.from.x.max(reach.to.x).saturating_add(radius) / 512;
        let max_y = reach.from.y.max(reach.to.y).saturating_add(radius) / 512;
        if min_x >= domain.exported_areas_wide || min_y >= domain.exported_areas_high {
            continue;
        }
        for y in min_y..=max_y.min(domain.exported_areas_high - 1) {
            for x in min_x..=max_x.min(domain.exported_areas_wide - 1) {
                charge(&mut work, 1)?;
                references = references.checked_add(1).ok_or(IndexError::Limit)?;
                admit(limits, records, references, areas)?;
                let at = usize::try_from(y * domain.exported_areas_wide + x)
                    .map_err(|_| IndexError::Limit)?;
                let list = &mut area_reaches[at];
                list.try_reserve(1).map_err(|_| IndexError::Limit)?;
                list.push(i);
            }
        }
    }
    // Expand only the original physical-width seeds. A newly copied incident row
    // never becomes another seed, so this cannot walk an unbounded downstream network.
    for list in &mut area_reaches {
        charge(&mut work, 1)?;
        let seeds = std::mem::take(list);
        let mut selected = BTreeSet::new();
        for &i in &seeds {
            charge(&mut work, 1)?;
            selected.insert(i);
        }
        for &i in &seeds {
            charge(&mut work, 1)?;
            let seed = &reaches[i];
            for endpoint in [seed.from, seed.to] {
                charge(&mut work, 1)?;
                for (dx, dy) in super::fine_flow::DIRECTIONS {
                    charge(&mut work, 1)?;
                    let Ok(x) = u32::try_from(i64::from(endpoint.x) + i64::from(dx)) else {
                        continue;
                    };
                    let Ok(y) = u32::try_from(i64::from(endpoint.y) + i64::from(dy)) else {
                        continue;
                    };
                    if x >= domain.width_cells || y >= domain.height_cells {
                        continue;
                    }
                    let other = GlobalCell { x, y };
                    for id in [
                        ReachId::from_step(endpoint, other),
                        ReachId::from_step(other, endpoint),
                    ] {
                        let id = id.ok_or(IndexError::Invalid("incident step identity"))?;
                        incident_reference(
                            id,
                            &reaches,
                            &mut selected,
                            &mut references,
                            &mut work,
                            limits,
                            records,
                            areas,
                        )?;
                    }
                }
                let id = ReachId::point(endpoint)
                    .ok_or(IndexError::Invalid("incident point identity"))?;
                incident_reference(
                    id,
                    &reaches,
                    &mut selected,
                    &mut references,
                    &mut work,
                    limits,
                    records,
                    areas,
                )?;
            }
        }
        charge(
            &mut work,
            u64::try_from(selected.len()).map_err(|_| IndexError::Limit)?,
        )?;
        list.try_reserve_exact(selected.len())
            .map_err(|_| IndexError::Limit)?;
        list.extend(selected);
    }
    let payload_bytes = admit(limits, records, references, areas)?;
    Ok(FinalIndex {
        domain,
        reaches,
        crossings,
        catchments,
        area_reaches,
        operations: limits.operations - work.remaining,
        payload_bytes,
    })
}

impl FinalIndex {
    /// Exact selected global ordinals for one exported area; no neighboring output is read.
    pub fn area(&self, area: AreaCoord) -> Option<&[usize]> {
        let x = u32::try_from(area.x).ok()?;
        let y = u32::try_from(area.y).ok()?;
        if x >= self.domain.exported_areas_wide || y >= self.domain.exported_areas_high {
            return None;
        }
        let at = usize::try_from(
            y.checked_mul(self.domain.exported_areas_wide)?
                .checked_add(x)?,
        )
        .ok()?;
        self.area_reaches.get(at).map(Vec::as_slice)
    }
}
