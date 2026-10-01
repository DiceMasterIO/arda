use super::*;
use crate::hydrology::hierarchy_tests::{between, m, outside, run};
fn fix_checksum(b: &mut [u8]) {
    let n = b.len();
    let s = checksum(&b[..n - 8]);
    b[n - 8..].copy_from_slice(&s.to_le_bytes());
}
#[test]
fn real_builder_rows_roundtrip_and_every_changed_byte_is_detected() {
    let e = Extent::new(8, 3).unwrap();
    let ms = [m(e, 1, -20), m(e, 3, -10), m(e, 5, 0)];
    let es = [
        between(e, ms[0], ms[1], 5),
        between(e, ms[1], ms[2], 5),
        outside(e, ms[2], 9),
    ];
    let (_, s) = run(e, &ms, &es);
    for (i, row) in s.unions.iter().enumerate() {
        if let Some(row) = row {
            let b = encode_union(*row, e, i as u64, 4, 5).unwrap();
            let got = decode_union(&b, e, i as u64, 4, 5).unwrap();
            assert_eq!(encode_union(got, e, i as u64, 4, 5).unwrap(), b);
            for j in 0..b.len() {
                let mut bad = b;
                bad[j] ^= 1;
                assert!(decode_union(&bad, e, i as u64, 4, 5).is_err());
            }
            for n in 0..b.len() {
                assert!(decode_union(&b[..n], e, i as u64, 4, 5).is_err());
            }
        }
    }
    for (i, row) in s.nodes.iter().enumerate() {
        if let Some(row) = row {
            let b = encode_node(*row, e, i as u64, 3, 5).unwrap();
            let got = decode_node(&b, e, i as u64, 3, 5).unwrap();
            assert_eq!(encode_node(got, e, i as u64, 3, 5).unwrap(), b);
            for j in 0..b.len() {
                let mut bad = b;
                bad[j] ^= 1;
                assert!(decode_node(&bad, e, i as u64, 3, 5).is_err());
            }
        }
    }
    for row in s.elders {
        let b = encode_elder(row, e).unwrap();
        assert_eq!(decode_elder(&b, e).unwrap(), row);
        for j in 0..b.len() {
            let mut bad = b;
            bad[j] ^= 1;
            assert!(decode_elder(&bad, e).is_err());
        }
    }
}
#[test]
fn canonical_tags_padding_ranges_and_absent_payloads_are_checked_even_with_matching_checksum() {
    let e = Extent::new(6, 3).unwrap();
    let ms = [m(e, 1, 0), m(e, 3, 1)];
    let (_, s) = run(e, &ms, &[between(e, ms[0], ms[1], 5), outside(e, ms[1], 9)]);
    let union = encode_union(s.unions[0].unwrap(), e, 0, 3, 3).unwrap();
    for (i, v) in [(0, 2), (1, 2), (18, 64), (86, 1)] {
        let mut b = union;
        b[i] = v;
        fix_checksum(&mut b);
        assert!(decode_union(&b, e, 0, 3, 3).is_err(), "index {i}");
    }
    let node = encode_node(s.nodes[0].unwrap(), e, 0, 2, 3).unwrap();
    for (i, v) in [(0, 2), (17, 2), (71, 1)] {
        let mut b = node;
        b[i] = v;
        fix_checksum(&mut b);
        assert!(decode_node(&b, e, 0, 2, 3).is_err());
    }
    let mut invalid = s.nodes[0].unwrap();
    invalid.parent = Some(0);
    assert!(encode_node(invalid, e, 0, 2, 3).is_err());
    invalid.parent = Some(99);
    assert!(encode_node(invalid, e, 0, 2, 3).is_err());
    let mut elder = encode_elder(s.elders[0], e).unwrap();
    elder[38] = 9;
    fix_checksum(&mut elder);
    assert!(decode_elder(&elder, e).is_err());
}
