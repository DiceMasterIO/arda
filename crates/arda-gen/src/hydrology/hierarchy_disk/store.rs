//! The hierarchy [`Store`] contract over paged scratch rows, the external
//! child-link sorter and the streamed node and elder outputs.

use super::*;

impl Store for DiskHierarchyStore {
    type Error = DiskError;
    fn reserve(&mut self, unions: u64, nodes: u64) -> Result<()> {
        if self.files.is_some() {
            return Err(DiskError::Invalid("repeated reserve"));
        }
        if unions == 0
            || unions - 1 > u64::from(self.extent.cells())
            || nodes != (unions - 1).saturating_mul(2).saturating_sub(1)
        {
            return Err(DiskError::Invalid("table counts"));
        }
        let available = self
            .meter
            .limits
            .cache_bytes
            .checked_sub(Self::path_charge(&self.directory)?)
            .ok_or(DiskError::Limit("path bytes"))?
            / CACHE_CHARGE;
        if available < 2 {
            return Err(DiskError::Limit("cache bytes"));
        }
        let capacity =
            usize::try_from(available - 1).map_err(|_| DiskError::Limit("cache count"))?;
        if Self::required_bytes(unions, nodes)? > self.meter.limits.scratch_bytes {
            return Err(DiskError::Limit("scratch bytes"));
        }
        let n_pages = pages(unions, 0) + pages(nodes, 1);
        let initial = 4 * HEADER + n_pages * PAGE as u64;
        if u128::from(initial) > self.meter.limits.io_bytes {
            return Err(DiskError::Limit("initial I/O bytes"));
        }
        if 2 * (4 + n_pages) > self.meter.limits.io_operations {
            return Err(DiskError::Limit("initial I/O operations"));
        }
        let mut files = [
            Scratch::create(self.directory.join("hierarchy-unions.pages"))?,
            Scratch::create(self.directory.join("hierarchy-nodes.pages"))?,
            Scratch::create(self.directory.join("hierarchy-basins.rows"))?,
            Scratch::create(self.directory.join("hierarchy-elders.rows"))?,
        ];
        for (kind, f) in files.iter_mut().enumerate() {
            let (count, width) = match kind {
                0 => (unions, UNION_BYTES),
                1 => (nodes, NODE_BYTES),
                2 => (0, BASIN_BYTES),
                _ => (0, ELDER_BYTES),
            };
            f.write(
                0,
                &header(
                    self.extent,
                    kind,
                    count,
                    u32::try_from(width).map_err(|_| DiskError::Invalid("row width"))?,
                )?,
                &mut self.meter,
            )?;
            if kind < 2 {
                for page in 0..pages(count, kind) {
                    f.write(HEADER + page * PAGE as u64, &[0; PAGE], &mut self.meter)?;
                }
            }
        }
        self.files = Some(files);
        self.unions = unions;
        self.nodes = nodes;
        self.capacity = capacity;
        Ok(())
    }
    fn union(&mut self, at: u64) -> Result<UnionRow> {
        let b = self.read_slot::<UNION_BYTES>(0, at)?;
        Ok(private_rows::decode_union(
            &b,
            self.extent,
            at,
            self.unions,
            self.nodes,
        )?)
    }
    fn put_union(&mut self, at: u64, row: UnionRow) -> Result<()> {
        self.active()?;
        let b = private_rows::encode_union(row, self.extent, at, self.unions, self.nodes)?;
        self.put_slot(0, at, &b)
    }
    fn node(&mut self, at: u64) -> Result<NodeRow> {
        let b = self.read_slot::<NODE_BYTES>(1, at)?;
        Ok(private_rows::decode_node(
            &b,
            self.extent,
            at,
            self.unions - 1,
            self.nodes,
        )?)
    }
    fn put_node(&mut self, at: u64, row: NodeRow) -> Result<()> {
        self.active()?;
        let b = private_rows::encode_node(row, self.extent, at, self.unions - 1, self.nodes)?;
        self.put_slot(1, at, &b)
    }
    fn link(&mut self, parent: BasinId, child: BasinId) -> Result<()> {
        self.active()?;
        Ok(self.children.push(parent, child)?)
    }
    fn finish_links(&mut self, count: u64) -> Result<()> {
        self.active()?;
        self.children.finish(count)?;
        self.child_count = count;
        self.children_finished = true;
        Ok(())
    }
    fn child_span(&mut self, parent: BasinId) -> Result<TableSpan> {
        self.active()?;
        Ok(self.children.span(parent)?)
    }
    fn emit(&mut self, row: BasinNodeRow) -> Result<()> {
        self.active()?;
        if !self.children_finished
            || self.emitted_nodes >= self.nodes
            || self.last_node.is_some_and(|id| id >= row.id.0)
        {
            return Err(DiskError::Invalid("basin output count/order"));
        }
        let child_total = self
            .emitted_child_count
            .checked_add(row.children.count)
            .ok_or(DiskError::Invalid("child count overflow"))?;
        if child_total > self.child_count {
            return Err(DiskError::Invalid("too many child references"));
        }
        let b = encode_record(&row).map_err(|_| DiskError::Invalid("public basin codec"))?;
        self.files
            .as_mut()
            .ok_or(DiskError::Invalid("not reserved"))?[2]
            .write(
                HEADER + self.emitted_nodes * BASIN_BYTES as u64,
                &b,
                &mut self.meter,
            )?;
        self.emitted_nodes += 1;
        self.last_node = Some(row.id.0);
        self.emitted_child_count = child_total;
        Ok(())
    }
    fn emit_elder(&mut self, row: ElderLink) -> Result<()> {
        self.active()?;
        if self.emitted_elders >= self.unions - 1
            || self.last_elder.is_some_and(|id| id >= row.leaf.0)
        {
            return Err(DiskError::Invalid("elder output count/order"));
        }
        let b = private_rows::encode_elder(row, self.extent)?;
        self.files
            .as_mut()
            .ok_or(DiskError::Invalid("not reserved"))?[3]
            .write(
                HEADER + self.emitted_elders * ELDER_BYTES as u64,
                &b,
                &mut self.meter,
            )?;
        self.emitted_elders += 1;
        self.last_elder = Some(row.leaf.0);
        Ok(())
    }
}
