//! Bounded drainage, stream order and downstream HAND references on final fine flow.
#![deny(missing_docs)]
use super::fine_flow::{FlowMetrics, FlowRecord, FlowStore};
use super::flow_adjacency::{self, AdjacencyError, MetricEdge};
use super::routing::{CellIndex, CellRecord, Receiver, RoutingStore, Tape};
use arda_core::hydrology::{BasinId, GlobalLake};

mod stages;

/// Annual litres giving the first 40 L/s channel; no rounded per-edge comparisons.
pub const CHANNEL_ANNUAL: u64 = 40 * 31_536_000;
/// Admission for summaries; routing/flow page and tape storage is separately admitted.
#[derive(Debug, Clone, Copy)]
pub struct MetricsLimits {
    /// Borrowed lake records, owned lake states and boundary-edge index allowance.
    pub ram_bytes: u64,
    /// Maximum actual positive-depth connected lakes.
    pub lakes: u64,
    /// Maximum outward boundary edges retained across all collapsed lakes.
    pub lake_edges: u64,
    /// Attempted record/tape operations, loop visits and key comparisons; each
    /// adjacency query reserves its full MAX_RECORD_READS before reading.
    pub operations: u64,
}
/// Work actually admitted; this is not a wall-clock deadline.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MetricsWork {
    /// Counted logical work, including conservative adjacency reservations.
    pub operations: u64,
    /// Fixed-size incident queries.
    pub adjacency_queries: u64,
    /// Dry nodes plus one node per connected lake.
    pub nodes: u64,
    /// Directed outgoing lake edges in the bounded index.
    pub lake_edges: u64,
    /// Dry cells whose scalar annual amount reaches the channel threshold.
    pub channel_cells: u64,
    /// Greatest completed stream order, including retained lake order.
    pub max_order: u8,
}
/// A failed private pass publishes no final metrics or area output.
#[derive(Debug, thiserror::Error)]
pub enum MetricsError<R, F> {
    /// Original immutable-record or reusable-tape backend error.
    #[error("flow metrics routing failed")]
    Routing(#[source] R),
    /// Original flow-record backend error.
    #[error("flow metrics scratch failed")]
    Store(#[source] F),
    /// The completed physical adjacency is inconsistent.
    #[error("flow metrics adjacency failed")]
    Adjacency(#[source] AdjacencyError<R, F>),
    /// Fresh-state, lake, DAG or physical-account authority is inconsistent.
    #[error("invalid flow metrics: {0}")]
    Invalid(&'static str),
    /// A count, annual amount, stream order or distance does not fit its type.
    #[error("flow metrics arithmetic overflow")]
    Overflow,
    /// An explicit count, allocation, working-set or logical-work ceiling failed.
    #[error("flow metrics resource limit: {0}")]
    Limit(&'static str),
}
type Result<T, R, F> = std::result::Result<T, MetricsError<R, F>>;
/// Conservative 64-bit payload allowance, including output-index construction.
#[must_use]
pub fn required_ram(lakes: u64, edges: u64) -> Option<u64> {
    lakes
        .checked_mul(512)?
        .checked_add(edges.checked_mul(64)?)?
        .checked_add(8192)
}
struct LakeState {
    id: BasinId,
    surface: i32,
    expected: u32,
    anchor: Option<CellIndex>,
    wet: u32,
    drainage: u32,
    pending: u64,
    max_order: u8,
    ties: u64,
    order: u8,
    edges: std::ops::Range<usize>,
    edge_count: usize,
    cursor: usize,
    processed: bool,
}
struct Pass<'a, R: RoutingStore, F: FlowStore> {
    routing: &'a mut R,
    flow: &'a mut F,
    limits: MetricsLimits,
    work: MetricsWork,
    lakes: Vec<LakeState>,
    edges: Vec<Option<MetricEdge>>,
}
impl<R: RoutingStore, F: FlowStore> Pass<'_, R, F> {
    fn tick(&mut self, n: u64) -> Result<(), R::Error, F::Error> {
        self.work.operations = self
            .work
            .operations
            .checked_add(n)
            .ok_or(MetricsError::Overflow)?;
        if self.work.operations > self.limits.operations {
            return Err(MetricsError::Limit("operations"));
        }
        Ok(())
    }
    fn terrain(&mut self, at: CellIndex) -> Result<CellRecord, R::Error, F::Error> {
        self.tick(1)?;
        self.routing.read(at).map_err(MetricsError::Routing)
    }
    fn read(&mut self, at: CellIndex) -> Result<FlowRecord, R::Error, F::Error> {
        self.tick(1)?;
        self.flow.read(at).map_err(MetricsError::Store)
    }
    fn write(&mut self, at: CellIndex, row: FlowRecord) -> Result<(), R::Error, F::Error> {
        self.tick(1)?;
        self.flow.write(at, row).map_err(MetricsError::Store)
    }
    fn push(&mut self, t: Tape, at: CellIndex) -> Result<(), R::Error, F::Error> {
        self.tick(1)?;
        self.routing.push(t, at).map_err(MetricsError::Routing)
    }
    fn get(&mut self, t: Tape, pos: u32) -> Result<CellIndex, R::Error, F::Error> {
        self.tick(1)?;
        self.routing.get(t, pos).map_err(MetricsError::Routing)
    }
    fn clear(&mut self, t: Tape) -> Result<(), R::Error, F::Error> {
        self.tick(1)?;
        self.routing.clear(t).map_err(MetricsError::Routing)
    }
    fn incident(&mut self, at: CellIndex) -> Result<[Option<MetricEdge>; 9], R::Error, F::Error> {
        self.tick(flow_adjacency::MAX_RECORD_READS)?;
        self.work.adjacency_queries = self
            .work
            .adjacency_queries
            .checked_add(1)
            .ok_or(MetricsError::Overflow)?;
        flow_adjacency::incident(self.routing, self.flow, at).map_err(MetricsError::Adjacency)
    }
    fn lake(&mut self, id: BasinId) -> Result<usize, R::Error, F::Error> {
        let (mut lo, mut hi) = (0, self.lakes.len());
        while lo < hi {
            self.tick(1)?;
            let mid = lo + (hi - lo) / 2;
            match self.lakes[mid].id.cmp(&id) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => return Ok(mid),
            }
        }
        Err(MetricsError::Invalid("unknown wet lake"))
    }
    fn index(&self, raw: u32) -> Result<CellIndex, R::Error, F::Error> {
        CellIndex::new(raw, self.routing.extent()).ok_or(MetricsError::Invalid("cell index"))
    }
    fn validate_fresh(&mut self) -> Result<(), R::Error, F::Error> {
        for raw in 0..self.routing.extent().cells() {
            self.tick(1)?;
            let at = self.index(raw)?;
            let terrain = self.terrain(at)?;
            let row = self.read(at)?;
            if row.metrics
                != (FlowMetrics {
                    scalar_annual: row.metrics.scalar_annual,
                    ..FlowMetrics::default()
                })
            {
                return Err(MetricsError::Invalid("metrics scratch is not fresh"));
            }
            if terrain.is_marine() {
                if row.lake.is_some() || row.metrics.scalar_annual != 0 {
                    return Err(MetricsError::Invalid("marine metric source"));
                }
                continue;
            }
            if !row.visited {
                return Err(MetricsError::Invalid("unfinished fine flow"));
            }
            if let Some(id) = row.lake {
                let j = self.lake(id)?;
                let lake = &mut self.lakes[j];
                if row.surface_mm != lake.surface
                    || row.surface_mm <= terrain.height()
                    || row.metrics.scalar_annual != 0
                {
                    return Err(MetricsError::Invalid("wet surface or source"));
                }
                lake.wet = lake.wet.checked_add(1).ok_or(MetricsError::Overflow)?;
                if lake.anchor.is_none() {
                    lake.anchor = Some(at);
                }
            } else {
                self.work.nodes = self
                    .work
                    .nodes
                    .checked_add(1)
                    .ok_or(MetricsError::Overflow)?;
            }
        }
        for j in 0..self.lakes.len() {
            self.tick(1)?;
            let lake = &mut self.lakes[j];
            if lake.wet != lake.expected || lake.anchor.is_none() {
                return Err(MetricsError::Invalid("lake membership count"));
            }
            lake.drainage = lake.wet;
        }
        self.work.nodes = self
            .work
            .nodes
            .checked_add(u64::try_from(self.lakes.len()).map_err(|_| MetricsError::Overflow)?)
            .ok_or(MetricsError::Overflow)?;
        Ok(())
    }
}
fn stream_order(maximum: u8, ties: u64, starts: bool) -> Option<u8> {
    if maximum == 0 {
        Some(u8::from(starts))
    } else {
        maximum.checked_add(u8::from(ties >= 2))
    }
}
/// Populate metrics in one private transaction, preserving every original routing record.
///
/// `lakes` is the canonical final positive-depth table. The fine-flow store must
/// hold a successfully completed tree solve with fresh default metric fields,
/// except scalar_annual contains each dry cell's original local runoff.
/// Drainage follows zero original edges as well as positive final edges, with
/// connected lakes collapsed before stream-order propagation. There is no
/// downstream reconvergence in the contracted underlying tree. HAND stops at
/// standing water and exterior; it never crosses a collapsed lake.
///
/// # Errors
/// Rejects mismatched extents, nonfresh scratch, bad memberships/adjacency, graph
/// cycles, arithmetic overflow, allocation and explicit RAM/count/work limits.
pub fn run<R: RoutingStore, F: FlowStore>(
    routing: &mut R,
    flow: &mut F,
    lakes: &[GlobalLake],
    limits: MetricsLimits,
) -> Result<MetricsWork, R::Error, F::Error> {
    if routing.extent() != flow.extent() {
        return Err(MetricsError::Invalid("extent mismatch"));
    }
    let count = u64::try_from(lakes.len()).map_err(|_| MetricsError::Overflow)?;
    if count > limits.lakes
        || count > u64::from(routing.extent().cells())
        || required_ram(count, 0).is_none_or(|n| n > limits.ram_bytes)
    {
        return Err(MetricsError::Limit("lakes or RAM"));
    }
    if count > limits.operations {
        return Err(MetricsError::Limit("operations"));
    }
    let mut states = Vec::new();
    states
        .try_reserve_exact(lakes.len())
        .map_err(|_| MetricsError::Limit("allocation"))?;
    for (i, lake) in lakes.iter().enumerate() {
        if (i > 0 && lakes[i - 1].basin >= lake.basin)
            || lake.submerged_cells == 0
            || lake.surface <= lake.deepest_bed
        {
            return Err(MetricsError::Invalid("lake table"));
        }
        states.push(LakeState {
            id: lake.basin,
            surface: lake.surface.raw(),
            expected: lake.submerged_cells,
            anchor: None,
            wet: 0,
            drainage: 0,
            pending: 0,
            max_order: 0,
            ties: 0,
            order: 0,
            edges: 0..0,
            edge_count: 0,
            cursor: 0,
            processed: false,
        });
    }
    let mut pass = Pass {
        routing,
        flow,
        limits,
        work: MetricsWork::default(),
        lakes: states,
        edges: Vec::new(),
    };
    pass.tick(count)?;
    pass.validate_fresh()?;
    pass.initialize()?;
    pass.forward()?;
    pass.finish()?;
    Ok(pass.work)
}
