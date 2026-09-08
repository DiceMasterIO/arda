//! Typed handoff from accepted physical MST streams to the checked hierarchy.
use super::{
    hierarchy::{self, ElderLink, Minimum, NodeRow, Store, UnionRow},
    mst::{MstProduct, SortError, StageError},
    routing::RoutingStore,
    witness_binding::{BindingError, BreakpointRegistry, WitnessResolver},
};
use arda_core::{
    formats::hydrology::{BasinNodeRow, TableSpan},
    hydrology::BasinId,
};
use std::marker::PhantomData;

/// Preserve the original source/storage/stream errors across the shared Store API.
#[derive(Debug, thiserror::Error)]
pub enum HandoffError<D, R> {
    /// Actual hierarchy backend failure.
    #[error("hierarchy storage failed")]
    Storage(#[source] D),
    /// Actual routing, geometry or registry failure.
    #[error("witness binding failed")]
    Binding(#[source] BindingError<R>),
    /// Exact private minimum stream failure.
    #[error(transparent)]
    Minimum(#[from] StageError),
    /// Exact accepted-witness stream failure.
    #[error(transparent)]
    Edge(#[from] SortError),
}
/// Successful hierarchy work/source reads, or a preserved typed handoff failure.
pub type HandoffResult<D, R> =
    Result<(hierarchy::Work, u64), hierarchy::BuildError<HandoffError<D, R>>>;
struct Joined<'a, S, R> {
    inner: &'a mut S,
    routing: PhantomData<R>,
}
impl<S: Store, R> Store for Joined<'_, S, R> {
    type Error = HandoffError<S::Error, R>;
    fn reserve(&mut self, u: u64, n: u64) -> Result<(), Self::Error> {
        self.inner.reserve(u, n).map_err(HandoffError::Storage)
    }
    fn union(&mut self, at: u64) -> Result<UnionRow, Self::Error> {
        self.inner.union(at).map_err(HandoffError::Storage)
    }
    fn put_union(&mut self, at: u64, row: UnionRow) -> Result<(), Self::Error> {
        self.inner.put_union(at, row).map_err(HandoffError::Storage)
    }
    fn node(&mut self, at: u64) -> Result<NodeRow, Self::Error> {
        self.inner.node(at).map_err(HandoffError::Storage)
    }
    fn put_node(&mut self, at: u64, row: NodeRow) -> Result<(), Self::Error> {
        self.inner.put_node(at, row).map_err(HandoffError::Storage)
    }
    fn link(&mut self, parent: BasinId, child: BasinId) -> Result<(), Self::Error> {
        self.inner
            .link(parent, child)
            .map_err(HandoffError::Storage)
    }
    fn finish_links(&mut self, n: u64) -> Result<(), Self::Error> {
        self.inner.finish_links(n).map_err(HandoffError::Storage)
    }
    fn child_span(&mut self, parent: BasinId) -> Result<TableSpan, Self::Error> {
        self.inner.child_span(parent).map_err(HandoffError::Storage)
    }
    fn emit(&mut self, row: BasinNodeRow) -> Result<(), Self::Error> {
        self.inner.emit(row).map_err(HandoffError::Storage)
    }
    fn emit_elder(&mut self, row: ElderLink) -> Result<(), Self::Error> {
        self.inner.emit_elder(row).map_err(HandoffError::Storage)
    }
}
/// Consume the actual bounded MST streams and validate every emitted physical witness.
/// The caller finalizes its disk Store only after success, then consumes registry
/// records as required reach starts and uses their destination for annual bookkeeping.
pub fn build_from_mst<S: Store, R: RoutingStore>(
    product: MstProduct,
    routing: &mut R,
    store: &mut S,
    registry: &mut BreakpointRegistry,
    limits: hierarchy::Limits,
    maximum_source_reads: u64,
) -> HandoffResult<S::Error, R::Error> {
    if product.extent != routing.extent() {
        return Err(hierarchy::BuildError::Invalid(
            "MST/receiver extent mismatch",
        ));
    }
    let mut joined = Joined {
        inner: store,
        routing: PhantomData,
    };
    let mut resolver = WitnessResolver::new(routing, registry, maximum_source_reads);
    let minima = product.minima.map(|m| {
        m.map(|m| Minimum {
            at: m.at,
            floor_mm: m.floor_mm,
        })
        .map_err(HandoffError::Minimum)
    });
    let edges = product.edges.map(|e| e.map_err(HandoffError::Edge));
    let work = hierarchy::build(
        product.extent,
        u64::from(product.work.leaves),
        minima,
        edges,
        &mut joined,
        limits,
        |w| resolver.resolve(w).map_err(HandoffError::Binding),
    )?;
    Ok((work, resolver.reads()))
}
