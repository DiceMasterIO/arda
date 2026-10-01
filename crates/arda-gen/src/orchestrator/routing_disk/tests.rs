use super::super::routing::{route_and_own, Limits, MemoryPages};
use super::*;
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "arda-routing-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn limits(e: Extent, cache_pages: u64) -> DiskLimits {
    DiskLimits {
        cache_bytes: (cache_pages + 1) * CACHE_CHARGE,
        scratch_bytes: DiskRoutingStore::scratch_required(e),
        io_bytes: 1_000_000_000,
        io_operations: 1_000_000,
    }
}

#[test]
fn one_page_cache_and_larger_cache_match_memory_and_persist_identical_records() {
    let extent = Extent::new(17, 35).unwrap();
    let heights: Vec<i32> = (0..extent.cells())
        .map(|i| if i == 593 { 0 } else { 100 })
        .collect();
    let marine = vec![false; heights.len()];
    let mut memory = MemoryPages::new(extent, &heights, &marine, 1_000_000).unwrap();
    route_and_own(&mut memory, Limits::for_extent(extent)).unwrap();
    let mut outputs = Vec::new();
    for capacity in [1, 8] {
        let temp = Temp::new();
        let mut disk = DiskRoutingStore::create(
            &temp.0,
            extent,
            heights.iter().map(|&h| CellRecord::prepared(h, false)),
            limits(extent, capacity),
        )
        .unwrap();
        route_and_own(&mut disk, Limits::for_extent(extent)).unwrap();
        for raw in 0..extent.cells() {
            let i = CellIndex::new(raw, extent).unwrap();
            assert_eq!(disk.read(i).unwrap(), memory.read(i).unwrap());
        }
        disk.flush().unwrap();
        let bytes = std::fs::read(disk.records_path()).unwrap();
        for page in 0..heights.len().div_ceil(PAGE_CELLS) {
            assert_eq!(
                &bytes[32 + page * 4096..32 + (page + 1) * 4096],
                &memory.encode_page(page).unwrap()
            );
        }
        outputs.push(bytes);
    }
    assert_eq!(outputs[0], outputs[1]);
}

#[test]
fn reservations_fail_before_files_and_io_budget_fails_explicitly() {
    let e = Extent::new(2, 2).unwrap();
    let temp = Temp::new();
    let mut l = limits(e, 1);
    l.scratch_bytes -= 1;
    assert!(matches!(
        DiskRoutingStore::create(&temp.0, e, [CellRecord::prepared(0, false); 4], l),
        Err(DiskError::Limit("scratch bytes"))
    ));
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 0);
    l = limits(e, 1);
    l.io_bytes = 31;
    assert!(matches!(
        DiskRoutingStore::create(&temp.0, e, [CellRecord::prepared(0, false); 4], l),
        Err(DiskError::Limit("I/O bytes"))
    ));
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 0);
    l = limits(e, 1);
    l.io_operations = 0;
    assert!(matches!(
        DiskRoutingStore::create(&temp.0, e, [CellRecord::prepared(0, false); 4], l),
        Err(DiskError::Limit("I/O operations"))
    ));
    assert_eq!(std::fs::read_dir(&temp.0).unwrap().count(), 0);
}

#[test]
fn tapes_reuse_capacity_but_never_expose_unwritten_entries() {
    let e = Extent::new(2, 2).unwrap();
    let temp = Temp::new();
    let mut disk = DiskRoutingStore::create(
        &temp.0,
        e,
        [CellRecord::prepared(0, false); 4],
        limits(e, 1),
    )
    .unwrap();
    for i in 0..4 {
        disk.push(Tape::Component, CellIndex::new(i, e).unwrap())
            .unwrap();
    }
    assert!(disk
        .push(Tape::Component, CellIndex::new(0, e).unwrap())
        .is_err());
    for i in (0..4).rev() {
        assert_eq!(disk.get(Tape::Component, i).unwrap().raw(), i);
    }
    disk.clear(Tape::Component).unwrap();
    assert!(disk.get(Tape::Component, 0).is_err());
    disk.push(Tape::Component, CellIndex::new(3, e).unwrap())
        .unwrap();
    assert_eq!(disk.get(Tape::Component, 0).unwrap().raw(), 3);
    assert!(disk.get(Tape::Component, 1).is_err());
}

#[test]
fn changed_terrain_and_corrupt_records_are_refused() {
    let e = Extent::new(2, 2).unwrap();
    let temp = Temp::new();
    let mut disk = DiskRoutingStore::create(
        &temp.0,
        e,
        [CellRecord::prepared(0, false); 4],
        limits(e, 1),
    )
    .unwrap();
    let at = CellIndex::new(0, e).unwrap();
    assert!(matches!(
        disk.write(at, CellRecord::prepared(1, false)),
        Err(DiskError::Invalid("immutable terrain"))
    ));
    disk.flush().unwrap();
    let mut file = OpenOptions::new()
        .write(true)
        .open(disk.records_path())
        .unwrap();
    file.seek(SeekFrom::Start(32 + 14)).unwrap();
    file.write_all(&[1]).unwrap();
    assert!(matches!(
        disk.read(at),
        Err(DiskError::Invalid("stored record"))
    ));
}

fn owned_record(extent: Extent, at: CellIndex) -> CellRecord {
    let mut bytes = CellRecord::prepared(0, false).encode();
    bytes[8..12].copy_from_slice(&at.raw().to_le_bytes());
    CellRecord::decode(bytes, extent, at).unwrap()
}

#[test]
fn failed_dirty_eviction_keeps_the_current_cached_value() {
    let extent = Extent::new(300, 1).unwrap();
    let temp = Temp::new();
    let mut disk = DiskRoutingStore::create(
        &temp.0,
        extent,
        [CellRecord::prepared(0, false); 300],
        limits(extent, 1),
    )
    .unwrap();
    let first = CellIndex::new(0, extent).unwrap();
    let changed = owned_record(extent, first);
    disk.write(first, changed).unwrap();
    disk.meter.limits.io_bytes = disk.work().bytes;
    assert!(matches!(
        disk.read(CellIndex::new(299, extent).unwrap()),
        Err(DiskError::Limit("I/O bytes"))
    ));
    assert_eq!(disk.read(first).unwrap(), changed);
    assert_eq!(disk.fifo.iter().copied().collect::<Vec<_>>(), [0]);
    assert!(disk.cache[&0].dirty);
}

#[test]
fn failed_flush_keeps_remaining_dirty_pages_and_fifo_consistent() {
    let extent = Extent::new(300, 1).unwrap();
    let temp = Temp::new();
    let mut disk = DiskRoutingStore::create(
        &temp.0,
        extent,
        [CellRecord::prepared(0, false); 300],
        limits(extent, 2),
    )
    .unwrap();
    let first = CellIndex::new(0, extent).unwrap();
    let last = CellIndex::new(299, extent).unwrap();
    disk.write(first, owned_record(extent, first)).unwrap();
    let changed = owned_record(extent, last);
    disk.write(last, changed).unwrap();
    disk.meter.limits.io_bytes = disk.work().bytes + PAGE_BYTES as u128;
    assert!(matches!(disk.flush(), Err(DiskError::Limit("I/O bytes"))));
    assert_eq!(disk.cache.len(), 1);
    assert_eq!(disk.fifo.iter().copied().collect::<Vec<_>>(), [1]);
    assert_eq!(disk.read(last).unwrap(), changed);
    assert!(disk.cache[&1].dirty);
}
