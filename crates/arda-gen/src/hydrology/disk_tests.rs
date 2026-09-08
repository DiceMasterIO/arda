use super::*;
use crate::hydrology::{
    hierarchy::{build, Minimum},
    hierarchy_tests::{between, m, outside, resolver, run},
    routing::CellIndex,
};
use crate::orchestrator::child_links::ChildLimits;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "arda-hierarchy-disk-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn limits(cache: u64) -> DiskLimits {
    DiskLimits {
        cache_bytes: cache,
        scratch_bytes: 1 << 30,
        io_bytes: u128::MAX,
        io_operations: u64::MAX,
    }
}
fn children(d: &Directory) -> DiskChildLinks {
    let mut l = ChildLimits::required(&d.0, 2, 4096).unwrap();
    l.io_bytes = u128::MAX;
    l.io_operations = u64::MAX;
    l.comparisons = u64::MAX;
    DiskChildLinks::create(&d.0, l).unwrap()
}
fn store(d: &Directory, e: Extent, cache: u64) -> DiskHierarchyStore {
    DiskHierarchyStore::new(
        &d.0,
        e,
        limits(cache + DiskHierarchyStore::path_charge(&d.0).unwrap()),
        children(d),
    )
    .unwrap()
}
fn fixture(n: u32) -> (Extent, Vec<Minimum>, Vec<super::super::saddles::Saddle>) {
    let e = Extent::new(2 * n + 2, 3).unwrap();
    let ms: Vec<_> = (0..n).map(|i| m(e, 2 * i + 1, 0)).collect();
    let mut edges: Vec<_> = ms
        .windows(2)
        .enumerate()
        .map(|(i, w)| between(e, w[0], w[1], if i % 3 == 0 { 5 } else { 9 }))
        .collect();
    edges.sort_unstable_by_key(|s| (s.sill_mm, s.left, s.right, s.from, s.to));
    if let Some(&last) = ms.last() {
        edges.push(outside(e, last, 15));
    }
    (e, ms, edges)
}
fn blank_union(e: Extent) -> UnionRow {
    UnionRow {
        minimum: Some(m(e, 1, -123)),
        parent: 0,
        rank: 0,
        joined_at: None,
        component: Some(0),
        event: None,
        event_base: None,
        elder: 0,
        elder_death: None,
    }
}
fn blank_node() -> NodeRow {
    NodeRow {
        anchor: arda_core::GlobalCell { x: 1, y: 1 },
        floor_mm: -123,
        birth_mm: -123,
        leaf: true,
        parent: None,
        spill: None,
        id: None,
        retained_parent: None,
    }
}
#[test]
fn complete_disk_and_memory_outputs_match_at_tiny_and_ordinary_cache_sizes() {
    for n in [0, 2, 120] {
        let (e, ms, es) = fixture(n);
        let (_, oracle) = run(e, &ms, &es);
        for cache in [16384, 9 * 8192] {
            let d = Directory::new();
            let mut disk = store(&d, e, cache);
            build(
                e,
                u64::from(n),
                ms.iter().copied().map(Ok),
                es.iter().copied().map(Ok),
                &mut disk,
                crate::hydrology::hierarchy_tests::limits(),
                |w| resolver(w).map_err(|_| DiskError::Invalid("fixture resolver")),
            )
            .unwrap();
            assert!(disk.output_counts().is_err());
            disk.finish().unwrap();
            let counts = disk.output_counts().unwrap();
            assert_eq!(
                counts,
                (oracle.output.len() as u64, oracle.elders.len() as u64)
            );
            for (i, row) in oracle.output.iter().enumerate() {
                assert_eq!(&disk.output_node(i as u64).unwrap(), row);
            }
            for (i, row) in oracle.elders.iter().enumerate() {
                assert_eq!(&disk.output_elder(i as u64).unwrap(), row);
            }
            assert!(disk.output_node(counts.0).is_err());
            assert!(disk.work().bytes > 0);
            assert!(disk.reserved_bytes().unwrap() <= disk.meter.limits.scratch_bytes);
        }
    }
}
#[test]
fn hierarchy_initialization_preflights_every_known_resource_before_its_files() {
    let e = Extent::new(6, 3).unwrap();
    let initial = 4 * HEADER + 2 * PAGE as u64;
    for which in 0..4 {
        let d = Directory::new();
        let mut disk = store(&d, e, 16384);
        match which {
            0 => disk.meter.limits.cache_bytes = 16383,
            1 => {
                disk.meter.limits.scratch_bytes =
                    DiskHierarchyStore::required_bytes(3, 3).unwrap() - 1;
            }
            2 => disk.meter.limits.io_bytes = u128::from(initial) - 1,
            _ => disk.meter.limits.io_operations = 0,
        };
        assert!(disk.reserve(3, 3).is_err());
        assert!(!std::fs::read_dir(&d.0).unwrap().any(|x| x
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("hierarchy-")));
        assert_eq!(disk.work(), DiskWork::default());
    }
}
#[test]
fn dirty_eviction_preserves_current_row_and_fifo_on_budget_and_real_write_errors() {
    for real_error in [false, true] {
        let d = Directory::new();
        let e = Extent::new(6, 3).unwrap();
        let mut disk = store(&d, e, 16384);
        disk.reserve(3, 3).unwrap();
        disk.put_union(0, blank_union(e)).unwrap();
        if real_error {
            let f = &mut disk.files.as_mut().unwrap()[0];
            f.file = OpenOptions::new().read(true).open(&f.path).unwrap();
        } else {
            disk.meter.limits.io_bytes = disk.work().bytes;
        }
        assert!(disk.put_node(0, blank_node()).is_err());
        assert_eq!(disk.union(0).unwrap().minimum.unwrap().floor_mm, -123);
        assert_eq!(disk.fifo.len(), 1);
        assert_eq!(disk.cache.len(), 1);
        assert!(disk.cache[&(0, 0)].dirty);
        if real_error {
            let f = &mut disk.files.as_mut().unwrap()[0];
            f.file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&f.path)
                .unwrap();
        } else {
            disk.meter.limits.io_bytes = u128::MAX;
        }
        disk.flush().unwrap();
        assert_eq!(disk.union(0).unwrap().minimum.unwrap().floor_mm, -123);
    }
}
#[test]
fn partial_flush_removes_only_successfully_saved_pages() {
    let d = Directory::new();
    let e = Extent::new(6, 3).unwrap();
    let mut disk = store(&d, e, 3 * 8192);
    disk.reserve(3, 3).unwrap();
    disk.put_union(0, blank_union(e)).unwrap();
    disk.put_node(0, blank_node()).unwrap();
    disk.meter.limits.io_bytes = disk.work().bytes + PAGE as u128;
    assert!(disk.flush().is_err());
    assert_eq!(disk.fifo.iter().copied().collect::<Vec<_>>(), [(1, 0)]);
    assert_eq!(disk.node(0).unwrap().floor_mm, -123);
    assert!(disk.cache[&(1, 0)].dirty);
    disk.meter.limits.io_bytes = u128::MAX;
    assert_eq!(disk.union(0).unwrap().minimum.unwrap().floor_mm, -123);
    disk.flush().unwrap();
    assert!(disk.fifo.is_empty());
    assert!(disk.cache.is_empty());
}
#[test]
fn headers_records_padding_and_truncation_are_checked_on_disk() {
    for kind in 0..4 {
        let d = Directory::new();
        let e = Extent::new(6, 3).unwrap();
        let mut disk = store(&d, e, 16384);
        disk.reserve(3, 3).unwrap();
        disk.put_union(0, blank_union(e)).unwrap();
        disk.flush().unwrap();
        let f = &mut disk.files.as_mut().unwrap()[0];
        match kind {
            0 => {
                f.file.seek(SeekFrom::Start(9)).unwrap();
                f.file.write_all(&[1]).unwrap();
            }
            1 => {
                f.file.seek(SeekFrom::Start(HEADER + 3)).unwrap();
                f.file.write_all(&[99]).unwrap();
            }
            2 => {
                f.file.seek(SeekFrom::Start(HEADER + 4095)).unwrap();
                f.file.write_all(&[1]).unwrap();
            }
            _ => f.file.set_len(HEADER + 17).unwrap(),
        };
        assert!(disk.union(0).is_err());
        assert!(disk.cache.is_empty());
        assert!(disk.fifo.is_empty());
    }
}
#[test]
fn empty_initialization_and_finish_have_exact_counted_io() {
    let d = Directory::new();
    let e = Extent::new(2, 2).unwrap();
    let mut disk = store(&d, e, 16384);
    disk.reserve(1, 0).unwrap();
    assert_eq!(
        disk.work(),
        DiskWork {
            bytes: u128::from(4 * HEADER) + PAGE as u128,
            operations: 10
        }
    );
    assert!(disk.finish().is_err());
    disk.finish_links(0).unwrap();
    disk.finish().unwrap();
    assert_eq!(
        disk.work(),
        DiskWork {
            bytes: u128::from(6 * HEADER) + PAGE as u128,
            operations: 16
        }
    );
    assert_eq!(disk.output_counts().unwrap(), (0, 0));
    assert!(disk.reserve(1, 0).is_err());
}
#[test]
fn uninitialized_out_of_range_and_wrong_identity_rows_are_typed_failures() {
    let d = Directory::new();
    let e = Extent::new(6, 3).unwrap();
    let mut disk = store(&d, e, 16384);
    assert!(disk.union(0).is_err());
    assert!(disk.put_node(0, blank_node()).is_err());
    assert!(disk.put_union(0, blank_union(e)).is_err());
    disk.reserve(3, 3).unwrap();
    assert!(disk.union(0).is_err());
    assert!(disk.node(3).is_err());
    let mut u = blank_union(e);
    u.minimum = Some(Minimum {
        at: CellIndex::new(1, e).unwrap(),
        floor_mm: 0,
    });
    u.parent = 77;
    assert!(disk.put_union(0, u).is_err());
    let mut n = blank_node();
    n.parent = Some(0);
    assert!(disk.put_node(0, n).is_err());
    assert!(disk
        .emit_elder(ElderLink {
            leaf: BasinId(0),
            elder_parent: Some(BasinId(0)),
            spill: resolver(crate::hydrology::hierarchy::Witness {
                source: BasinId(0),
                source_anchor: arda_core::GlobalCell { x: 0, y: 0 },
                from: arda_core::GlobalCell { x: 0, y: 0 },
                to: None,
                sill: arda_core::HeightMm::new(1)
            })
            .unwrap()
            .spill
        })
        .is_err());
}

#[test]
fn finalized_output_rejects_trailing_or_truncated_records() {
    for trailing in [false, true] {
        let d = Directory::new();
        let (e, ms, es) = fixture(2);
        let mut disk = store(&d, e, 16384);
        build(
            e,
            2,
            ms.into_iter().map(Ok),
            es.into_iter().map(Ok),
            &mut disk,
            crate::hydrology::hierarchy_tests::limits(),
            |w| resolver(w).map_err(|_| DiskError::Invalid("fixture resolver")),
        )
        .unwrap();
        disk.finish().unwrap();
        let f = &mut disk.files.as_mut().unwrap()[2];
        if trailing {
            f.file.seek(SeekFrom::End(0)).unwrap();
            f.file.write_all(&[1]).unwrap();
        } else {
            let len = f.file.metadata().unwrap().len();
            f.file.set_len(len - 1).unwrap();
        }
        assert!(disk.output_node(0).is_err());
    }
}
