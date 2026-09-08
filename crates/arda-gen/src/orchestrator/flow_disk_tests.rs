#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "arda-flow-disk-{}-{stamp}-{id}",
            std::process::id()
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn path(&self) -> PathBuf {
        self.0.join("flow.bin")
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn extent() -> Extent {
    Extent::new(19, 19).unwrap()
}
fn at(n: u32, e: Extent) -> CellIndex {
    CellIndex::new(n, e).unwrap()
}
fn limits(path: &Path, e: Extent, cache: u32) -> FlowLimits {
    FlowLimits {
        cache_pages: cache,
        ram_bytes: ram_required(path, cache).unwrap(),
        scratch_bytes: scratch_required(e),
        io_bytes: 100_000_000,
        io_operations: 100_000,
    }
}
fn source(n: u32) -> FlowRecord {
    FlowRecord {
        net: i128::from(n) - 180,
        lake: (n.is_multiple_of(2)).then_some(BasinId(u64::from(n) + 1)),
        surface_mm: if n.is_multiple_of(2) { -17 } else { 0 },
        metrics: FlowMetrics {
            scalar_annual: u64::from(n),
            ..FlowMetrics::default()
        },
        ..FlowRecord::default()
    }
}
fn store(path: &Path, e: Extent, cache: u32) -> DiskFlowStore {
    DiskFlowStore::create(
        path,
        e,
        (0..e.cells()).map(|n| Ok::<_, &'static str>(source(n))),
        limits(path, e, cache),
    )
    .unwrap()
}
fn repair_checksum(row: &mut [u8]) {
    let sum = checksum(&row[..72]);
    row[72..80].copy_from_slice(&sum.to_le_bytes());
}
#[test]
fn exact_rows_round_trip_extreme_signed_amounts_flags_and_canonical_empty_fields() {
    let e = extent();
    for net in [i128::MIN, -1, 0, 1, i128::MAX] {
        let row = FlowRecord {
            net,
            parent: Some(at(19, e)),
            visited: true,
            selected: 255,
            exterior: true,
            lake: Some(BasinId(u64::MAX)),
            surface_mm: i32::MIN,
            metrics: FlowMetrics {
                catchment_cells: u32::MAX,
                drainage_cells: u32::MAX,
                pending: 8,
                max_in_order: 255,
                max_in_ties: 2,
                order: 255,
                hand_distance_mm: u64::MAX,
                hand_at: Some(at(20, e)),
                scalar_annual: u64::MAX,
            },
        };
        let bytes = encode_record(row, e).unwrap();
        assert_eq!(bytes.len(), 80);
        assert_eq!(&bytes[..16], &net.to_le_bytes());
        assert_eq!(&bytes[16..20], &19_u32.to_le_bytes());
        assert_eq!(bytes[20], 15);
        assert_eq!(decode_record(&bytes, e).unwrap(), row);
    }
    assert!(encode_record(
        FlowRecord {
            parent: Some(at(0, e)),
            ..FlowRecord::default()
        },
        e
    )
    .is_err());
    assert!(encode_record(
        FlowRecord {
            surface_mm: 1,
            ..FlowRecord::default()
        },
        e
    )
    .is_err());
    let foreign = CellIndex::new(500, Extent::new(1, 501).unwrap()).unwrap();
    assert!(encode_record(
        FlowRecord {
            parent: Some(foreign),
            visited: true,
            ..FlowRecord::default()
        },
        e
    )
    .is_err());
    let empty = encode_record(FlowRecord::default(), e).unwrap();
    for index in [20, 34, 39, 68, 72] {
        let mut bad = empty;
        bad[index] ^= 128;
        if index < 72 {
            repair_checksum(&mut bad);
        }
        assert!(decode_record(&bad, e).is_err());
    }
    let mut bad = empty;
    bad[16] = 1;
    repair_checksum(&mut bad);
    assert!(decode_record(&bad, e).is_err());
    let mut bad = empty;
    bad[26] = 1;
    repair_checksum(&mut bad);
    assert!(decode_record(&bad, e).is_err());
    assert!(decode_record(&empty[..79], e).is_err());
}
#[test]
fn real_pages_match_memory_with_tiny_and_ordinary_fifo_caches() {
    for cache in [1, 3, 8] {
        let t = Temp::new();
        let p = t.path();
        let e = extent();
        let mut disk = store(&p, e, cache);
        let mut want: Vec<_> = (0..e.cells()).map(source).collect();
        assert_eq!(std::fs::metadata(&p).unwrap().len(), scratch_required(e));
        assert_eq!(disk.work().bytes, u128::from(scratch_required(e)));
        assert_eq!(
            disk.work().operations,
            2 + u64::from(e.cells()).div_ceil(PER_PAGE)
        );
        for n in [0, 50, 51, 101, 102, 152, 153, 305, 306, 360] {
            assert_eq!(
                disk.read(at(n, e)).unwrap(),
                want[usize::try_from(n).unwrap()]
            );
            let changed = FlowRecord {
                net: 123456789,
                visited: true,
                parent: Some(at(n.saturating_sub(1), e)),
                metrics: FlowMetrics {
                    catchment_cells: 31,
                    drainage_cells: 99,
                    pending: 2,
                    max_in_order: 4,
                    max_in_ties: 2,
                    order: 5,
                    hand_distance_mm: 123456,
                    hand_at: Some(at(1, e)),
                    scalar_annual: 987654321,
                },
                ..want[usize::try_from(n).unwrap()]
            };
            disk.write(at(n, e), changed).unwrap();
            want[usize::try_from(n).unwrap()] = changed;
        }
        disk.flush().unwrap();
        assert!(disk.cache.is_empty());
        assert!(disk.fifo.is_empty());
        let mut reopened = DiskFlowStore::open(&p, e, limits(&p, e, cache)).unwrap();
        for n in (0..e.cells()).rev() {
            assert_eq!(
                reopened.read(at(n, e)).unwrap(),
                want[usize::try_from(n).unwrap()]
            );
        }
    }
}
#[test]
fn cache_hits_are_free_and_clean_reads_do_not_flush() {
    let t = Temp::new();
    let p = t.path();
    let e = extent();
    let mut d = store(&p, e, 1);
    let before = d.work();
    d.read(at(0, e)).unwrap();
    let after = d.work();
    assert_eq!(after.operations - before.operations, 2);
    assert_eq!(after.bytes - before.bytes, 4096);
    d.read(at(50, e)).unwrap();
    assert_eq!(d.work(), after);
    d.flush().unwrap();
    assert_eq!(d.work(), after);
}
#[test]
fn original_fallible_source_errors_short_long_and_nonfresh_rows_remain_private() {
    #[derive(Debug, PartialEq)]
    struct SourceError(u32);
    let e = extent();
    for fail_at in [0, 85, e.cells()] {
        let t = Temp::new();
        let p = t.path();
        let input = (0..=e.cells()).map(|n| {
            if n == fail_at {
                Err(SourceError(n))
            } else {
                Ok(source(n))
            }
        });
        assert!(
            matches!(DiskFlowStore::create(&p,e,input,limits(&p,e,1)),Err(CreateError::Input(SourceError(n))) if n==fail_at)
        );
        assert!(p.exists()); // parent transaction alone owns this abandoned private path
    }
    for n in [e.cells() - 1, e.cells() + 1] {
        let t = Temp::new();
        let p = t.path();
        assert!(matches!(
            DiskFlowStore::create(
                &p,
                e,
                (0..n).map(|i| Ok::<_, ()>(source(i))),
                limits(&p, e, 1)
            ),
            Err(CreateError::Storage(FlowDiskError::Invalid(_)))
        ));
    }
    for fresh in [
        FlowRecord {
            visited: true,
            ..FlowRecord::default()
        },
        FlowRecord {
            selected: 1,
            ..FlowRecord::default()
        },
        FlowRecord {
            exterior: true,
            ..FlowRecord::default()
        },
    ] {
        let t = Temp::new();
        let p = t.path();
        assert!(matches!(
            DiskFlowStore::create(
                &p,
                e,
                std::iter::repeat_n(Ok::<_, ()>(fresh), usize::try_from(e.cells()).unwrap()),
                limits(&p, e, 1)
            ),
            Err(CreateError::Storage(FlowDiskError::Invalid(
                "source already routed"
            )))
        ));
    }
}
#[test]
fn admission_and_fresh_namespace_fail_before_overwriting_existing_bytes() {
    let t = Temp::new();
    let p = t.path();
    let e = extent();
    let exact = limits(&p, e, 1);
    for bad in [
        FlowLimits {
            cache_pages: 0,
            ..exact
        },
        FlowLimits {
            ram_bytes: exact.ram_bytes - 1,
            ..exact
        },
        FlowLimits {
            scratch_bytes: exact.scratch_bytes - 1,
            ..exact
        },
        FlowLimits {
            io_bytes: u128::from(scratch_required(e)) - 1,
            ..exact
        },
        FlowLimits {
            io_operations: 1,
            ..exact
        },
    ] {
        assert!(
            DiskFlowStore::create(&p, e, (0..e.cells()).map(|n| Ok::<_, ()>(source(n))), bad)
                .is_err()
        );
        assert!(!p.exists());
    }
    std::fs::write(&p, b"owned-existing").unwrap();
    assert!(
        DiskFlowStore::create(&p, e, (0..e.cells()).map(|n| Ok::<_, ()>(source(n))), exact)
            .is_err()
    );
    assert_eq!(std::fs::read(&p).unwrap(), b"owned-existing");
}
#[test]
fn corrupt_header_checksum_record_padding_tail_padding_and_truncation_are_detected() {
    let e = extent();
    for (offset, header_error) in [
        (0, true),
        (32, true),
        (56, true),
        (64 + 72, false),
        (64 + 34, false),
        (64 + 4080, false),
        (64 + 7 * 4096 + 4 * 80, false),
    ] {
        let t = Temp::new();
        let p = t.path();
        drop(store(&p, e, 1));
        let mut bytes = std::fs::read(&p).unwrap();
        bytes[offset] ^= 1;
        std::fs::write(&p, bytes).unwrap();
        let opened = DiskFlowStore::open(&p, e, limits(&p, e, 1));
        if header_error {
            assert!(opened.is_err());
        } else {
            let mut d = opened.unwrap();
            let n = if offset >= 64 + 7 * 4096 { 357 } else { 0 };
            assert!(d.read(at(n, e)).is_err());
            assert!(d.cache.is_empty());
        }
    }
    let t = Temp::new();
    let p = t.path();
    drop(store(&p, e, 1));
    let f = OpenOptions::new().write(true).open(&p).unwrap();
    f.set_len(scratch_required(e) - 1).unwrap();
    assert!(DiskFlowStore::open(&p, e, limits(&p, e, 1)).is_err());
}
#[test]
fn dirty_write_failure_retains_page_and_fifo_and_drop_does_not_retry() {
    let t = Temp::new();
    let p = t.path();
    let e = extent();
    let mut d = store(&p, e, 1);
    let before = std::fs::read(&p).unwrap();
    d.write(
        at(0, e),
        FlowRecord {
            net: 999,
            ..source(0)
        },
    )
    .unwrap();
    d.file = File::open(&p).unwrap();
    let work = d.work();
    assert!(matches!(d.read(at(85, e)), Err(FlowDiskError::Io { .. })));
    assert_eq!(d.fifo.front(), Some(&0));
    assert_eq!(d.cache.len(), 1);
    assert!(d.cache[&0].dirty);
    assert_eq!(d.read(at(0, e)).unwrap().net, 999);
    assert_eq!(d.work().bytes - work.bytes, 4096);
    assert_eq!(d.work().operations - work.operations, 2);
    drop(d);
    assert_eq!(std::fs::read(&p).unwrap(), before);
}
#[test]
fn dirty_budget_failure_preserves_state_and_invalid_ordinals_do_not_do_io() {
    let t = Temp::new();
    let p = t.path();
    let e = extent();
    let mut d = store(&p, e, 1);
    d.write(
        at(0, e),
        FlowRecord {
            net: 999,
            ..source(0)
        },
    )
    .unwrap();
    d.meter.limits.io_bytes = d.work().bytes;
    assert!(matches!(d.flush(), Err(FlowDiskError::Limit(_))));
    assert!(d.cache[&0].dirty);
    assert_eq!(d.fifo.front(), Some(&0));
    let old = d.work();
    let foreign = CellIndex::new(500, Extent::new(1, 501).unwrap()).unwrap();
    assert!(d.read(foreign).is_err());
    assert!(d.write(foreign, FlowRecord::default()).is_err());
    assert_eq!(d.work(), old);
}

#[test]
fn metrics_refs_tags_sentinels_and_fresh_initialization_are_explicit() {
    let e = extent();
    let empty = encode_record(FlowRecord::default(), e).unwrap();
    for (offset, value) in [
        (34, 2),
        (35, 9),
        (37, 9),
        (39, 1),
        (56, 1),
        (68, 1),
        (48, 0),
    ] {
        let mut bad = empty;
        bad[offset] = value;
        repair_checksum(&mut bad);
        assert!(decode_record(&bad, e).is_err(), "offset {offset}");
    }
    let foreign = CellIndex::new(500, Extent::new(1, 501).unwrap()).unwrap();
    let bad = FlowRecord {
        metrics: FlowMetrics {
            hand_at: Some(foreign),
            ..FlowMetrics::default()
        },
        ..FlowRecord::default()
    };
    assert!(encode_record(bad, e).is_err());
    for metrics in [
        FlowMetrics {
            pending: 1,
            ..FlowMetrics::default()
        },
        FlowMetrics {
            catchment_cells: 1,
            ..FlowMetrics::default()
        },
        FlowMetrics {
            hand_at: Some(at(0, e)),
            ..FlowMetrics::default()
        },
    ] {
        let t = Temp::new();
        let path = t.path();
        let row = FlowRecord {
            metrics,
            ..FlowRecord::default()
        };
        assert!(matches!(
            DiskFlowStore::create(
                &path,
                e,
                std::iter::repeat_n(Ok::<_, ()>(row), usize::try_from(e.cells()).unwrap()),
                limits(&path, e, 1)
            ),
            Err(CreateError::Storage(FlowDiskError::Invalid(
                "source already routed"
            )))
        ));
    }
}
