//! One build pass over the caller's store: checked row access, union-find
//! roots, minimum binding and spill assignment, with work accounting.

use super::*;

pub(super) struct Pass<'a, S> {
    pub(super) store: &'a mut S,
    pub(super) limits: Limits,
    pub(super) work: Work,
    pub(super) leaves: u64,
}
impl<S: Store> Pass<'_, S> {
    pub(super) fn op<T>(
        &mut self,
        f: impl FnOnce(&mut S) -> std::result::Result<T, S::Error>,
    ) -> Result<T, S::Error> {
        if self.work.store_operations >= self.limits.store_operations {
            return Err(BuildError::Limit);
        }
        self.work.store_operations += 1;
        f(self.store).map_err(BuildError::Backend)
    }
    pub(super) fn u(&mut self, i: u64) -> Result<UnionRow, S::Error> {
        if i > self.leaves {
            return Err(BuildError::Invalid("union index"));
        }
        self.op(|s| s.union(i))
    }
    pub(super) fn n(&mut self, i: u64) -> Result<NodeRow, S::Error> {
        self.op(|s| s.node(i))
    }
    pub(super) fn root(&mut self, mut i: u64, before: Option<i32>) -> Result<u64, S::Error> {
        // Union by rank limits valid depth to log2(leaves+1); 64 also terminates corrupt cycles.
        for _ in 0..64 {
            let r = self.u(i)?;
            if r.parent == i {
                return Ok(i);
            }
            let at = r
                .joined_at
                .ok_or(BuildError::Invalid("union parent lacks event"))?;
            if before.is_some_and(|h| at >= h) {
                return Ok(i);
            }
            i = r.parent;
        }
        Err(BuildError::Invalid("cyclic or overdeep union forest"))
    }
    pub(super) fn bound_minimum(&mut self, extent: Extent, c: GlobalCell) -> Result<u64, S::Error> {
        let last = CellIndex::new(extent.cells() - 1, extent)
            .ok_or(BuildError::Invalid("empty extent"))?;
        let (mx, my) = extent.coordinates(last);
        if c.x > mx || c.y > my {
            return Err(BuildError::Invalid("bound terminal extent"));
        }
        let raw =
            c.y.checked_mul(mx + 1)
                .and_then(|v| v.checked_add(c.x))
                .ok_or(BuildError::Invalid("bound terminal ordinal"))?;
        let at =
            CellIndex::new(raw, extent).ok_or(BuildError::Invalid("bound terminal ordinal"))?;
        self.find_minimum(at)
    }
    pub(super) fn check_binding(
        &mut self,
        extent: Extent,
        w: Witness,
        bound: BoundSpill,
    ) -> Result<(), S::Error> {
        let expected = self.bound_minimum(extent, w.source_anchor)?;
        let actual = self.bound_minimum(extent, bound.source_terminal)?;
        let expected = self.root(expected, Some(w.sill.raw()))?;
        let actual = self.root(actual, Some(w.sill.raw()))?;
        if expected != actual {
            return Err(BuildError::Invalid(
                "spill source outside pre-event component",
            ));
        }
        if let Some(target) = bound.target_terminal {
            let target = self.bound_minimum(extent, target)?;
            if self.root(target, Some(w.sill.raw()))? == expected {
                return Err(BuildError::Invalid(
                    "spill target inside pre-event component",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn find_minimum(&mut self, at: CellIndex) -> Result<u64, S::Error> {
        let (mut low, mut high) = (0, self.leaves);
        while low < high {
            let mid = low + (high - low) / 2;
            let m = self
                .u(mid)?
                .minimum
                .ok_or(BuildError::Invalid("missing minimum"))?;
            if m.at < at {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        if low == self.leaves || self.u(low)?.minimum.map(|m| m.at) != Some(at) {
            return Err(BuildError::Invalid("unknown closed terminal"));
        }
        Ok(low)
    }
    pub(super) fn node_index(&mut self, n: Node) -> Result<u64, S::Error> {
        match n {
            Node::Closed(at) => self.find_minimum(at),
            Node::Exterior => Ok(self.leaves),
        }
    }
    pub(super) fn pin(&mut self, i: u64, level: i32) -> Result<UnionRow, S::Error> {
        let mut r = self.u(i)?;
        if r.event != Some(level) {
            r.event = Some(level);
            r.event_base = r.component;
            self.op(|s| s.put_union(i, r))?;
        }
        Ok(r)
    }
    pub(super) fn base(&mut self, leaf: u64, level: i32) -> Result<u64, S::Error> {
        let root = self.root(leaf, Some(level))?;
        self.pin(root, level)?
            .event_base
            .ok_or(BuildError::Invalid("outgoing source was already exterior"))
    }
    pub(super) fn set_spill(
        &mut self,
        node: u64,
        parent: Option<u64>,
        spill: PendingSpill,
    ) -> Result<(), S::Error> {
        let mut r = self.n(node)?;
        if r.spill.is_some() || spill.sill_mm < r.birth_mm {
            return Err(BuildError::Invalid("invalid repeated node death"));
        }
        r.parent = parent;
        r.spill = Some(spill);
        self.op(|s| s.put_node(node, r))
    }
}
pub(super) fn point(extent: Extent, at: CellIndex) -> GlobalCell {
    let (x, y) = extent.coordinates(at);
    GlobalCell { x, y }
}
pub(super) fn anchor(c: GlobalCell) -> u64 {
    (u64::from(c.y) << 32) | u64::from(c.x)
}
pub(super) fn key(extent: Extent, e: Saddle) -> (i32, u64, u64, u64, u64) {
    let node = |n| match n {
        Node::Closed(at) => extent.anchor_key(at),
        Node::Exterior => u64::MAX,
    };
    let a = extent.anchor_key(e.from);
    let b = e.to.map_or(u64::MAX, |at| extent.anchor_key(at));
    (e.sill_mm, node(e.left), node(e.right), a.min(b), a.max(b))
}
