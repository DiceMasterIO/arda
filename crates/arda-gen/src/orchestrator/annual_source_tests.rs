//! Register as a child of continent so the unchanged private grid can be constructed.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::{climate::ContinentClimate, ContinentGrid};
use crate::hydrology::{
    annual::LeafNet,
    annual_aggregation::{cell_budget, Terminal},
    forcing_sample::ForcingWindow,
    prepared_domain::PreparedDomain,
    routing::{CellIndex, CellRecord, Extent, RoutingStore, Tape},
    types::PreparedTerrain,
};
use crate::orchestrator::{
    annual_source::*,
    prepared_files::{self, PreparedReader, PreparedWriter},
};
use arda_core::hydrology::HydrologyDomain;
use arda_core::{
    AreaCoord, BasinId, ClimateRegime, GenerateConfig, GlobalCell, HeightMm, LatitudeBand, Litres,
    RainfallMm, SizeKm,
};
use std::{collections::BTreeMap, path::PathBuf};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let p = std::env::temp_dir().join(format!(
            "arda-annual-source-{}-{}",
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
fn config() -> GenerateConfig {
    GenerateConfig::new(SizeKm::new(64, 64), LatitudeBand::new(35, 55), 15).unwrap()
}
fn domain() -> PreparedDomain {
    PreparedDomain::for_config(config()).unwrap()
}
fn limits() -> SourceLimits {
    SourceLimits {
        ram_bytes: required_ram(domain()).unwrap(),
        operations: required_operations(domain()).unwrap(),
    }
}
fn climate() -> (ContinentGrid, ContinentClimate) {
    (
        ContinentGrid {
            width: 64,
            height: 64,
            height_mm: vec![0; 4096],
        },
        ContinentClimate {
            temperature: vec![1000; 4096],
            rainfall: vec![100; 4096],
            regime: vec![ClimateRegime::Temperate; 4096],
            moisture: vec![0; 4096],
            ocean: vec![true; 4096],
            ocean_distance_km: vec![0; 4096],
        },
    )
}
fn land(x: u32, y: u32) -> Option<(i32, u32, u8)> {
    if y == 0 && x <= 2 {
        return Some((
            [-1_000_000, -999_999, 0][usize::try_from(x).unwrap()],
            0,
            if x == 0 { 8 } else { 3 },
        ));
    }
    if [0, 1, 512].contains(&y) && (x == 511 || x == 512) {
        return Some((
            10,
            y * 640 + if x == 511 { 510 } else { 513 },
            if x == 511 { 3 } else { 4 },
        ));
    }
    if x == 639 && y == 0 {
        return Some((10, 639, 10));
    }
    None
}
struct Routing {
    extent: Extent,
    reads: u64,
    fail: bool,
    overrides: BTreeMap<u32, CellRecord>,
}
impl Routing {
    fn new() -> Self {
        Self {
            extent: Extent::new(640, 640).unwrap(),
            reads: 0,
            fail: false,
            overrides: BTreeMap::new(),
        }
    }
    fn record(&self, at: CellIndex) -> CellRecord {
        let (x, y) = self.extent.coordinates(at);
        let (h, o, d, m) = land(x, y).map_or((0, at.raw(), 9, true), |(h, o, d)| (h, o, d, false));
        let mut b = CellRecord::prepared(h, m).encode();
        b[4..8].copy_from_slice(&0u32.to_le_bytes());
        b[8..12].copy_from_slice(&o.to_le_bytes());
        b[12] = d;
        CellRecord::decode(b, self.extent, at).unwrap()
    }
}
impl RoutingStore for Routing {
    type Error = u8;
    fn extent(&self) -> Extent {
        self.extent
    }
    fn read(&mut self, at: CellIndex) -> Result<CellRecord, u8> {
        self.reads += 1;
        if self.fail {
            return Err(73);
        }
        Ok(self
            .overrides
            .get(&at.raw())
            .copied()
            .unwrap_or_else(|| self.record(at)))
    }
    fn write(&mut self, _: CellIndex, _: CellRecord) -> Result<(), u8> {
        Err(99)
    }
    fn mark_marine(&mut self, _: CellIndex) -> Result<(), u8> {
        Err(99)
    }
    fn clear(&mut self, _: Tape) -> Result<(), u8> {
        Err(99)
    }
    fn len(&self, _: Tape) -> u32 {
        0
    }
    fn push(&mut self, _: Tape, _: CellIndex) -> Result<(), u8> {
        Err(99)
    }
    fn get(&mut self, _: Tape, _: u32) -> Result<CellIndex, u8> {
        Err(99)
    }
}
fn reader(t: &Temp) -> PreparedReader {
    let d = domain();
    let l = prepared_files::Limits {
        ram_bytes: prepared_files::ram_required(&t.0, d, 2).unwrap(),
        scratch_bytes: prepared_files::scratch_required(d),
        cache_tiles: 2,
        io_bytes: 1 << 32,
        io_operations: 10000,
        tile_queries: 1_000_000,
    };
    let mut w = PreparedWriter::new(&t.0, d, l).unwrap();
    for i in [3, 1, 2, 0] {
        let e = d.entry(i).unwrap();
        let heights = (0..512u32)
            .flat_map(|y| {
                (0..512u32).map(move |x| {
                    let (ax, ay) = (
                        u32::try_from(e.area.x).unwrap() * 512 + x,
                        u32::try_from(e.area.y).unwrap() * 512 + y,
                    );
                    HeightMm::new(land(ax, ay).map_or(0, |v| v.0))
                })
            })
            .collect();
        w.write(&PreparedTerrain {
            area: e.area,
            valid: e.valid,
            heights,
            annual_rain: vec![RainfallMm::new(777); 262144],
            temperature_base_centi: vec![1000; 262144],
        })
        .unwrap();
    }
    w.finish().unwrap()
}
fn leaf() -> LeafNet {
    LeafNet {
        leaf: BasinId(0),
        account: BasinId(0),
        lake: Some(BasinId(0)),
        surface: Some(HeightMm::new(-999_999)),
        runoff: Litres(100),
        paid_wet_cost: Litres(2),
        marginal_cost: Litres(73),
        net_litres: 25,
    }
}
#[test]
fn actual_prepared_rain_physical_temperature_and_two_area_cache_boundaries() {
    let t = Temp::new();
    let mut p = reader(&t);
    let (g, c) = climate();
    let mut r = Routing::new();
    let e = r.extent;
    let fd = HydrologyDomain {
        width_cells: 640,
        height_cells: 640,
        exported_areas_wide: 1,
        exported_areas_high: 1,
    };
    let window =
        ForcingWindow::for_area(&g, &c, config().latitude_band(), fd, AreaCoord::new(0, 0))
            .unwrap();
    let expected = window
        .sample_with_annual_temperature(
            GlobalCell { x: 0, y: 0 },
            crate::area::temperature::temperature_from_base(1000, HeightMm::new(-1_000_000), false),
        )
        .unwrap();
    let uncorrected = window
        .sample_with_annual_temperature(GlobalCell { x: 0, y: 0 }, arda_core::TempCentiC::new(1000))
        .unwrap();
    assert_ne!(
        expected.periods.map(|m| m.evaporation_um),
        uncorrected.periods.map(|m| m.evaporation_um)
    );
    let mut source = AnnualSource::new(&mut p, &g, &c, config(), limits()).unwrap();
    let mut n = 0;
    for raw in 0..e.cells() {
        let cell = source
            .cell(&mut r, CellIndex::new(raw, e).unwrap())
            .unwrap();
        if let Some((_, _, _)) = land(cell.at.x, cell.at.y) {
            n += 1;
            assert_eq!(cell.rain.raw(), 777);
            if raw == 0 {
                assert_eq!(
                    cell.evaporation_um,
                    expected.periods.map(|m| m.evaporation_um)
                );
                assert_eq!(cell.terminal, Terminal::Basin(BasinId(0)));
            }
            if raw == 511 {
                assert_eq!(cell.terminal, Terminal::Sea);
            }
            if raw == 639 {
                assert_eq!(cell.terminal, Terminal::DomainExport);
            }
        } else {
            assert_eq!(cell.terminal, Terminal::Marine);
            assert_eq!(
                cell_budget(cell.rain, &cell.evaporation_um).precipitation,
                Litres(0)
            );
        }
    }
    let work = source.finish().unwrap();
    assert_eq!(work.samples, n);
    assert_eq!(work.windows, 4);
    assert_eq!(work.cells, 409600);
    assert!(work.operations <= limits().operations);
}
#[test]
fn fine_source_debits_marginal_once_even_when_immutable_terminal_is_wet() {
    let t = Temp::new();
    let mut p = reader(&t);
    let (g, c) = climate();
    let mut r = Routing::new();
    let e = r.extent;
    let leaves = [leaf()];
    let mut source = AnnualSource::new(&mut p, &g, &c, config(), limits()).unwrap();
    let mut debit = 0;
    for raw in 0..e.cells() {
        let cell = source
            .fine_cell(&mut r, CellIndex::new(raw, e).unwrap(), &leaves)
            .unwrap();
        debit += cell.marginal.0;
        if raw == 0 {
            assert_eq!(cell.wet, Some((BasinId(0), -999_999)));
            assert_eq!(cell.marginal, Litres(73));
            assert_eq!(cell.precipitation, Litres(7_770_000));
            assert_eq!(
                cell.land_loss.0,
                (cell.precipitation.0 / 2).min(cell.evaporation.0)
            );
        }
        if raw == 1 {
            assert!(cell.wet.is_none());
            assert_eq!(cell.marginal, Litres(0));
        }
    }
    assert_eq!(debit, 73);
    assert_eq!(source.finish().unwrap().windows, 4);
}
#[test]
fn marine_has_no_forcing_or_prepared_query_and_errors_do_not_become_end_of_stream() {
    let t = Temp::new();
    let mut p = reader(&t);
    let before = p.work();
    let (g, mut c) = climate();
    c.rainfall.clear();
    let mut r = Routing::new();
    let e = r.extent;
    let at = CellIndex::new(0, e).unwrap();
    let mut b = CellRecord::prepared(-7, true).encode();
    b[8..12].copy_from_slice(&0u32.to_le_bytes());
    b[12] = 9;
    r.overrides.insert(0, CellRecord::decode(b, e, at).unwrap());
    {
        let mut s = AnnualSource::new(&mut p, &g, &c, config(), limits()).unwrap();
        let cell = s.cell(&mut r, at).unwrap();
        assert_eq!(cell.terminal, Terminal::Marine);
        assert_eq!(s.work().windows, 0);
        assert!(s.finish().is_err());
    }
    assert_eq!(p.work().io_bytes, before.io_bytes);
    r.overrides.clear();
    r.fail = true;
    let mut s = AnnualSource::new(&mut p, &g, &c, config(), limits()).unwrap();
    assert!(matches!(
        s.cell(&mut r, at),
        Err(CellSourceError::Routing(73))
    ));
    r.fail = false;
    assert!(matches!(
        s.cell(&mut r, at),
        Err(CellSourceError::Source(SourceError::Invalid(_)))
    ));
}
#[test]
fn admission_order_mode_and_leaf_authority_are_checked() {
    let t = Temp::new();
    let mut p = reader(&t);
    let (g, c) = climate();
    let mut r = Routing::new();
    let e = r.extent;
    let at = CellIndex::new(0, e).unwrap();
    for l in [
        SourceLimits {
            ram_bytes: limits().ram_bytes - 1,
            ..limits()
        },
        SourceLimits {
            operations: limits().operations - 1,
            ..limits()
        },
    ] {
        assert!(matches!(
            AnnualSource::new(&mut p, &g, &c, config(), l),
            Err(SourceError::Limit(_))
        ));
    }
    assert_eq!(r.reads, 0);
    {
        let mut s = AnnualSource::new(&mut p, &g, &c, config(), limits()).unwrap();
        assert!(s.cell(&mut r, CellIndex::new(1, e).unwrap()).is_err());
        assert_eq!(r.reads, 0);
    }
    for leaves in [
        vec![],
        vec![leaf(), leaf()],
        vec![LeafNet {
            surface: None,
            ..leaf()
        }],
    ] {
        let mut s = AnnualSource::new(&mut p, &g, &c, config(), limits()).unwrap();
        assert!(s.fine_cell(&mut r, at, &leaves).is_err());
        assert!(s.finish().is_err());
    }
    {
        let mut s = AnnualSource::new(&mut p, &g, &c, config(), limits()).unwrap();
        s.cell(&mut r, at).unwrap();
        assert!(s
            .fine_cell(&mut r, CellIndex::new(1, e).unwrap(), &[leaf()])
            .is_err());
    }
}
#[test]
fn source_authority_and_forcing_failures_are_typed() {
    let t = Temp::new();
    let mut p = reader(&t);
    let (g, mut c) = climate();
    let mut r = Routing::new();
    let e = r.extent;
    let at = CellIndex::new(0, e).unwrap();
    r.overrides.insert(0, CellRecord::prepared(0, false));
    {
        let mut s = AnnualSource::new(&mut p, &g, &c, config(), limits()).unwrap();
        assert!(matches!(
            s.cell(&mut r, at),
            Err(CellSourceError::Source(SourceError::Invalid(
                "unresolved terminal owner"
            )))
        ));
    }
    let mut b = r.record(at).encode();
    b[0..4].copy_from_slice(&(-1i32).to_le_bytes());
    r.overrides.insert(0, CellRecord::decode(b, e, at).unwrap());
    {
        let mut s = AnnualSource::new(&mut p, &g, &c, config(), limits()).unwrap();
        assert!(matches!(
            s.cell(&mut r, at),
            Err(CellSourceError::Source(SourceError::Invalid(
                "prepared/routing physical height"
            )))
        ));
    }
    r.overrides.clear();
    c.rainfall.clear();
    let mut s = AnnualSource::new(&mut p, &g, &c, config(), limits()).unwrap();
    assert!(matches!(
        s.cell(&mut r, at),
        Err(CellSourceError::Source(SourceError::Forcing(_)))
    ));
}
