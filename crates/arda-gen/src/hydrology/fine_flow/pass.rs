//! One flow pass over the receiver forest: checked store access, the tape,
//! each cell's connections and the signed directed-flow emission.

use super::*;

pub(super) struct Connections {
    pub(super) neighbors: [Option<CellIndex>; 8],
    pub(super) boundary: Option<FlowTarget>,
}
pub(super) struct Pass<'a, R: RoutingStore, F: FlowStore, E> {
    pub(super) routing: &'a mut R,
    pub(super) store: &'a mut F,
    pub(super) extent: Extent,
    pub(super) max_x: u32,
    pub(super) max_y: u32,
    pub(super) limits: FlowLimits,
    pub(super) work: FlowWork,
    pub(super) marker: std::marker::PhantomData<fn() -> E>,
}
impl<R: RoutingStore, F: FlowStore, E> Pass<'_, R, F, E> {
    pub(super) fn tick(&mut self) -> Result<(), R::Error, F::Error, E> {
        if self.work.operations >= self.limits.operations {
            return Err(FlowError::Limit);
        }
        self.work.operations += 1;
        Ok(())
    }
    pub(super) fn terrain(
        &mut self,
        at: CellIndex,
    ) -> Result<crate::hydrology::routing::CellRecord, R::Error, F::Error, E> {
        self.tick()?;
        self.routing.read(at).map_err(FlowError::Routing)
    }
    pub(super) fn read(&mut self, at: CellIndex) -> Result<FlowRecord, R::Error, F::Error, E> {
        self.tick()?;
        self.store.read(at).map_err(FlowError::Store)
    }
    pub(super) fn write(
        &mut self,
        at: CellIndex,
        row: FlowRecord,
    ) -> Result<(), R::Error, F::Error, E> {
        self.tick()?;
        self.store.write(at, row).map_err(FlowError::Store)
    }
    pub(super) fn push(&mut self, at: CellIndex) -> Result<(), R::Error, F::Error, E> {
        self.tick()?;
        self.routing
            .push(Tape::Frontier, at)
            .map_err(FlowError::Routing)
    }
    pub(super) fn get(&mut self, position: u32) -> Result<CellIndex, R::Error, F::Error, E> {
        self.tick()?;
        self.routing
            .get(Tape::Frontier, position)
            .map_err(FlowError::Routing)
    }
    pub(super) fn point(&self, at: CellIndex) -> GlobalCell {
        let (x, y) = self.extent.coordinates(at);
        GlobalCell { x, y }
    }
    pub(super) fn rim(&self, at: CellIndex) -> bool {
        let (x, y) = self.extent.coordinates(at);
        x == 0 || y == 0 || x == self.max_x || y == self.max_y
    }
    pub(super) fn neighbor(&self, at: CellIndex, direction: usize) -> Option<CellIndex> {
        let (x, y) = self.extent.coordinates(at);
        let (dx, dy) = DIRECTIONS[direction];
        let x = u32::try_from(i64::from(x) + i64::from(dx)).ok()?;
        let y = u32::try_from(i64::from(y) + i64::from(dy)).ok()?;
        if x > self.max_x || y > self.max_y {
            return None;
        }
        CellIndex::new(y * (self.max_x + 1) + x, self.extent)
    }
    pub(super) fn direction(
        &mut self,
        from: CellIndex,
        to: CellIndex,
    ) -> Result<usize, R::Error, F::Error, E> {
        for direction in 0..8 {
            self.tick()?;
            if self.neighbor(from, direction) == Some(to) {
                return Ok(direction);
            }
        }
        Err(FlowError::Invalid("saddle adjacency"))
    }
    pub(super) fn connections(
        &mut self,
        at: CellIndex,
    ) -> Result<Connections, R::Error, F::Error, E> {
        let terrain = self.terrain(at)?;
        let row = self.read(at)?;
        if terrain.is_marine() {
            return Err(FlowError::Invalid("marine fine-tree vertex"));
        }
        let receiver = terrain
            .receiver(self.extent, at)
            .ok_or(FlowError::Invalid("unresolved receiver"))?;
        let mut boundary = if receiver == Receiver::Stop(OutletKind::DomainExport) {
            if !self.rim(at) {
                return Err(FlowError::Invalid("interior export"));
            }
            Some(FlowTarget::DomainExport)
        } else {
            None
        };
        if row.exterior {
            if !self.rim(at) || boundary.is_some() {
                return Err(FlowError::Invalid("duplicate/interior boundary edge"));
            }
            boundary = Some(FlowTarget::DomainExport);
        }
        let mut edges = [None; 8];
        for (direction, edge) in edges.iter_mut().enumerate() {
            self.tick()?;
            let selected = row.selected & (1 << direction) != 0;
            let Some(to) = self.neighbor(at, direction) else {
                if selected {
                    return Err(FlowError::Invalid("selected edge outside domain"));
                }
                continue;
            };
            let next = self.terrain(to)?;
            let forward = receiver == Receiver::Cell(to);
            if next.is_marine() {
                if selected || forward {
                    if selected && forward || boundary.is_some() {
                        return Err(FlowError::Invalid("duplicate exterior connection"));
                    }
                    boundary = Some(FlowTarget::Sea(self.point(to)));
                }
            } else {
                let other = self.read(to)?;
                if selected != (other.selected & (1 << (7 - direction)) != 0) {
                    return Err(FlowError::Invalid("asymmetric selected edge"));
                }
                let original =
                    forward || next.receiver(self.extent, to) == Some(Receiver::Cell(at));
                if selected && original {
                    return Err(FlowError::Invalid("selected edge duplicates receiver"));
                }
                if selected || original {
                    *edge = Some(to);
                }
            }
        }
        Ok(Connections {
            neighbors: edges,
            boundary,
        })
    }
    pub(super) fn emit(
        &mut self,
        from: CellIndex,
        to: FlowTarget,
        amount: u128,
        output: &mut impl FnMut(DirectedFlow) -> std::result::Result<(), E>,
    ) -> Result<(), R::Error, F::Error, E> {
        if amount == 0 {
            return Ok(());
        }
        let source = self.read(from)?;
        let source_bed = self.terrain(from)?.height();
        let source_head = if source.lake.is_some() {
            source.surface_mm
        } else {
            source_bed
        };
        match to {
            FlowTarget::Cell(at) => {
                let target = self.ordinal(at)?;
                let target_row = self.read(target)?;
                let target_bed = self.terrain(target)?.height();
                if source.lake.is_some() && source.lake == target_row.lake {
                    if source.surface_mm != target_row.surface_mm {
                        return Err(FlowError::Invalid("same-lake surface disagreement"));
                    }
                    return Ok(());
                }
                let target_head = if target_row.lake.is_some() {
                    target_row.surface_mm
                } else {
                    target_bed
                };
                if source_head < target_head {
                    return Err(FlowError::Invalid("unsupported uphill visible flow"));
                }
            }
            FlowTarget::Sea(_) => {
                if source_head < 0 {
                    return Err(FlowError::Invalid("unsupported outflow below sea"));
                }
            }
            FlowTarget::DomainExport => {
                if !self.rim(from) {
                    return Err(FlowError::Invalid("interior domain export"));
                }
            }
        }
        self.tick()?;
        output(DirectedFlow {
            from: self.point(from),
            to,
            annual: Litres(amount),
        })
        .map_err(FlowError::Stream)?;
        self.work.emitted += 1;
        Ok(())
    }
    pub(super) fn ordinal(&self, at: GlobalCell) -> Result<CellIndex, R::Error, F::Error, E> {
        if at.x > self.max_x || at.y > self.max_y {
            return Err(FlowError::Invalid("coordinate extent"));
        }
        CellIndex::new(at.y * (self.max_x + 1) + at.x, self.extent)
            .ok_or(FlowError::Invalid("ordinal extent"))
    }
}
pub(super) fn edge_key(extent: Extent, e: Saddle) -> (i32, u64, u64, u64, u64) {
    let node = |n| match n {
        Node::Closed(at) => extent.anchor_key(at),
        Node::Exterior => u64::MAX,
    };
    let a = extent.anchor_key(e.from);
    let b = e.to.map_or(u64::MAX, |at| extent.anchor_key(at));
    (e.sill_mm, node(e.left), node(e.right), a.min(b), a.max(b))
}
