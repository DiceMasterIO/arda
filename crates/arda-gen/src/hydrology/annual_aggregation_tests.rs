#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::super::annual::AnnualDestination;
use super::*;
use arda_core::hydrology::{ReceivingAccount, SpillConnection};

fn limits() -> AggregationLimits {
    AggregationLimits {
        ram_bytes: 1_000_000,
        bands: 100,
        operations: 1000,
    }
}
fn node(
    id: u64,
    floor: i32,
    birth: i32,
    spill: i32,
    parent: Option<u64>,
    children: std::ops::Range<usize>,
) -> AnnualNode {
    AnnualNode {
        id: BasinId(id),
        parent: parent.map(BasinId),
        floor: HeightMm::new(floor),
        birth: HeightMm::new(birth),
        spill: Some(SpillConnection {
            from: GlobalCell { x: 0, y: 0 },
            to: None,
            sill: HeightMm::new(spill),
            receiving: ReceivingAccount::DomainExport,
        }),
        destination: parent.is_none().then_some(AnnualDestination::DomainExport),
        children,
        bands: 0..0,
        local_runoff: Litres(0),
        local_precipitation: Litres(0),
        local_land_loss: Litres(0),
    }
}
fn leaf() -> Vec<AnnualNode> {
    vec![node(0, 0, 0, 10, None, 0..0)]
}
fn nested() -> Vec<AnnualNode> {
    vec![
        node(0, -10, -10, 10, Some(100), 0..0),
        node(1, 0, 0, 10, Some(100), 0..0),
        node(100, -10, 10, 30, None, 0..2),
    ]
}
fn cell(x: u32, bed: i32, terminal: Terminal) -> AnnualCell {
    AnnualCell {
        at: GlobalCell { x, y: 0 },
        bed: HeightMm::new(bed),
        rain: RainfallMm::new(100),
        evaporation_um: [10_000; 12],
        terminal,
    }
}
fn at_owner(x: u32, bed: i32, id: u64) -> AnnualCell {
    cell(x, bed, Terminal::Basin(BasinId(id)))
}

#[test]
fn monthly_evaporation_is_summed_and_land_loss_is_capped() {
    let low = cell_budget(
        RainfallMm::new(100),
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
    );
    assert_eq!(
        low,
        CellBudget {
            precipitation: Litres(1_000_000),
            evaporation: Litres(780),
            land_loss: Litres(780),
            runoff: Litres(999_220)
        }
    );
    let high = cell_budget(RainfallMm::new(100), &[10_000; 12]);
    assert_eq!(
        high,
        CellBudget {
            precipitation: Litres(1_000_000),
            evaporation: Litres(1_200_000),
            land_loss: Litres(500_000),
            runoff: Litres(500_000)
        }
    );
    let dry = cell_budget(RainfallMm::new(0), &[10_000; 12]);
    assert_eq!(dry.land_loss, Litres(0));
    assert_eq!(dry.runoff, Litres(0));
    let maximum = cell_budget(RainfallMm::new(u16::MAX), &[u32::MAX; 12]);
    assert_eq!(maximum.precipitation, Litres(655_350_000));
    assert_eq!(maximum.evaporation, Litres(515_396_075_400));
}

#[test]
fn nested_bands_keep_exact_height_owner_and_all_high_sources() {
    let mut a = Aggregator::new(Extent::new(8, 1).unwrap(), nested(), limits()).unwrap();
    for (x, (bed, owner)) in [
        (-10, 0),
        (0, 1),
        (10, 0),
        (10, 1),
        (20, 0),
        (30, 0),
        (45, 0),
        (-10, 0),
    ]
    .into_iter()
    .enumerate()
    {
        a.push(&at_owner(u32::try_from(x).unwrap(), bed, owner))
            .unwrap();
    }
    let result = a.finish().unwrap();
    assert_eq!(result.catchment_cells, vec![6, 2, 0]);
    assert_eq!(result.nodes[0].local_precipitation, Litres(6_000_000));
    assert_eq!(result.nodes[0].local_land_loss, Litres(3_000_000));
    assert_eq!(result.nodes[0].local_runoff, Litres(3_000_000));
    assert_eq!(result.nodes[1].local_precipitation, Litres(2_000_000));
    assert_eq!(result.nodes[1].local_runoff, Litres(1_000_000));
    assert_eq!(result.nodes[2].local_runoff, Litres(0));
    assert_eq!(
        result
            .nodes
            .iter()
            .map(|n| n.bands.clone())
            .collect::<Vec<_>>(),
        vec![0..1, 1..2, 2..5]
    );
    assert_eq!(
        result
            .bands
            .iter()
            .map(|b| (b.bed.raw(), b.owner.0, b.cells))
            .collect::<Vec<_>>(),
        vec![(-10, 0, 2), (0, 1, 1), (10, 0, 1), (10, 1, 1), (20, 0, 1)]
    );
    assert_eq!(result.bands[0].precipitation, Litres(2_000_000));
    assert_eq!(result.bands[0].effective_land_loss, Litres(1_000_000));
    assert_eq!(result.bands[0].open_water_evaporation, Litres(2_400_000));
    // Two high cells pay baseline land loss and remain sources, but cannot be lake bands.
    assert_eq!(result.bands.iter().map(|b| b.cells).sum::<u32>(), 6);
}

#[test]
fn marine_is_not_negative_land_and_open_exports_stay_separate() {
    let mut a = Aggregator::new(
        Extent::new(4, 1).unwrap(),
        vec![node(0, -20, -20, 10, None, 0..0)],
        limits(),
    )
    .unwrap();
    a.push(&cell(0, -30, Terminal::Marine)).unwrap();
    a.push(&at_owner(1, -20, 0)).unwrap();
    let mut sea = cell(2, 5, Terminal::Sea);
    sea.rain = RainfallMm::new(200);
    a.push(&sea).unwrap();
    let mut domain = cell(3, 5, Terminal::DomainExport);
    domain.evaporation_um = [0; 12];
    a.push(&domain).unwrap();
    let out = a.finish().unwrap();
    assert_eq!(out.marine_cells, 1);
    assert_eq!(out.catchment_cells, vec![1]);
    assert_eq!(out.nodes[0].local_runoff, Litres(500_000));
    assert_eq!(
        out.sea,
        DirectSource {
            cells: 1,
            precipitation: Litres(2_000_000),
            land_loss: Litres(1_000_000),
            runoff: Litres(1_000_000)
        }
    );
    assert_eq!(
        out.domain,
        DirectSource {
            cells: 1,
            precipitation: Litres(1_000_000),
            land_loss: Litres(0),
            runoff: Litres(1_000_000)
        }
    );
}

#[test]
fn complete_row_major_coverage_is_required_and_failure_cannot_publish() {
    let extent = Extent::new(2, 1).unwrap();
    let mut a = Aggregator::new(extent, leaf(), limits()).unwrap();
    assert_eq!(
        a.push(&at_owner(1, 0, 0)),
        Err(AggregationError::Invalid("cell order or coverage"))
    );
    assert!(a.finish().is_err());
    let mut a = Aggregator::new(extent, leaf(), limits()).unwrap();
    a.push(&at_owner(0, 0, 0)).unwrap();
    assert!(a.finish().is_err());
    let mut a = Aggregator::new(extent, leaf(), limits()).unwrap();
    a.push(&at_owner(0, 0, 0)).unwrap();
    assert!(a.push(&at_owner(0, 0, 0)).is_err());
    assert!(a.finish().is_err());
    let mut a = Aggregator::new(Extent::new(1, 1).unwrap(), leaf(), limits()).unwrap();
    a.push(&at_owner(0, 0, 0)).unwrap();
    assert_eq!(
        a.push(&at_owner(1, 0, 0)),
        Err(AggregationError::Invalid("too many cells"))
    );
    assert!(a.finish().is_err());
}

#[test]
fn actual_two_dimensional_coordinate_order_is_used() {
    let mut a = Aggregator::new(Extent::new(2, 2).unwrap(), Vec::new(), limits()).unwrap();
    for (y, x) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
        let mut c = cell(x, 0, Terminal::Marine);
        c.at.y = y;
        a.push(&c).unwrap();
    }
    let out = a.finish().unwrap();
    assert_eq!(out.marine_cells, 4);
    assert_eq!(out.operations, 4);
}

#[test]
fn invalid_ownership_and_marine_sign_are_rejected() {
    for c in [
        at_owner(0, 0, 99),
        at_owner(0, -1, 0),
        cell(0, 1, Terminal::Marine),
    ] {
        let mut a = Aggregator::new(Extent::new(1, 1).unwrap(), leaf(), limits()).unwrap();
        assert!(a.push(&c).is_err());
        assert!(a.finish().is_err());
    }
    let mut a = Aggregator::new(Extent::new(1, 1).unwrap(), nested(), limits()).unwrap();
    assert_eq!(
        a.push(&at_owner(0, 10, 100)),
        Err(AggregationError::Invalid(
            "nonleaf owner or bed below floor"
        ))
    );
}

#[test]
fn retained_node_shape_parent_and_source_authority_are_checked() {
    let extent = Extent::new(1, 1).unwrap();
    let mut inputs = Vec::new();
    let mut v = leaf();
    v[0].spill = None;
    inputs.push(v);
    let mut v = leaf();
    v[0].birth = HeightMm::new(1);
    inputs.push(v);
    let mut v = leaf();
    v[0].spill.as_mut().unwrap().sill = HeightMm::new(0);
    inputs.push(v);
    let mut v = leaf();
    v[0].local_runoff = Litres(1);
    inputs.push(v);
    let mut v = leaf();
    v[0].bands = 0..1;
    inputs.push(v);
    let mut v = leaf();
    v[0].parent = Some(BasinId(99));
    inputs.push(v);
    let mut v = leaf();
    v[0].parent = Some(BasinId(0));
    inputs.push(v);
    let mut v = nested();
    v.swap(0, 1);
    inputs.push(v);
    let mut v = nested();
    v[2].children = 0..1;
    inputs.push(v);
    let mut v = nested();
    v[2].children = std::ops::Range { start: 2, end: 1 };
    inputs.push(v);
    for input in inputs {
        assert!(Aggregator::new(extent, input, limits()).is_err());
    }
}

#[test]
fn zero_capacity_parent_cycle_is_rejected_before_any_cell() {
    let nodes = vec![
        node(0, 0, 0, 0, Some(1), 0..2),
        node(1, 0, 0, 0, Some(0), 2..4),
    ];
    assert!(Aggregator::new(Extent::new(1, 1).unwrap(), nodes, limits()).is_err());
}

#[test]
fn owned_node_capacity_and_band_overlap_are_charged_before_growth() {
    let mut nodes = Vec::with_capacity(128);
    nodes.extend(leaf());
    let l = AggregationLimits {
        ram_bytes: required_ram(1, 0).unwrap(),
        ..limits()
    };
    assert!(matches!(
        Aggregator::new(Extent::new(1, 1).unwrap(), nodes, l),
        Err(AggregationError::Limit("RAM"))
    ));
    let mut a = Aggregator::new(Extent::new(1, 1).unwrap(), leaf(), l).unwrap();
    assert_eq!(
        a.push(&at_owner(0, 0, 0)),
        Err(AggregationError::Limit("bands or RAM"))
    );
    assert!(a.finish().is_err());
    assert_eq!(required_ram(u64::MAX, 1), None);
    assert_eq!(required_ram(0, u64::MAX), None);
}

#[test]
fn band_limit_counts_distinct_keys_not_cell_visits() {
    let l = AggregationLimits {
        bands: 1,
        ..limits()
    };
    let mut a = Aggregator::new(Extent::new(2, 1).unwrap(), leaf(), l).unwrap();
    a.push(&at_owner(0, 0, 0)).unwrap();
    a.push(&at_owner(1, 0, 0)).unwrap();
    assert_eq!(a.finish().unwrap().bands.len(), 1);
    let mut a = Aggregator::new(Extent::new(2, 1).unwrap(), leaf(), l).unwrap();
    a.push(&at_owner(0, 0, 0)).unwrap();
    assert_eq!(
        a.push(&at_owner(1, 1, 0)),
        Err(AggregationError::Limit("bands or RAM"))
    );
}

#[test]
fn work_admission_includes_nodes_ancestry_and_final_rows() {
    let e = Extent::new(1, 1).unwrap();
    let l = AggregationLimits {
        operations: 0,
        ..limits()
    };
    assert!(matches!(
        Aggregator::new(e, leaf(), l),
        Err(AggregationError::Limit("operations"))
    ));
    let mut a = Aggregator::new(
        e,
        leaf(),
        AggregationLimits {
            operations: 4,
            ..limits()
        },
    )
    .unwrap();
    assert_eq!(
        a.push(&at_owner(0, 0, 0)),
        Err(AggregationError::Limit("operations"))
    );
    let mut a = Aggregator::new(
        e,
        leaf(),
        AggregationLimits {
            operations: 6,
            ..limits()
        },
    )
    .unwrap();
    a.push(&at_owner(0, 0, 0)).unwrap();
    assert!(matches!(
        a.finish(),
        Err(AggregationError::Limit("operations"))
    ));
    let mut a = Aggregator::new(
        e,
        leaf(),
        AggregationLimits {
            operations: 7,
            ..limits()
        },
    )
    .unwrap();
    a.push(&at_owner(0, 0, 0)).unwrap();
    assert_eq!(a.finish().unwrap().operations, 7);
}

#[test]
fn checked_annual_source_bounds_abort_without_a_partial_result() {
    assert_eq!(add(MAX_ANNUAL, 0), Ok(MAX_ANNUAL));
    assert_eq!(add(MAX_ANNUAL, 1), Err(AggregationError::Overflow));
    assert_eq!(add(u128::MAX, 1), Err(AggregationError::Overflow));
    let mut a = Aggregator::new(Extent::new(1, 1).unwrap(), leaf(), limits()).unwrap();
    a.total_e = MAX_ANNUAL;
    assert_eq!(a.push(&at_owner(0, 0, 0)), Err(AggregationError::Overflow));
    assert!(a.finish().is_err());
    let mut direct = DirectSource {
        cells: u32::MAX,
        ..DirectSource::default()
    };
    assert_eq!(
        source_add(&mut direct, cell_budget(RainfallMm::new(100), &[0; 12])),
        Err(AggregationError::Overflow)
    );
}
