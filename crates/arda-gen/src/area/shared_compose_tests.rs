#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use crate::hydrology::types::{MarineBoundary, PreparedExtent, PreparedTerrain};
use arda_core::hydrology::{BasinId, GlobalLake, Litres, ReceivingAccount, SpillConnection};
use arda_core::{
    AreaCoord, CellCoord, Cover, DischargeMilli, GlobalCell, HeightMm, RainfallMm, TerrainKind,
};
fn coord(x: u16, y: u16) -> CellCoord {
    CellCoord::new(x, y).unwrap()
}
fn fixture() -> (PreparedTerrain, Vec<SolvedCell>) {
    (
        PreparedTerrain {
            area: AreaCoord::new(0, 0),
            valid: PreparedExtent {
                width: 512,
                height: 512,
                boundary: MarineBoundary::default(),
            },
            heights: vec![HeightMm::new(1000); 262144],
            annual_rain: vec![RainfallMm::new(700); 262144],
            temperature_base_centi: vec![2000; 262144],
        },
        vec![
            SolvedCell {
                hand_mm: u32::MAX,
                ..SolvedCell::default()
            };
            262144
        ],
    )
}
fn lake(id: u64, surface: i32, bed: i32, count: u32, annual: u128, from: GlobalCell) -> GlobalLake {
    GlobalLake {
        basin: BasinId(id),
        surface: HeightMm::new(surface),
        deepest_bed: HeightMm::new(bed),
        submerged_cells: count,
        outlet: Some(SpillConnection {
            from,
            to: None,
            sill: HeightMm::new(surface),
            receiving: ReceivingAccount::DomainExport,
        }),
        annual_outflow: Litres(annual),
        mean_outflow: DischargeMilli::new(u64::try_from(annual / 31_536_000).unwrap()),
    }
}
#[test]
fn negative_land_and_connected_sea_apply_different_signed_temperature_and_rain() {
    let (mut p, mut w) = fixture();
    let dry = coord(0, 0);
    let sea = coord(511, 511);
    p.heights[dry.index()] = HeightMm::new(-1_000_000);
    p.heights[sea.index()] = HeightMm::new(-1_000_000);
    w[sea.index()].marine = true;
    let (cells, lakes) = compose_shared(&p, &w, &[], required_ram(0).unwrap()).unwrap();
    let d = cells.get(dry);
    let s = cells.get(sea);
    assert_eq!(d.terrain, TerrainKind::Land);
    assert_eq!(d.temperature.raw(), 2650);
    assert_eq!(d.rainfall.raw(), 700);
    assert_eq!(s.terrain, TerrainKind::Sea);
    assert_eq!(s.temperature.raw(), 2000);
    assert_eq!(s.rainfall.raw(), 0);
    assert_eq!(s.wetness, 255);
    assert!(lakes.is_empty());
}
#[test]
fn one_millimetre_membership_reaches_rims_and_corner_with_global_identity() {
    let (mut p, mut w) = fixture();
    let members = [
        coord(0, 0),
        coord(511, 0),
        coord(0, 511),
        coord(511, 511),
        coord(0, 100),
    ];
    for at in members {
        p.heights[at.index()] = HeightMm::new(999);
        w[at.index()].lake = Some(BasinId(9));
    }
    let global = [lake(9, 1000, 999, 5, 1, GlobalCell { x: 511, y: 511 })];
    let before = p.clone();
    let water_before = w.clone();
    let global_before = global.clone();
    let one = compose_shared(&p, &w, &global, required_ram(1).unwrap()).unwrap();
    let two = compose_shared(&p, &w, &global, required_ram(1).unwrap()).unwrap();
    assert_eq!(one, two);
    assert_eq!(p, before);
    assert_eq!(w, water_before);
    assert_eq!(global, global_before);
    assert_eq!(one.1.len(), 1);
    let l = &one.1[0];
    assert_eq!(l.global_id, BasinId(9));
    assert_eq!(l.depth_mm, 1);
    assert_eq!(l.cells.len(), 5);
    assert_eq!(l.outlet, Some(coord(511, 511)));
    assert_eq!(global[0].mean_outflow.raw(), 0);
    for at in members {
        let c = one.0.get(at);
        assert_eq!(c.terrain, TerrainKind::Lake);
        assert_eq!(c.height.raw(), 999);
        assert_eq!(c.rainfall.raw(), 0);
        assert_eq!(c.discharge.raw(), 0);
    }
    assert_eq!(one.0.get(coord(1, 1)).terrain, TerrainKind::Land);
}
#[test]
fn outlet_belongs_only_to_actual_source_area_and_positive_annual_outflow() {
    let (mut p, mut w) = fixture();
    let at = coord(1, 1);
    w[at.index()].lake = Some(BasinId(3));
    p.heights[at.index()] = HeightMm::new(0);
    p.area = AreaCoord::new(1, 2);
    let mut g = lake(3, 10, 0, 1, 1, GlobalCell { x: 513, y: 1025 });
    let (_, l) =
        compose_shared(&p, &w, std::slice::from_ref(&g), required_ram(1).unwrap()).unwrap();
    assert_eq!(l[0].outlet, Some(at));
    assert_eq!(l[0].depth_mm, 10);
    g.outlet.as_mut().unwrap().from.x = 1;
    let (_, l) =
        compose_shared(&p, &w, std::slice::from_ref(&g), required_ram(1).unwrap()).unwrap();
    assert_eq!(l[0].outlet, None);
    g.outlet.as_mut().unwrap().from.x = 513;
    g.annual_outflow = Litres(0);
    let (_, l) = compose_shared(&p, &w, &[g], required_ram(1).unwrap()).unwrap();
    assert_eq!(l[0].outlet, None);
}
#[test]
fn channels_keep_checked_width_initiation_and_existing_hand_marsh_rule() {
    assert_eq!(SolvedCell::default().hand_mm, u32::MAX);
    let (p, mut w) = fixture();
    let channel = coord(1, 1);
    let marsh = coord(2, 2);
    let meadow = coord(3, 3);
    let terrace = coord(4, 4);
    w[channel.index()] = SolvedCell {
        drainage_cells: 900,
        discharge: DischargeMilli::new(25_000),
        order: 2,
        hand_mm: 0,
        ..SolvedCell::default()
    };
    w[marsh.index()] = SolvedCell {
        drainage_cells: 9,
        hand_mm: 999,
        hand_discharge: DischargeMilli::new(200),
        ..SolvedCell::default()
    };
    w[meadow.index()] = SolvedCell {
        hand_mm: 999,
        hand_discharge: DischargeMilli::new(199),
        ..SolvedCell::default()
    };
    w[terrace.index()] = SolvedCell {
        hand_mm: 1000,
        hand_discharge: DischargeMilli::new(200),
        ..SolvedCell::default()
    };
    let (c, _) = compose_shared(&p, &w, &[], required_ram(0).unwrap()).unwrap();
    assert_eq!(c.get(channel).watercourse_width_dm, 200);
    assert_eq!(c.get(channel).drainage_area_cells, 900);
    assert_eq!(c.get(channel).height_above_river_dm, 0);
    assert_eq!(c.get(channel).cover, Cover::Grass);
    assert_eq!(c.get(marsh).cover, Cover::Marsh);
    assert_eq!(c.get(meadow).cover, Cover::Grass);
    assert_eq!(c.get(terrace).cover, Cover::Grass);
    assert_eq!(c.get(coord(9, 9)).height_above_river_dm, u16::MAX);
    w[channel.index()].discharge = DischargeMilli::new(39);
    assert!(matches!(
        compose_shared(&p, &w, &[], required_ram(0).unwrap()),
        Err(ComposeError::Invalid(_))
    ));
    w[channel.index()].discharge = DischargeMilli::new(40);
    assert!(compose_shared(&p, &w, &[], required_ram(0).unwrap()).is_ok());
    w[channel.index()].discharge = DischargeMilli::new(33_249_619_483);
    let (wide, _) = compose_shared(&p, &w, &[], required_ram(0).unwrap()).unwrap();
    assert_eq!(wide.get(channel).watercourse_width_dm, 230_649);
    w[channel.index()].hand_mm = 1;
    assert!(matches!(
        compose_shared(&p, &w, &[], required_ram(0).unwrap()),
        Err(ComposeError::Invalid(_))
    ));
    w[channel.index()].hand_mm = 0;
    w[channel.index()].discharge = DischargeMilli::new(u64::MAX);
    assert!(matches!(
        compose_shared(&p, &w, &[], required_ram(0).unwrap()),
        Err(ComposeError::Width(_))
    ));
}
#[test]
fn arbitrary_positive_area_overflow_is_rejected_before_outlet_projection() {
    let (mut p, w) = fixture();
    p.area = AreaCoord::new(i32::MAX, 0);
    assert!(matches!(
        compose_shared(&p, &w, &[], required_ram(0).unwrap()),
        Err(ComposeError::Invalid(_))
    ));
}
#[test]
fn signed_height_extremes_cannot_overflow_scalar_slope() {
    let (mut p, w) = fixture();
    p.heights[0] = HeightMm::new(i32::MAX);
    p.heights[1] = HeightMm::new(i32::MIN);
    let (cells, _) = compose_shared(&p, &w, &[], required_ram(0).unwrap()).unwrap();
    assert_eq!(cells.get(coord(0, 0)).aspect_deg, 90);
    assert!(cells.get(coord(0, 0)).slope_milli_deg > 0);
    assert_eq!(cells.get(coord(0, 0)).temperature.raw(), -30_000);
    assert_eq!(cells.get(coord(1, 0)).temperature.raw(), 30_000);
}
#[test]
fn invalid_shape_membership_authority_and_ram_do_not_mutate_inputs() {
    let (p, w) = fixture();
    let low = required_ram(0).unwrap() - 1;
    assert!(matches!(
        compose_shared(&p, &w, &[], low),
        Err(ComposeError::Limit)
    ));
    let mut malformed = p.clone();
    malformed.valid.width = 511;
    assert!(matches!(
        compose_shared(&malformed, &w, &[], u64::MAX),
        Err(ComposeError::Invalid(_))
    ));
    malformed = p.clone();
    malformed.heights.pop();
    assert!(matches!(
        compose_shared(&malformed, &w, &[], u64::MAX),
        Err(ComposeError::Invalid(_))
    ));
    malformed = p.clone();
    malformed.area = AreaCoord::new(-1, 0);
    assert!(matches!(
        compose_shared(&malformed, &w, &[], u64::MAX),
        Err(ComposeError::Invalid(_))
    ));
    let mut invalid = w.clone();
    invalid[0].marine = true;
    assert!(matches!(
        compose_shared(&p, &invalid, &[], u64::MAX),
        Err(ComposeError::Invalid(_))
    ));
    invalid[0].marine = false;
    invalid[0].lake = Some(BasinId(1));
    assert!(matches!(
        compose_shared(&p, &invalid, &[], u64::MAX),
        Err(ComposeError::Invalid(_))
    ));
    let g = lake(1, 1000, 999, 1, 0, GlobalCell { x: 0, y: 0 });
    assert!(matches!(
        compose_shared(&p, &invalid, std::slice::from_ref(&g), u64::MAX),
        Err(ComposeError::Invalid(_))
    ));
    let mut wet = p.clone();
    wet.heights[0] = HeightMm::new(998);
    assert!(matches!(
        compose_shared(&wet, &invalid, std::slice::from_ref(&g), u64::MAX),
        Err(ComposeError::Invalid(_))
    ));
    wet.heights[0] = HeightMm::new(999);
    invalid[0].marine = true;
    assert!(matches!(
        compose_shared(&wet, &invalid, std::slice::from_ref(&g), u64::MAX),
        Err(ComposeError::Invalid(_))
    ));
    invalid[0].marine = false;
    invalid[0].discharge = DischargeMilli::new(1);
    assert!(matches!(
        compose_shared(&wet, &invalid, std::slice::from_ref(&g), u64::MAX),
        Err(ComposeError::Invalid(_))
    ));
    invalid[0].discharge = DischargeMilli::new(0);
    let mut small = g.clone();
    small.submerged_cells = 0;
    assert!(matches!(
        compose_shared(&wet, &invalid, &[small], u64::MAX),
        Err(ComposeError::Invalid(_))
    ));
    assert!(matches!(
        compose_shared(&p, &w, &[g.clone(), g], u64::MAX),
        Err(ComposeError::Invalid(_))
    ));
    let mut bad_global = lake(1, 1000, 999, 1, 1, GlobalCell { x: 0, y: 0 });
    bad_global.outlet = None;
    assert!(matches!(
        compose_shared(&p, &w, &[bad_global.clone()], u64::MAX),
        Err(ComposeError::Invalid(_))
    ));
    bad_global.annual_outflow = Litres(0);
    bad_global.mean_outflow = DischargeMilli::new(1);
    assert!(matches!(
        compose_shared(&p, &w, &[bad_global], u64::MAX),
        Err(ComposeError::Invalid(_))
    ));
    assert_eq!(p, fixture().0);
    assert_eq!(w, fixture().1);
}

#[test]
fn actual_zero_depth_dry_spill_cell_can_be_a_local_lake_outlet() {
    let (mut p, mut w) = fixture();
    p.heights[coord(1, 1).index()] = HeightMm::new(0);
    w[coord(1, 1).index()].lake = Some(BasinId(8));
    p.heights[coord(2, 1).index()] = HeightMm::new(10);
    let g = lake(8, 10, 0, 1, 1, GlobalCell { x: 2, y: 1 });
    let (cells, lakes) = compose_shared(&p, &w, &[g], required_ram(1).unwrap()).unwrap();
    assert_eq!(lakes[0].outlet, Some(coord(2, 1)));
    assert_eq!(lakes[0].cells, vec![coord(1, 1)]);
    assert_eq!(cells.get(coord(2, 1)).terrain, TerrainKind::Land);
    if let Ok(maximum) = usize::try_from(u64::MAX) {
        assert!(required_ram(maximum).is_none());
    }
}
