use super::*;
use std::io::Cursor;
fn spill() -> SpillConnection {
    SpillConnection {
        from: GlobalCell { x: 511, y: 10 },
        to: Some(GlobalCell { x: 512, y: 11 }),
        sill: HeightMm::new(4000),
        receiving: ReceivingAccount::Sea,
    }
}
fn state() -> AnnualCatchment {
    AnnualCatchment {
        catchment: CatchmentId(7),
        terminal: GlobalCell { x: 500, y: 10 },
        contributing_cells: 100,
        basin: Some(BasinId(55)),
        representative_lake: Some(BasinId(999)),
        potential_spill: Some(spill()),
        receiving: ReceivingAccount::Sea,
    }
}
fn reach() -> GlobalReach {
    GlobalReach {
        id: ReachId(4004),
        from: GlobalCell { x: 500, y: 0 },
        to: GlobalCell { x: 550, y: 50 },
        receiving: ReceivingAccount::Sea,
        catchment: CatchmentId(7),
        drainage_cells: 100,
        annual_volume: Litres(31_536_000_000),
        mean_discharge: DischargeMilli::new(1000),
    }
}
fn lake() -> GlobalLake {
    GlobalLake {
        basin: BasinId(999),
        surface: HeightMm::new(4000),
        deepest_bed: HeightMm::new(3000),
        submerged_cells: 1,
        outlet: Some(spill()),
        annual_outflow: Litres(1),
        mean_outflow: DischargeMilli::new(0),
    }
}
fn context() -> AreaHydrologyContext {
    let reach = reach();
    AreaHydrologyContext {
        model_revision: 2,
        lakes: vec![lake()],
        catchments: vec![state()],
        crossings: vec![SharedCrossing {
            id: CrossingId {
                low: spill().from,
                high: spill().to.unwrap(),
            },
            from: spill().from,
            to: spill().to.unwrap(),
            reach: reach.id,
            catchment: reach.catchment,
            drainage_cells: reach.drainage_cells,
            annual_volume: reach.annual_volume,
            mean_discharge: reach.mean_discharge,
            receiving: reach.receiving,
        }],
        reaches: vec![reach],
    }
}
fn metadata() -> HydrologyMetadata {
    HydrologyMetadata {
        model_revision: 2,
        domain: HydrologyDomain {
            width_cells: 1024,
            height_cells: 1024,
            exported_areas_wide: 2,
            exported_areas_high: 2,
        },
        basin_count: 10,
        lake_count: 2,
        reach_count: 8,
        crossing_count: 3,
        catchment_count: 10,
        budget: AnnualWaterBalance {
            land_precipitation: Litres(100),
            land_loss: Litres(40),
            lake_precipitation: Litres(20),
            lake_evaporation: Litres(30),
            marginal_evaporation: Litres(10),
            sea_outflow: Litres(25),
            domain_outflow: Litres(15),
        },
    }
}
fn roundtrip<T: FixedRecord + std::fmt::Debug + PartialEq>(v: &T) {
    let b = encode_record(v).unwrap();
    assert_eq!(b.len(), T::WIDTH);
    assert_eq!(&decode_record::<T>(&b).unwrap(), v);
    for n in 0..b.len() {
        assert!(decode_record::<T>(&b[..n]).is_err());
    }
    let mut trailing = b.clone();
    trailing.push(0);
    assert!(decode_record::<T>(&trailing).is_err());
}
#[test]
fn exact_schema_widths_and_channel_little_endian() {
    assert_eq!(ChannelEdge::WIDTH, 32);
    assert_eq!(SpillConnection::SIZE, 30);
    assert_eq!(GlobalLake::WIDTH, 75);
    assert_eq!(GlobalReach::WIDTH, 69);
    assert_eq!(SharedCrossing::WIDTH, 85);
    assert_eq!(AnnualCatchment::WIDTH, 78);
    assert_eq!(HydrologyMetadata::WIDTH, 172);
    assert_eq!(BasinNodeRow::WIDTH, 76);
    let v = ChannelEdge {
        from: GlobalCell {
            x: 0x01020304,
            y: 9,
        },
        to: GlobalCell {
            x: 0x01020305,
            y: 10,
        },
        from_width_dm: 70000,
        to_width_dm: 230650,
        discharge: DischargeMilli::new(0x0102030405060708),
    };
    let b = encode_record(&v).unwrap();
    assert_eq!(&b[..4], &[4, 3, 2, 1]);
    assert_eq!(&b[24..32], &[8, 7, 6, 5, 4, 3, 2, 1]);
    roundtrip(&v);
}
#[test]
fn every_annual_authority_roundtrips() {
    let c = context();
    roundtrip(&c.lakes[0]);
    roundtrip(&c.catchments[0]);
    roundtrip(&c.crossings[0]);
    roundtrip(&c.reaches[0]);
    roundtrip(&metadata());
    roundtrip(&BasinNodeRow {
        id: BasinId(999),
        parent: None,
        anchor: GlobalCell { x: 2, y: 3 },
        floor: HeightMm::new(-1),
        children: TableSpan {
            offset: 0,
            count: 999999999,
        },
        spill: None,
    });
}
#[test]
fn context_roundtrip_keeps_merged_parent_and_positive_sub_litre_per_second_outflow() {
    let c = context();
    let b = encode_area_context(&c).unwrap();
    assert_eq!(
        decode_area_context(&b, ContextLimits::default()).unwrap(),
        c
    );
    assert_eq!(
        encode_area_context(&decode_area_context(&b, ContextLimits::default()).unwrap()).unwrap(),
        b
    );
}
#[test]
fn every_context_truncation_and_trailing_byte_fails() {
    let b = encode_area_context(&context()).unwrap();
    for n in 0..b.len() {
        assert!(
            decode_area_context(&b[..n], ContextLimits::default()).is_err(),
            "prefix {n}"
        );
    }
    let mut b = b;
    b.push(0);
    assert!(decode_area_context(&b, ContextLimits::default()).is_err());
}
#[test]
fn empty_context_is_revision_two_and_has_exact_header() {
    let c = AreaHydrologyContext::default();
    assert_eq!(c.model_revision, 2);
    let b = encode_area_context(&c).unwrap();
    assert_eq!(b.len(), 28);
    assert_eq!(&b[..8], b"ARDACTX4");
    assert_eq!(
        decode_area_context(&b, ContextLimits::default()).unwrap(),
        c
    );
}
#[test]
fn annual_budget_surface_and_revision_invariants_are_enforced() {
    let mut v = lake();
    v.deepest_bed = v.surface;
    assert!(encode_record(&v).is_err());
    v = lake();
    v.outlet = None;
    assert!(encode_record(&v).is_err());
    v = lake();
    v.mean_outflow = DischargeMilli::new(1);
    assert!(encode_record(&v).is_err());
    let mut m = metadata();
    m.budget.domain_outflow.0 += 1;
    assert!(encode_record(&m).is_err());
    m = metadata();
    m.budget.land_precipitation = Litres(u128::MAX);
    assert!(matches!(
        encode_record(&m),
        Err(HydrologyFormatError::Overflow)
    ));
    m = metadata();
    m.model_revision = 1;
    assert!(encode_record(&m).is_err());
    let mut bytes = encode_area_context(&context()).unwrap();
    bytes[8..12].copy_from_slice(&1u32.to_le_bytes());
    assert!(decode_area_context(&bytes, ContextLimits::default()).is_err());
    let mut c = context();
    c.lakes.clear();
    c.catchments[0].representative_lake = None;
    assert!(
        encode_area_context(&c).is_ok(),
        "dry physical basin has no fake lake"
    );
    for tag in 7..=10 {
        assert!(TableKind::parse(tag).is_err());
    }
}
#[test]
fn reserved_tags_and_absent_payloads_are_rejected() {
    let mut b = encode_record(&lake()).unwrap();
    b[20] = 2;
    assert!(decode_record::<GlobalLake>(&b).is_err());
    let node = BasinNodeRow {
        id: BasinId(4),
        parent: None,
        anchor: GlobalCell { x: 0, y: 0 },
        floor: HeightMm::new(0),
        children: TableSpan::default(),
        spill: None,
    };
    let mut b = encode_record(&node).unwrap();
    b[8] = 7;
    assert!(decode_record::<BasinNodeRow>(&b).is_err());
    b = encode_record(&node).unwrap();
    b[9] = 1;
    assert!(decode_record::<BasinNodeRow>(&b).is_err());
    let mut b = encode_record(&reach()).unwrap();
    b[24] = 99;
    assert!(decode_record::<GlobalReach>(&b).is_err());
    b = encode_record(&reach()).unwrap();
    b[25] = 1;
    assert!(decode_record::<GlobalReach>(&b).is_err());
}
#[test]
fn streaming_tables_reject_counts_sizes_tags_order_and_partial_writes() {
    let mut writer = TableWriter::<_, GlobalReach>::create(Vec::new(), 2).unwrap();
    writer.write_record(&reach()).unwrap();
    assert!(writer.write_record(&reach()).is_err());
    let mut next = reach();
    next.id = ReachId(4007);
    writer.write_record(&next).unwrap();
    let b = writer.finish().unwrap();
    let mut reader = TableReader::<_, GlobalReach>::open(Cursor::new(&b), 10000, 2).unwrap();
    assert_eq!(reader.count(), 2);
    assert_eq!(reader.read_at(1).unwrap(), next);
    assert!(reader.read_at(2).is_err());
    assert_eq!(reader.next_record().unwrap().unwrap(), reach());
    assert_eq!(reader.next_record().unwrap().unwrap(), next);
    assert!(reader.next_record().unwrap().is_none());
    let mut bad = b.clone();
    bad[8..12].copy_from_slice(&999_u32.to_le_bytes());
    assert!(TableReader::<_, GlobalReach>::open(Cursor::new(bad), 10000, 2).is_err());
    let mut bad = b.clone();
    bad[16..24].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(TableReader::<_, GlobalReach>::open(Cursor::new(bad), 10000, u64::MAX).is_err());
    for n in 0..b.len() {
        assert!(TableReader::<_, GlobalReach>::open(Cursor::new(&b[..n]), 10000, 2).is_err());
    }
    let mut trailing = b.clone();
    trailing.push(0);
    assert!(TableReader::<_, GlobalReach>::open(Cursor::new(trailing), 10000, 2).is_err());
    assert!(TableWriter::<_, GlobalReach>::create(Vec::new(), 1)
        .unwrap()
        .finish()
        .is_err());
    assert!(TableHeader::for_type::<GlobalReach>(u64::MAX).is_err());
}
#[test]
fn internal_references_are_checked_without_world_closure() {
    let mut c = context();
    c.reaches.clear();
    assert!(encode_area_context(&c).is_err());
    c = context();
    c.catchments.clear();
    assert!(encode_area_context(&c).is_err());
    c = context();
    c.crossings[0].annual_volume.0 += 1;
    assert!(encode_area_context(&c).is_err());
    c = context();
    c.reaches[0].receiving = ReceivingAccount::Reach(ReachId(9000));
    c.crossings[0].receiving = c.reaches[0].receiving;
    assert!(
        encode_area_context(&c).is_ok(),
        "external closure must not load world"
    );
    assert!(TableSpan {
        offset: u64::MAX,
        count: 1
    }
    .validate_total(u64::MAX)
    .is_err());
    assert!(TableSpan {
        offset: 9,
        count: 2
    }
    .validate_total(10)
    .is_err());
}
#[test]
fn child_ranges_stream_without_root_vectors() {
    let mut w = TableWriter::<_, BasinId>::create(Vec::new(), 4).unwrap();
    for id in [2, 3, 2, 9] {
        w.write_record(&BasinId(id)).unwrap();
    }
    let mut r = TableReader::<_, BasinId>::open(Cursor::new(w.finish().unwrap()), 1000, 4).unwrap();
    assert_eq!(
        r.span(TableSpan {
            offset: 0,
            count: 2
        })
        .unwrap()
        .collect::<Result<Vec<_>>>()
        .unwrap(),
        vec![BasinId(2), BasinId(3)]
    );
    assert!(r
        .span(TableSpan {
            offset: 0,
            count: 4
        })
        .unwrap()
        .collect::<Result<Vec<_>>>()
        .is_err());
    assert!(validate_span_partition(
        [
            TableSpan {
                offset: 0,
                count: 2
            },
            TableSpan::default(),
            TableSpan {
                offset: 2,
                count: 2
            }
        ],
        4
    )
    .is_ok());
    assert!(validate_span_partition(
        [
            TableSpan {
                offset: 0,
                count: 2
            },
            TableSpan {
                offset: 1,
                count: 2
            }
        ],
        4
    )
    .is_err());
}

#[test]
fn limits_fail_before_count_driven_allocation() {
    let b = encode_area_context(&context()).unwrap();
    let limits = ContextLimits {
        max_bytes: 100,
        max_records: 1,
    };
    assert!(decode_area_context(&b, limits).is_err());
    assert!(encode_area_context_with_limits(&context(), limits).is_err());
    let mut b = encode_area_context(&AreaHydrologyContext::default()).unwrap();
    b[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_area_context(&b, ContextLimits::default()).is_err());
}
#[test]
fn only_actual_modeled_boundary_can_export_without_neighbor() {
    let domain = HydrologyDomain {
        width_cells: 1024,
        height_cells: 1024,
        exported_areas_wide: 2,
        exported_areas_high: 2,
    };
    let mut s = spill();
    s.to = None;
    s.receiving = ReceivingAccount::DomainExport;
    assert!(validate_spill_domain(&s, domain).is_err());
    s.from.x = 0;
    assert!(validate_spill_domain(&s, domain).is_ok());
}
#[test]
fn junction_accounts_keep_width_and_potential_references_need_no_outgoing_table() {
    let mut g = reach();
    let junction = JunctionId::at(g.to);
    g.receiving = ReceivingAccount::Junction(junction);
    let bytes = encode_record(&g).unwrap();
    assert_eq!(bytes.len(), 69);
    assert_eq!(bytes[24], 4);
    assert_eq!(&bytes[25..33], &junction.0.to_le_bytes());
    roundtrip(&g);
    let mut bad = bytes.clone();
    bad[24] = 5;
    assert!(decode_record::<GlobalReach>(&bad).is_err());
    let mut bad = g.clone();
    bad.receiving = ReceivingAccount::Junction(JunctionId(junction.0 + 1));
    assert!(encode_record(&bad).is_err());
    bad = g.clone();
    bad.id = ReachId(g.id.0 + 8);
    assert!(encode_record(&bad).is_err());
    let mut c = AreaHydrologyContext {
        reaches: vec![g],
        catchments: vec![state()],
        ..AreaHydrologyContext::default()
    };
    let potential = c.catchments[0].potential_spill.as_mut().unwrap();
    let to = potential.to.unwrap();
    potential.receiving = ReceivingAccount::Junction(JunctionId::at(to));
    c.catchments[0].receiving = potential.receiving;
    let bytes = encode_area_context(&c).unwrap();
    assert_eq!(
        decode_area_context(&bytes, ContextLimits::default()).unwrap(),
        c
    );
    c.catchments[0].potential_spill.as_mut().unwrap().receiving =
        ReceivingAccount::Junction(JunctionId(0));
    assert!(encode_area_context(&c).is_err());
}
#[test]
fn point_reaches_preserve_width_and_validate_terminal_flow_semantics() {
    let mut p = reach();
    p.id = ReachId::point(p.from).unwrap();
    p.to = p.from;
    p.receiving = ReceivingAccount::Lake(BasinId(55));
    roundtrip(&p);
    assert_eq!(encode_record(&p).unwrap().len(), 69);
    let mut bad = p.clone();
    bad.to.x += 1;
    assert!(encode_record(&bad).is_err());
    bad = p.clone();
    bad.id = ReachId::from_step(
        p.from,
        GlobalCell {
            x: p.from.x + 1,
            y: p.from.y,
        },
    )
    .unwrap();
    assert!(encode_record(&bad).is_err());
    for receiving in [
        ReceivingAccount::Sea,
        ReceivingAccount::Reach(ReachId(4)),
        ReceivingAccount::Junction(JunctionId::at(p.from)),
    ] {
        bad = p.clone();
        bad.receiving = receiving;
        assert!(encode_record(&bad).is_err());
    }
    p.annual_volume = Litres(40 * 31_536_000);
    p.mean_discharge = DischargeMilli::new(40);
    roundtrip(&p);
    p.annual_volume = Litres(40 * 31_536_000 - 1);
    p.mean_discharge = DischargeMilli::new(39);
    assert!(encode_record(&p).is_err());
    p.receiving = ReceivingAccount::DomainExport;
    roundtrip(&p);
    p.annual_volume = Litres(1);
    p.mean_discharge = DischargeMilli::new(0);
    roundtrip(&p);
    p.annual_volume = Litres(0);
    assert!(encode_record(&p).is_err());
}
