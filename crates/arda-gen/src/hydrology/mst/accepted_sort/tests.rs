use super::runs::{decode, encode, header};
use super::*;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let id = NEXT.fetch_add(1, AtomicOrdering::Relaxed);
        let p = std::env::temp_dir().join(format!(
            "arda-accepted-sort-{}-{now}-{id}",
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
fn extent() -> Extent {
    Extent::new(1024, 3).unwrap()
}
fn edge(i: u32) -> Saddle {
    let e = extent();
    let from = (i * 17) % 1000;
    Saddle {
        left: Node::Closed(CellIndex::new(i % 101 + 1, e).unwrap()),
        right: Node::Exterior,
        sill_mm: i32::try_from(i % 7).unwrap() - 3,
        from: CellIndex::new(from, e).unwrap(),
        to: Some(CellIndex::new(from + 1, e).unwrap()),
    }
}
fn collect(reader: &mut EdgeReader) -> Vec<Saddle> {
    reader.map(|v| v.unwrap()).collect()
}
#[test]
fn exact_witness_bytes_across_tiny_runs_and_multiple_generations() {
    let e = extent();
    let mut expected: Vec<Saddle> = (0..257).map(edge).collect();
    expected.sort_unstable_by_key(|&a| edge_key(e, a));
    let expected_bytes: Vec<u8> = expected.iter().flat_map(|a| encode(*a)).collect();
    for buffer in [1, 2, 17, 512] {
        let dir = Temp::new();
        let limits = SortLimits::required(&dir.0, 257, buffer).unwrap();
        let mut sort = AcceptedSorter::create(&dir.0, e, 257, limits).unwrap();
        for i in 0..257 {
            sort.push(edge(i * 73 % 257)).unwrap();
        }
        let mut reader = sort.finish().unwrap();
        assert_eq!(reader.expected_count(), 257);
        assert_eq!(collect(&mut reader), expected);
        assert!(reader.next().is_none());
        let bytes = fs::read(reader.path()).unwrap();
        assert_eq!(&bytes[..32], &header(e, 257).unwrap());
        assert_eq!(&bytes[32..], &expected_bytes);
        let work = reader.work();
        assert_eq!(work.io_bytes, limits.io_bytes);
        assert_eq!(work.io_operations, limits.io_operations);
        assert!(work.comparisons <= limits.comparisons);
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
    }
}
#[test]
fn exact_reservations_cover_empty_short_and_partial_runs() {
    for n in 0..35 {
        for buffer in [1, 2, 3, 8, 64] {
            let dir = Temp::new();
            let limits = SortLimits::required(&dir.0, n, buffer).unwrap();
            let mut sort = AcceptedSorter::create(&dir.0, extent(), n, limits).unwrap();
            for i in (0..u32::try_from(n).unwrap()).rev() {
                sort.push(edge(i)).unwrap();
            }
            let mut reader = sort.finish().unwrap();
            assert_eq!(collect(&mut reader).len(), usize::try_from(n).unwrap());
            assert_eq!(reader.work().io_bytes, limits.io_bytes);
            assert_eq!(reader.work().io_operations, limits.io_operations);
            assert!(reader.work().comparisons <= limits.comparisons);
        }
    }
}
#[test]
fn duplicates_in_one_batch_and_across_runs_poison() {
    for buffer in [1, 2, 66] {
        let dir = Temp::new();
        let limits = SortLimits::required(&dir.0, 66, buffer).unwrap();
        let mut sort = AcceptedSorter::create(&dir.0, extent(), 66, limits).unwrap();
        let mut failed = false;
        for i in 0..66 {
            if let Err(err) = sort.push(edge(if i == 65 { 0 } else { i })) {
                assert!(matches!(err, SortError::Duplicate));
                failed = true;
                break;
            }
        }
        if failed {
            assert!(matches!(sort.push(edge(90)), Err(SortError::Poisoned)));
            assert!(matches!(sort.finish(), Err(SortError::Poisoned)));
        } else {
            assert!(matches!(sort.finish(), Err(SortError::Duplicate)));
        }
    }
}
#[test]
fn admission_limits_and_namespace_collisions_precede_mutation() {
    for field in 0..5 {
        let dir = Temp::new();
        let mut limits = SortLimits::required(&dir.0, 100, 9).unwrap();
        match field {
            0 => limits.ram_bytes -= 1,
            1 => limits.scratch_bytes -= 1,
            2 => limits.io_bytes -= 1,
            3 => limits.io_operations -= 1,
            _ => limits.comparisons -= 1,
        };
        assert!(matches!(
            AcceptedSorter::create(&dir.0, extent(), 100, limits),
            Err(SortError::Limit(_))
        ));
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 0);
    }
    let dir = Temp::new();
    assert!(SortLimits::required(&dir.0, 1, 0).is_err());
    assert!(SortLimits::required(&dir.0, u64::MAX, 1).is_err());
    let limits = SortLimits::required(&dir.0, 1, 1).unwrap();
    fs::write(dir.0.join("accepted-sort.lock"), b"old").unwrap();
    assert!(matches!(
        AcceptedSorter::create(&dir.0, extent(), 1, limits),
        Err(SortError::Io(_))
    ));
    assert_eq!(fs::read(dir.0.join("accepted-sort.lock")).unwrap(), b"old");
    let dir = Temp::new();
    let limits = SortLimits::required(&dir.0, 1, 1).unwrap();
    let mut sort = AcceptedSorter::create(&dir.0, extent(), 1, limits).unwrap();
    fs::write(dir.0.join("accepted-run-0-0.bin"), b"old").unwrap();
    assert!(matches!(sort.push(edge(0)), Err(SortError::Io(_))));
    assert_eq!(
        fs::read(dir.0.join("accepted-run-0-0.bin")).unwrap(),
        b"old"
    );
}
#[test]
fn sort_comparison_and_real_dirty_write_fail_without_retry() {
    let dir = Temp::new();
    let limits = SortLimits::required(&dir.0, 4, 4).unwrap();
    let mut sort = AcceptedSorter::create(&dir.0, extent(), 4, limits).unwrap();
    sort.budget.limits.comparisons = 2;
    for i in 0..3 {
        sort.push(edge(i)).unwrap();
    }
    assert!(matches!(
        sort.push(edge(3)),
        Err(SortError::Limit("comparisons"))
    ));
    assert_eq!(sort.work().comparisons, 2);
    assert!(matches!(sort.finish(), Err(SortError::Poisoned)));
    let dir = Temp::new();
    let limits = SortLimits::required(&dir.0, 1, 1).unwrap();
    let mut budget = Budget {
        limits,
        work: SortWork::default(),
    };
    let path = dir.0.join("real-error.bin");
    let mut writer = Writer::create(&path, extent(), 1, &mut budget).unwrap();
    writer.row(edge(0), &mut budget).unwrap();
    writer.file = File::open(&path).unwrap();
    assert!(matches!(
        writer.finish(1, &mut budget),
        Err(SortError::Io(_))
    ));
    assert_eq!(budget.work.io_bytes, 64);
    assert_eq!(fs::metadata(path).unwrap().len(), 0);
}
#[test]
fn invalid_edges_counts_and_truncated_runs_abort() {
    let dir = Temp::new();
    let limits = SortLimits::required(&dir.0, 2, 1).unwrap();
    let mut sort = AcceptedSorter::create(&dir.0, extent(), 2, limits).unwrap();
    sort.push(edge(0)).unwrap();
    assert!(matches!(
        sort.finish(),
        Err(SortError::Invalid("expected edge count"))
    ));
    let dir = Temp::new();
    let limits = SortLimits::required(&dir.0, 1, 1).unwrap();
    let mut sort = AcceptedSorter::create(&dir.0, extent(), 1, limits).unwrap();
    let mut bad = edge(0);
    bad.right = bad.left;
    assert!(matches!(sort.push(bad), Err(SortError::Invalid("saddle"))));
    assert!(matches!(sort.finish(), Err(SortError::Poisoned)));
    let dir = Temp::new();
    let limits = SortLimits::required(&dir.0, 2, 1).unwrap();
    let mut sort = AcceptedSorter::create(&dir.0, extent(), 2, limits).unwrap();
    sort.push(edge(0)).unwrap();
    sort.push(edge(1)).unwrap();
    OpenOptions::new()
        .write(true)
        .open(dir.0.join("accepted-run-0-0.bin"))
        .unwrap()
        .set_len(63)
        .unwrap();
    assert!(matches!(
        sort.finish(),
        Err(SortError::Corrupt("file length"))
    ));
}
#[test]
fn final_reader_checks_payload_header_order_and_owned_budget() {
    for mutation in 0..4 {
        let dir = Temp::new();
        let limits = SortLimits::required(&dir.0, 2, 2).unwrap();
        let mut sort = AcceptedSorter::create(&dir.0, extent(), 2, limits).unwrap();
        sort.push(edge(0)).unwrap();
        sort.push(edge(1)).unwrap();
        if mutation == 0 {
            let p = dir.0.join("accepted-run-0-0.bin");
            let mut b = fs::read(&p).unwrap();
            b[8] ^= 1;
            fs::write(&p, b).unwrap();
            assert!(matches!(sort.finish(), Err(SortError::Corrupt("header"))));
            continue;
        }
        let mut reader = sort.finish().unwrap();
        if mutation == 3 {
            reader.budget.limits.io_bytes = reader.work().io_bytes;
            assert!(matches!(reader.next(), Some(Err(SortError::Limit("I/O")))));
            assert!(reader.next().is_none());
            continue;
        }
        let mut b = fs::read(reader.path()).unwrap();
        if mutation == 1 {
            b[32 + 8] ^= 1;
        } else {
            let a = b[32..64].to_vec();
            b.copy_within(64..96, 32);
            b[64..96].copy_from_slice(&a);
        }
        fs::write(reader.path(), b).unwrap();
        if mutation == 1 {
            assert!(matches!(
                reader.next(),
                Some(Err(SortError::Corrupt("checksum")))
            ));
        } else {
            assert!(reader.next().unwrap().is_ok());
            assert!(matches!(
                reader.next(),
                Some(Err(SortError::Corrupt("run order")))
            ));
        }
        assert!(reader.next().is_none());
    }
}
#[test]
fn hierarchy_key_preserves_real_export_witness_and_signed_sill() {
    let e = extent();
    let mut a = edge(0);
    a.from = CellIndex::new(1023, e).unwrap();
    a.to = None;
    a.sill_mm = i32::MIN;
    assert_eq!(decode(&encode(a), e).unwrap(), a);
    let k = edge_key(e, a);
    assert_eq!(k.0, i32::MIN);
    assert_eq!(k.2, u64::MAX);
    assert_eq!(k.3, 1023);
    assert_eq!(k.4, u64::MAX);
    let mut b = edge(1);
    b.left = Node::Closed(CellIndex::new(5, e).unwrap());
    b.right = Node::Closed(CellIndex::new(1025, e).unwrap());
    b.from = CellIndex::new(1024, e).unwrap();
    b.to = Some(CellIndex::new(1, e).unwrap());
    assert_eq!(decode(&encode(b), e).unwrap(), b);
    assert_eq!(edge_key(e, b).3, 1);
    assert_eq!(edge_key(e, b).4, 1u64 << 32);
}
#[test]
fn retained_file_replays_with_exact_independent_pass_admission() {
    let t = Temp::new();
    let n = 257;
    let limits = SortLimits::required(&t.0, n, 2).unwrap();
    let mut sorter = AcceptedSorter::create(&t.0, extent(), n, limits).unwrap();
    for i in (0..u32::try_from(n).unwrap()).rev() {
        sorter.push(edge(i)).unwrap();
    }
    let mut original = sorter.finish().unwrap();
    let first = original.next().unwrap().unwrap();
    let read_limits = ReadLimits::required(original.path(), n).unwrap();
    let mut replay = original.replay(read_limits).unwrap();
    let all = collect(&mut replay);
    assert_eq!(all.len(), usize::try_from(n).unwrap());
    assert_eq!(all[0], first);
    let work = replay.work();
    assert_eq!(work.io_bytes, read_limits.io_bytes);
    assert_eq!(work.io_operations, read_limits.io_operations);
    assert_eq!(work.comparisons, read_limits.comparisons);
    assert_eq!(collect(&mut original), all[1..]);
    let mut second = EdgeReader::open(replay.path(), extent(), n, read_limits).unwrap();
    assert_eq!(collect(&mut second), all);
    assert!(replay.path().exists());
}
#[test]
fn replay_admission_precedes_open_and_codec_failure_poison_is_preserved() {
    let t = Temp::new();
    let missing = t.0.join("missing");
    let exact = ReadLimits::required(&missing, 2).unwrap();
    for insufficient in [
        ReadLimits {
            ram_bytes: exact.ram_bytes - 1,
            ..exact
        },
        ReadLimits {
            io_bytes: exact.io_bytes - 1,
            ..exact
        },
        ReadLimits {
            io_operations: exact.io_operations - 1,
            ..exact
        },
        ReadLimits {
            comparisons: exact.comparisons - 1,
            ..exact
        },
    ] {
        assert!(matches!(
            EdgeReader::open(&missing, extent(), 2, insufficient),
            Err(SortError::Limit(_))
        ));
    }
    let mut sorter =
        AcceptedSorter::create(&t.0, extent(), 2, SortLimits::required(&t.0, 2, 2).unwrap())
            .unwrap();
    sorter.push(edge(1)).unwrap();
    sorter.push(edge(2)).unwrap();
    let original = sorter.finish().unwrap();
    let limits = ReadLimits::required(original.path(), 2).unwrap();
    let mut bytes = fs::read(original.path()).unwrap();
    bytes[40] ^= 1;
    fs::write(original.path(), bytes).unwrap();
    let mut replay = original.replay(limits).unwrap();
    assert!(matches!(replay.next().unwrap(), Err(SortError::Corrupt(_))));
    assert!(replay.next().is_none());
    assert!(matches!(replay.replay(limits), Err(SortError::Poisoned)));
    assert!(matches!(
        EdgeReader::open(
            original.path(),
            extent(),
            3,
            ReadLimits::required(original.path(), 3).unwrap()
        ),
        Err(SortError::Corrupt(_))
    ));
    assert!(matches!(
        ReadLimits::required(&missing, u64::MAX),
        Err(SortError::Overflow)
    ));
}
#[test]
fn empty_accepted_file_replays_as_an_exact_header_only_pass() {
    let t = Temp::new();
    let original =
        AcceptedSorter::create(&t.0, extent(), 0, SortLimits::required(&t.0, 0, 1).unwrap())
            .unwrap()
            .finish()
            .unwrap();
    let limits = ReadLimits::required(original.path(), 0).unwrap();
    let mut replay = original.replay(limits).unwrap();
    assert!(replay.next().is_none());
    assert_eq!(replay.work().io_bytes, 32);
    assert_eq!(replay.work().io_operations, 3);
    assert_eq!(replay.work().comparisons, 0);
}
