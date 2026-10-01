use super::decode::decode_inner;
use super::*;
use crate::hydrology::{
    AnnualCatchment, AreaHydrologyContext, CatchmentId, GlobalLake, GlobalReach, Litres, ReachId,
    ReceivingAccount, SpillConnection,
};
use crate::{
    BasinId, DischargeMilli, FormatError, GlobalCell, HeightMm, Lake, RiverSegment, Terminus,
};
fn cc(x: u16, y: u16) -> CellCoord {
    CellCoord::new(x, y).unwrap()
}
fn sample() -> AreaObjects {
    let q = DischargeMilli::new(u64::from(u32::MAX) + 8);
    let reach = GlobalReach {
        id: ReachId(4),
        from: GlobalCell { x: 0, y: 0 },
        to: GlobalCell { x: 2, y: 1 },
        receiving: ReceivingAccount::Sea,
        catchment: CatchmentId(5),
        drainage_cells: 100,
        annual_volume: Litres(u128::from(q.raw()) * 31_536_000),
        mean_discharge: q,
    };
    let lake = GlobalLake {
        basin: BasinId(99),
        surface: HeightMm::new(3000),
        deepest_bed: HeightMm::new(2999),
        submerged_cells: 2,
        outlet: None,
        annual_outflow: Litres(0),
        mean_outflow: DischargeMilli::new(0),
    };
    let state = AnnualCatchment {
        catchment: CatchmentId(5),
        terminal: GlobalCell { x: 2, y: 1 },
        contributing_cells: 100,
        basin: Some(BasinId(99)),
        representative_lake: Some(BasinId(99)),
        potential_spill: Some(SpillConnection {
            from: GlobalCell { x: 3, y: 1 },
            to: Some(GlobalCell { x: 4, y: 1 }),
            sill: HeightMm::new(4000),
            receiving: ReceivingAccount::Sea,
        }),
        receiving: ReceivingAccount::Sea,
    };
    AreaObjects {
        rivers: vec![RiverSegment {
            global_id: reach.id,
            id: 70000,
            order: 2,
            width_dm: 80000,
            discharge: q,
            feeds: None,
            ends: Terminus::Lake,
            course: vec![cc(0, 0), cc(1, 0), cc(2, 1)],
        }],
        lakes: vec![Lake {
            global_id: lake.basin,
            id: 90000,
            surface: lake.surface,
            depth_mm: 1,
            outlet: None,
            cells: vec![cc(2, 1), cc(3, 1)],
        }],
        channel_edges: vec![ChannelEdge {
            from: GlobalCell { x: 0, y: 0 },
            to: GlobalCell { x: 1, y: 0 },
            from_width_dm: 80000,
            to_width_dm: 80000,
            discharge: q,
        }],
        global: AreaHydrologyContext {
            reaches: vec![reach],
            lakes: vec![lake],
            catchments: vec![state],
            ..AreaHydrologyContext::default()
        },
    }
}
#[derive(Clone)]
struct Section {
    kind: u16,
    count: u32,
    body: Vec<u8>,
}
fn sections(bytes: &[u8]) -> Vec<Section> {
    let mut r = Reader::new(bytes, 0);
    r.take(8).unwrap();
    let n = r.u16().unwrap();
    let mut out = Vec::new();
    for _ in 0..n {
        let kind = r.u16().unwrap();
        let count = r.u32().unwrap();
        let len = usize::try_from(r.u32().unwrap()).unwrap();
        out.push(Section {
            kind,
            count,
            body: r.take(len).unwrap().to_vec(),
        });
    }
    r.finish().unwrap();
    out
}
fn pack(parts: &[Section]) -> Vec<u8> {
    let mut out = OBJECTS_MAGIC.to_vec();
    out.extend(u16::try_from(parts.len()).unwrap().to_le_bytes());
    for s in parts {
        out.extend(s.kind.to_le_bytes());
        out.extend(s.count.to_le_bytes());
        out.extend(u32::try_from(s.body.len()).unwrap().to_le_bytes());
        out.extend(&s.body);
    }
    out
}
fn decode(bytes: &[u8]) -> Result<AreaObjects> {
    decode_inner(bytes, ObjectsLimits::default())
}
fn rejects_change(change: impl FnOnce(&mut AreaObjects)) {
    let mut o = sample();
    change(&mut o);
    assert!(encode_objects(&o).is_err());
}
#[test]
fn roundtrip_preserves_widened_fields_global_ids_and_whole_mm_depth() {
    let o = sample();
    let b = encode_objects(&o).unwrap();
    assert_eq!(decode(&b).unwrap(), o);
    assert_eq!(encode_objects(&o).unwrap(), b);
    let parts = sections(&b);
    assert_eq!(parts.len(), 4);
    assert_eq!(parts[0].body.len(), 34 + 12);
    assert_eq!(parts[1].body.len(), 29 + 8);
    assert_eq!(parts[2].body.len(), 32);
    assert_eq!(parts[3].count, 1);
    assert_eq!(&parts[0].body[..8], &4_u64.to_le_bytes());
    assert_eq!(&parts[0].body[8..12], &70000_u32.to_le_bytes());
    assert_eq!(&parts[0].body[13..17], &80000_u32.to_le_bytes());
    assert_eq!(
        &parts[0].body[17..25],
        &(u64::from(u32::MAX) + 8).to_le_bytes()
    );
    assert_eq!(&parts[0].body[30..34], &3_u32.to_le_bytes());
    assert_eq!(o.lakes[0].depth_mm, 1);
    assert!(o.rivers[0].course.contains(&o.lakes[0].cells[0]));
}
#[test]
fn empty_container_requires_new_sections_and_revision_two() {
    let o = AreaObjects::empty();
    let b = encode_objects(&o).unwrap();
    assert_eq!(b.len(), 78);
    assert_eq!(decode(&b).unwrap(), o);
    assert_eq!(o.global.model_revision, 2);
}
#[test]
fn every_prefix_trailing_data_and_path_errors_are_rejected() {
    let b = encode_objects(&sample()).unwrap();
    for n in 0..b.len() {
        assert!(decode(&b[..n]).is_err(), "accepted prefix {n}");
    }
    let mut tail = b.clone();
    tail.push(0);
    assert!(matches!(
        decode(&tail),
        Err(ObjectsFormatError::TrailingBytes)
    ));
    let mut bad = b.clone();
    bad[0] = 0;
    assert!(
        matches!(decode_objects("area/file",&bad),Err(FormatError::BadMagic{path,layer:"objects"}) if path=="area/file")
    );
    assert!(
        matches!(decode_objects("area/file",&b[..b.len()-1]),Err(FormatError::Objects{path,..}) if path=="area/file")
    );
}
#[test]
fn record_cannot_borrow_bytes_from_following_section() {
    let b = encode_objects(&sample()).unwrap();
    let mut p = sections(&b);
    // Keep the next section intact; a declared river body ends before its
    // course's final coordinate. It must fail inside this exact slice.
    p[0].body.truncate(42);
    assert!(matches!(
        decode(&pack(&p)),
        Err(ObjectsFormatError::Truncated {
            offset: 54,
            needed: 12,
            available: 8
        })
    ));
    let mut p = sections(&b);
    p[1].body.truncate(33);
    assert!(matches!(
        decode(&pack(&p)),
        Err(ObjectsFormatError::Truncated {
            needed: 8,
            available: 4,
            ..
        })
    ));
}
#[test]
fn required_sections_unique_order_and_single_context_are_enforced() {
    let b = encode_objects(&sample()).unwrap();
    for k in 1..=4 {
        let mut p = sections(&b);
        p.retain(|s| s.kind != k);
        assert!(matches!(decode(&pack(&p)),Err(ObjectsFormatError::MissingSection(v)) if v==k));
    }
    let mut p = sections(&b);
    p.insert(1, p[0].clone());
    assert!(matches!(
        decode(&pack(&p)),
        Err(ObjectsFormatError::SectionOrder)
    ));
    let mut p = sections(&b);
    p.swap(0, 1);
    assert!(matches!(
        decode(&pack(&p)),
        Err(ObjectsFormatError::SectionOrder)
    ));
    for n in [0, 2] {
        let mut p = sections(&b);
        p[3].count = n;
        assert!(decode(&pack(&p)).is_err());
    }
}
#[test]
fn known_sections_consume_exactly_unknown_sections_are_length_bounded() {
    let b = encode_objects(&sample()).unwrap();
    for i in 0..4 {
        let mut p = sections(&b);
        p[i].body.push(0);
        assert!(decode(&pack(&p)).is_err());
    }
    let mut p = sections(&b);
    p.push(Section {
        kind: 999,
        count: u32::MAX,
        body: vec![1, 2, 3],
    });
    let good = pack(&p);
    assert_eq!(decode(&good).unwrap(), sample());
    assert!(decode(&good[..good.len() - 1]).is_err());
    p.push(p[4].clone());
    assert!(matches!(
        decode(&pack(&p)),
        Err(ObjectsFormatError::SectionOrder)
    ));
}
#[test]
fn malformed_counts_fail_before_count_driven_allocation() {
    let b = encode_objects(&sample()).unwrap();
    for i in 0..3 {
        let mut p = sections(&b);
        p[i].count = u32::MAX;
        assert!(matches!(
            decode(&pack(&p)),
            Err(ObjectsFormatError::Limit(_))
        ));
    }
    for (i, offset) in [(0, 30), (1, 25)] {
        let mut p = sections(&b);
        p[i].body[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            decode(&pack(&p)),
            Err(ObjectsFormatError::Limit(_))
        ));
    }
    let mut p = sections(&b);
    p[2].count = 0;
    assert!(decode(&pack(&p)).is_err());
    let limits = ObjectsLimits {
        max_bytes: b.len() - 1,
        ..ObjectsLimits::default()
    };
    assert!(encode_objects_with_limits(&sample(), limits).is_err());
    assert!(decode_objects_with_limits("x", &b, limits).is_err());
    let limits = ObjectsLimits {
        max_sections: 3,
        ..ObjectsLimits::default()
    };
    assert!(encode_objects_with_limits(&sample(), limits).is_err());
    assert!(decode_objects_with_limits("x", &b, limits).is_err());
    let limits = ObjectsLimits {
        max_course_cells: 2,
        ..ObjectsLimits::default()
    };
    assert!(encode_objects_with_limits(&sample(), limits).is_err());
    assert!(decode_objects_with_limits("x", &b, limits).is_err());
    let limits = ObjectsLimits {
        context: ContextLimits {
            max_records: 0,
            ..ContextLimits::default()
        },
        ..ObjectsLimits::default()
    };
    assert!(encode_objects_with_limits(&sample(), limits).is_err());
    assert!(decode_objects_with_limits("x", &b, limits).is_err());
}
#[test]
fn unknown_tags_noncanonical_none_and_bad_coordinates_fail() {
    let b = encode_objects(&sample()).unwrap();
    for (section, offset, value) in [(0, 29, 9), (1, 20, 2), (1, 21, 1)] {
        let mut p = sections(&b);
        p[section].body[offset] = value;
        assert!(decode(&pack(&p)).is_err());
    }
    for (section, offset) in [(0, 34), (1, 29)] {
        let mut p = sections(&b);
        p[section].body[offset..offset + 2].copy_from_slice(&512_u16.to_le_bytes());
        assert!(decode(&pack(&p)).is_err());
    }
}
#[test]
fn local_records_require_matching_copied_authority() {
    rejects_change(|o| o.rivers[0].global_id = ReachId(123));
    rejects_change(|o| o.rivers[0].discharge = DischargeMilli::new(41));
    rejects_change(|o| o.lakes[0].global_id = BasinId(123));
    rejects_change(|o| o.lakes[0].surface = HeightMm::new(3001));
    rejects_change(|o| o.lakes[0].depth_mm = 2);
    rejects_change(|o| o.lakes[0].outlet = Some(cc(2, 1)));
    rejects_change(|o| o.rivers[0].id = 0);
}
#[test]
fn dry_basin_endpoint_and_tiny_supported_lake_outlet_roundtrip() {
    let mut o = sample();
    o.rivers[0].ends = Terminus::Basin;
    o.global.reaches[0].receiving = ReceivingAccount::Lake(BasinId(99));
    o.lakes.clear();
    o.global.lakes.clear();
    o.global.catchments[0].representative_lake = None;
    let bytes = encode_objects(&o).unwrap();
    assert_eq!(decode(&bytes).unwrap(), o);
    assert_eq!(Terminus::from_u8(4), Some(Terminus::Basin));
    o.global.catchments[0].basin = None;
    assert!(encode_objects(&o).is_err());
    let mut o = sample();
    let at = o.lakes[0].cells[0];
    o.lakes[0].outlet = Some(at);
    let global = &mut o.global.lakes[0];
    global.annual_outflow = Litres(1);
    global.outlet = Some(SpillConnection {
        from: GlobalCell {
            x: u32::from(at.x()),
            y: u32::from(at.y()),
        },
        to: Some(GlobalCell {
            x: u32::from(at.x()) + 1,
            y: u32::from(at.y()),
        }),
        sill: global.surface,
        receiving: ReceivingAccount::Sea,
    });
    assert_eq!(global.mean_outflow.raw(), 0);
    let bytes = encode_objects(&o).unwrap();
    assert_eq!(decode(&bytes).unwrap(), o);
}
#[test]
fn courses_are_nonempty_simple_directed_d8() {
    rejects_change(|o| o.rivers[0].course.clear());
    rejects_change(|o| o.rivers[0].course.push(cc(400, 400)));
    rejects_change(|o| o.rivers[0].course.push(cc(1, 0)));
    rejects_change(|o| o.rivers[0].ends = Terminus::Junction);
    rejects_change(|o| o.rivers[0].feeds = Some(70000));
}
fn connected() -> AreaObjects {
    let mut o = sample();
    let mut upstream = o.rivers[0].clone();
    upstream.id = 1;
    upstream.ends = Terminus::Junction;
    upstream.feeds = Some(70000);
    upstream.course = vec![cc(0, 1), cc(0, 0)];
    o.rivers.insert(0, upstream);
    o
}
#[test]
fn shared_endpoints_and_multiple_fragments_per_global_id_are_valid() {
    let mut o = connected();
    let mut lake = o.lakes[0].clone();
    o.lakes[0].cells.truncate(1);
    lake.id += 1;
    lake.cells.remove(0);
    o.lakes.push(lake);
    let b = encode_objects(&o).unwrap();
    assert_eq!(decode(&b).unwrap(), o);
    o.rivers[0].course = vec![cc(0, 1)];
    assert!(encode_objects(&o).is_ok());
}
#[test]
fn feeds_targets_are_present_connected_and_acyclic() {
    let mut o = connected();
    o.rivers[0].feeds = Some(123);
    assert!(encode_objects(&o).is_err());
    let mut o = connected();
    o.rivers[0].feeds = Some(1);
    assert!(encode_objects(&o).is_err());
    let mut o = connected();
    o.rivers[0].course = vec![cc(10, 10)];
    assert!(encode_objects(&o).is_err());
    let mut o = connected();
    o.rivers[0].course = vec![cc(0, 0)];
    o.rivers[1].course = vec![cc(1, 0)];
    o.rivers[1].ends = Terminus::Junction;
    o.rivers[1].feeds = Some(1);
    assert!(matches!(
        encode_objects(&o),
        Err(ObjectsFormatError::Invalid("local river feeds cycle"))
    ));
}
#[test]
fn lake_memberships_are_canonical_disjoint_and_bounded_by_global_count() {
    rejects_change(|o| o.lakes[0].cells.reverse());
    rejects_change(|o| o.lakes[0].cells.push(cc(4, 1)));
    rejects_change(|o| {
        let mut l = o.lakes[0].clone();
        l.id += 1;
        o.lakes.push(l);
    });
    rejects_change(|o| o.channel_edges.push(o.channel_edges[0]));
}

mod divergence;
