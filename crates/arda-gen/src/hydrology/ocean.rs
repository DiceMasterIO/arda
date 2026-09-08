//! Connected marine membership on actual prepared terrain before receiver routing.
//!
//! The sea is the D8 component of cells at or below zero connected to the actual
//! modeled outer rim. Enclosed negative depressions remain nonmarine. This pass
//! uses the existing paged records and frontier tape, with no new full-grid mask.
#![deny(missing_docs)]

use super::routing::{CellIndex, CellRecord, Extent, RoutingStore, Tape};

/// Explicit logical work ceilings; the backend meters physical I/O separately.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Kernel record reads, excluding the backend's checked marking read.
    pub reads: u64,
    /// Fresh record marks; each performs one additional backend validation read.
    pub marks: u64,
    /// Frontier entries appended and consumed, counted separately per operation.
    pub tape_operations: u64,
}
impl Limits {
    /// Conservative linear bounds: N validation + rim + 8N neighbor reads,
    /// at most N marks and 2N frontier operations.
    #[must_use]
    pub fn for_extent(extent: Extent) -> Self {
        let n = u64::from(extent.cells());
        Self {
            reads: 10 * n,
            marks: n,
            tape_operations: 2 * n,
        }
    }
}

/// Successful logical work, independent of cache size and physical disk costs.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Work {
    /// Kernel coherent record reads.
    pub reads: u64,
    /// Cells marked marine; also the exact resulting marine cell count.
    pub marine_cells: u64,
    /// Frontier appends plus reads.
    pub tape_operations: u64,
}

/// A failed pass leaves private scratch that must not be published or reused.
#[derive(Debug, thiserror::Error)]
pub enum OceanError<E> {
    /// Underlying scratch read, marking, or tape operation failed.
    #[error("ocean scratch failed")]
    Storage(#[source] E),
    /// The caller's declared logical work allowance was exhausted.
    #[error("ocean work reservation exhausted")]
    Limit,
    /// Input already contains routing state or an unverified marine mask.
    #[error("ocean input is not fresh unclassified terrain")]
    InvalidPrepared,
}

struct Pass<'a, S: RoutingStore> {
    store: &'a mut S,
    limits: Limits,
    work: Work,
}
impl<S: RoutingStore> Pass<'_, S> {
    fn read(&mut self, at: CellIndex) -> Result<CellRecord, OceanError<S::Error>> {
        if self.work.reads >= self.limits.reads {
            return Err(OceanError::Limit);
        }
        self.work.reads += 1;
        self.store.read(at).map_err(OceanError::Storage)
    }
    fn discover(&mut self, at: CellIndex) -> Result<(), OceanError<S::Error>> {
        let record = self.read(at)?;
        if record.height() > 0 || record.is_marine() {
            return Ok(());
        }
        if self.work.marine_cells >= self.limits.marks {
            return Err(OceanError::Limit);
        }
        self.work.marine_cells += 1;
        self.store.mark_marine(at).map_err(OceanError::Storage)?;
        self.tape_op()?;
        self.store
            .push(Tape::Frontier, at)
            .map_err(OceanError::Storage)
    }
    fn tape_op(&mut self) -> Result<(), OceanError<S::Error>> {
        if self.work.tape_operations >= self.limits.tape_operations {
            return Err(OceanError::Limit);
        }
        self.work.tape_operations += 1;
        Ok(())
    }
}

/// Solves fine marine connectivity and leaves fresh routing records with a final
/// marine bit. Physical height, plateau rank and ownership remain unchanged.
///
/// All N records are validated before changing any record or clearing a tape.
/// The pass refuses pre-seeded marine flags, so disconnected negative terrain
/// cannot inherit an erroneous coarse ocean classification. After success the
/// frontier is empty and routing can reuse both tapes. A failed pass aborts the
/// private generation transaction; retry requires fresh scratch.
///
/// # Errors
/// Returns invalid prepared input, exhausted logical work, or the original
/// backend error. It never supplies a partial marine mask as a solved result.
pub fn connect<S: RoutingStore>(
    store: &mut S,
    limits: Limits,
) -> Result<Work, OceanError<S::Error>> {
    let extent = store.extent();
    let last = CellIndex::new(extent.cells() - 1, extent).ok_or(OceanError::InvalidPrepared)?;
    let (last_x, last_y) = extent.coordinates(last);
    let width = last_x + 1;
    let mut pass = Pass {
        store,
        limits,
        work: Work::default(),
    };
    for raw in 0..extent.cells() {
        let at = CellIndex::new(raw, extent).ok_or(OceanError::InvalidPrepared)?;
        let r = pass.read(at)?;
        if r != CellRecord::prepared(r.height(), false) {
            return Err(OceanError::InvalidPrepared);
        }
    }
    pass.store
        .clear(Tape::Frontier)
        .map_err(OceanError::Storage)?;
    // Visit each actual rim cell once, including one-cell-wide/tall rectangles.
    for raw in 0..extent.cells() {
        let at = CellIndex::new(raw, extent).ok_or(OceanError::InvalidPrepared)?;
        let (x, y) = extent.coordinates(at);
        if x == 0 || y == 0 || x == last_x || y == last_y {
            pass.discover(at)?;
        }
    }
    let mut head = 0;
    while head < pass.store.len(Tape::Frontier) {
        pass.tape_op()?;
        let at = pass
            .store
            .get(Tape::Frontier, head)
            .map_err(OceanError::Storage)?;
        head += 1;
        let (x, y) = extent.coordinates(at);
        for ny in y.saturating_sub(1)..=y.saturating_add(1).min(last_y) {
            for nx in x.saturating_sub(1)..=x.saturating_add(1).min(last_x) {
                if nx == x && ny == y {
                    continue;
                }
                let neighbor =
                    CellIndex::new(ny * width + nx, extent).ok_or(OceanError::InvalidPrepared)?;
                pass.discover(neighbor)?;
            }
        }
    }
    pass.store
        .clear(Tape::Frontier)
        .map_err(OceanError::Storage)?;
    Ok(pass.work)
}

#[cfg(test)]
mod tests {
    use super::super::routing::{route_and_own, MemoryPages, OutletKind, Receiver};
    use super::*;
    use crate::orchestrator::routing_disk::{DiskLimits, DiskRoutingStore};
    use std::path::PathBuf;
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            use std::sync::atomic::{AtomicU32, Ordering};
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let p = std::env::temp_dir().join(format!(
                "arda-ocean-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn memory(w: u32, h: u32, z: &[i32]) -> MemoryPages {
        MemoryPages::new(
            Extent::new(w, h).unwrap(),
            z,
            &vec![false; z.len()],
            1 << 28,
        )
        .unwrap()
    }
    fn mask<S: RoutingStore>(s: &mut S) -> Vec<bool>
    where
        S::Error: std::fmt::Debug,
    {
        (0..s.extent().cells())
            .map(|i| {
                let at = CellIndex::new(i, s.extent()).unwrap();
                s.read(at).unwrap().is_marine()
            })
            .collect()
    }
    // Independent relaxation reference; no queue, backend, or production helper.
    fn oracle(w: usize, h: usize, z: &[i32]) -> Vec<bool> {
        let mut wet = vec![false; z.len()];
        for i in 0..z.len() {
            wet[i] = z[i] <= 0 && (i % w == 0 || i % w == w - 1 || i / w == 0 || i / w == h - 1);
        }
        loop {
            let old = wet.clone();
            for i in 0..z.len() {
                if z[i] > 0 {
                    continue;
                }
                let x = i % w;
                let y = i / w;
                for ny in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                    for nx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                        wet[i] |= old[ny * w + nx];
                    }
                }
            }
            if wet == old {
                return wet;
            }
        }
    }
    #[test]
    fn zero_and_diagonal_seams_connect_but_enclosed_negative_bowl_does_not() {
        let mut z = vec![10; 49];
        z[0] = -1;
        z[8] = 0;
        z[16] = -2;
        z[24] = 0;
        z[12] = -100;
        let mut s = memory(7, 7, &z);
        let e = s.extent();
        let work = connect(&mut s, Limits::for_extent(e)).unwrap();
        assert_eq!(work.marine_cells, 4);
        assert_eq!(mask(&mut s), oracle(7, 7, &z));
        assert!(!s.read(CellIndex::new(12, e).unwrap()).unwrap().is_marine());
        assert_eq!(s.len(Tape::Frontier), 0);
        for (i, &height) in z.iter().enumerate() {
            let r = s
                .read(CellIndex::new(u32::try_from(i).unwrap(), e).unwrap())
                .unwrap();
            assert_eq!(r.height(), height);
            assert_eq!(r.distance(), None);
            assert_eq!(r.owner(), None);
        }
        route_and_own(&mut s, super::super::routing::Limits::for_extent(e)).unwrap();
        let inland = CellIndex::new(12, e).unwrap();
        assert_eq!(
            s.read(inland).unwrap().receiver(e, inland),
            Some(Receiver::Stop(OutletKind::ClosedDepression))
        );
        assert!(s.mark_marine(inland).is_err());
    }
    #[test]
    fn random_small_domains_match_independent_relaxation_and_linear_work() {
        let mut random = 436342u64;
        for (w, h) in [(1, 1), (1, 19), (17, 1), (2, 2), (23, 19)] {
            for _ in 0..12 {
                let z: Vec<i32> = (0..w * h)
                    .map(|_| {
                        random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                        i32::try_from((random >> 32) % 5).unwrap() - 3
                    })
                    .collect();
                let mut s = memory(w, h, &z);
                let e = s.extent();
                let limits = Limits::for_extent(e);
                let work = connect(&mut s, limits).unwrap();
                assert_eq!(mask(&mut s), oracle(w as usize, h as usize, &z));
                assert!(
                    work.reads <= limits.reads
                        && work.marine_cells <= limits.marks
                        && work.tape_operations <= limits.tape_operations
                );
            }
        }
    }
    #[test]
    fn invalid_initial_state_fails_before_mutation_and_limits_are_explicit() {
        let e = Extent::new(3, 3).unwrap();
        let at = CellIndex::new(8, e).unwrap();
        let mut s = memory(3, 3, &[-1; 9]);
        s.mark_marine(at).unwrap();
        let before = s.encode_page(0).unwrap();
        assert!(matches!(
            connect(&mut s, Limits::for_extent(e)),
            Err(OceanError::InvalidPrepared)
        ));
        assert_eq!(before, s.encode_page(0).unwrap());
        for field in 0..3 {
            let mut s = memory(3, 3, &[-1; 9]);
            let mut limits = Limits::for_extent(e);
            match field {
                0 => limits.reads = 0,
                1 => limits.marks = 0,
                _ => limits.tape_operations = 0,
            }
            assert!(matches!(connect(&mut s, limits), Err(OceanError::Limit)));
        }
        let mut positive = memory(1, 1, &[1]);
        assert!(positive
            .mark_marine(CellIndex::new(0, positive.extent()).unwrap())
            .is_err());
    }
    #[test]
    fn disk_cache_sizes_preserve_cross_area_connectivity_and_persisted_bytes() {
        let e = Extent::new(517, 5).unwrap();
        let mut z = vec![20; e.cells() as usize];
        for x in 0..516 {
            z[2 * 517 + x] = -1;
        }
        z[3 * 517 + 515] = 0;
        let expected = oracle(517, 5, &z);
        let mut bytes = Vec::new();
        for cached_pages in [1, 8] {
            let tmp = Temp::new();
            let limits = DiskLimits {
                cache_bytes: (cached_pages + 1) * 8192,
                scratch_bytes: DiskRoutingStore::scratch_required(e),
                io_bytes: 1 << 30,
                io_operations: 1 << 24,
            };
            let mut s = DiskRoutingStore::create(
                &tmp.0,
                e,
                z.iter().map(|&h| CellRecord::prepared(h, false)),
                limits,
            )
            .unwrap();
            connect(&mut s, Limits::for_extent(e)).unwrap();
            assert_eq!(mask(&mut s), expected);
            s.flush().unwrap();
            bytes.push(std::fs::read(s.records_path()).unwrap());
            assert!(s.mark_marine(CellIndex::new(2 * 517, e).unwrap()).is_err());
        }
        assert_eq!(bytes[0], bytes[1]);
    }
    #[test]
    fn real_disk_budget_failure_propagates_without_a_successful_mask() {
        let e = Extent::new(3, 3).unwrap();
        let tmp = Temp::new();
        let limits = DiskLimits {
            cache_bytes: 16384,
            scratch_bytes: DiskRoutingStore::scratch_required(e),
            io_bytes: 32 + 4096,
            io_operations: 4,
        };
        let mut s = DiskRoutingStore::create(
            &tmp.0,
            e,
            (0..9).map(|_| CellRecord::prepared(-1, false)),
            limits,
        )
        .unwrap();
        assert!(matches!(
            connect(&mut s, Limits::for_extent(e)),
            Err(OceanError::Storage(_))
        ));
    }
}
