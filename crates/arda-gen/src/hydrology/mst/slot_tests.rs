use super::super::{hierarchy::Minimum, test_support::Directory};
use super::*;
use std::{
    fs,
    io::{Seek, SeekFrom, Write},
};
fn store(leaves: u32, cache_pages: u64) -> (Directory, SlotStore) {
    let d = Directory::new();
    let e = Extent::new(200, 2).unwrap();
    let mut meter = Meter::new(u128::MAX, u64::MAX);
    let mut minima = Scratch::create(d.0.join("minima"), &mut meter).unwrap();
    minima
        .write(
            0,
            &io::header(e, 0, u64::from(leaves), 16).unwrap(),
            &mut meter,
        )
        .unwrap();
    for i in 0..leaves {
        let m = Minimum {
            at: CellIndex::new(i, e).unwrap(),
            floor_mm: i32::try_from(i).unwrap() - 50,
        };
        minima
            .write(64 + 16 * u64::from(i), &io::minimum_bytes(m), &mut meter)
            .unwrap();
    }
    let s = SlotStore::create(
        &d.0,
        e,
        leaves,
        &mut minima,
        meter,
        Limits {
            cache_pages,
            reads: u64::MAX,
            writes: u64::MAX,
            comparisons: u64::MAX,
        },
    )
    .unwrap();
    (d, s)
}
#[test]
fn eviction_and_partial_flush_keep_dirty_pages_after_budget_error() {
    let (_d, mut s) = store(130, 2);
    let mut a = s.read(0).unwrap();
    a.parent = 1;
    s.write(0, a).unwrap();
    let mut b = s.read(64).unwrap();
    b.parent = 65;
    s.write(64, b).unwrap();
    s.meter.bytes = s.meter.work.bytes + 4096;
    assert!(matches!(s.flush(), Err(StageError::Limit("I/O bytes"))));
    assert_eq!(s.cache.len(), 1);
    assert_eq!(s.fifo.iter().copied().collect::<Vec<_>>(), vec![1]);
    assert_eq!(s.read(64).unwrap(), b);
    s.meter.bytes = u128::MAX;
    s.flush().unwrap();
    assert_eq!(s.read(0).unwrap(), a);
    assert_eq!(s.read(64).unwrap(), b);
    let (_d, mut s) = store(130, 1);
    let mut a = s.read(0).unwrap();
    a.parent = 1;
    s.write(0, a).unwrap();
    s.meter.operations = s.meter.work.operations;
    assert!(matches!(
        s.read(64),
        Err(StageError::Limit("I/O operations"))
    ));
    assert_eq!(s.cache.len(), 1);
    assert_eq!(s.fifo.iter().copied().collect::<Vec<_>>(), vec![0]);
    assert_eq!(s.read(0).unwrap(), a);
}
#[test]
fn actual_read_only_file_failure_retains_dirty_cache() {
    let (_d, mut s) = store(130, 1);
    let mut row = s.read(0).unwrap();
    row.parent = 1;
    s.write(0, row).unwrap();
    s.file.file = fs::File::open(&s.file.path).unwrap();
    assert!(matches!(s.read(64), Err(StageError::Io { .. })));
    assert_eq!(s.fifo.len(), 1);
    assert_eq!(s.read(0).unwrap(), row);
    assert!(matches!(s.flush(), Err(StageError::Io { .. })));
    assert_eq!(s.read(0).unwrap(), row);
}
#[test]
fn checked_file_header_checksum_padding_truncation_and_length() {
    for case in 0..6 {
        let (_d, mut s) = store(3, 1);
        match case {
            0 => {
                s.file.file.seek(SeekFrom::Start(0)).unwrap();
                s.file.file.write_all(b"x").unwrap();
            }
            1 => {
                s.file.file.seek(SeekFrom::Start(64 + 8)).unwrap();
                s.file.file.write_all(&[99]).unwrap();
            }
            2 => {
                s.file.file.seek(SeekFrom::Start(64 + 4 * 64)).unwrap();
                s.file.file.write_all(&[1]).unwrap();
            }
            3 => s.file.file.set_len(100).unwrap(),
            4 => s.file.file.set_len(9000).unwrap(),
            _ => {
                let bytes = fs::read(&s.file.path).unwrap();
                s.file.file.seek(SeekFrom::Start(128)).unwrap();
                s.file.file.write_all(&bytes[64..128]).unwrap();
            }
        }
        assert!(matches!(s.read(0), Err(StageError::Invalid(_))));
        assert!(s.cache.is_empty());
        assert!(s.fifo.is_empty());
    }
}
#[test]
fn explicit_slot_binding_cycle_immutable_and_operation_failures() {
    let (_d, mut s) = store(130, 1);
    let at = CellIndex::new(129, s.extent).unwrap();
    assert_eq!(s.bind(at).unwrap(), 129);
    assert!(matches!(
        s.bind(CellIndex::new(199, s.extent).unwrap()),
        Err(StageError::Invalid("unknown closed owner"))
    ));
    let mut a = s.read(0).unwrap();
    let mut b = s.read(1).unwrap();
    a.parent = 1;
    b.parent = 0;
    s.write(0, a).unwrap();
    s.write(1, b).unwrap();
    assert!(matches!(s.root(0), Err(StageError::Cycle)));
    a.floor_mm += 1;
    assert!(matches!(
        s.write(0, a),
        Err(StageError::Invalid("changed immutable minimum"))
    ));
    s.read_limit = s.work.reads;
    assert!(matches!(s.read(0), Err(StageError::Limit("slot reads"))));
    s.write_limit = s.work.writes;
    assert!(matches!(
        s.write(1, b),
        Err(StageError::Limit("slot writes"))
    ));
    s.comparison_limit = s.work.comparisons;
    assert!(matches!(
        s.compare(),
        Err(StageError::Limit("key comparisons"))
    ));
}
#[test]
fn malformed_minima_are_rejected_before_component_initialization() {
    let d = Directory::new();
    let e = Extent::new(3, 3).unwrap();
    let mut meter = Meter::new(u128::MAX, u64::MAX);
    let mut minima = Scratch::create(d.0.join("minima"), &mut meter).unwrap();
    minima.write(0, &[0; 64], &mut meter).unwrap();
    assert!(matches!(
        SlotStore::create(
            &d.0,
            e,
            0,
            &mut minima,
            meter,
            Limits {
                cache_pages: 1,
                reads: 100,
                writes: 100,
                comparisons: 100
            }
        ),
        Err(StageError::Invalid("minimum header/length"))
    ));
    assert!(!d.0.join("mst-slots.pages").exists());
}

#[test]
fn final_binding_equality_is_charged_and_exterior_short_circuits() {
    for allowance in [2, 3] {
        let (_d, mut s) = store(3, 1);
        let at = CellIndex::new(1, s.extent).unwrap();
        s.comparison_limit = s.work.comparisons + allowance;
        let result = s.bind(at);
        if allowance == 2 {
            assert!(matches!(result, Err(StageError::Limit("key comparisons"))));
        } else {
            assert_eq!(result.unwrap(), 1);
        }
        assert_eq!(s.work.comparisons, s.comparison_limit);
    }
    let (_d, mut s) = store(3, 1);
    s.comparison_limit = s.work.comparisons + 2;
    let at = CellIndex::new(199, s.extent).unwrap();
    assert!(matches!(
        s.bind(at),
        Err(StageError::Invalid("unknown closed owner"))
    ));
    assert_eq!(s.work.comparisons, s.comparison_limit);
}
