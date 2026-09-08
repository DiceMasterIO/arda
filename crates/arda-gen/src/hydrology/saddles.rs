//! Stream real physical saddle candidates between final receiver owners.

use super::routing::{CellIndex, CellRecord, Extent, OutletKind, Receiver, RoutingStore};

/// A contracted topology node; Exterior is never a spatial endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Node {
    /// Immutable terminal of a closed physical depression.
    Closed(CellIndex),
    /// Common topology-only open receiver; actual witnesses remain distinct.
    Exterior,
}

/// One physical witness, oriented from the lesser topology node to the greater.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Saddle {
    /// Canonical lesser node; always a closed depression.
    pub left: Node,
    /// Canonical greater node, possibly Exterior.
    pub right: Node,
    /// Maximum physical endpoint height, or actual rim height for an outside spill.
    pub sill_mm: i32,
    /// Actual endpoint on the left node's dry catchment.
    pub from: CellIndex,
    /// Actual adjacent right endpoint, absent only for an outer-rim spill.
    pub to: Option<CellIndex>,
}

impl Saddle {
    /// Pair-reduction order: owner pair, physical sill, then canonical real witnesses.
    /// Keep the first record per owner pair after sorting by this key.
    #[must_use]
    pub fn key(self) -> (Node, Node, i32, u32, u32) {
        let from = self.from.raw();
        let to = self.to.map_or(u32::MAX, CellIndex::raw);
        (
            self.left,
            self.right,
            self.sill_mm,
            from.min(to),
            from.max(to),
        )
    }

    /// Fixed 24-byte private row; owner ordinals are scoped by the file's extent.
    #[must_use]
    pub fn encode(self) -> [u8; 24] {
        let node = |n| match n {
            Node::Closed(at) => at.raw(),
            Node::Exterior => u32::MAX,
        };
        let mut row = [0; 24];
        row[..4].copy_from_slice(&node(self.left).to_le_bytes());
        row[4..8].copy_from_slice(&node(self.right).to_le_bytes());
        row[8..12].copy_from_slice(&self.sill_mm.to_le_bytes());
        row[12..16].copy_from_slice(&self.from.raw().to_le_bytes());
        row[16..20].copy_from_slice(&self.to.map_or(u32::MAX, CellIndex::raw).to_le_bytes());
        row
    }

    /// Validates canonical nodes, endpoint adjacency and the absent-endpoint rim rule.
    /// Source ownership and sill height are checked when generating the stream.
    #[must_use]
    pub fn decode(row: [u8; 24], extent: Extent) -> Option<Self> {
        let raw = |i| u32::from_le_bytes([row[i], row[i + 1], row[i + 2], row[i + 3]]);
        let node = |v| {
            if v == u32::MAX {
                Some(Node::Exterior)
            } else {
                CellIndex::new(v, extent).map(Node::Closed)
            }
        };
        let left = node(raw(0))?;
        let right = node(raw(4))?;
        let from = CellIndex::new(raw(12), extent)?;
        let to = if raw(16) == u32::MAX {
            None
        } else {
            Some(CellIndex::new(raw(16), extent)?)
        };
        if left >= right || row[20..] != [0; 4] {
            return None;
        }
        let (x, y) = extent.coordinates(from);
        match to {
            Some(to) => {
                let (a, b) = extent.coordinates(to);
                if from == to || x.abs_diff(a) > 1 || y.abs_diff(b) > 1 {
                    return None;
                }
            }
            None => {
                if right != Node::Exterior || !on_rim(extent, from)? {
                    return None;
                }
            }
        }
        Some(Self {
            left,
            right,
            sill_mm: i32::from_le_bytes([row[8], row[9], row[10], row[11]]),
            from,
            to,
        })
    }
}

/// Logical scan ceilings; disk reads and output bytes have independent stage limits.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Coherent record reads attempted.
    pub reads: u64,
    /// Candidates submitted to the caller's bounded sink.
    pub candidates: u64,
}
impl Limits {
    /// Conservative linear bounds without a full-grid label allocation.
    #[must_use]
    pub fn for_extent(extent: Extent) -> Self {
        let n = u64::from(extent.cells());
        Self {
            reads: 10 * n,
            candidates: 5 * n,
        }
    }
}

/// Completed logical work; failed sink/read operations are not a successful scan.
#[derive(Debug, Default, Clone, Copy)]
pub struct Work {
    /// Record reads attempted after admission.
    pub reads: u64,
    /// Candidate writes attempted after admission.
    pub candidates: u64,
}

/// Input, backend, sink or declared work-limit failure; no partial result is final.
#[derive(Debug, thiserror::Error)]
pub enum ScanError<Storage, Output> {
    /// The routing backend failed.
    #[error("saddle scan routing storage failed")]
    Storage(#[source] Storage),
    /// The caller's candidate sink failed.
    #[error("saddle scan candidate output failed")]
    Output(#[source] Output),
    /// Selected work reservation is exhausted.
    #[error("saddle scan exhausted its work reservation")]
    WorkLimit,
    /// The previously resolved owner is absent or is not a terminal.
    #[error("saddle scan found invalid resolved ownership")]
    InvalidOwnership,
}

fn on_rim(extent: Extent, at: CellIndex) -> Option<bool> {
    let (x, y) = extent.coordinates(at);
    let last = CellIndex::new(extent.cells() - 1, extent)?;
    let (last_x, last_y) = extent.coordinates(last);
    Some(x == 0 || y == 0 || x == last_x || y == last_y)
}

struct Pass<'a, S: RoutingStore> {
    store: &'a mut S,
    extent: Extent,
    limits: Limits,
    work: Work,
}
impl<S: RoutingStore> Pass<'_, S> {
    fn read<E>(&mut self, at: CellIndex) -> Result<CellRecord, ScanError<S::Error, E>> {
        if self.work.reads >= self.limits.reads {
            return Err(ScanError::WorkLimit);
        }
        self.work.reads += 1;
        self.store.read(at).map_err(ScanError::Storage)
    }
    fn node<E>(
        &mut self,
        at: CellIndex,
        record: CellRecord,
    ) -> Result<Node, ScanError<S::Error, E>> {
        let owner = record.owner().ok_or(ScanError::InvalidOwnership)?;
        let terminal = if owner == at {
            record
        } else {
            self.read(owner)?
        };
        if terminal.owner() != Some(owner) || terminal.height() > record.height() {
            return Err(ScanError::InvalidOwnership);
        }
        match terminal.receiver(self.extent, owner) {
            Some(Receiver::Stop(OutletKind::ClosedDepression)) if !terminal.is_marine() => {
                Ok(Node::Closed(owner))
            }
            Some(Receiver::Stop(OutletKind::MarineEntry | OutletKind::DomainExport)) => {
                Ok(Node::Exterior)
            }
            _ => Err(ScanError::InvalidOwnership),
        }
    }
    fn emit<E>(
        &mut self,
        sink: &mut impl FnMut(Saddle) -> Result<(), E>,
        edge: Saddle,
    ) -> Result<(), ScanError<S::Error, E>> {
        if self.work.candidates >= self.limits.candidates {
            return Err(ScanError::WorkLimit);
        }
        self.work.candidates += 1;
        sink(edge).map_err(ScanError::Output)
    }
}

/// Streams each actual undirected D8 adjacency once, plus closed-owner rim exits.
///
/// Candidates are not globally retained in memory. A sink may reduce node pairs
/// by `Saddle::key`, or reduce current component minima during a Boruvka replay.
/// Owner lookup uses the final dry receiver authority. A dry receiver pointing inward does not remove
/// a physical outward spill from a world-rim cell.
///
/// # Errors
/// Propagates backend/output failures, exhausted reservations, and invalid owners.
pub fn scan<S: RoutingStore, E>(
    store: &mut S,
    limits: Limits,
    mut sink: impl FnMut(Saddle) -> Result<(), E>,
) -> Result<Work, ScanError<S::Error, E>> {
    let extent = store.extent();
    let mut pass = Pass {
        store,
        extent,
        limits,
        work: Work::default(),
    };
    let last = CellIndex::new(extent.cells() - 1, extent).ok_or(ScanError::InvalidOwnership)?;
    let (last_x, last_y) = extent.coordinates(last);
    let width = last_x + 1;
    for raw in 0..extent.cells() {
        let at = CellIndex::new(raw, extent).ok_or(ScanError::InvalidOwnership)?;
        let record = pass.read(at)?;
        let node = pass.node(at, record)?;
        if matches!(node, Node::Closed(_))
            && on_rim(extent, at).ok_or(ScanError::InvalidOwnership)?
        {
            pass.emit(
                &mut sink,
                Saddle {
                    left: node,
                    right: Node::Exterior,
                    sill_mm: record.height(),
                    from: at,
                    to: None,
                },
            )?;
        }
        let (x, y) = extent.coordinates(at);
        for (dx, dy) in [(1_i64, 0_i64), (-1, 1), (0, 1), (1, 1)] {
            let a = i64::from(x) + dx;
            let b = i64::from(y) + dy;
            if a < 0 || b < 0 || a > i64::from(last_x) || b > i64::from(last_y) {
                continue;
            }
            let a = u32::try_from(a).map_err(|_| ScanError::InvalidOwnership)?;
            let b = u32::try_from(b).map_err(|_| ScanError::InvalidOwnership)?;
            let other = CellIndex::new(b * width + a, extent).ok_or(ScanError::InvalidOwnership)?;
            let adjacent = pass.read(other)?;
            let other_node = pass.node(other, adjacent)?;
            if node == other_node {
                continue;
            }
            let (left, right, from, to) = if node < other_node {
                (node, other_node, at, other)
            } else {
                (other_node, node, other, at)
            };
            pass.emit(
                &mut sink,
                Saddle {
                    left,
                    right,
                    sill_mm: record.height().max(adjacent.height()),
                    from,
                    to: Some(to),
                },
            )?;
        }
    }
    Ok(pass.work)
}

#[cfg(test)]
mod tests {
    use super::super::routing::{self, MemoryPages};
    use super::*;
    use std::{collections::BTreeMap, convert::Infallible};

    fn routed(w: u32, h: u32, heights: &[i32], marine: &[bool]) -> MemoryPages {
        let extent = Extent::new(w, h).unwrap();
        let mut pages = MemoryPages::new(extent, heights, marine, 64 * 1024 * 1024).unwrap();
        routing::route_and_own(&mut pages, routing::Limits::for_extent(extent)).unwrap();
        pages
    }
    fn edges(pages: &mut MemoryPages) -> Vec<Saddle> {
        let mut result = Vec::new();
        scan(pages, Limits::for_extent(pages.extent()), |e| {
            result.push(e);
            Ok::<_, Infallible>(())
        })
        .unwrap();
        result
    }
    fn reduced(rows: &[Saddle]) -> BTreeMap<(Node, Node), Saddle> {
        let mut out = BTreeMap::new();
        for &row in rows {
            out.entry((row.left, row.right))
                .and_modify(|prior: &mut Saddle| {
                    if row.key() < prior.key() {
                        *prior = row;
                    }
                })
                .or_insert(row);
        }
        out
    }

    #[test]
    fn inward_rim_bowl_retains_its_actual_world_exit() {
        let mut pages = routed(3, 3, &[5, 4, 5, 4, -2, 4, 5, 4, 5], &[false; 9]);
        let extent = pages.extent();
        let rows = edges(&mut pages);
        assert_eq!(rows.len(), 8);
        assert!(rows.iter().all(|row| row.to.is_none()));
        let minimum = *reduced(&rows).values().next().unwrap();
        assert_eq!(minimum.sill_mm, 4);
        assert_eq!(minimum.from.raw(), 1);
        assert_eq!(
            minimum.left,
            Node::Closed(CellIndex::new(4, extent).unwrap())
        );
        assert_eq!(minimum.right, Node::Exterior);
    }

    #[test]
    fn equal_sill_pair_reduction_keeps_actual_canonical_witness() {
        let heights = [8, 8, 8, 8, 8, 8, 0, 3, 1, 8, 8, 8, 8, 8, 8];
        let mut pages = routed(5, 3, &heights, &[false; 15]);
        let rows = edges(&mut pages);
        let mut reverse = rows.clone();
        reverse.reverse();
        assert_eq!(reduced(&rows), reduced(&reverse));
        let closed: Vec<_> = reduced(&rows)
            .into_values()
            .filter(|r| matches!(r.right, Node::Closed(_)))
            .collect();
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].sill_mm, 3);
        assert_eq!((closed[0].from.raw(), closed[0].to.unwrap().raw()), (7, 8));
        for row in rows {
            assert_eq!(Saddle::decode(row.encode(), pages.extent()), Some(row));
        }
    }

    #[test]
    fn no_fake_exterior_coordinate_and_corrupt_rows_are_refused() {
        let mut pages = routed(3, 3, &[5, 4, 5, 4, -2, 4, 5, 4, 5], &[false; 9]);
        let extent = pages.extent();
        let row = edges(&mut pages)[0];
        let mut bytes = row.encode();
        bytes[12..16].copy_from_slice(&4_u32.to_le_bytes());
        assert!(Saddle::decode(bytes, extent).is_none());
        let mut bytes = row.encode();
        bytes[20] = 1;
        assert!(Saddle::decode(bytes, extent).is_none());
        let mut bytes = row.encode();
        bytes[16..20].copy_from_slice(&99_u32.to_le_bytes());
        assert!(Saddle::decode(bytes, extent).is_none());
    }

    #[test]
    fn open_ocean_owners_collapse_without_losing_real_land_witnesses() {
        let mut pages = routed(
            5,
            3,
            &[0, 8, 8, 8, 8, 0, 3, 1, 2, 8, 0, 8, 8, 8, 8],
            &[
                true, false, false, false, false, true, false, false, false, false, true, false,
                false, false, false,
            ],
        );
        let rows = edges(&mut pages);
        assert!(rows.iter().all(|r| matches!(r.left, Node::Closed(_))));
        assert!(rows
            .iter()
            .any(|r| r.to.is_some() && r.right == Node::Exterior));
        assert!(reduced(&rows).values().any(|r| r.sill_mm == 3));
    }

    #[test]
    fn incomplete_owners_and_limits_abort_before_output() {
        let extent = Extent::new(3, 3).unwrap();
        let mut unowned = MemoryPages::new(extent, &[1; 9], &[false; 9], 64 * 1024).unwrap();
        assert!(matches!(
            scan(&mut unowned, Limits::for_extent(extent), |_| Ok::<
                _,
                Infallible,
            >(
                ()
            )),
            Err(ScanError::InvalidOwnership)
        ));
        let mut pages = routed(3, 3, &[5, 4, 5, 4, -2, 4, 5, 4, 5], &[false; 9]);
        assert!(matches!(
            scan(
                &mut pages,
                Limits {
                    reads: 0,
                    candidates: 100
                },
                |_| Ok::<_, Infallible>(())
            ),
            Err(ScanError::WorkLimit)
        ));
        assert!(matches!(
            scan(
                &mut pages,
                Limits {
                    reads: 100,
                    candidates: 0
                },
                |_| Ok::<_, Infallible>(())
            ),
            Err(ScanError::WorkLimit)
        ));
        assert!(matches!(
            scan(&mut pages, Limits::for_extent(extent), |_| Err("full sink")),
            Err(ScanError::Output("full sink"))
        ));
    }
}
