use super::pass::Pass;
use super::*;
fn store(w: u32, h: u32, z: &[i32]) -> MemoryPages {
    let e = Extent::new(w, h).unwrap();
    MemoryPages::new(e, z, &vec![false; z.len()], 1 << 28).unwrap()
}
fn run(s: &mut MemoryPages) -> Work {
    route_and_own(s, Limits::for_extent(s.extent())).unwrap()
}
#[test]
fn flat_reaches_lower_outlet_at_higher_identity() {
    let mut s = store(
        9,
        3,
        &[
            20, 20, 20, 20, 20, 20, 20, 20, 20, 20, 10, 10, 10, 10, 10, 10, 9, 20, 20, 20, 20, 20,
            20, 20, 20, 20, 20,
        ],
    );
    run(&mut s);
    assert_eq!(s.read(CellIndex(10)).unwrap().owner(), Some(CellIndex(16)));
    let r = s.read(CellIndex(10)).unwrap();
    assert!(matches!(r.receiver(s.extent(),CellIndex(10)),Some(Receiver::Cell(next)) if next.0>10));
}
#[test]
fn multiple_outlets_and_neighbor_order_have_identical_ownership() {
    let z = vec![
        20, 20, 20, 20, 20, 20, 20, 20, 9, 10, 10, 10, 8, 20, 20, 20, 20, 20, 20, 20, 20,
    ];
    let mut a = store(7, 3, &z);
    let mut b = store(7, 3, &z);
    run(&mut a);
    let extent = b.extent();
    let mut p = Pass {
        store: &mut b,
        extent,
        limits: Limits::for_extent(extent),
        work: Work::default(),
    };
    p.receivers([7, 6, 5, 4, 3, 2, 1, 0]).unwrap();
    p.ownership().unwrap();
    for i in 0..21 {
        assert_eq!(a.read(CellIndex(i)).unwrap(), b.read(CellIndex(i)).unwrap());
    }
    assert_eq!(a.read(CellIndex(10)).unwrap().owner(), Some(CellIndex(8)));
    assert_eq!(a.read(CellIndex(11)).unwrap().owner(), Some(CellIndex(12)));
}
#[test]
fn course_crosses_pages_and_area_width_without_cycle() {
    let w = 600;
    let mut z = vec![20; w * 3];
    for x in 1..w - 1 {
        z[w + x] = 10;
    }
    z[w + w - 2] = 9;
    let mut s = store(u32::try_from(w).unwrap(), 3, &z);
    let work = run(&mut s);
    assert!(work.cell_reads < 64 * s.extent().cells() as u64);
    let mut at = CellIndex(u32::try_from(w).unwrap() + 1);
    for _ in 0..w {
        let r = s.read(at).unwrap();
        assert_eq!(
            r.owner(),
            Some(CellIndex(u32::try_from(2 * w - 2).unwrap()))
        );
        match r.receiver(s.extent(), at).unwrap() {
            Receiver::Cell(next) => at = next,
            Receiver::Stop(OutletKind::ClosedDepression) => break,
            _ => panic!("unexpected terminal"),
        }
    }
    assert_eq!(at, CellIndex(u32::try_from(2 * w - 2).unwrap()));
    for page in 0..s.pages.len() {
        let bytes = s.encode_page(page).unwrap();
        s.replace_page(page, bytes).unwrap();
    }
}
#[test]
fn marine_and_actual_outer_exit_remain_distinct() {
    let e = Extent::new(3, 3).unwrap();
    let mut z = vec![20; 9];
    z[3] = -2;
    z[4] = 1;
    let mut marine = vec![false; 9];
    marine[3] = true;
    let mut s = MemoryPages::new(e, &z, &marine, 1 << 20).unwrap();
    run(&mut s);
    assert_eq!(s.read(CellIndex(4)).unwrap().owner(), Some(CellIndex(3)));
    assert_eq!(
        s.read(CellIndex(3)).unwrap().receiver(e, CellIndex(3)),
        Some(Receiver::Stop(OutletKind::MarineEntry))
    );
    let mut flat = store(3, 3, &[10; 9]);
    run(&mut flat);
    assert_eq!(flat.read(CellIndex(4)).unwrap().owner(), Some(CellIndex(0)));
    assert_eq!(
        flat.read(CellIndex(0)).unwrap().receiver(e, CellIndex(0)),
        Some(Receiver::Stop(OutletKind::DomainExport))
    );
}
#[test]
fn corrupt_cycle_and_resource_exhaustion_are_errors() {
    let mut s = store(3, 3, &[10; 9]);
    run(&mut s);
    let mut a = s.read(CellIndex(4)).unwrap();
    a.distance = 0;
    a.receiver = 4;
    s.write(CellIndex(4), a).unwrap();
    let mut b = s.read(CellIndex(5)).unwrap();
    b.distance = 0;
    b.receiver = 3;
    s.write(CellIndex(5), b).unwrap();
    assert!(matches!(
        resolve_ownership(&mut s, Limits::for_extent(Extent::new(3, 3).unwrap())),
        Err(RoutingError::InvalidTopology)
    ));
    let mut s = store(3, 3, &[10; 9]);
    let mut limits = Limits::for_extent(s.extent());
    limits.cell_reads = 1;
    assert!(matches!(
        route_and_own(&mut s, limits),
        Err(RoutingError::WorkLimit)
    ));
    assert!(MemoryPages::new(s.extent(), &[10; 9], &[false; 9], 1).is_none());
}
#[test]
fn closed_plateau_has_one_canonical_owner_without_physical_epsilon() {
    let mut z = vec![20; 25];
    for y in 1..4 {
        for x in 1..4 {
            z[y * 5 + x] = 10;
        }
    }
    let mut s = store(5, 5, &z);
    run(&mut s);
    for (i, &height) in z.iter().enumerate() {
        let r = s.read(CellIndex(u32::try_from(i).unwrap())).unwrap();
        assert_eq!(r.height(), height);
        assert_eq!(r.owner(), Some(CellIndex(6)));
    }
    assert_eq!(
        s.read(CellIndex(6))
            .unwrap()
            .receiver(s.extent(), CellIndex(6)),
        Some(Receiver::Stop(OutletKind::ClosedDepression))
    );
}
#[test]
fn varied_physical_heights_and_flats_obey_linear_limits_and_order() {
    let mut random = 436342u64;
    for _ in 0..32 {
        let mut z = Vec::new();
        for _ in 0..23 * 19 {
            random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
            z.push(((random >> 32) % 7) as i32 - 3);
        }
        let mut a = store(23, 19, &z);
        let mut b = store(23, 19, &z);
        run(&mut a);
        let extent = b.extent();
        let mut p = Pass {
            store: &mut b,
            extent,
            limits: Limits::for_extent(extent),
            work: Work::default(),
        };
        p.receivers([5, 2, 7, 0, 3, 6, 1, 4]).unwrap();
        p.ownership().unwrap();
        for i in 0..extent.cells() {
            let r = a.read(CellIndex(i)).unwrap();
            assert_eq!(r, b.read(CellIndex(i)).unwrap());
            assert!(r.owner().is_some());
        }
        resolve_ownership(&mut a, Limits::for_extent(extent)).unwrap();
    }
}
#[test]
fn checked_backend_rejects_cross_domain_and_immutable_writes() {
    let mut s = store(3, 3, &[10; 9]);
    let invalid = CellIndex::new(9, Extent::new(4, 4).unwrap()).unwrap();
    assert_eq!(s.read(invalid), Err(MemoryError::InvalidIndex));
    assert_eq!(
        s.get(Tape::Component, 0),
        Err(MemoryError::InvalidTapePosition)
    );
    assert_eq!(
        s.write(CellIndex(0), CellRecord::prepared(9, false)),
        Err(MemoryError::ImmutableTerrain)
    );
    let mut page = s.encode_page(0).unwrap();
    page[..4].copy_from_slice(&9i32.to_le_bytes());
    assert!(s.replace_page(0, page).is_none());
}
#[test]
fn page_decoder_rejects_ghost_export_and_unreserved_bits() {
    let e = Extent::new(3, 3).unwrap();
    let mut r = CellRecord::prepared(10, false);
    r.receiver = EXPORT;
    assert!(CellRecord::decode(r.encode(), e, CellIndex(4)).is_none());
    let mut bytes = CellRecord::prepared(10, false).encode();
    bytes[15] = 1;
    assert!(CellRecord::decode(bytes, e, CellIndex(0)).is_none());
}
