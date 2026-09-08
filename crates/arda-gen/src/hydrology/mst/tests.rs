use super::{
    hierarchy::{self, Minimum},
    io::{self, StageError},
    producer::{self, MstLimits},
    routing::{self, CellIndex, Extent, MemoryPages, OutletKind, Receiver, RoutingStore},
    saddles::{self, Node, Saddle},
    test_support::{self as hierarchy_tests, Directory},
};
use std::fs;
pub(crate) fn limits(cache_pages: u64) -> MstLimits {
    MstLimits {
        ram_bytes: 1 << 24,
        scratch_bytes: 1 << 28,
        io_bytes: 1 << 34,
        io_operations: 10_000_000,
        cache_pages,
        sort_buffer_records: 2,
        slot_reads: 100_000_000,
        slot_writes: 100_000_000,
        comparisons: 100_000_000,
        routing_reads: 100_000_000,
        scan_candidates: 100_000_000,
    }
}
fn routing(e: Extent, z: &[i32], sea: &[bool]) -> MemoryPages {
    let mut r = MemoryPages::new(e, z, sea, u64::MAX).unwrap();
    routing::route_and_own(&mut r, routing::Limits::for_extent(e)).unwrap();
    r
}
fn oracle_key(e: Extent, s: Saddle) -> (i32, u64, u64, u64, u64) {
    let packed = |at| {
        let (x, y) = e.coordinates(at);
        (u64::from(y) << 32) | u64::from(x)
    };
    let node = |n| match n {
        Node::Closed(at) => packed(at),
        Node::Exterior => u64::MAX,
    };
    let a = packed(s.from);
    let b = s.to.map_or(u64::MAX, packed);
    (s.sill_mm, node(s.left), node(s.right), a.min(b), a.max(b))
}
fn oracle(r: &mut MemoryPages) -> (Vec<Minimum>, Vec<Saddle>) {
    let e = r.extent();
    let minima: Vec<_> = (0..e.cells())
        .filter_map(|raw| {
            let at = CellIndex::new(raw, e).unwrap();
            let row = r.read(at).unwrap();
            (row.receiver(e, at) == Some(Receiver::Stop(OutletKind::ClosedDepression))).then_some(
                Minimum {
                    at,
                    floor_mm: row.height(),
                },
            )
        })
        .collect();
    let mut edges = Vec::new();
    saddles::scan(r, saddles::Limits::for_extent(e), |edge| {
        edges.push(edge);
        Ok::<_, ()>(())
    })
    .unwrap();
    edges.sort_unstable_by_key(|&s| oracle_key(e, s));
    let index = |n| match n {
        Node::Exterior => minima.len(),
        Node::Closed(at) => minima.iter().position(|m| m.at == at).unwrap(),
    };
    let mut labels: Vec<_> = (0..=minima.len()).collect();
    let mut accepted = Vec::new();
    for edge in edges {
        let a = labels[index(edge.left)];
        let b = labels[index(edge.right)];
        if a != b {
            accepted.push(edge);
            for label in &mut labels {
                if *label == b {
                    *label = a;
                }
            }
        }
    }
    assert_eq!(accepted.len(), minima.len());
    (minima, accepted)
}
// Independent raw-cell minimax Dijkstra, with actual rim heights and marine exits.
fn raw_escape(e: Extent, z: &[i32], sea: &[bool]) -> Vec<i32> {
    let (xmax, ymax) = e.coordinates(CellIndex::new(e.cells() - 1, e).unwrap());
    let width = xmax + 1;
    let n = z.len();
    let mut out = vec![i32::MAX; n];
    let mut done = vec![false; n];
    for i in 0..n {
        let raw = u32::try_from(i).unwrap();
        let x = raw % width;
        let y = raw / width;
        if x == 0 || y == 0 || x == xmax || y == ymax || sea[i] {
            out[i] = z[i];
        }
    }
    for _ in 0..n {
        let at = (0..n)
            .filter(|&i| !done[i])
            .min_by_key(|&i| out[i])
            .unwrap();
        done[at] = true;
        let raw = u32::try_from(at).unwrap();
        let x = i64::from(raw % width);
        let y = i64::from(raw / width);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (a, b) = (x + dx, y + dy);
                if (dx != 0 || dy != 0)
                    && a >= 0
                    && b >= 0
                    && a <= i64::from(xmax)
                    && b <= i64::from(ymax)
                {
                    let j = usize::try_from(b * i64::from(width) + a).unwrap();
                    out[j] = out[j].min(out[at].max(z[j]));
                }
            }
        }
    }
    out
}
fn tree_escape(ms: &[Minimum], edges: &[Saddle]) -> Vec<i32> {
    let index = |n| match n {
        Node::Exterior => ms.len(),
        Node::Closed(at) => ms.iter().position(|m| m.at == at).unwrap(),
    };
    let mut result = vec![None; ms.len() + 1];
    result[ms.len()] = Some(i32::MIN);
    for _ in 0..ms.len() {
        for edge in edges {
            let a = index(edge.left);
            let b = index(edge.right);
            match (result[a], result[b]) {
                (Some(v), None) => result[b] = Some(v.max(edge.sill_mm)),
                (None, Some(v)) => result[a] = Some(v.max(edge.sill_mm)),
                _ => {}
            }
        }
    }
    result[..ms.len()].iter().map(|v| v.unwrap()).collect()
}
fn check(e: Extent, z: &[i32], sea: &[bool], cache: u64) -> producer::Work {
    let directory = Directory::new();
    let mut r = routing(e, z, sea);
    let (ms, es) = oracle(&mut r);
    let escape = raw_escape(e, z, sea);
    for (m, h) in ms.iter().zip(tree_escape(&ms, &es)) {
        assert_eq!(h, escape[usize::try_from(m.at.raw()).unwrap()]);
    }
    let mut product = producer::produce(&mut r, &directory.0, limits(cache)).unwrap();
    assert_eq!(product.work.leaves, u32::try_from(ms.len()).unwrap());
    assert_eq!(product.work.accepted, product.work.leaves);
    let produced_minima: Vec<_> = product.minima.by_ref().collect::<Result<_, _>>().unwrap();
    let produced_edges: Vec<_> = product.edges.by_ref().collect::<Result<_, _>>().unwrap();
    assert_eq!(produced_minima, ms);
    assert_eq!(
        produced_edges
            .iter()
            .map(|s| s.encode())
            .collect::<Vec<_>>(),
        es.iter().map(|s| s.encode()).collect::<Vec<_>>()
    );
    let (_, expected) = hierarchy_tests::run(e, &ms, &es);
    let (_, actual) = hierarchy_tests::run(e, &produced_minima, &produced_edges);
    assert_eq!(actual.output, expected.output);
    assert_eq!(actual.elders, expected.elders);
    assert!(product.work.comparisons > 0 || ms.is_empty());
    assert!(product.work.io.bytes <= limits(cache).io_bytes);
    product.work
}
#[test]
fn actual_saddle_replays_match_independent_kruskal_and_raw_minimax() {
    let mut most_rounds = 0;
    for seed in 0..32_u32 {
        let w = 5 + seed % 6;
        let e = Extent::new(w, 7).unwrap();
        let mut x = u64::from(seed) + 1;
        let z: Vec<_> = (0..e.cells())
            .map(|_| {
                x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
                i32::try_from((x >> 32) % 11).unwrap() - 5
            })
            .collect();
        most_rounds = most_rounds.max(check(e, &z, &vec![false; z.len()], 1).rounds);
    }
    assert!(
        most_rounds >= 2,
        "controls must exercise a frozen second round"
    );
}
#[test]
fn tied_many_minima_have_cache_independent_full_bytes() {
    let e = Extent::new(19, 19).unwrap();
    let z: Vec<_> = (0..e.cells())
        .map(|raw| {
            let (x, y) = e.coordinates(CellIndex::new(raw, e).unwrap());
            if x % 2 == 1 && y % 2 == 1 {
                0
            } else {
                10
            }
        })
        .collect();
    for cache in [1, 2, 8] {
        check(e, &z, &vec![false; z.len()], cache);
    }
}
#[test]
fn negative_land_ocean_and_empty_closed_set_controls() {
    let e = Extent::new(5, 5).unwrap();
    let mut z = vec![3; 25];
    z[12] = -9;
    check(e, &z, &[false; 25], 1);
    check(e, &[-2; 25], &[true; 25], 1);
    check(Extent::new(1, 1).unwrap(), &[10], &[false], 1);
    let mut sea = [false; 25];
    sea[0] = true;
    z[0] = -2;
    check(e, &z, &sea, 1);
    let e = Extent::new(7, 5).unwrap();
    let mut z: Vec<_> = (0..e.cells())
        .map(|raw| {
            let (x, y) = e.coordinates(CellIndex::new(raw, e).unwrap());
            if x == 0 || x == 6 || y == 0 || y == 4 {
                10
            } else {
                -2
            }
        })
        .collect();
    z[15] = -9;
    z[19] = -8;
    check(e, &z, &[false; 35], 1);
}
#[test]
fn final_streams_feed_actual_hierarchy_without_loading_a_table() {
    let e = Extent::new(5, 5).unwrap();
    let mut z = vec![8; 25];
    z[6] = 0;
    z[18] = 1;
    let mut r = routing(e, &z, &[false; 25]);
    let (ms, es) = oracle(&mut r);
    let (_, expected) = hierarchy_tests::run(e, &ms, &es);
    let dir = Directory::new();
    let product = producer::produce(&mut r, &dir.0, limits(1)).unwrap();
    let mut actual = hierarchy_tests::Memory::default();
    hierarchy::build(
        e,
        u64::from(product.work.leaves),
        product.minima.map(|r| r.map_err(|_| "minimum read")),
        product.edges.map(|r| r.map_err(|_| "edge read")),
        &mut actual,
        hierarchy_tests::limits(),
        hierarchy_tests::resolver,
    )
    .unwrap();
    assert_eq!(actual.output, expected.output);
    assert_eq!(actual.elders, expected.elders);
}
#[test]
fn preflight_rejects_known_resources_before_any_files() {
    for resource in 0..8 {
        let e = Extent::new(3, 3).unwrap();
        let mut z = [9; 9];
        z[4] = 0;
        let mut r = routing(e, &z, &[false; 9]);
        let dir = Directory::new();
        let mut cap = limits(1);
        match resource {
            0 => cap.ram_bytes = 0,
            1 => cap.scratch_bytes = 0,
            2 => cap.io_bytes = 0,
            3 => cap.io_operations = 0,
            4 => cap.comparisons = 0,
            5 => cap.cache_pages = 0,
            6 => cap.routing_reads = 0,
            _ => cap.routing_reads = 17,
        }
        assert!(producer::produce(&mut r, &dir.0, cap).is_err());
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 0);
    }
}
#[test]
fn counted_work_limits_fail_with_typed_errors() {
    for resource in 0..4 {
        let e = Extent::new(3, 3).unwrap();
        let mut z = [9; 9];
        z[4] = 0;
        let mut r = routing(e, &z, &[false; 9]);
        let dir = Directory::new();
        let mut cap = limits(1);
        match resource {
            0 => cap.slot_reads = 0,
            1 => cap.slot_writes = 0,
            2 => cap.scan_candidates = 0,
            _ => cap.routing_reads = 18,
        }
        assert!(matches!(
            producer::produce(&mut r, &dir.0, cap),
            Err(producer::MstError::Stage(StageError::Limit(_)))
        ));
    }
}
#[test]
fn unresolved_input_and_corrupt_final_minimum_are_rejected() {
    let e = Extent::new(3, 3).unwrap();
    let mut z = [9; 9];
    z[4] = 0;
    let dir = Directory::new();
    let mut raw = MemoryPages::new(e, &z, &[false; 9], u64::MAX).unwrap();
    assert!(matches!(
        producer::produce(&mut raw, &dir.0, limits(1)),
        Err(producer::MstError::Stage(StageError::Invalid(_)))
    ));
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 0);
    let mut r = routing(e, &z, &[false; 9]);
    let mut product = producer::produce(&mut r, &dir.0, limits(1)).unwrap();
    let mut bytes = fs::read(product.minima.path()).unwrap();
    bytes[64] ^= 1;
    fs::write(product.minima.path(), bytes).unwrap();
    assert!(matches!(
        product.minima.next(),
        Some(Err(StageError::Invalid("minimum checksum")))
    ));
    assert!(product.minima.next().is_none());
}
#[test]
fn minimum_record_exact_signed_height_and_checked_extent() {
    let e = Extent::new(3, 3).unwrap();
    let m = Minimum {
        at: CellIndex::new(4, e).unwrap(),
        floor_mm: i32::MIN,
    };
    let bytes = io::minimum_bytes(m);
    assert_eq!(io::minimum(&bytes, e).unwrap(), m);
    assert!(io::minimum(&bytes, Extent::new(1, 1).unwrap()).is_err());
    assert!(io::minimum(&bytes[..15], e).is_err());
}
#[test]
fn small_paged_cost_and_exact_initial_admission() {
    for (w, cache) in [(3, 1), (19, 1), (19, 8)] {
        let e = Extent::new(w, w).unwrap();
        let z: Vec<_> = (0..e.cells())
            .map(|raw| {
                let (x, y) = e.coordinates(CellIndex::new(raw, e).unwrap());
                if x % 2 == 1 && y % 2 == 1 {
                    0
                } else {
                    10
                }
            })
            .collect();
        let mut r = routing(e, &z, &vec![false; z.len()]);
        let dir = Directory::new();
        let begun = std::time::Instant::now();
        let mut product = producer::produce(&mut r, &dir.0, limits(cache)).unwrap();
        let minima = product
            .minima
            .by_ref()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .len();
        let accepted = product
            .edges
            .by_ref()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(minima, accepted.len());
        eprintln!("paged-MST width={w} leaves={minima} cache_pages={cache} elapsed_us={} producer={:?} minimum_final_io={:?} sort_final={:?}",begun.elapsed().as_micros(),product.work,product.minima.io_work(),product.edges.work());
    }
    // No closed terminals: the producer's exact known minimum equals all its I/O.
    let e = Extent::new(1, 1).unwrap();
    let dir = Directory::new();
    let mut r = routing(e, &[0], &[true]);
    let sort = super::accepted_sort::SortLimits::required(&dir.0, 0, 2).unwrap();
    let mut cap = limits(1);
    cap.io_bytes = sort.io_bytes + 4352;
    cap.io_operations = sort.io_operations + 14;
    let product = producer::produce(&mut r, &dir.0, cap).unwrap();
    assert_eq!(
        product.work.io,
        io::IoWork {
            bytes: 4352,
            operations: 14
        }
    );
    for delta in [false, true] {
        let dir = Directory::new();
        let mut r = routing(e, &[0], &[true]);
        let mut cap = cap;
        if delta {
            cap.io_bytes -= 1;
        } else {
            cap.io_operations -= 1;
        }
        assert!(matches!(
            producer::produce(&mut r, &dir.0, cap),
            Err(producer::MstError::Stage(StageError::Limit(
                "known minimum I/O reservation"
            )))
        ));
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 0);
    }
}
