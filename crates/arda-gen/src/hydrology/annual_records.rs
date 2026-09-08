//! Final representative lake records and whole-domain source/export reconciliation.
#![deny(missing_docs)]

use super::annual::{AnnualNode, AnnualSolution};
use super::annual_aggregation::DirectSource;
use super::annual_transfers::TreeTransfer;
use super::routing::{CellIndex, OutletKind, Receiver, RoutingStore};
use arda_core::hydrology::{
    AnnualWaterBalance, GlobalLake, JunctionId, Litres, ReceivingAccount, SpillConnection,
};
use arda_core::{DischargeMilli, GlobalCell};

const SECONDS: u128 = 31_536_000;

/// Invalid solved authority, checked arithmetic, explicit admission or an original read error.
#[derive(Debug, thiserror::Error)]
pub enum RecordError<E> {
    /// Actual immutable physical receiver read failed.
    #[error("annual record terrain read failed")]
    Source(#[source] E),
    /// Source, state and witnessed transfer authorities disagree.
    #[error("invalid annual records: {0}")]
    Invalid(&'static str),
    /// Whole-litre or rate arithmetic failed.
    #[error("annual record arithmetic overflow")]
    Overflow,
    /// Ordinary payload, counted read or output reservation failed.
    #[error("annual record resource limit")]
    Limit,
}

/// Own output reservation; solution, transfer and node arrays remain caller-owned.
#[derive(Debug, Clone, Copy)]
pub struct RecordLimits {
    /// Conservative output bytes including allocation slack.
    pub ram_bytes: u64,
    /// Maximum transfer target/owner reads attempted.
    pub source_reads: u64,
}

fn spill_key(s: &SpillConnection) -> (u32, u32, u32, u32) {
    (
        s.from.y,
        s.from.x,
        s.to.map_or(u32::MAX, |p| p.y),
        s.to.map_or(u32::MAX, |p| p.x),
    )
}

/// Make one authoritative row per actually connected lake.
/// Annual overflow is the outward supported flux across its associated physical-catchment
/// spill edges. A spilling row names the canonical actively carrying witness; a nonspilling
/// row retains its potential witness. All other carrying witnesses remain in the flow graph.
/// Source precipitation on a dry spill arm is included once in that catchment's outward flux.
pub fn lake_records<R: RoutingStore>(
    routing: &mut R,
    nodes: &[AnnualNode],
    solution: &AnnualSolution,
    transfers: &[TreeTransfer],
    limits: RecordLimits,
) -> Result<Vec<GlobalLake>, RecordError<R::Error>> {
    let count = u64::try_from(solution.lakes.len()).map_err(|_| RecordError::Limit)?;
    if count
        .checked_mul(256)
        .and_then(|n| n.checked_add(4096))
        .is_none_or(|n| n > limits.ram_bytes)
    {
        return Err(RecordError::Limit);
    }
    if nodes.windows(2).any(|p| p[0].id >= p[1].id)
        || solution.lakes.windows(2).any(|p| p[0].basin >= p[1].basin)
        || solution.leaf_net.windows(2).any(|p| p[0].leaf >= p[1].leaf)
    {
        return Err(RecordError::Invalid("canonical identity order"));
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(solution.lakes.len())
        .map_err(|_| RecordError::Limit)?;
    for lake in &solution.lakes {
        let i = nodes
            .binary_search_by_key(&lake.basin, |n| n.id)
            .map_err(|_| RecordError::Invalid("missing lake node"))?;
        if lake.submerged_cells == 0 || nodes[i].floor >= lake.surface {
            return Err(RecordError::Invalid("nonpositive lake geometry"));
        }
        rows.push(GlobalLake {
            basin: lake.basin,
            surface: lake.surface,
            deepest_bed: nodes[i].floor,
            submerged_cells: lake.submerged_cells,
            outlet: lake.potential_spill,
            annual_outflow: Litres(0),
            mean_outflow: DischargeMilli::new(0),
        });
    }
    let extent = routing.extent();
    let last =
        CellIndex::new(extent.cells() - 1, extent).ok_or(RecordError::Invalid("empty domain"))?;
    let (mx, my) = extent.coordinates(last);
    let index = |p: GlobalCell| {
        if p.x > mx || p.y > my {
            return None;
        }
        CellIndex::new(p.y * (mx + 1) + p.x, extent)
    };
    let mut reads = 0_u64;
    for transfer in transfers {
        if transfer.annual_volume.0 == 0 {
            return Err(RecordError::Invalid("zero transfer row"));
        }
        let source = solution
            .leaf_net
            .binary_search_by_key(&transfer.from_leaf, |l| l.leaf)
            .map_err(|_| RecordError::Invalid("missing source leaf"))?;
        let id = solution.leaf_net[source]
            .lake
            .ok_or(RecordError::Invalid("dry source spills"))?;
        let row = rows
            .binary_search_by_key(&id, |l| l.basin)
            .map_err(|_| RecordError::Invalid("missing emitting lake"))?;
        if rows[row].surface != transfer.sill || index(transfer.from).is_none() {
            return Err(RecordError::Invalid("unsupported source spill"));
        }
        let receiving = if let Some(to) = transfer.to {
            if transfer.from.x.abs_diff(to.x) > 1
                || transfer.from.y.abs_diff(to.y) > 1
                || transfer.from == to
            {
                return Err(RecordError::Invalid("spill adjacency"));
            }
            let at = index(to).ok_or(RecordError::Invalid("target domain"))?;
            if reads >= limits.source_reads {
                return Err(RecordError::Limit);
            }
            reads += 1;
            let target = routing.read(at).map_err(RecordError::Source)?;
            if target.is_marine() {
                ReceivingAccount::Sea
            } else {
                let owner = target
                    .owner()
                    .ok_or(RecordError::Invalid("unset target owner"))?;
                let leaf = arda_core::hydrology::BasinId(extent.anchor_key(owner));
                let wet = if let Ok(i) = solution.leaf_net.binary_search_by_key(&leaf, |l| l.leaf) {
                    let leaf = &solution.leaf_net[i];
                    match (leaf.lake, leaf.surface) {
                        (Some(id), Some(surface)) if target.height() < surface.raw() => {
                            let receiving = rows
                                .binary_search_by_key(&id, |l| l.basin)
                                .map_err(|_| RecordError::Invalid("missing receiving lake"))?;
                            if rows[receiving].surface != surface || receiving == row {
                                return Err(RecordError::Invalid(
                                    "receiving lake surface or internal transfer",
                                ));
                            }
                            Some(id)
                        }
                        (Some(_), Some(_)) | (None, None) => None,
                        _ => {
                            return Err(RecordError::Invalid("receiving wet identity and surface"))
                        }
                    }
                } else {
                    None
                };
                if let Some(id) = wet {
                    ReceivingAccount::Lake(id)
                } else if target.receiver(extent, at)
                    == Some(Receiver::Stop(OutletKind::ClosedDepression))
                {
                    ReceivingAccount::Lake(arda_core::hydrology::BasinId(extent.anchor_key(at)))
                } else {
                    ReceivingAccount::Junction(JunctionId::at(to))
                }
            }
        } else {
            if transfer.from.x != 0
                && transfer.from.y != 0
                && transfer.from.x != mx
                && transfer.from.y != my
            {
                return Err(RecordError::Invalid("interior domain export"));
            }
            ReceivingAccount::DomainExport
        };
        let spill = SpillConnection {
            from: transfer.from,
            to: transfer.to,
            sill: transfer.sill,
            receiving,
        };
        let lake = &mut rows[row];
        if lake.annual_outflow.0 == 0
            || lake
                .outlet
                .is_none_or(|s| spill_key(&spill) < spill_key(&s))
        {
            lake.outlet = Some(spill);
        }
        lake.annual_outflow.0 = lake
            .annual_outflow
            .0
            .checked_add(transfer.annual_volume.0)
            .ok_or(RecordError::Overflow)?;
    }
    for row in &mut rows {
        row.mean_outflow = DischargeMilli::new(
            u64::try_from(row.annual_outflow.0 / SECONDS).map_err(|_| RecordError::Overflow)?,
        );
    }
    Ok(rows)
}

/// Add direct open catchments and split final exports by their real fine-grid destination.
/// The annual solver's total export must agree with the physical pass; assigning a root's
/// provisional sea/domain bucket never overrides actual fine boundary geometry.
pub fn reconcile_balance<E>(
    mut closed: AnnualWaterBalance,
    sea: DirectSource,
    domain: DirectSource,
    actual_sea: Litres,
    actual_domain: Litres,
) -> Result<AnnualWaterBalance, RecordError<E>> {
    let add = |a: u128, b: u128| a.checked_add(b).ok_or(RecordError::Overflow);
    for source in [sea, domain] {
        if add(source.land_loss.0, source.runoff.0)? != source.precipitation.0 {
            return Err(RecordError::Invalid("direct source ledger"));
        }
    }
    let predicted = add(
        add(closed.sea_outflow.0, closed.domain_outflow.0)?,
        add(sea.runoff.0, domain.runoff.0)?,
    )?;
    if predicted != add(actual_sea.0, actual_domain.0)? {
        return Err(RecordError::Invalid("annual/fine export disagreement"));
    }
    closed.land_precipitation.0 = add(
        closed.land_precipitation.0,
        add(sea.precipitation.0, domain.precipitation.0)?,
    )?;
    closed.land_loss.0 = add(
        closed.land_loss.0,
        add(sea.land_loss.0, domain.land_loss.0)?,
    )?;
    closed.sea_outflow = actual_sea;
    closed.domain_outflow = actual_domain;
    let sources = add(closed.land_precipitation.0, closed.lake_precipitation.0)?;
    let losses = add(
        add(closed.land_loss.0, closed.lake_evaporation.0)?,
        closed.marginal_evaporation.0,
    )?;
    if sources != add(losses, add(actual_sea.0, actual_domain.0)?)?
        || closed.land_loss.0 > closed.land_precipitation.0
    {
        return Err(RecordError::Invalid("whole-domain ledger"));
    }
    Ok(closed)
}
