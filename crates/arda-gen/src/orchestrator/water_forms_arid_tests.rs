//! Recipe-7 arid forms: the arid terminal origin, saline flags and playa
//! runs (logic/02 §world-water arid basins).
#![allow(clippy::unwrap_used)]

use super::*;
use crate::formation::water::Sink;
use arda_core::{Cell, HeightMm, Lake, TerrainKind};

fn c(x: u16, y: u16) -> CellCoord {
    CellCoord::new(x, y).unwrap()
}

/// A two-cell lake on an arid sink in area (1, 0), ringed by playa
/// cells: crust beside the lake, mudflat beyond.
fn fixture(outflow: u128) -> (AreaCells, AreaObjects, WaterFeatures, Vec<GlobalLake>) {
    let mut cells = AreaCells::flat(Cell {
        terrain: TerrainKind::Land,
        ..Cell::default()
    });
    let mut objects = AreaObjects::empty();
    let lake_cells = vec![c(10, 20), c(11, 20)];
    for &at in &lake_cells {
        cells.set(
            at,
            Cell {
                terrain: TerrainKind::Lake,
                height: HeightMm::new(-2_000),
                ..Cell::default()
            },
        );
    }
    objects.lakes.push(Lake {
        global_id: BasinId(3),
        id: 1,
        surface: HeightMm::new(-1_000),
        depth_mm: 1_000,
        outlet: None,
        cells: lake_cells,
    });
    let mut features = WaterFeatures::default();
    features.sinks.push(Sink {
        x_um: (512 + 10) * CELL_UM,
        y_um: 20 * CELL_UM,
        radius_um: 1_500_000_000,
        kind: SinkKind::AridTerminal,
    });
    // A karst polje inside the same lake must not name it.
    features.sinks.push(Sink {
        x_um: (512 + 11) * CELL_UM,
        y_um: 20 * CELL_UM,
        radius_um: 75_000_000,
        kind: SinkKind::Karst,
    });
    for x in 8..=14 {
        features.pan_cells.push((512 + x, 20, u8::from(x >= 13)));
    }
    features.pan_cells.push((5, 20, 0)); // area (0, 0): not here
    features.index();
    let lakes = vec![GlobalLake {
        basin: BasinId(3),
        surface: HeightMm::new(-1_000),
        deepest_bed: HeightMm::new(-2_000),
        submerged_cells: 2,
        outlet: None,
        annual_outflow: Litres(outflow),
        mean_outflow: arda_core::DischargeMilli::new(0),
    }];
    (cells, objects, features, lakes)
}

#[test]
fn an_arid_terminal_lake_is_named_saline_and_sits_in_its_pan() {
    let (cells, objects, features, lakes) = fixture(0);
    let mut origins = BTreeMap::new();
    let w = area_water(
        AreaCoord::new(1, 0),
        &cells,
        &objects,
        &features,
        &lakes,
        &mut origins,
    );
    assert_eq!(w.lakes[0].origin, LakeOrigin::AridTerminal);
    assert!(w.lakes[0].terminal && w.lakes[0].saline);
    // Lake cells hold water, not a pan.
    assert_eq!(
        w.pans,
        vec![
            PanRun {
                y: 20,
                x0: 8,
                len: 2,
                kind: PanKind::SaltCrust
            },
            PanRun {
                y: 20,
                x0: 12,
                len: 1,
                kind: PanKind::SaltCrust
            },
            PanRun {
                y: 20,
                x0: 13,
                len: 2,
                kind: PanKind::Mudflat
            },
        ]
    );
    // A fragment elsewhere that found a smaller form takes the basin's.
    let mut far = AreaWater {
        lakes: vec![LakeForm {
            origin: LakeOrigin::Oxbow,
            terminal: true,
            saline: true,
        }],
        ..AreaWater::default()
    };
    resolve_origins(&mut far, &[BasinId(3)], &origins);
    assert_eq!(far.lakes[0].origin, LakeOrigin::AridTerminal);
}

#[test]
fn an_arid_basin_lake_that_spills_is_tectonic() {
    let (cells, objects, features, lakes) = fixture(5_000);
    let mut origins = BTreeMap::new();
    let w = area_water(
        AreaCoord::new(1, 0),
        &cells,
        &objects,
        &features,
        &lakes,
        &mut origins,
    );
    assert_eq!(w.lakes[0].origin, LakeOrigin::Tectonic);
    assert!(!w.lakes[0].terminal && !w.lakes[0].saline);
}
