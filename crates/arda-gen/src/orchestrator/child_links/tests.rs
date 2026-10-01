use super::*;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
static SERIAL: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let id = SERIAL.fetch_add(1, AtomicOrdering::Relaxed);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "arda-child-links-{}-{stamp}-{id}",
            std::process::id()
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn generous(dir: &Path, buffer: u64, n: u64) -> ChildLimits {
    let mut l = ChildLimits::required(dir, buffer, n).unwrap();
    l.io_bytes += 10_000_000;
    l.io_operations += 100_000;
    l.comparisons += 1_000_000;
    l
}
fn child_bytes(pairs: &[(u64, u64)]) -> Vec<u8> {
    pairs.iter().flat_map(|p| p.1.to_le_bytes()).collect()
}
#[test]
fn declared_lookup_reservation_covers_cold_pages_and_absent_keys() {
    let temp = Temp::new();
    let n = 1000;
    let queries = 1200;
    let limits = ChildLimits::required(&temp.0, 9, n)
        .unwrap()
        .with_lookups(queries)
        .unwrap();
    let mut links = DiskChildLinks::create(&temp.0, limits).unwrap();
    for id in (0..n).rev() {
        links.push(BasinId(2 * id + 10_000), BasinId(id)).unwrap();
    }
    links.finish(n).unwrap();
    for q in 0..queries {
        let id = (q * 683) % 2200;
        links.cache_page = None;
        let span = links.span(BasinId(10_000 + id)).unwrap();
        assert_eq!(span.count, u64::from(id < 2 * n && id % 2 == 0));
    }
    assert!(links.work().io_bytes <= limits.io_bytes);
    assert!(links.work().io_operations <= limits.io_operations);
    assert!(links.work().comparisons <= limits.comparisons);
    assert!(matches!(
        ChildLimits::required(&temp.0, 9, n)
            .unwrap()
            .with_lookups(u64::MAX),
        Err(ChildError::Overflow)
    ));
}
#[test]
fn multiple_runs_and_generations_have_exact_canonical_spans() {
    for buffer in [1, 2, 17, 512] {
        let temp = Temp::new();
        let n = 257;
        let required = ChildLimits::required(&temp.0, buffer, n).unwrap();
        let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, buffer, n)).unwrap();
        let mut expected = Vec::new();
        for i in 0..n {
            let child = (i * 73) % n;
            let parent = 10_000 + child % 17;
            links.push(BasinId(parent), BasinId(child)).unwrap();
            expected.push((parent, child));
        }
        links.finish(n).unwrap();
        expected.sort_unstable();
        assert_eq!(
            fs::read(temp.0.join(CHILD_FILE)).unwrap(),
            child_bytes(&expected)
        );
        let work = links.work();
        assert!(work.io_bytes <= required.io_bytes);
        assert!(work.io_operations <= required.io_operations);
        assert!(work.comparisons <= required.comparisons);
        let mut index = Vec::new();
        let mut offset = 0;
        for parent in 10_000..10_017 {
            let count = expected.iter().filter(|p| p.0 == parent).count() as u64;
            let span = links.span(BasinId(parent)).unwrap();
            assert_eq!(span, TableSpan { offset, count });
            index.extend_from_slice(&parent.to_le_bytes());
            index.extend_from_slice(&offset.to_le_bytes());
            index.extend_from_slice(&count.to_le_bytes());
            offset += count;
        }
        assert_eq!(fs::read(temp.0.join(SPAN_FILE)).unwrap(), index);
        assert_eq!(
            links.span(BasinId(99)).unwrap(),
            TableSpan {
                offset: 0,
                count: 0
            }
        );
        let names: Vec<_> = fs::read_dir(&temp.0)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names.len(), 3);
    }
}
#[test]
fn empty_one_link_and_large_parent_use_the_same_stream_contract() {
    for n in [0, 1, 1025] {
        let temp = Temp::new();
        let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, 7, n)).unwrap();
        for child in (0..n).rev() {
            links.push(BasinId(9000), BasinId(child)).unwrap();
        }
        links.finish(n).unwrap();
        assert_eq!(
            links.span(BasinId(9000)).unwrap(),
            TableSpan {
                offset: 0,
                count: n
            }
        );
        assert_eq!(fs::metadata(temp.0.join(CHILD_FILE)).unwrap().len(), 8 * n);
        assert_eq!(
            fs::metadata(temp.0.join(SPAN_FILE)).unwrap().len(),
            if n == 0 { 0 } else { 24 }
        );
    }
}
#[test]
fn exact_construction_reservations_cover_short_and_partial_batches() {
    for n in 0..35 {
        for buffer in [1, 2, 3, 8, 64] {
            let temp = Temp::new();
            let limits = ChildLimits::required(&temp.0, buffer, n).unwrap();
            let mut links = DiskChildLinks::create(&temp.0, limits).unwrap();
            for child in (0..n).rev() {
                links.push(BasinId(100), BasinId(child)).unwrap();
            }
            links.finish(n).unwrap();
            let work = links.work();
            assert!(work.io_bytes <= limits.io_bytes);
            assert!(work.io_operations <= limits.io_operations);
            assert!(work.comparisons <= limits.comparisons);
        }
    }
}
#[test]
fn duplicate_pairs_within_and_across_runs_abort_permanently() {
    let temp = Temp::new();
    let mut same = DiskChildLinks::create(&temp.0, generous(&temp.0, 2, 2)).unwrap();
    same.push(BasinId(9), BasinId(1)).unwrap();
    assert!(matches!(
        same.push(BasinId(9), BasinId(1)),
        Err(ChildError::Duplicate)
    ));
    assert!(matches!(same.finish(2), Err(ChildError::Poisoned)));
    let temp = Temp::new();
    let mut apart = DiskChildLinks::create(&temp.0, generous(&temp.0, 1, 66)).unwrap();
    for child in 0..65 {
        apart.push(BasinId(9000), BasinId(child)).unwrap();
    }
    apart.push(BasinId(9000), BasinId(0)).unwrap();
    assert!(matches!(apart.finish(66), Err(ChildError::Duplicate)));
    assert!(matches!(
        apart.span(BasinId(9000)),
        Err(ChildError::Poisoned)
    ));
}
#[test]
fn admission_and_namespace_failures_do_not_overwrite_files() {
    let temp = Temp::new();
    let required = ChildLimits::required(&temp.0, 2, 4).unwrap();
    for field in 0..5 {
        let mut l = required;
        match field {
            0 => l.ram_bytes -= 1,
            1 => l.scratch_bytes -= 1,
            2 => l.io_bytes -= 1,
            3 => l.io_operations -= 1,
            _ => l.comparisons -= 1,
        };
        assert!(matches!(
            DiskChildLinks::create(&temp.0, l),
            Err(ChildError::Limit(_))
        ));
        assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 0);
    }
    assert!(matches!(
        ChildLimits::required(&temp.0, 0, 1),
        Err(ChildError::Invalid(_))
    ));
    assert!(matches!(
        ChildLimits::required(&temp.0, u64::MAX, u64::MAX),
        Err(ChildError::Overflow)
    ));
    fs::write(temp.0.join("child-links.lock"), b"existing").unwrap();
    assert!(matches!(
        DiskChildLinks::create(&temp.0, required),
        Err(ChildError::Io(_))
    ));
    assert_eq!(
        fs::read(temp.0.join("child-links.lock")).unwrap(),
        b"existing"
    );
    let temp = Temp::new();
    let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, 1, 1)).unwrap();
    fs::write(temp.0.join("child-run-0-0.bin"), b"existing run").unwrap();
    assert!(matches!(
        links.push(BasinId(2), BasinId(1)),
        Err(ChildError::Io(_))
    ));
    assert_eq!(
        fs::read(temp.0.join("child-run-0-0.bin")).unwrap(),
        b"existing run"
    );
    assert!(matches!(links.finish(1), Err(ChildError::Poisoned)));
}
#[test]
fn comparison_and_dirty_buffer_limits_stop_inside_the_operation() {
    let temp = Temp::new();
    let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, 8, 8)).unwrap();
    links.budget.limit.comparisons = 2;
    for child in 0..7 {
        links.push(BasinId(100), BasinId(7 - child)).unwrap();
    }
    assert!(matches!(
        links.push(BasinId(100), BasinId(0)),
        Err(ChildError::Limit("comparisons"))
    ));
    assert_eq!(links.work().comparisons, 2);
    assert!(!temp.0.join("child-run-0-0.bin").exists());
    let temp = Temp::new();
    let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, 2, 2)).unwrap();
    links.budget.limit.io_bytes = 0;
    links.push(BasinId(10), BasinId(2)).unwrap();
    assert!(matches!(
        links.push(BasinId(10), BasinId(1)),
        Err(ChildError::Limit("I/O"))
    ));
    assert_eq!(
        fs::metadata(temp.0.join("child-run-0-0.bin"))
            .unwrap()
            .len(),
        0
    );
    drop(links);
    assert_eq!(
        fs::metadata(temp.0.join("child-run-0-0.bin"))
            .unwrap()
            .len(),
        0
    );
}
#[test]
fn counts_truncated_runs_and_corrupt_span_pages_are_rejected() {
    let temp = Temp::new();
    let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, 1, 2)).unwrap();
    links.push(BasinId(10), BasinId(1)).unwrap();
    assert!(matches!(
        links.finish(2),
        Err(ChildError::Invalid("expected link count"))
    ));
    assert!(!temp.0.join(CHILD_FILE).exists());
    let temp = Temp::new();
    let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, 1, 2)).unwrap();
    for i in 0..2 {
        links.push(BasinId(10), BasinId(i)).unwrap();
    }
    fs::write(temp.0.join("child-run-0-0.bin"), [0; 15]).unwrap();
    assert!(matches!(
        links.finish(2),
        Err(ChildError::Corrupt("run length"))
    ));
    let temp = Temp::new();
    let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, 3, 3)).unwrap();
    for i in 0..3 {
        links.push(BasinId(10 + i), BasinId(i)).unwrap();
    }
    links.finish(3).unwrap();
    let mut bytes = fs::read(temp.0.join(SPAN_FILE)).unwrap();
    bytes[16..24].fill(0);
    fs::write(temp.0.join(SPAN_FILE), bytes).unwrap();
    assert!(matches!(
        links.span(BasinId(10)),
        Err(ChildError::Corrupt("span bounds"))
    ));
    assert!(matches!(links.span(BasinId(11)), Err(ChildError::Poisoned)));
}
#[test]
fn lookup_pages_are_bounded_and_budget_exhaustion_poisoned() {
    let temp = Temp::new();
    let n = 1000;
    let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, 31, n)).unwrap();
    for i in (0..n).rev() {
        links.push(BasinId(10_000 + i), BasinId(i)).unwrap();
    }
    links.finish(n).unwrap();
    for i in [0, 169, 170, 340, 511, 999, 341] {
        assert_eq!(
            links.span(BasinId(10_000 + i)).unwrap(),
            TableSpan {
                offset: i,
                count: 1
            }
        );
        assert!(links.cache_rows <= 170);
    }
    links.budget.limit.comparisons = links.work().comparisons;
    assert!(matches!(
        links.span(BasinId(10_501)),
        Err(ChildError::Limit("comparisons"))
    ));
    assert!(matches!(
        links.span(BasinId(10_501)),
        Err(ChildError::Poisoned)
    ));
}
#[test]
fn real_buffered_write_error_is_charged_and_never_retried_on_drop() {
    let temp = Temp::new();
    let mut budget = Budget {
        limit: generous(&temp.0, 2, 2),
        work: ChildWork::default(),
    };
    let path = temp.0.join("read-only-writer.bin");
    let mut writer = Writer::create(&path, &mut budget).unwrap();
    writer.row(&[7; 32], &mut budget).unwrap();
    writer.file = File::open(&path).unwrap();
    assert!(matches!(
        writer.finish(32, &mut budget),
        Err(ChildError::Io(_))
    ));
    assert_eq!(budget.work.io_bytes, 32);
    assert_eq!(fs::metadata(path).unwrap().len(), 0);
}
#[test]
fn individually_bounded_span_cannot_omit_the_child_table_prefix() {
    let temp = Temp::new();
    let mut links = DiskChildLinks::create(&temp.0, generous(&temp.0, 3, 3)).unwrap();
    for child in 0..3 {
        links.push(BasinId(10), BasinId(child)).unwrap();
    }
    links.finish(3).unwrap();
    let mut bytes = fs::read(temp.0.join(SPAN_FILE)).unwrap();
    bytes[8..16].copy_from_slice(&1_u64.to_le_bytes());
    bytes[16..24].copy_from_slice(&2_u64.to_le_bytes());
    fs::write(temp.0.join(SPAN_FILE), bytes).unwrap();
    assert!(matches!(
        links.span(BasinId(10)),
        Err(ChildError::Corrupt("span coverage"))
    ));
}
