//! Signed annual flow on the actual receiver forest plus accepted witnessed MST.
use super::routing::{CellIndex, Extent, OutletKind, Receiver, RoutingStore, Tape};
use super::saddles::{Node, Saddle};
use arda_core::hydrology::{BasinId, Litres};
use arda_core::GlobalCell;

mod pass;
use pass::{edge_key, Pass};

/// D8 bit order shared with physical routing scratch.
pub const DIRECTIONS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];
/// Reusable metric scratch, populated by the later global dry/wet-component pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlowMetrics {
    /// Original immutable-owner catchment count.
    pub catchment_cells: u32,
    /// Final directed upstream drainage count.
    pub drainage_cells: u32,
    /// Remaining incoming metric contributions.
    pub pending: u8,
    /// Maximum incoming stream order seen so far.
    pub max_in_order: u8,
    /// Number of incoming contributions sharing that maximum.
    pub max_in_ties: u8,
    /// Final directed stream order.
    pub order: u8,
    /// Chosen downstream channel distance; MAX with no chosen target.
    pub hand_distance_mm: u64,
    /// Chosen actual downstream reference cell.
    pub hand_at: Option<CellIndex>,
    /// Initially dry local R; later the documented scalar annual flow summary.
    pub scalar_annual: u64,
}
impl Default for FlowMetrics {
    fn default() -> Self {
        Self {
            catchment_cells: 0,
            drainage_cells: 0,
            pending: 0,
            max_in_order: 0,
            max_in_ties: 0,
            order: 0,
            hand_distance_mm: u64::MAX,
            hand_at: None,
            scalar_annual: 0,
        }
    }
}
/// One fixed logical fine-flow scratch row; disk encoding is explicit and private.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlowRecord {
    /// Signed source initially, then the complete rooted subtree sum.
    pub net: i128,
    /// Actual neighboring BFS parent; absent before visitation or at an exterior seed.
    pub parent: Option<CellIndex>,
    /// Whether BFS has assigned this cell to the rooted fine tree.
    pub visited: bool,
    /// Accepted-saddle D8 adjacency, using DIRECTIONS order; symmetric on land.
    pub selected: u8,
    /// Accepted saddle with an actual absent outside endpoint.
    pub exterior: bool,
    /// Actual positive-depth representative lake, absent on dry land and ocean.
    pub lake: Option<BasinId>,
    /// Shared representative surface; canonical zero when lake is absent.
    pub surface_mm: i32,
    /// Later metrics share this same private fixed-record store.
    pub metrics: FlowMetrics,
}
/// Indexed fixed rows; the concrete disk adapter and tiny memory fixture share this contract.
pub trait FlowStore {
    /// Backend failure; any failure abandons the private output transaction.
    type Error;
    /// The same exact rectangle as the immutable routing store.
    fn extent(&self) -> Extent;
    /// Read a coherent initialized row.
    /// # Errors
    /// Returns checked index, corruption, resource or I/O failures from the backend.
    fn read(&mut self, at: CellIndex) -> std::result::Result<FlowRecord, Self::Error>;
    /// Write a coherent row, preserving dirty data if eviction fails.
    /// # Errors
    /// Returns checked index, record, resource or I/O failures from the backend.
    fn write(&mut self, at: CellIndex, row: FlowRecord) -> std::result::Result<(), Self::Error>;
}
/// One canonical source row per modeled ordinal, including zero-valued marine rows.
#[derive(Debug, Clone, Copy)]
pub struct CellSource {
    /// Strictly consecutive source ordinal; prevents repeated terminal debits.
    pub at: CellIndex,
    /// Annual physical precipitation P.
    pub precipitation: Litres,
    /// Annual effective land loss A, with 0<=A<=P.
    pub land_loss: Litres,
    /// Annual open-water evaporation E.
    pub evaporation: Litres,
    /// One-time unsupported band's explicit adjustment, only at a closed terminal.
    pub marginal: Litres,
    /// Actual positive-depth lake identity and surface.
    pub wet: Option<(BasinId, i32)>,
}
/// Actual physical endpoint of a directed nonzero annual edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowTarget {
    /// Another nonmarine cell.
    Cell(GlobalCell),
    /// Actual neighboring connected marine cell; sea surface is zero.
    Sea(GlobalCell),
    /// Outside the rectangle at the actual source rim cell.
    DomainExport,
}
/// Streamed visible edge or true exterior export; internal same-lake edges are omitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectedFlow {
    /// Actual upstream cell after signed flow orientation.
    pub from: GlobalCell,
    /// Actual target, without a synthetic boundary coordinate.
    pub to: FlowTarget,
    /// Strictly positive annual whole litres on this edge.
    pub annual: Litres,
}
/// Caller-owned logical work and aggregate-source admission; storage has separate byte/I/O caps.
#[derive(Debug, Clone, Copy)]
pub struct FlowLimits {
    /// Attempted routing/flow/tape operations, source/edge reads and neighbor visits.
    pub operations: u64,
    /// Maximum sum of absolute initialized cell sources, at most 2^80 litres.
    pub absolute_litres: u128,
    /// Exact accepted MST edge count, equal to the closed terminal count.
    pub accepted_edges: u64,
}
/// Counted work and exact external net after all internal transfers cancel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlowWork {
    /// Attempted logical work.
    pub operations: u64,
    /// Nonmarine cells visited and accumulated exactly once.
    pub cells: u64,
    /// Visible nonzero edges, including real sea/rim exports.
    pub emitted: u64,
    /// Sum of nonnegative real exterior exports.
    pub exterior_litres: u128,
}
/// Failure publishes no final geometry; callbacks write only into the private transaction.
#[derive(Debug, thiserror::Error)]
pub enum FlowError<R, F, E> {
    /// Immutable terrain/routing or reusable tape operation failed.
    #[error("fine-flow routing failed")]
    Routing(#[source] R),
    /// Fixed scratch operation failed.
    #[error("fine-flow scratch failed")]
    Store(#[source] F),
    /// Input source/edge or output callback failed.
    #[error("fine-flow stream failed")]
    Stream(#[source] E),
    /// Invalid physical tree, source, geometry or supported direction.
    #[error("invalid fine flow: {0}")]
    Invalid(&'static str),
    /// Numeric, source, or work admission failed.
    #[error("fine-flow resource limit")]
    Limit,
}
type Result<T, R, F, E> = std::result::Result<T, FlowError<R, F, E>>;
/// Solve the real fine tree; neither coarse injection nor a second overflow source is added.
///
/// Routing and wet membership have already been globally solved. Source/edge
/// inputs and output share E; the caller maps concrete stream failures into its
/// orchestration error. Output is private until the full function succeeds.
/// Original receiver records remain unchanged; only their reusable Frontier tape
/// is cleared and reused. Callers must finish any prior tape consumer first.
///
/// # Errors
/// Rejects missing/duplicate sources, invalid saddles, disconnected/cyclic fine
/// trees, repeated or nonterminal marginal debits, each exterior import,
/// unsupported visible flow, storage/stream failures and exhausted reservations.
pub fn run<R: RoutingStore, F: FlowStore, E>(
    routing: &mut R,
    store: &mut F,
    sources: impl IntoIterator<Item = std::result::Result<CellSource, E>>,
    saddles: impl IntoIterator<Item = std::result::Result<Saddle, E>>,
    limits: FlowLimits,
    output: impl FnMut(DirectedFlow) -> std::result::Result<(), E>,
) -> Result<FlowWork, R::Error, F::Error, E> {
    let mut sources = sources.into_iter();
    run_sources(
        routing,
        store,
        |_, _| sources.next().transpose(),
        saddles,
        limits,
        output,
    )
}
/// Request each canonical source with a short borrow of the existing routing store.
/// The callback may read routing while constructing its one source row; it cannot
/// retain that borrow between calls. Exactly N callbacks occur on successful input.
/// This is the production streaming entry point; no second routing store is needed.
/// # Errors
/// Propagates source errors and every physical/resource error documented by `run`.
pub fn run_with_source<R: RoutingStore, F: FlowStore, E>(
    routing: &mut R,
    store: &mut F,
    mut get_source: impl FnMut(&mut R, CellIndex) -> std::result::Result<CellSource, E>,
    saddles: impl IntoIterator<Item = std::result::Result<Saddle, E>>,
    limits: FlowLimits,
    output: impl FnMut(DirectedFlow) -> std::result::Result<(), E>,
) -> Result<FlowWork, R::Error, F::Error, E> {
    run_sources(
        routing,
        store,
        |routing, at| at.map(|at| get_source(routing, at)).transpose(),
        saddles,
        limits,
        output,
    )
}
#[allow(clippy::too_many_lines)]
fn run_sources<R: RoutingStore, F: FlowStore, E>(
    routing: &mut R,
    store: &mut F,
    mut get_source: impl FnMut(&mut R, Option<CellIndex>) -> std::result::Result<Option<CellSource>, E>,
    saddles: impl IntoIterator<Item = std::result::Result<Saddle, E>>,
    limits: FlowLimits,
    mut output: impl FnMut(DirectedFlow) -> std::result::Result<(), E>,
) -> Result<FlowWork, R::Error, F::Error, E> {
    let extent = routing.extent();
    if extent != store.extent() || limits.absolute_litres > 1_u128 << 80 {
        return Err(FlowError::Limit);
    }
    let last =
        CellIndex::new(extent.cells() - 1, extent).ok_or(FlowError::Invalid("empty extent"))?;
    let (max_x, max_y) = extent.coordinates(last);
    let mut p = Pass {
        routing,
        store,
        extent,
        max_x,
        max_y,
        limits,
        work: FlowWork::default(),
        marker: std::marker::PhantomData,
    };
    let mut count = 0_u32;
    let mut closed = 0_u64;
    let mut absolute = 0_u128;
    let mut total = 0_i128;
    loop {
        p.tick()?;
        let expected = CellIndex::new(count, extent);
        let Some(source) = get_source(p.routing, expected).map_err(FlowError::Stream)? else {
            break;
        };
        if count >= extent.cells() || source.at.raw() != count {
            return Err(FlowError::Invalid("source order/count"));
        }
        count += 1;
        let terrain = p.terrain(source.at)?;
        let receiver = terrain
            .receiver(extent, source.at)
            .ok_or(FlowError::Invalid("unresolved source receiver"))?;
        if terrain.owner().is_none() {
            return Err(FlowError::Invalid("unresolved source owner"));
        }
        let mut row = FlowRecord::default();
        if terrain.is_marine() {
            if source.precipitation.0 != 0
                || source.land_loss.0 != 0
                || source.evaporation.0 != 0
                || source.marginal.0 != 0
                || source.wet.is_some()
            {
                return Err(FlowError::Invalid("marine source"));
            }
        } else {
            p.work.cells += 1;
            if receiver == Receiver::Stop(OutletKind::ClosedDepression) {
                if terrain.owner() != Some(source.at) {
                    return Err(FlowError::Invalid("terminal owner"));
                }
                closed += 1;
            }
            if source.land_loss.0 > source.precipitation.0 {
                return Err(FlowError::Invalid("land loss exceeds precipitation"));
            }
            if source.wet.is_none() {
                row.metrics.scalar_annual =
                    u64::try_from(source.precipitation.0 - source.land_loss.0)
                        .map_err(|_| FlowError::Limit)?;
            }
            if source.marginal.0 != 0
                && (receiver != Receiver::Stop(OutletKind::ClosedDepression)
                    || terrain.owner() != Some(source.at))
            {
                return Err(FlowError::Invalid("nonterminal marginal debit"));
            }
            let positive = i128::try_from(source.precipitation.0).map_err(|_| FlowError::Limit)?;
            let loss = i128::try_from(if source.wet.is_some() {
                source.evaporation.0
            } else {
                source.land_loss.0
            })
            .map_err(|_| FlowError::Limit)?;
            let marginal = i128::try_from(source.marginal.0).map_err(|_| FlowError::Limit)?;
            row.net = positive
                .checked_sub(loss)
                .and_then(|n| n.checked_sub(marginal))
                .ok_or(FlowError::Limit)?;
            if let Some((lake, surface)) = source.wet {
                if surface <= terrain.height() {
                    return Err(FlowError::Invalid("nonpositive wet depth"));
                }
                row.lake = Some(lake);
                row.surface_mm = surface;
            }
            absolute = absolute
                .checked_add(row.net.unsigned_abs())
                .ok_or(FlowError::Limit)?;
            if absolute > limits.absolute_litres {
                return Err(FlowError::Limit);
            }
            total = total.checked_add(row.net).ok_or(FlowError::Limit)?;
        }
        p.write(source.at, row)?;
    }
    if count != extent.cells() || closed != limits.accepted_edges {
        return Err(FlowError::Invalid("missing source/terminal count"));
    }
    let mut saddles = saddles.into_iter();
    let mut edge_count = 0_u64;
    let mut previous = None;
    loop {
        p.tick()?;
        let Some(edge) = saddles.next() else { break };
        let edge = edge.map_err(FlowError::Stream)?;
        if edge_count >= limits.accepted_edges
            || Saddle::decode(edge.encode(), extent) != Some(edge)
        {
            return Err(FlowError::Invalid("saddle count/codec"));
        }
        p.tick()?;
        let key = edge_key(extent, edge);
        if previous.is_some_and(|v| v >= key) {
            return Err(FlowError::Invalid("saddle order"));
        }
        previous = Some(key);
        edge_count += 1;
        let Node::Closed(left) = edge.left else {
            return Err(FlowError::Invalid("saddle source"));
        };
        let from = p.terrain(edge.from)?;
        if from.is_marine() || from.owner() != Some(left) {
            return Err(FlowError::Invalid("saddle source ownership"));
        }
        let mut left_row = p.read(edge.from)?;
        if let Some(to) = edge.to {
            let target = p.terrain(to)?;
            if edge.sill_mm != from.height().max(target.height()) {
                return Err(FlowError::Invalid("saddle physical sill"));
            }
            match edge.right {
                Node::Closed(right) => {
                    if target.is_marine() || target.owner() != Some(right) {
                        return Err(FlowError::Invalid("saddle target ownership"));
                    }
                }
                Node::Exterior => {
                    let owner = target
                        .owner()
                        .ok_or(FlowError::Invalid("open target owner"))?;
                    let terminal = p.terrain(owner)?;
                    if !matches!(
                        terminal.receiver(extent, owner),
                        Some(Receiver::Stop(
                            OutletKind::DomainExport | OutletKind::MarineEntry
                        ))
                    ) {
                        return Err(FlowError::Invalid("saddle open target"));
                    }
                }
            }
            let direction = p.direction(edge.from, to)?;
            if left_row.selected & (1 << direction) != 0 {
                return Err(FlowError::Invalid("duplicate selected edge"));
            }
            left_row.selected |= 1 << direction;
            p.write(edge.from, left_row)?;
            if !target.is_marine() {
                let mut right_row = p.read(to)?;
                if right_row.selected & (1 << (7 - direction)) != 0 {
                    return Err(FlowError::Invalid("duplicate reverse selected edge"));
                }
                right_row.selected |= 1 << (7 - direction);
                p.write(to, right_row)?;
            }
        } else {
            if edge.sill_mm != from.height() || !p.rim(edge.from) || left_row.exterior {
                return Err(FlowError::Invalid("invalid actual boundary saddle"));
            }
            left_row.exterior = true;
            p.write(edge.from, left_row)?;
        }
    }
    if edge_count != limits.accepted_edges {
        return Err(FlowError::Invalid("missing saddles"));
    }
    p.tick()?;
    p.routing
        .clear(Tape::Frontier)
        .map_err(FlowError::Routing)?;
    for raw in 0..extent.cells() {
        p.tick()?;
        let at = CellIndex::new(raw, extent).ok_or(FlowError::Invalid("scan ordinal"))?;
        if p.terrain(at)?.is_marine() {
            continue;
        }
        if p.connections(at)?.boundary.is_some() {
            let mut row = p.read(at)?;
            row.visited = true;
            p.write(at, row)?;
            p.push(at)?;
        }
    }
    let mut head = 0_u32;
    while head < p.routing.len(Tape::Frontier) {
        let at = p.get(head)?;
        head += 1;
        let parent = p.read(at)?.parent;
        for to in p.connections(at)?.neighbors.into_iter().flatten() {
            p.tick()?;
            let mut child = p.read(to)?;
            if !child.visited {
                child.visited = true;
                child.parent = Some(at);
                p.write(to, child)?;
                p.push(to)?;
            } else if parent != Some(to) && child.parent != Some(at) {
                return Err(FlowError::Invalid("cycle in fine tree"));
            }
        }
    }
    if u64::from(head) != p.work.cells {
        return Err(FlowError::Invalid("disconnected fine tree"));
    }
    for position in (0..head).rev() {
        let at = p.get(position)?;
        let row = p.read(at)?;
        if let Some(parent) = row.parent {
            let mut target = p.read(parent)?;
            target.net = target.net.checked_add(row.net).ok_or(FlowError::Limit)?;
            p.write(parent, target)?;
            if row.net > 0 {
                p.emit(
                    at,
                    FlowTarget::Cell(p.point(parent)),
                    row.net.unsigned_abs(),
                    &mut output,
                )?;
            } else if row.net < 0 {
                p.emit(
                    parent,
                    FlowTarget::Cell(p.point(at)),
                    row.net.unsigned_abs(),
                    &mut output,
                )?;
            }
        } else {
            if row.net < 0 {
                return Err(FlowError::Invalid("external import"));
            }
            let boundary = p
                .connections(at)?
                .boundary
                .ok_or(FlowError::Invalid("root lacks physical boundary"))?;
            p.work.exterior_litres = p
                .work
                .exterior_litres
                .checked_add(row.net.unsigned_abs())
                .ok_or(FlowError::Limit)?;
            p.emit(at, boundary, row.net.unsigned_abs(), &mut output)?;
        }
    }
    if i128::try_from(p.work.exterior_litres).map_err(|_| FlowError::Limit)? != total {
        return Err(FlowError::Invalid("fine-tree conservation"));
    }
    Ok(p.work)
}
