use super::*;
use arda_core::{GenerateConfig, HeightMm, LatitudeBand, RainfallMm, SizeKm};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "arda-prepared-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn domain() -> PreparedDomain {
    PreparedDomain::for_config(
        GenerateConfig::new(SizeKm::new(64, 64), LatitudeBand::new(35, 55), 15).unwrap(),
    )
    .unwrap()
}
fn limits(path: &Path, d: PreparedDomain, cache: u32) -> Limits {
    Limits {
        ram_bytes: ram_required(path, d, cache).unwrap(),
        scratch_bytes: scratch_required(d),
        cache_tiles: cache,
        io_bytes: 1 << 32,
        io_operations: 10000,
        tile_queries: 10000,
    }
}
fn terrain(d: PreparedDomain, i: u32) -> PreparedTerrain {
    let e = d.entry(i).unwrap();
    let mut heights = Vec::new();
    let mut annual_rain = Vec::new();
    let mut temperature_base_centi = Vec::new();
    for y in 0..512 {
        for x in 0..512 {
            let ax = e.area.x * 512 + x;
            let ay = e.area.y * 512 + y;
            heights.push(HeightMm::new(ax - 2 * ay));
            annual_rain.push(RainfallMm::new(u16::try_from((ax + ay) % 65536).unwrap()));
            temperature_base_centi.push(40000 - ax - 3 * ay);
        }
    }
    PreparedTerrain {
        area: e.area,
        valid: e.valid,
        heights,
        annual_rain,
        temperature_base_centi,
    }
}
fn write_all(path: &Path, d: PreparedDomain, order: &[u32], cache: u32) -> PreparedReader {
    let mut w = PreparedWriter::new(path, d, limits(path, d, cache)).unwrap();
    for &i in order {
        w.write(&terrain(d, i)).unwrap();
    }
    w.finish().unwrap()
}
#[test]
fn canonical_bytes_and_cells_ignore_completion_order_cache_and_reload() {
    let d = domain();
    let orders = [[0, 1, 2, 3], [3, 2, 1, 0], [2, 0, 3, 1]];
    let mut baseline = None;
    for order in orders {
        let temp = Temp::new();
        let reader = write_all(&temp.0, d, &order, 1);
        assert!(reader.work().io_bytes > 0);
        drop(reader);
        let bytes: Vec<_> = (0..d.count())
            .map(|i| std::fs::read(tile_path(&temp.0, i)).unwrap())
            .chain(std::iter::once(
                std::fs::read(temp.0.join("prepared-index.bin")).unwrap(),
            ))
            .collect();
        if let Some(before) = &baseline {
            assert_eq!(&bytes, before);
        } else {
            baseline = Some(bytes.clone());
        }
        for capacity in [1, 2, 4] {
            let mut reader =
                PreparedReader::open(&temp.0, d, limits(&temp.0, d, capacity)).unwrap();
            for (x, y) in [
                (0, 0),
                (511, 511),
                (512, 511),
                (511, 512),
                (512, 512),
                (639, 639),
                (0, 0),
            ] {
                let cell = reader.cell(GlobalCell { x, y }).unwrap();
                let x = i32::try_from(x).unwrap();
                let y = i32::try_from(y).unwrap();
                assert_eq!(cell.height.raw(), x - 2 * y);
                assert_eq!(i32::from(cell.annual_rain.raw()), x + y);
                assert_eq!(cell.temperature_base_centi, 40000 - x - 3 * y);
            }
            assert!(reader.cell(GlobalCell { x: 640, y: 0 }).is_err());
            assert!(reader.cell(GlobalCell { x: 0, y: 640 }).is_err());
        }
        let after: Vec<_> = (0..d.count())
            .map(|i| std::fs::read(tile_path(&temp.0, i)).unwrap())
            .chain(std::iter::once(
                std::fs::read(temp.0.join("prepared-index.bin")).unwrap(),
            ))
            .collect();
        assert_eq!(bytes, after);
    }
}
#[test]
fn index_completion_and_exact_tile_identity_are_required() {
    let d = domain();
    let tmp = Temp::new();
    let l = limits(&tmp.0, d, 1);
    assert!(matches!(
        PreparedReader::open(&tmp.0, d, l),
        Err(PreparedError::Io { .. })
    ));
    let mut w = PreparedWriter::new(&tmp.0, d, l).unwrap();
    let mut bad = terrain(d, 0);
    bad.valid.boundary.east = true;
    assert!(matches!(w.write(&bad), Err(PreparedError::Invalid(_))));
    assert_eq!(std::fs::read_dir(&tmp.0).unwrap().count(), 0);
    w.write(&terrain(d, 0)).unwrap();
    let before = std::fs::read(tile_path(&tmp.0, 0)).unwrap();
    assert!(matches!(
        w.write(&terrain(d, 0)),
        Err(PreparedError::Invalid(_))
    ));
    assert_eq!(before, std::fs::read(tile_path(&tmp.0, 0)).unwrap());
    assert!(matches!(w.finish(), Err(PreparedError::Invalid(_))));
    assert!(!tmp.0.join("prepared-index.bin").exists());
}
#[test]
fn index_and_tile_corruption_and_missing_sources_fail_explicitly() {
    let d = domain();
    let tmp = Temp::new();
    drop(write_all(&tmp.0, d, &[0, 1, 2, 3], 1));
    let index = tmp.0.join("prepared-index.bin");
    let bytes = std::fs::read(&index).unwrap();
    for byte in [0, 8, 16, 24, 31] {
        let mut broken = bytes.clone();
        broken[byte] ^= 1;
        std::fs::write(&index, broken).unwrap();
        assert!(PreparedReader::open(&tmp.0, d, limits(&tmp.0, d, 1)).is_err());
    }
    std::fs::write(&index, &bytes).unwrap();
    let path = tile_path(&tmp.0, 0);
    let original = std::fs::read(&path).unwrap();
    let mut broken = original.clone();
    broken[32] ^= 1;
    std::fs::write(&path, &broken).unwrap();
    let mut reader = PreparedReader::open(&tmp.0, d, limits(&tmp.0, d, 1)).unwrap();
    assert!(matches!(
        reader.tile(AreaCoord::new(0, 0)),
        Err(PreparedError::Invalid("tile checksum"))
    ));
    broken.pop();
    std::fs::write(&path, &broken).unwrap();
    assert!(matches!(
        reader.tile(AreaCoord::new(0, 0)),
        Err(PreparedError::Invalid("file length"))
    ));
    std::fs::remove_file(&path).unwrap();
    assert!(matches!(
        reader.tile(AreaCoord::new(0, 0)),
        Err(PreparedError::Io { .. })
    ));
}
#[test]
fn admission_preserves_namespace_and_failed_io_never_creates_completion_index() {
    let d = domain();
    let tmp = Temp::new();
    let base = limits(&tmp.0, d, 1);
    for ram in [true, false] {
        let mut l = base;
        if ram {
            l.ram_bytes -= 1;
        } else {
            l.scratch_bytes -= 1;
        }
        assert!(matches!(
            PreparedWriter::new(&tmp.0, d, l),
            Err(PreparedError::Limit(_))
        ));
        assert_eq!(std::fs::read_dir(&tmp.0).unwrap().count(), 0);
    }
    let path = tile_path(&tmp.0, 0);
    std::fs::write(&path, b"preserve").unwrap();
    let mut w = PreparedWriter::new(&tmp.0, d, base).unwrap();
    assert!(matches!(
        w.write(&terrain(d, 0)),
        Err(PreparedError::Io { .. })
    ));
    assert_eq!(std::fs::read(&path).unwrap(), b"preserve");
    let other = Temp::new();
    let mut low = limits(&other.0, d, 1);
    low.io_bytes = 0;
    let mut w = PreparedWriter::new(&other.0, d, low).unwrap();
    assert!(matches!(
        w.write(&terrain(d, 0)),
        Err(PreparedError::Limit(_))
    ));
    assert!(!other.0.join("prepared-index.bin").exists());
}
#[test]
fn cache_hits_charge_queries_without_repeated_file_reads_and_maximum_is_bounded() {
    let d = domain();
    let tmp = Temp::new();
    drop(write_all(&tmp.0, d, &[0, 1, 2, 3], 2));
    let mut l = limits(&tmp.0, d, 2);
    l.tile_queries = 3;
    let mut reader = PreparedReader::open(&tmp.0, d, l).unwrap();
    reader.cell(GlobalCell { x: 0, y: 0 }).unwrap();
    let first = reader.work();
    reader.cell(GlobalCell { x: 1, y: 1 }).unwrap();
    reader.cell(GlobalCell { x: 511, y: 0 }).unwrap();
    assert_eq!(reader.work().io_bytes, first.io_bytes);
    assert_eq!(reader.work().io_operations, first.io_operations);
    assert_eq!(reader.work().tile_queries, 3);
    assert!(matches!(
        reader.cell(GlobalCell { x: 2, y: 0 }),
        Err(PreparedError::Limit("tile queries"))
    ));
    let max = PreparedDomain::for_config(
        GenerateConfig::new(SizeKm::new(4000, 4000), LatitudeBand::new(-80, 80), 200).unwrap(),
    )
    .unwrap();
    assert_eq!(scratch_required(max), 16_000_249_672);
    assert!(
        ram_required(Path::new("/private/prepared"), max, max.columns()).unwrap()
            < 256 * 1024 * 1024
    );
}

#[test]
fn actual_prepared_files_feed_routing_once_per_tile_and_preserve_source_errors() {
    use super::super::routing::{CellIndex, Extent, RoutingStore};
    use super::super::routing_disk::{CreateError, DiskLimits, DiskRoutingStore};
    let d = domain();
    let extent = Extent::new(d.width(), d.height()).unwrap();
    let source = Temp::new();
    let mut l = limits(&source.0, d, d.columns());
    l.tile_queries = u64::from(d.width()) * u64::from(d.height()) + 100;
    let mut writer = PreparedWriter::new(&source.0, d, l).unwrap();
    for i in [3, 0, 2, 1] {
        writer.write(&terrain(d, i)).unwrap();
    }
    let mut reader = writer.finish().unwrap();
    let before = reader.work();
    let out = Temp::new();
    let disk_limits = DiskLimits {
        cache_bytes: 16384,
        scratch_bytes: DiskRoutingStore::scratch_required(extent),
        io_bytes: 1 << 30,
        io_operations: 1 << 24,
    };
    let mut disk =
        DiskRoutingStore::try_create(&out.0, extent, reader.routing_records(), disk_limits)
            .unwrap();
    let expected_bytes: u128 = d
        .entries()
        .map(|e| encoded_len(e.valid).unwrap() as u128)
        .sum();
    assert_eq!(reader.work().io_bytes - before.io_bytes, expected_bytes);
    for (x, y) in [(0, 0), (511, 511), (512, 511), (511, 512), (639, 639)] {
        let at = CellIndex::new(y * d.width() + x, extent).unwrap();
        let r = disk.read(at).unwrap();
        assert_eq!(
            r.height(),
            i32::try_from(x).unwrap() - 2 * i32::try_from(y).unwrap()
        );
        assert!(!r.is_marine());
        assert_eq!(r.owner(), None);
    }
    drop(disk);
    drop(reader);
    std::fs::remove_file(tile_path(&source.0, 2)).unwrap();
    let mut broken = PreparedReader::open(&source.0, d, l).unwrap();
    let out2 = Temp::new();
    let error =
        DiskRoutingStore::try_create(&out2.0, extent, broken.routing_records(), disk_limits)
            .err()
            .unwrap();
    assert!(matches!(
        error,
        CreateError::Input(PreparedError::Io { .. })
    ));
}
