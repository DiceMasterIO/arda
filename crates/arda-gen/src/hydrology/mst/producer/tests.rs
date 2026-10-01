use super::super::test_support::Directory;
use super::*;

#[test]
fn final_minimum_reader_owns_remaining_comparison_and_io_budgets() {
    for comparison_failure in [false, true] {
        let directory = Directory::new();
        let extent = Extent::new(3, 3).unwrap();
        let mut meter = Meter::new(u128::MAX, u64::MAX);
        let mut file = Scratch::create(directory.0.join("minima"), &mut meter).unwrap();
        file.write(0, &io::header(extent, 0, 2, 16).unwrap(), &mut meter)
            .unwrap();
        for i in 0..2 {
            file.write(
                64 + u64::from(i) * 16,
                &io::minimum_bytes(Minimum {
                    at: CellIndex::new(i, extent).unwrap(),
                    floor_mm: 0,
                }),
                &mut meter,
            )
            .unwrap();
        }
        let mut reader = MinimumReader::new(file, extent, 2, meter, 0, 0).unwrap();
        if comparison_failure {
            assert!(reader.next().unwrap().is_ok());
            assert!(matches!(
                reader.next(),
                Some(Err(StageError::Limit("key comparisons")))
            ));
        } else {
            reader.meter.bytes = reader.meter.work.bytes;
            assert!(matches!(
                reader.next(),
                Some(Err(StageError::Limit("I/O bytes")))
            ));
        }
        assert!(reader.next().is_none());
    }
}

#[test]
fn disconnected_frozen_component_is_a_typed_failure() {
    let directory = Directory::new();
    let extent = Extent::new(3, 3).unwrap();
    let mut meter = Meter::new(u128::MAX, u64::MAX);
    let mut minima = Scratch::create(directory.0.join("minima"), &mut meter).unwrap();
    minima
        .write(0, &io::header(extent, 0, 1, 16).unwrap(), &mut meter)
        .unwrap();
    minima
        .write(
            64,
            &io::minimum_bytes(Minimum {
                at: CellIndex::new(4, extent).unwrap(),
                floor_mm: 0,
            }),
            &mut meter,
        )
        .unwrap();
    let mut slots = SlotStore::create(
        &directory.0,
        extent,
        1,
        &mut minima,
        meter,
        super::super::slots::Limits {
            cache_pages: 1,
            reads: 100,
            writes: 100,
            comparisons: 100,
        },
    )
    .unwrap();
    let cap = SortLimits::required(&directory.0, 1, 2).unwrap();
    let mut sorter = AcceptedSorter::create(&directory.0, extent, 1, cap).unwrap();
    assert!(matches!(
        finish_round(&mut slots, &mut sorter, &mut Work::default()),
        Err(MstError::Stage(StageError::Disconnected))
    ));
}
