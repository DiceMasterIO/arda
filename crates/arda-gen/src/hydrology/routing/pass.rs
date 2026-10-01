//! The receiver and ownership passes over any [`RoutingStore`], and the
//! public entry points that run them under explicit work limits.

use super::*;

pub(super) struct Pass<'a, S: RoutingStore> {
    pub(super) store: &'a mut S,
    pub(super) extent: Extent,
    pub(super) limits: Limits,
    pub(super) work: Work,
}
impl<S: RoutingStore> Pass<'_, S> {
    pub(super) fn read(&mut self, i: CellIndex) -> Result<CellRecord, RoutingError<S::Error>> {
        if self.work.cell_reads >= self.limits.cell_reads {
            return Err(RoutingError::WorkLimit);
        }
        self.work.cell_reads += 1;
        self.store.read(i).map_err(RoutingError::Storage)
    }
    pub(super) fn write(
        &mut self,
        i: CellIndex,
        r: CellRecord,
    ) -> Result<(), RoutingError<S::Error>> {
        if self.work.cell_writes >= self.limits.cell_writes {
            return Err(RoutingError::WorkLimit);
        }
        self.work.cell_writes += 1;
        self.store.write(i, r).map_err(RoutingError::Storage)
    }
    pub(super) fn push(&mut self, t: Tape, i: CellIndex) -> Result<(), RoutingError<S::Error>> {
        if self.work.tape_writes >= self.limits.tape_writes
            || self.store.len(t) >= self.extent.cells()
        {
            return Err(RoutingError::WorkLimit);
        }
        self.work.tape_writes += 1;
        self.store.push(t, i).map_err(RoutingError::Storage)
    }
    pub(super) fn get(&mut self, t: Tape, p: u32) -> Result<CellIndex, RoutingError<S::Error>> {
        if self.work.tape_reads >= self.limits.tape_reads {
            return Err(RoutingError::WorkLimit);
        }
        self.work.tape_reads += 1;
        self.store.get(t, p).map_err(RoutingError::Storage)
    }
    pub(super) fn clear(&mut self, t: Tape) -> Result<(), RoutingError<S::Error>> {
        self.store.clear(t).map_err(RoutingError::Storage)
    }
    pub(super) fn receivers(&mut self, order: [usize; 8]) -> Result<(), RoutingError<S::Error>> {
        for raw in 0..self.extent.cells() {
            let start = CellIndex(raw);
            let mut first = self.read(start)?;
            if first.flags & 2 != 0 {
                continue;
            }
            first.flags |= 2;
            if first.is_marine() {
                first.receiver = MARINE;
                first.distance = 0;
                self.write(start, first)?;
                continue;
            }
            let level = first.height;
            self.clear(Tape::Component)?;
            self.clear(Tape::Frontier)?;
            self.write(start, first)?;
            self.push(Tape::Component, start)?;
            let mut minimum = start;
            let mut cursor = 0;
            while cursor < self.store.len(Tape::Component) {
                let i = self.get(Tape::Component, cursor)?;
                cursor += 1;
                minimum = minimum.min(i);
                for &dir in &order {
                    if let Some(j) = self.extent.neighbor(i, dir) {
                        let mut r = self.read(j)?;
                        if r.height == level && !r.is_marine() && r.flags & 2 == 0 {
                            r.flags |= 2;
                            self.write(j, r)?;
                            self.push(Tape::Component, j)?;
                        }
                    }
                }
            }
            for position in 0..self.store.len(Tape::Component) {
                let i = self.get(Tape::Component, position)?;
                let mut r = self.read(i)?;
                let mut best: Option<(i64, i64, CellIndex, usize)> = None;
                for &dir in &order {
                    if let Some(j) = self.extent.neighbor(i, dir) {
                        let z = self.read(j)?.height;
                        if z < level {
                            let drop = i64::from(level) - i64::from(z);
                            let (dx, dy) = DIRS[dir];
                            let distance = if dx != 0 && dy != 0 { 141400 } else { 100000 };
                            let better = best.is_none_or(|(old, old_distance, old_cell, _)| {
                                drop * old_distance > old * distance
                                    || (drop * old_distance == old * distance && j < old_cell)
                            });
                            if better {
                                best = Some((drop, distance, j, dir));
                            }
                        }
                    }
                }
                if let Some((_, _, _, dir)) = best {
                    r.distance = 0;
                    r.receiver = u8::try_from(dir).map_err(|_| RoutingError::InvalidTopology)?;
                } else if self.extent.boundary(i) {
                    r.distance = 0;
                    r.receiver = EXPORT;
                } else {
                    continue;
                }
                self.write(i, r)?;
                self.push(Tape::Frontier, i)?;
            }
            if self.store.len(Tape::Frontier) == 0 {
                let mut r = self.read(minimum)?;
                r.distance = 0;
                r.receiver = CLOSED;
                self.write(minimum, r)?;
                self.push(Tape::Frontier, minimum)?;
            }
            cursor = 0;
            while cursor < self.store.len(Tape::Frontier) {
                let i = self.get(Tape::Frontier, cursor)?;
                cursor += 1;
                let next = self
                    .read(i)?
                    .distance
                    .checked_add(1)
                    .ok_or(RoutingError::InvalidTopology)?;
                for &dir in &order {
                    if let Some(j) = self.extent.neighbor(i, dir) {
                        let mut r = self.read(j)?;
                        if r.height == level && !r.is_marine() && r.distance == NONE {
                            r.distance = next;
                            self.write(j, r)?;
                            self.push(Tape::Frontier, j)?;
                        }
                    }
                }
            }
            for position in 0..self.store.len(Tape::Component) {
                let i = self.get(Tape::Component, position)?;
                let mut r = self.read(i)?;
                if r.distance == 0 {
                    continue;
                }
                let mut best = None;
                for &dir in &order {
                    if let Some(j) = self.extent.neighbor(i, dir) {
                        let q = self.read(j)?;
                        if q.height == level
                            && !q.is_marine()
                            && q.distance.checked_add(1) == Some(r.distance)
                            && best.is_none_or(|(old, _)| j < old)
                        {
                            best = Some((j, dir));
                        }
                    }
                }
                r.receiver = u8::try_from(best.ok_or(RoutingError::InvalidTopology)?.1)
                    .map_err(|_| RoutingError::InvalidTopology)?;
                self.write(i, r)?;
            }
        }
        Ok(())
    }
    pub(super) fn ownership(&mut self) -> Result<(), RoutingError<S::Error>> {
        for raw in 0..self.extent.cells() {
            let start = CellIndex(raw);
            if self.read(start)?.owner != NONE {
                continue;
            }
            self.clear(Tape::Component)?;
            let mut at = start;
            let owner = loop {
                let r = self.read(at)?;
                if r.distance == NONE {
                    return Err(RoutingError::InvalidTopology);
                }
                if r.owner != NONE {
                    break CellIndex(r.owner);
                }
                self.push(Tape::Component, at)?;
                match r
                    .receiver(self.extent, at)
                    .ok_or(RoutingError::InvalidTopology)?
                {
                    Receiver::Stop(_) => break at,
                    Receiver::Cell(next) => {
                        let q = self.read(next)?;
                        if !(q.height < r.height
                            || (q.height == r.height && q.distance < r.distance))
                        {
                            return Err(RoutingError::InvalidTopology);
                        }
                        at = next;
                    }
                }
            };
            for position in (0..self.store.len(Tape::Component)).rev() {
                let i = self.get(Tape::Component, position)?;
                let mut r = self.read(i)?;
                r.owner = owner.0;
                self.write(i, r)?;
            }
        }
        Ok(())
    }
}
/// Assigns exact physical receivers and immutable terminal ownership in one pass.
/// The fixed neighbor order is NW,N,NE,W,E,SW,S,SE; ties use packed-coordinate order.
pub fn route_and_own<S: RoutingStore>(
    store: &mut S,
    limits: Limits,
) -> Result<Work, RoutingError<S::Error>> {
    let extent = store.extent();
    let mut p = Pass {
        store,
        extent,
        limits,
        work: Work::default(),
    };
    p.receivers([0, 1, 2, 3, 4, 5, 6, 7])?;
    p.ownership()?;
    Ok(p.work)
}
/// Clears previous ownership, then validates and resolves written receiver chains.
pub fn resolve_ownership<S: RoutingStore>(
    store: &mut S,
    limits: Limits,
) -> Result<Work, RoutingError<S::Error>> {
    let extent = store.extent();
    let mut p = Pass {
        store,
        extent,
        limits,
        work: Work::default(),
    };
    for raw in 0..extent.cells() {
        let i = CellIndex(raw);
        let mut r = p.read(i)?;
        r.owner = NONE;
        p.write(i, r)?;
    }
    p.ownership()?;
    Ok(p.work)
}
