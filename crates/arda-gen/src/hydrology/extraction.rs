//! Exact saved edge authorities and disjoint area channel-cell ownership.
use super::fine_flow::{FlowRecord, FlowStore};
use super::flow_adjacency::{self, MetricEdge};
use super::routing::{CellIndex, CellRecord, OutletKind, Receiver, RoutingStore};
use arda_core::hydrology::{
    self, BasinId, CatchmentId, ChannelEdge, CrossingId, GlobalReach, JunctionId, Litres, ReachId,
    ReceivingAccount, SharedCrossing,
};
use arda_core::{DischargeMilli, GlobalCell};

/// Conservative per-cell logical work reservation, including every fixed-degree destination query.
pub const MAX_CELL_WORK: u64 = 640;

const YEAR: u128 = 31_536_000;
const INITIATION: u128 = 40 * YEAR;
/// Local ending is resolved after the bounded area's owned cells are numbered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellEnd {
    /// Connected sea at an actual neighbor.
    Sea,
    /// Actual positive-depth lake.
    Lake,
    /// Physical dry absorbing basin terminal.
    Basin,
    /// Actual modeled exterior at the source/target rim.
    DomainExport,
    /// Actual dry downstream point and number of positive outgoing branches.
    Junction {
        /// Actual global dry destination.
        at: GlobalCell,
        /// Number of positive real outgoing edges, including any exterior edge.
        branches: u8,
    },
}
/// One exclusively owned channel cell, separate from the global geometric endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnedChannelCell {
    /// Actual dry initiated cell; each area includes this coordinate exactly once.
    pub at: GlobalCell,
    /// Dominant outgoing edge or the explicit absorbing/export point.
    pub reach: ReachId,
    /// Actual geometric target, which need not be owned by this local fragment.
    pub to: GlobalCell,
    /// Final directed order at this dry cell.
    pub order: u8,
    /// Exact documented scalar mean Q.
    pub discharge: DischargeMilli,
    /// Physical full width from the shared integer formula.
    pub width_dm: u32,
    /// Physical ending of the owned edge, independent of owned course membership.
    pub end: CellEnd,
}
/// Fixed-capacity result, emitted once while visiting its source cell.
pub struct CellBatch {
    /// At most eight D8 edges plus one explicit point; empty slots follow records.
    pub reaches: [Option<GlobalReach>; 9],
    /// Canonical crossings for saved D8 edges, including required small split branches.
    pub crossings: [Option<SharedCrossing>; 8],
    /// Initiated physical D8 geometry only; point footprints use global point rows.
    pub channels: [Option<ChannelEdge>; 8],
    /// Disjoint local ownership; absent on lake/sea and noninitiated dry cells.
    pub owned: Option<OwnedChannelCell>,
}
impl Default for CellBatch {
    fn default() -> Self {
        Self {
            reaches: std::array::from_fn(|_| None),
            crossings: std::array::from_fn(|_| None),
            channels: [None; 8],
            owned: None,
        }
    }
}
/// Caller-owned operation reservation, shared across the whole extraction pass.
#[derive(Debug, Clone, Copy)]
pub struct ExtractionBudget {
    /// Remaining reserved record reads and explicit fixed-size edge visits.
    pub remaining: u64,
}
/// Typed authority/read/resource failure; batches are published only after success.
#[derive(Debug, thiserror::Error)]
pub enum ExtractError<R, F> {
    /// Physical routing read failed.
    #[error("saved-flow routing read failed")]
    Routing(#[source] R),
    /// Completed flow/metric read failed.
    #[error("saved-flow metric read failed")]
    Flow(#[source] F),
    /// Incident tree authority failed validation.
    #[error("saved-flow adjacency failed")]
    Adjacency(#[source] flow_adjacency::AdjacencyError<R, F>),
    /// Inconsistent final source, geometry, identities or metrics.
    #[error("invalid saved-flow extraction: {0}")]
    Invalid(&'static str),
    /// Operation/numeric admission failed.
    #[error("saved-flow extraction resource limit")]
    Limit,
}
type Result<T, R, F> = std::result::Result<T, ExtractError<R, F>>;
struct Pass<'a, R: RoutingStore, F: FlowStore> {
    routing: &'a mut R,
    flow: &'a mut F,
    budget: &'a mut ExtractionBudget,
}
impl<R: RoutingStore, F: FlowStore> Pass<'_, R, F> {
    fn charge(&mut self, n: u64) -> Result<(), R::Error, F::Error> {
        self.budget.remaining = self
            .budget
            .remaining
            .checked_sub(n)
            .ok_or(ExtractError::Limit)?;
        Ok(())
    }
    fn terrain(&mut self, at: CellIndex) -> Result<CellRecord, R::Error, F::Error> {
        self.charge(1)?;
        self.routing.read(at).map_err(ExtractError::Routing)
    }
    fn read(&mut self, at: CellIndex) -> Result<FlowRecord, R::Error, F::Error> {
        self.charge(1)?;
        self.flow.read(at).map_err(ExtractError::Flow)
    }
    fn incident(&mut self, at: CellIndex) -> Result<[Option<MetricEdge>; 9], R::Error, F::Error> {
        self.charge(flow_adjacency::MAX_RECORD_READS)?;
        flow_adjacency::incident(self.routing, self.flow, at).map_err(ExtractError::Adjacency)
    }
    fn point(&self, at: CellIndex) -> GlobalCell {
        let (x, y) = self.routing.extent().coordinates(at);
        GlobalCell { x, y }
    }
    fn destination(
        &mut self,
        at: CellIndex,
    ) -> Result<(ReceivingAccount, CellEnd), R::Error, F::Error> {
        let terrain = self.terrain(at)?;
        if terrain.is_marine() {
            return Ok((ReceivingAccount::Sea, CellEnd::Sea));
        }
        let row = self.read(at)?;
        if let Some(lake) = row.lake {
            return Ok((ReceivingAccount::Lake(lake), CellEnd::Lake));
        }
        let mut branches = 0_u8;
        let mut only_export = false;
        for edge in self.incident(at)?.into_iter().flatten() {
            self.charge(1)?;
            if edge.from == at && edge.annual.0 > 0 {
                branches += 1;
                only_export = edge.to.is_none();
            }
        }
        if branches == 0 {
            if terrain.receiver(self.routing.extent(), at)
                != Some(Receiver::Stop(OutletKind::ClosedDepression))
                || terrain.owner() != Some(at)
            {
                return Err(ExtractError::Invalid(
                    "dry absorption outside immutable terminal",
                ));
            }
            Ok((
                ReceivingAccount::Lake(BasinId(self.routing.extent().anchor_key(at))),
                CellEnd::Basin,
            ))
        } else if branches == 1 && only_export {
            Ok((ReceivingAccount::DomainExport, CellEnd::DomainExport))
        } else {
            let at = self.point(at);
            Ok((
                ReceivingAccount::Junction(JunctionId::at(at)),
                CellEnd::Junction { at, branches },
            ))
        }
    }
}
fn mean<R, F>(annual: u128) -> Result<DischargeMilli, R, F> {
    Ok(DischargeMilli::new(
        u64::try_from(annual / YEAR).map_err(|_| ExtractError::Limit)?,
    ))
}
fn width<R, F>(q: DischargeMilli) -> Result<u32, R, F> {
    hydrology::channel_width_dm(q)
        .filter(|v| *v > 0)
        .ok_or(ExtractError::Limit)
}
fn append<T, const N: usize, R, F>(rows: &mut [Option<T>; N], value: T) -> Result<(), R, F> {
    let slot = rows
        .iter_mut()
        .find(|v| v.is_none())
        .ok_or(ExtractError::Invalid("fixed batch degree"))?;
    *slot = Some(value);
    Ok(())
}
/// Extract one immutable cell after successful fine flow and final metric completion.
///
/// The caller visits each source once, orders global output tables and collects the
/// exact area/halo context. This performs no routing, annual solve or source injection.
/// At an active split only its immediate positive branches are additionally saved;
/// a small continuation ends at its real Junction without recursive expansion.
///
/// # Errors
/// Returns typed read, geometry, metric, numeric, degree or operation-budget failures.
#[allow(clippy::too_many_lines)]
pub fn cell<R: RoutingStore, F: FlowStore>(
    routing: &mut R,
    flow: &mut F,
    at: CellIndex,
    budget: &mut ExtractionBudget,
) -> Result<CellBatch, R::Error, F::Error> {
    if routing.extent() != flow.extent() || CellIndex::new(at.raw(), routing.extent()) != Some(at) {
        return Err(ExtractError::Invalid("extent"));
    }
    let mut p = Pass {
        routing,
        flow,
        budget,
    };
    let mut batch = CellBatch::default();
    let terrain = p.terrain(at)?;
    if terrain.is_marine() {
        return Ok(batch);
    }
    let row = p.read(at)?;
    if !row.visited {
        return Err(ExtractError::Invalid("unfinished fine flow"));
    }
    let owner = terrain
        .owner()
        .ok_or(ExtractError::Invalid("missing immutable owner"))?;
    let catchment = CatchmentId(p.routing.extent().anchor_key(owner));
    let incident = p.incident(at)?;
    let mut outgoing = 0_u8;
    let mut active = false;
    for edge in incident.into_iter().flatten() {
        p.charge(1)?;
        if edge.annual.0 >= INITIATION {
            active = true;
        }
        if edge.from == at && edge.annual.0 > 0 {
            outgoing += 1;
        }
    }
    let extra = outgoing >= 2 && active;
    let mut dominant: Option<(u128, ReachId, GlobalCell, CellEnd)> = None;
    for edge in incident.into_iter().flatten() {
        p.charge(1)?;
        if edge.from != at || edge.annual.0 == 0 || edge.annual.0 < INITIATION && !extra {
            continue;
        }
        let from = p.point(at);
        let q = mean(edge.annual.0)?;
        let (id, to, receiving, end) = if let Some(target) = edge.to {
            let to = p.point(target);
            let id = ReachId::from_step(from, to).ok_or(ExtractError::Invalid("step identity"))?;
            let (receiving, end) = p.destination(target)?;
            (id, to, receiving, end)
        } else {
            (
                ReachId::point(from).ok_or(ExtractError::Invalid("point identity"))?,
                from,
                ReceivingAccount::DomainExport,
                CellEnd::DomainExport,
            )
        };
        if dominant.is_none_or(|(amount, old, _, _)| {
            edge.annual.0 > amount || edge.annual.0 == amount && id < old
        }) {
            dominant = Some((edge.annual.0, id, to, end));
        }
        if row.metrics.drainage_cells == 0 {
            return Err(ExtractError::Invalid("missing final drainage metrics"));
        }
        let reach = GlobalReach {
            id,
            from,
            to,
            receiving,
            catchment,
            drainage_cells: row.metrics.drainage_cells,
            annual_volume: edge.annual,
            mean_discharge: q,
        };
        if edge.to.is_some() {
            if from.x / 512 != to.x / 512 || from.y / 512 != to.y / 512 {
                p.charge(8)?;
                append(
                    &mut batch.crossings,
                    SharedCrossing {
                        id: CrossingId {
                            low: from.min(to),
                            high: from.max(to),
                        },
                        from,
                        to,
                        reach: id,
                        catchment,
                        drainage_cells: reach.drainage_cells,
                        annual_volume: edge.annual,
                        mean_discharge: q,
                        receiving,
                    },
                )?;
            }
            if edge.annual.0 >= INITIATION {
                let width = width(q)?;
                p.charge(8)?;
                append(
                    &mut batch.channels,
                    ChannelEdge {
                        from,
                        to,
                        from_width_dm: width,
                        to_width_dm: width,
                        discharge: q,
                    },
                )?;
            }
        }
        p.charge(9)?;
        append(&mut batch.reaches, reach)?;
    }
    if row.lake.is_none() && u128::from(row.metrics.scalar_annual) >= INITIATION {
        let scalar = u128::from(row.metrics.scalar_annual);
        let q = mean(scalar)?;
        let width = width(q)?;
        if row.metrics.order == 0 || row.metrics.drainage_cells == 0 {
            return Err(ExtractError::Invalid("initiated cell lacks final metrics"));
        }
        let (reach, to, end) = if let Some((amount, id, to, end)) = dominant {
            if amount != scalar {
                return Err(ExtractError::Invalid(
                    "cell scalar differs from greatest outgoing annual flow",
                ));
            }
            (id, to, end)
        } else {
            if outgoing != 0
                || terrain.owner() != Some(at)
                || terrain.receiver(p.routing.extent(), at)
                    != Some(Receiver::Stop(OutletKind::ClosedDepression))
            {
                return Err(ExtractError::Invalid(
                    "point absorption is not a dry terminal",
                ));
            }
            let from = p.point(at);
            let id =
                ReachId::point(from).ok_or(ExtractError::Invalid("absorbing point identity"))?;
            p.charge(9)?;
            append(
                &mut batch.reaches,
                GlobalReach {
                    id,
                    from,
                    to: from,
                    receiving: ReceivingAccount::Lake(BasinId(p.routing.extent().anchor_key(at))),
                    catchment,
                    drainage_cells: row.metrics.drainage_cells,
                    annual_volume: Litres(scalar),
                    mean_discharge: q,
                },
            )?;
            (id, from, CellEnd::Basin)
        };
        batch.owned = Some(OwnedChannelCell {
            at: p.point(at),
            reach,
            to,
            order: row.metrics.order,
            discharge: q,
            width_dm: width,
            end,
        });
    }
    // Fixed-size stable insertion sort: canonical IDs without an allocator or uncounted library sort.
    for i in 1..9 {
        for j in (1..=i).rev() {
            p.charge(1)?;
            let swap = match (&batch.reaches[j - 1], &batch.reaches[j]) {
                (Some(a), Some(b)) => a.id > b.id,
                (None, Some(_)) => true,
                _ => false,
            };
            if swap {
                batch.reaches.swap(j - 1, j);
            } else {
                break;
            }
        }
    }
    Ok(batch)
}
