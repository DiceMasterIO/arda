//! The metric stages of one pass: initialization from receivers and lakes,
//! forward drainage and stream-order propagation, and the HAND finish.

use super::*;

impl<R: RoutingStore, F: FlowStore> Pass<'_, R, F> {
    pub(super) fn initialize(&mut self) -> Result<(), R::Error, F::Error> {
        for raw in 0..self.routing.extent().cells() {
            self.tick(1)?;
            let at = self.index(raw)?;
            let terrain = self.terrain(at)?;
            if terrain.is_marine() {
                continue;
            }
            let mut row = self.read(at)?;
            let lake = row.lake.map(|id| self.lake(id)).transpose()?;
            let mut incoming = 0_u128;
            let mut pending = 0_u8;
            let mut maximum = 0_u128;
            for edge in self.incident(at)?.into_iter().flatten() {
                self.tick(1)?;
                if edge.from == at {
                    maximum = maximum.max(edge.annual.0);
                    if let Some(j) = lake {
                        self.lakes[j].edge_count = self.lakes[j]
                            .edge_count
                            .checked_add(1)
                            .ok_or(MetricsError::Overflow)?;
                    }
                } else if edge.to == Some(at) {
                    incoming = incoming
                        .checked_add(edge.annual.0)
                        .ok_or(MetricsError::Overflow)?;
                    pending = pending.checked_add(1).ok_or(MetricsError::Overflow)?;
                } else {
                    return Err(MetricsError::Invalid("nonincident edge"));
                }
            }
            if let Some(j) = lake {
                self.lakes[j].pending = self.lakes[j]
                    .pending
                    .checked_add(u64::from(pending))
                    .ok_or(MetricsError::Overflow)?;
            } else {
                row.metrics.pending = pending;
                row.metrics.drainage_cells = 1;
                let amount = if maximum > 0 {
                    maximum
                } else {
                    incoming
                        .checked_add(u128::from(row.metrics.scalar_annual))
                        .ok_or(MetricsError::Overflow)?
                };
                row.metrics.scalar_annual =
                    u64::try_from(amount).map_err(|_| MetricsError::Overflow)?;
                if row.metrics.scalar_annual >= CHANNEL_ANNUAL {
                    self.work.channel_cells += 1;
                }
            }
            self.write(at, row)?;
            let owner = terrain
                .owner()
                .ok_or(MetricsError::Invalid("missing original owner"))?;
            let terminal = self.terrain(owner)?;
            if terminal.owner() != Some(owner)
                || !matches!(
                    terminal.receiver(self.routing.extent(), owner),
                    Some(Receiver::Stop(_))
                )
            {
                return Err(MetricsError::Invalid("original owner is not terminal"));
            }
            let mut owner_row = self.read(owner)?;
            owner_row.metrics.catchment_cells = owner_row
                .metrics
                .catchment_cells
                .checked_add(1)
                .ok_or(MetricsError::Overflow)?;
            self.write(owner, owner_row)?;
        }
        let mut total = 0_usize;
        for j in 0..self.lakes.len() {
            self.tick(1)?;
            let state = &mut self.lakes[j];
            let end = total
                .checked_add(state.edge_count)
                .ok_or(MetricsError::Overflow)?;
            state.edges = total..end;
            state.cursor = total;
            total = end;
        }
        let count = u64::try_from(total).map_err(|_| MetricsError::Overflow)?;
        if count > self.limits.lake_edges
            || required_ram(
                u64::try_from(self.lakes.len()).map_err(|_| MetricsError::Overflow)?,
                count,
            )
            .is_none_or(|n| n > self.limits.ram_bytes)
        {
            return Err(MetricsError::Limit("lake boundary edges or RAM"));
        }
        self.edges
            .try_reserve_exact(total)
            .map_err(|_| MetricsError::Limit("allocation"))?;
        self.edges.resize(total, None);
        self.work.lake_edges = count;
        self.clear(Tape::Component)?;
        self.clear(Tape::Frontier)?;
        for raw in 0..self.routing.extent().cells() {
            self.tick(1)?;
            let at = self.index(raw)?;
            if self.terrain(at)?.is_marine() {
                continue;
            }
            let row = self.read(at)?;
            if let Some(id) = row.lake {
                let j = self.lake(id)?;
                for edge in self.incident(at)?.into_iter().flatten() {
                    self.tick(1)?;
                    if edge.from != at {
                        continue;
                    }
                    let state = &mut self.lakes[j];
                    if state.cursor >= state.edges.end {
                        return Err(MetricsError::Invalid("changed lake edges"));
                    }
                    self.edges[state.cursor] = Some(edge);
                    state.cursor += 1;
                }
            } else if row.metrics.pending == 0 {
                self.push(Tape::Component, at)?;
            }
        }
        for j in 0..self.lakes.len() {
            self.tick(1)?;
            if self.lakes[j].cursor != self.lakes[j].edges.end {
                return Err(MetricsError::Invalid("missing lake edge"));
            }
            if self.lakes[j].pending == 0 {
                self.push(
                    Tape::Component,
                    self.lakes[j]
                        .anchor
                        .ok_or(MetricsError::Invalid("missing lake anchor"))?,
                )?;
            }
        }
        Ok(())
    }
    fn propagate(
        &mut self,
        edge: MetricEdge,
        drainage: u32,
        order: u8,
    ) -> Result<(), R::Error, F::Error> {
        self.tick(1)?;
        let Some(to) = edge.to else {
            return Ok(());
        };
        if self.terrain(to)?.is_marine() {
            return Ok(());
        }
        let mut row = self.read(to)?;
        let contribution = if edge.annual.0 >= u128::from(CHANNEL_ANNUAL) {
            order
        } else {
            0
        };
        if let Some(id) = row.lake {
            let j = self.lake(id)?;
            let state = &mut self.lakes[j];
            if state.pending == 0 || state.processed {
                return Err(MetricsError::Invalid("lake indegree underflow"));
            }
            state.pending -= 1;
            state.drainage = state
                .drainage
                .checked_add(drainage)
                .ok_or(MetricsError::Overflow)?;
            if contribution > state.max_order {
                state.max_order = contribution;
                state.ties = 1;
            } else if contribution > 0 && contribution == state.max_order {
                state.ties = state.ties.checked_add(1).ok_or(MetricsError::Overflow)?;
            }
            if state.pending == 0 {
                let anchor = state
                    .anchor
                    .ok_or(MetricsError::Invalid("missing lake anchor"))?;
                self.push(Tape::Component, anchor)?;
            }
        } else {
            if row.metrics.pending == 0 {
                return Err(MetricsError::Invalid("dry indegree underflow"));
            }
            row.metrics.pending -= 1;
            row.metrics.drainage_cells = row
                .metrics
                .drainage_cells
                .checked_add(drainage)
                .ok_or(MetricsError::Overflow)?;
            if contribution > row.metrics.max_in_order {
                row.metrics.max_in_order = contribution;
                row.metrics.max_in_ties = 1;
            } else if contribution > 0 && contribution == row.metrics.max_in_order {
                row.metrics.max_in_ties = row
                    .metrics
                    .max_in_ties
                    .checked_add(1)
                    .ok_or(MetricsError::Overflow)?;
            }
            let ready = row.metrics.pending == 0;
            self.write(to, row)?;
            if ready {
                self.push(Tape::Component, to)?;
            }
        }
        Ok(())
    }
    pub(super) fn forward(&mut self) -> Result<(), R::Error, F::Error> {
        let mut cursor = 0_u32;
        while cursor < self.routing.len(Tape::Component) {
            self.tick(1)?;
            let at = self.get(Tape::Component, cursor)?;
            cursor = cursor.checked_add(1).ok_or(MetricsError::Overflow)?;
            let mut row = self.read(at)?;
            if let Some(id) = row.lake {
                let j = self.lake(id)?;
                if self.lakes[j].anchor != Some(at)
                    || self.lakes[j].processed
                    || self.lakes[j].pending != 0
                {
                    return Err(MetricsError::Invalid("invalid queued lake"));
                }
                let range = self.lakes[j].edges.clone();
                let mut starts = false;
                for i in range.clone() {
                    self.tick(1)?;
                    let edge =
                        self.edges[i].ok_or(MetricsError::Invalid("missing indexed edge"))?;
                    starts |= edge.annual.0 >= u128::from(CHANNEL_ANNUAL);
                }
                let order = stream_order(self.lakes[j].max_order, self.lakes[j].ties, starts)
                    .ok_or(MetricsError::Overflow)?;
                self.lakes[j].order = order;
                self.lakes[j].processed = true;
                self.work.max_order = self.work.max_order.max(order);
                let drainage = self.lakes[j].drainage;
                for i in range {
                    let edge =
                        self.edges[i].ok_or(MetricsError::Invalid("missing indexed edge"))?;
                    self.propagate(edge, drainage, order)?;
                }
            } else {
                if row.metrics.pending != 0 {
                    return Err(MetricsError::Invalid("queued dry indegree"));
                }
                row.metrics.order = stream_order(
                    row.metrics.max_in_order,
                    u64::from(row.metrics.max_in_ties),
                    row.metrics.scalar_annual >= CHANNEL_ANNUAL,
                )
                .ok_or(MetricsError::Overflow)?;
                self.work.max_order = self.work.max_order.max(row.metrics.order);
                self.write(at, row)?;
                for edge in self.incident(at)?.into_iter().flatten() {
                    self.tick(1)?;
                    if edge.from == at {
                        self.propagate(edge, row.metrics.drainage_cells, row.metrics.order)?;
                    }
                }
            }
            self.push(Tape::Frontier, at)?;
        }
        if u64::from(cursor) != self.work.nodes
            || u64::from(self.routing.len(Tape::Frontier)) != self.work.nodes
        {
            return Err(MetricsError::Invalid("metric graph cycle or missing node"));
        }
        Ok(())
    }
    pub(super) fn finish(&mut self) -> Result<(), R::Error, F::Error> {
        for position in (0..self.routing.len(Tape::Frontier)).rev() {
            self.tick(1)?;
            let at = self.get(Tape::Frontier, position)?;
            let mut row = self.read(at)?;
            if row.lake.is_some() {
                continue;
            }
            let mut best = if row.metrics.scalar_annual >= CHANNEL_ANNUAL {
                Some((0, at))
            } else {
                None
            };
            if best.is_none() {
                for edge in self.incident(at)?.into_iter().flatten() {
                    self.tick(1)?;
                    if edge.from != at {
                        continue;
                    }
                    let Some(to) = edge.to else {
                        continue;
                    };
                    if self.terrain(to)?.is_marine() {
                        continue;
                    }
                    let downstream = self.read(to)?;
                    if downstream.lake.is_some() {
                        continue;
                    }
                    let Some(channel) = downstream.metrics.hand_at else {
                        continue;
                    };
                    let (x, y) = self.routing.extent().coordinates(at);
                    let (tx, ty) = self.routing.extent().coordinates(to);
                    let step = if x == tx || y == ty { 100_000 } else { 141_400 };
                    let distance = downstream
                        .metrics
                        .hand_distance_mm
                        .checked_add(step)
                        .ok_or(MetricsError::Overflow)?;
                    let key = (distance, self.routing.extent().anchor_key(channel));
                    if best.is_none_or(|(d, c)| key < (d, self.routing.extent().anchor_key(c))) {
                        best = Some((distance, channel));
                    }
                }
            }
            (row.metrics.hand_distance_mm, row.metrics.hand_at) =
                best.map_or((u64::MAX, None), |(d, c)| (d, Some(c)));
            self.write(at, row)?;
        }
        for raw in 0..self.routing.extent().cells() {
            self.tick(1)?;
            let at = self.index(raw)?;
            let mut row = self.read(at)?;
            if let Some(id) = row.lake {
                let j = self.lake(id)?;
                row.metrics.drainage_cells = self.lakes[j].drainage;
                row.metrics.order = self.lakes[j].order;
                // Fine pending/order-merge fields are dry-node scratch; lake reduction used its own u64 state.
                row.metrics.pending = 0;
                row.metrics.max_in_order = 0;
                row.metrics.max_in_ties = 0;
                self.write(at, row)?;
            }
        }
        Ok(())
    }
}
