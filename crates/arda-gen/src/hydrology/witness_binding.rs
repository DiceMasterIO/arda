//! Bind physical spill witnesses to actual receiver ownership and physical dry junctions.
use super::{
    hierarchy::{BoundSpill, Witness},
    routing::{CellIndex, CellRecord, Extent, OutletKind, Receiver, RoutingStore},
};
use arda_core::{
    hydrology::{BasinId, JunctionId, ReceivingAccount, SpillConnection},
    GlobalCell,
};
use std::collections::BTreeMap;

/// Eventual annual destination, distinct from the visible immediate spill endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    /// Actual closed receiver terminal's immutable physical leaf identity.
    Basin(BasinId),
    /// Connected fine marine authority.
    Sea,
    /// Actual modeled-domain export.
    DomainExport,
}
/// Actual dry breakpoint and original receiver authority, independent of supported flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JunctionBreakpoint {
    /// Packed (y,x) identity of `at`, with no fabricated outgoing direction.
    pub id: JunctionId,
    /// Actual dry target cell where final incident courses meet.
    pub at: GlobalCell,
    /// Original terrain D8 receiver, not the final signed flow direction; None at a dry rim outlet.
    pub original_next: Option<GlobalCell>,
    /// Annual destination obtained from the solved terminal ownership table.
    pub destination: Destination,
}
/// Binding, record-count or caller-admission failure.
#[derive(Debug, thiserror::Error)]
pub enum BindingError<E> {
    /// Caller-owned routing source failure.
    #[error("spill receiver source failed")]
    Source(#[source] E),
    /// Inconsistent physical geometry or solved receiver authority.
    #[error("invalid spill binding: {0}")]
    Invalid(&'static str),
    /// Explicit ordinary-RAM, record or source-read limit.
    #[error("spill binding exceeded {0}")]
    Limit(&'static str),
}
/// Concrete bounded index of actual required dry junctions, ordered by stable ID.
/// The parent transaction owns it until reach extraction consumes these records.
pub struct BreakpointRegistry {
    records: BTreeMap<u64, JunctionBreakpoint>,
    maximum: u64,
}
impl BreakpointRegistry {
    /// Conservative owned-record/tree allowance, including one transient tree node.
    pub fn required_ram(maximum: u64) -> Option<u64> {
        maximum.checked_mul(256)?.checked_add(4096)
    }
    /// Admit the complete declared registry before inserting any record.
    pub fn new<E>(maximum: u64, ram_bytes: u64) -> Result<Self, BindingError<E>> {
        if Self::required_ram(maximum).is_none_or(|need| need > ram_bytes) {
            return Err(BindingError::Limit("breakpoint RAM"));
        }
        Ok(Self {
            records: BTreeMap::new(),
            maximum,
        })
    }
    fn insert<E>(&mut self, record: JunctionBreakpoint) -> Result<(), BindingError<E>> {
        if let Some(old) = self.records.get(&record.id.0) {
            return if *old == record {
                Ok(())
            } else {
                Err(BindingError::Invalid("changed canonical breakpoint"))
            };
        }
        if u64::try_from(self.records.len()).map_err(|_| BindingError::Limit("breakpoint count"))?
            >= self.maximum
        {
            return Err(BindingError::Limit("breakpoint count"));
        }
        self.records.insert(record.id.0, record);
        Ok(())
    }
    /// Actual unique breakpoints, streamed in packed-coordinate identity order.
    pub fn records(&self) -> impl Iterator<Item = &JunctionBreakpoint> {
        self.records.values()
    }
    /// Indexed lookup required by annual binding and final course extraction; absent references are errors.
    pub fn get<E>(&self, id: JunctionId) -> Result<JunctionBreakpoint, BindingError<E>> {
        self.records
            .get(&id.0)
            .copied()
            .ok_or(BindingError::Invalid("unknown junction breakpoint"))
    }
    /// Resolve the annual target without skipping the immediate dry-course identity.
    pub fn destination<E>(
        &self,
        account: ReceivingAccount,
    ) -> Result<Destination, BindingError<E>> {
        Ok(match account {
            ReceivingAccount::Lake(id) => Destination::Basin(id),
            ReceivingAccount::Sea => Destination::Sea,
            ReceivingAccount::DomainExport => Destination::DomainExport,
            ReceivingAccount::Junction(id) => self.get::<E>(id)?.destination,
            ReceivingAccount::Reach(_) => {
                return Err(BindingError::Invalid(
                    "final reach in potential spill authority",
                ))
            }
        })
    }
}
fn point(extent: Extent, at: CellIndex) -> GlobalCell {
    let (x, y) = extent.coordinates(at);
    GlobalCell { x, y }
}
fn ordinal(extent: Extent, at: GlobalCell) -> Option<CellIndex> {
    let (x, y) = extent.coordinates(CellIndex::new(extent.cells() - 1, extent)?);
    if at.x > x || at.y > y {
        return None;
    }
    CellIndex::new(at.y.checked_mul(x + 1)?.checked_add(at.x)?, extent)
}
fn rim(extent: Extent, at: GlobalCell) -> bool {
    let Some(last) = CellIndex::new(extent.cells() - 1, extent) else {
        return false;
    };
    let last = point(extent, last);
    at.x == 0 || at.y == 0 || at.x == last.x || at.y == last.y
}

/// Small indexed reader over the caller's immutable final routing authority.
pub struct WitnessResolver<'a, S: RoutingStore> {
    routing: &'a mut S,
    registry: &'a mut BreakpointRegistry,
    reads: u64,
    maximum_reads: u64,
}
impl<'a, S: RoutingStore> WitnessResolver<'a, S> {
    /// Borrow the accepted source and admitted registry for this complete binding pass.
    pub fn new(
        routing: &'a mut S,
        registry: &'a mut BreakpointRegistry,
        maximum_reads: u64,
    ) -> Self {
        Self {
            routing,
            registry,
            reads: 0,
            maximum_reads,
        }
    }
    /// Attempted indexed source reads; at most four records per witness.
    pub fn reads(&self) -> u64 {
        self.reads
    }
    fn read(&mut self, at: CellIndex) -> Result<CellRecord, BindingError<S::Error>> {
        if self.reads >= self.maximum_reads {
            return Err(BindingError::Limit("routing reads"));
        }
        self.reads += 1;
        self.routing.read(at).map_err(BindingError::Source)
    }
    fn terminal(
        &mut self,
        at: CellIndex,
        row: CellRecord,
    ) -> Result<(CellIndex, Destination), BindingError<S::Error>> {
        let extent = self.routing.extent();
        let owner = row
            .owner()
            .ok_or(BindingError::Invalid("unresolved receiver owner"))?;
        let terminal = if owner == at { row } else { self.read(owner)? };
        if terminal.owner() != Some(owner)
            || terminal.height() > row.height()
            || (matches!(row.receiver(extent, at), Some(Receiver::Stop(_))) && owner != at)
        {
            return Err(BindingError::Invalid("terminal authority"));
        }
        let destination = match terminal.receiver(extent, owner) {
            Some(Receiver::Stop(OutletKind::ClosedDepression)) if !terminal.is_marine() => {
                Destination::Basin(BasinId(extent.anchor_key(owner)))
            }
            Some(Receiver::Stop(OutletKind::MarineEntry))
                if terminal.is_marine() && terminal.height() <= 0 =>
            {
                Destination::Sea
            }
            Some(Receiver::Stop(OutletKind::DomainExport))
                if !terminal.is_marine() && rim(extent, point(extent, owner)) =>
            {
                Destination::DomainExport
            }
            _ => return Err(BindingError::Invalid("terminal classification")),
        };
        Ok((owner, destination))
    }
    /// Verify real source/target geometry and bind the immediate physical account.
    /// The hierarchy verifies returned terminal membership before publishing the row.
    pub fn resolve(&mut self, w: Witness) -> Result<BoundSpill, BindingError<S::Error>> {
        let extent = self.routing.extent();
        let from = ordinal(extent, w.from).ok_or(BindingError::Invalid("source extent"))?;
        let source = self.read(from)?;
        let (source_terminal, source_destination) = self.terminal(from, source)?;
        if source.is_marine() || !matches!(source_destination, Destination::Basin(_)) {
            return Err(BindingError::Invalid(
                "spill source is not a closed catchment",
            ));
        }
        let (mut target_terminal, mut breakpoint) = (None, None);
        let receiving = if let Some(to) = w.to {
            if to == w.from || to.x.abs_diff(w.from.x) > 1 || to.y.abs_diff(w.from.y) > 1 {
                return Err(BindingError::Invalid("nonadjacent witness"));
            }
            let at = ordinal(extent, to).ok_or(BindingError::Invalid("target extent"))?;
            let row = self.read(at)?;
            if source.height().max(row.height()) != w.sill.raw() {
                return Err(BindingError::Invalid("physical sill"));
            }
            let (owner, destination) = self.terminal(at, row)?;
            if matches!(destination, Destination::Basin(_)) {
                target_terminal = Some(point(extent, owner));
            }
            match row
                .receiver(extent, at)
                .ok_or(BindingError::Invalid("unresolved target receiver"))?
            {
                Receiver::Stop(OutletKind::MarineEntry) if row.is_marine() => ReceivingAccount::Sea,
                Receiver::Stop(OutletKind::ClosedDepression) if !row.is_marine() => {
                    ReceivingAccount::Lake(BasinId(extent.anchor_key(at)))
                }
                receiver if !row.is_marine() => {
                    let next = match receiver {
                        Receiver::Cell(next) => Some(point(extent, next)),
                        Receiver::Stop(OutletKind::DomainExport) if rim(extent, to) => None,
                        _ => return Err(BindingError::Invalid("dry target receiver")),
                    };
                    let id = JunctionId::at(to);
                    breakpoint = Some(JunctionBreakpoint {
                        id,
                        at: to,
                        original_next: next,
                        destination,
                    });
                    ReceivingAccount::Junction(id)
                }
                _ => return Err(BindingError::Invalid("marine target receiver")),
            }
        } else {
            if !rim(extent, w.from) || source.height() != w.sill.raw() {
                return Err(BindingError::Invalid("physical domain export"));
            }
            ReceivingAccount::DomainExport
        };
        if let Some(record) = breakpoint {
            self.registry.insert(record)?;
        }
        Ok(BoundSpill {
            spill: SpillConnection {
                from: w.from,
                to: w.to,
                sill: w.sill,
                receiving,
            },
            source_terminal: point(extent, source_terminal),
            target_terminal,
        })
    }
}
