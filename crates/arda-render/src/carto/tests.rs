use super::*;
use crate::{AtlasHalo, AtlasNeighbor, AtlasTerrain, ImageQuality};
use arda_core::{Cell, CellCoord, DischargeMilli, HeightMm, TerrainKind, AREA_CELLS};

fn standalone_atlas(cells: &AreaCells) -> AtlasTerrain {
    let mut halo = AtlasHalo::new();
    for direction in [
        AtlasNeighbor::North,
        AtlasNeighbor::NorthEast,
        AtlasNeighbor::East,
        AtlasNeighbor::SouthEast,
        AtlasNeighbor::South,
        AtlasNeighbor::SouthWest,
        AtlasNeighbor::West,
        AtlasNeighbor::NorthWest,
    ] {
        halo.mark_world_edge(direction).unwrap();
    }
    AtlasTerrain::new(cells, halo).unwrap()
}

#[test]
fn atlas_area_exact_centers_and_adjacent_rows() {
    let mut cells = AreaCells::flat(Cell {
        terrain: TerrainKind::Land,
        ..Cell::default()
    });
    for y in 0..AREA_CELLS {
        for x in 0..AREA_CELLS {
            cells.set(
                CellCoord::new(x, y).unwrap(),
                Cell {
                    height: HeightMm::new(i32::from(x) * 2_000 + i32::from(y) * 3_000),
                    terrain: TerrainKind::Land,
                    ..Cell::default()
                },
            );
        }
    }
    let terrain = standalone_atlas(&cells);
    let objects = AreaObjects::default();
    let origin = GlobalCell { x: 0, y: 0 };
    let mut png = Vec::new();
    render_area_png_to_atlas(
        &cells,
        &objects,
        origin,
        AreaImageScale::Custom(ImageQuality::new(1536).unwrap()),
        &terrain,
        &mut png,
    )
    .unwrap();
    for (x, y) in [(0, 0), (255, 255), (511, 511)] {
        assert_eq!(
            pixel_at(&png, x * 3 + 1, y * 3 + 1),
            terrain.colour(
                CellCoord::new(u16::try_from(x).unwrap(), u16::try_from(y).unwrap()).unwrap()
            )
        );
    }

    let mut step = AreaCells::flat(Cell {
        height: HeightMm::new(1_000_000),
        terrain: TerrainKind::Land,
        ..Cell::default()
    });
    for x in 0..AREA_CELLS {
        step.set(
            CellCoord::new(x, 0).unwrap(),
            Cell {
                height: HeightMm::new(0),
                terrain: TerrainKind::Land,
                ..Cell::default()
            },
        );
    }
    let terrain = standalone_atlas(&step);
    png.clear();
    render_area_png_to_atlas(
        &step,
        &objects,
        origin,
        AreaImageScale::Custom(ImageQuality::new(1536).unwrap()),
        &terrain,
        &mut png,
    )
    .unwrap();
    assert_eq!(
        pixel_at(&png, 1, 1),
        terrain.colour(CellCoord::new(0, 0).unwrap())
    );
    assert_eq!(
        pixel_at(&png, 1, 2),
        terrain
            .sample(
                crate::atlas::axis_kernel(1, 1536).unwrap(),
                crate::atlas::axis_kernel(2, 1536).unwrap(),
                TerrainKind::Land
            )
            .unwrap()
    );
    assert_ne!(pixel_at(&png, 1, 1), pixel_at(&png, 1, 2));
}

#[test]
fn atlas_area_routes_512_and_513_and_reports_class_mismatch() {
    let cells = AreaCells::flat(Cell {
        height: HeightMm::new(200_000),
        terrain: TerrainKind::Land,
        ..Cell::default()
    });
    let terrain = standalone_atlas(&cells);
    let objects = AreaObjects::default();
    let origin = GlobalCell { x: 0, y: 0 };
    for side in [512, 513] {
        let mut png = Vec::new();
        render_area_png_to_atlas(
            &cells,
            &objects,
            origin,
            AreaImageScale::Custom(ImageQuality::new(side).unwrap()),
            &terrain,
            &mut png,
        )
        .unwrap();
        for (x, y) in [(0, 0), (side / 2, side / 2), (side - 1, side - 1)] {
            assert_eq!(
                pixel_at(&png, x as usize, y as usize),
                terrain
                    .sample(
                        crate::atlas::axis_kernel(x, side).unwrap(),
                        crate::atlas::axis_kernel(y, side).unwrap(),
                        TerrainKind::Land
                    )
                    .unwrap()
            );
        }
    }
    let sea = AreaCells::flat(Cell {
        terrain: TerrainKind::Sea,
        ..Cell::default()
    });
    let mismatched = standalone_atlas(&sea);
    assert!(matches!(
        render_area_png_to_atlas(
            &cells,
            &objects,
            origin,
            AreaImageScale::Preview,
            &mismatched,
            Vec::new()
        ),
        Err(RenderError::AtlasContext { .. })
    ));
}

#[test]
fn standing_water_depth_is_bounded_monotonic_and_visible() {
    let shallow = [58, 137, 180];
    let deep = [22, 68, 126];
    let mut previous = shallow;
    for depth in 0..=3100 {
        let colour = lake_colour(depth);
        assert!(colour[2] >= colour[1] + 40 && colour[1] > colour[0]);
        for i in 0..3 {
            assert!((deep[i]..=shallow[i]).contains(&colour[i]));
            assert!(colour[i] <= previous[i]);
        }
        previous = colour;
    }
    for bed in [
        i32::MIN,
        0,
        180_000,
        500_000,
        1_400_000,
        2_800_000,
        i32::MAX,
    ] {
        let land = land_colour(bed);
        for depth in [1, 13, 28, 750, 3000, u32::MAX] {
            let colour = lake_colour(depth);
            let distance: u32 = colour
                .iter()
                .zip(land)
                .map(|(&a, b)| u32::from(a.abs_diff(b)))
                .sum();
            assert!(
                distance >= 100,
                "water must remain distinct from the land palette"
            );
        }
    }
    assert_eq!(lake_colour(0), shallow);
    assert_eq!(lake_colour(3000), deep);
    assert_eq!(lake_colour(u32::MAX), deep);
    assert_eq!(lake_colour(13), [57, 135, 178]);
    assert_eq!(lake_colour(28), [56, 132, 176]);
}

fn cell(discharge_milli: u64, terrain: TerrainKind) -> Cell {
    Cell {
        terrain,
        discharge: DischargeMilli::new(discharge_milli),
        watercourse_order: u8::from(discharge_milli > 0),
        ..Cell::default()
    }
}

#[test]
fn streamed_area_png_matches_collected_pixels_and_propagates_writer_errors() {
    let cells = uniform_tile(0);
    let objects = AreaObjects::empty();
    let origin = GlobalCell { x: 0, y: 0 };
    for scale in [
        AreaImageScale::Preview,
        AreaImageScale::Custom(crate::ImageQuality::new(513).unwrap()),
    ] {
        let mut streamed = Vec::new();
        render_area_png_to(&cells, &objects, origin, scale, &mut streamed).unwrap();
        assert_eq!(
            streamed,
            render_area_png(&cells, &objects, origin, scale).unwrap()
        );
        let expected = crate::channels::raster_results(&cells, [], origin, scale, &[]).unwrap();
        let mut reader = png::Decoder::new(streamed.as_slice()).read_info().unwrap();
        assert_eq!(reader.info().width, scale.side());
        assert_eq!(reader.info().height, scale.side());
        let mut decoded = vec![0; reader.output_buffer_size()];
        reader.next_frame(&mut decoded).unwrap();
        assert_eq!(decoded, expected);
    }
    // A real filesystem writer failure exercises the PNG error boundary.
    let readonly = std::fs::File::open(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    assert!(matches!(
        render_area_png_to(&cells, &objects, origin, AreaImageScale::Preview, readonly),
        Err(RenderError::Png)
    ));
}

#[test]
fn terminal_point_saved_record_is_the_only_point_render_authority() {
    use arda_core::hydrology::{
        BasinId, CatchmentId, GlobalReach, Litres, ReachId, ReceivingAccount,
    };
    let cells = uniform_tile(0);
    let origin = GlobalCell { x: 0, y: 0 };
    let mut objects = AreaObjects::empty();
    let plain = render_area_png(&cells, &objects, origin, AreaImageScale::Preview).unwrap();
    let at = GlobalCell { x: 10, y: 10 };
    objects.global.reaches.push(GlobalReach {
        id: ReachId::point(at).unwrap(),
        from: at,
        to: at,
        receiving: ReceivingAccount::Lake(BasinId(1)),
        catchment: CatchmentId(1),
        drainage_cells: 1,
        annual_volume: Litres(40 * 31_536_000),
        mean_discharge: DischargeMilli::new(40),
    });
    let visible = render_area_png(&cells, &objects, origin, AreaImageScale::Preview).unwrap();
    assert_ne!(pixel_at(&plain, 10, 10), pixel_at(&visible, 10, 10));
    assert_eq!(pixel_at(&plain, 11, 10), pixel_at(&visible, 11, 10));
    let row = &mut objects.global.reaches[0];
    row.receiving = ReceivingAccount::DomainExport;
    row.annual_volume = Litres(1);
    row.mean_discharge = DischargeMilli::new(0);
    assert_eq!(
        plain,
        render_area_png(&cells, &objects, origin, AreaImageScale::Preview).unwrap()
    );
}

#[test]
fn river_bands_split_on_the_calibrated_discharge_cuts() {
    assert_eq!(river_band(RIVER_Q_MIN - 1), None);
    assert_eq!(river_band(RIVER_Q_MIN), Some(RiverBand::Light));
    assert_eq!(river_band(RIVER_Q_MID - 1), Some(RiverBand::Light));
    assert_eq!(river_band(RIVER_Q_MID), Some(RiverBand::Mid));
    assert_eq!(river_band(RIVER_Q_MAX - 1), Some(RiverBand::Mid));
    assert_eq!(river_band(RIVER_Q_MAX), Some(RiverBand::Dark));
}

#[test]
fn a_dry_cell_is_never_a_river_however_high_its_order() {
    // The whole point of the retouch: selection is by how much water
    // a channel carries, not by where it sits in the hierarchy.
    let mut c = cell(0, TerrainKind::Land);
    c.watercourse_order = 9;
    assert_eq!(river_band(c.discharge.raw()), None);
}

#[test]
fn every_water_colour_is_distinguishable_from_every_other() {
    // Guards the defect this retouch fixed: the old lake fill
    // [58,110,190] and mid river band [60,105,185] differed by 2
    // units of blue and were the same colour on the page. Cheap
    // proxy for a perceptual metric — a generous Manhattan floor in
    // sRGB, which the old pair (ΔE00 2.02, Manhattan 12) fails.
    let water = [
        ("lake", LAKE_FILL),
        ("light", river_band_colour(RiverBand::Light)),
        ("mid", river_band_colour(RiverBand::Mid)),
        ("dark", river_band_colour(RiverBand::Dark)),
        ("sea", OVERVIEW_SEA),
    ];
    for (i, (an, a)) in water.iter().enumerate() {
        for (bn, b) in water.iter().skip(i + 1) {
            let d: i32 = (0..3)
                .map(|k| (i32::from(a[k]) - i32::from(b[k])).abs())
                .sum();
            assert!(d >= 40, "{an} and {bn} are too close (Manhattan {d})");
        }
    }
}

#[test]
fn blocks_cover_every_cell_of_the_tile() {
    // The shipped renderer used `block = 512 / px`, so at the default
    // px = 48 it read cells 0..=479 and silently dropped 480..=511 of
    // every tile. Walk the same bounds the render loop derives and
    // assert they tile 0..512 with no gap and no overlap.
    for px in [1u32, 2, 3, 7, 16, 48, 64, 512] {
        let side = u32::from(AREA_CELLS);
        let mut covered = 0u32;
        let mut prev_end = 0u32;
        for i in 0..px {
            let x0 = i * side / px;
            let x1 = ((i + 1) * side / px).max(x0 + 1);
            assert_eq!(x0, prev_end, "gap or overlap at px={px} block={i}");
            covered += x1 - x0;
            prev_end = x1;
        }
        assert_eq!(prev_end, side, "px={px} stops short of the tile");
        assert_eq!(covered, side, "px={px} does not cover the tile exactly");
    }
}

fn uniform_tile(discharge_milli: u64) -> AreaCells {
    AreaCells::flat(cell(discharge_milli, TerrainKind::Land))
}

fn pixel_at(png_bytes: &[u8], x: usize, y: usize) -> [u8; 3] {
    let Ok(mut reader) = png::Decoder::new(png_bytes).read_info() else {
        panic!("invalid PNG header");
    };
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let Ok(info) = reader.next_frame(&mut buf) else {
        panic!("invalid PNG frame");
    };
    let row = y * info.line_size;
    let col = x * 3;
    [buf[row + col], buf[row + col + 1], buf[row + col + 2]]
}

#[test]
fn the_strongest_band_in_a_block_wins_it() {
    let dark = uniform_tile(RIVER_Q_MAX);
    let light = uniform_tile(RIVER_Q_MIN);
    let dry = uniform_tile(0);
    let areas = [(0, 0, &dark), (1, 0, &light), (0, 1, &dry)];
    let Ok(png) = render_overview_png(&areas, 2, 2, 1) else {
        panic!("render_overview_png failed");
    };
    assert_eq!(pixel_at(&png, 0, 0), river_band_colour(RiverBand::Dark));
    assert_eq!(pixel_at(&png, 1, 0), river_band_colour(RiverBand::Light));
    assert_ne!(pixel_at(&png, 0, 1), river_band_colour(RiverBand::Light));
}

#[test]
fn rendering_is_byte_identical_on_repeat() {
    // logic/04: re-export must produce the same bytes.
    let t = uniform_tile(RIVER_Q_MID);
    let areas = [(0, 0, &t)];
    let (Ok(a), Ok(b)) = (
        render_overview_png(&areas, 1, 1, 8),
        render_overview_png(&areas, 1, 1, 8),
    ) else {
        panic!("render failed");
    };
    assert_eq!(a, b);
}

#[test]
fn the_dark_band_widens_and_the_light_band_does_not() {
    // Hierarchy has to come from the data now that no continent
    // course is drawn: a trunk gets an extra pixel, a stream does not.
    for (q, widens) in [(RIVER_Q_MAX, true), (RIVER_Q_MIN, false)] {
        let wet = uniform_tile(q);
        let dry = uniform_tile(0);
        let areas = [(0, 0, &wet), (1, 0, &dry), (0, 1, &dry), (1, 1, &dry)];
        let Ok(png) = render_overview_png(&areas, 2, 2, 1) else {
            panic!("render failed");
        };
        let spread = pixel_at(&png, 1, 0) == river_band_colour(RiverBand::Dark);
        assert_eq!(spread, widens, "band at discharge {q} widened: {spread}");
    }
}

#[test]
fn a_lake_covering_the_block_outranks_a_river_in_it() {
    // px = 1 makes the whole tile one block, so `side` lake cells
    // square is exactly the share of it the rule is given.
    let mut cells = AreaCells::flat(cell(RIVER_Q_MAX, TerrainKind::Land));
    let side = AREA_CELLS / 2; // a quarter of the tile by area
    for y in 0..side {
        for x in 0..side {
            let Some(at) = CellCoord::new(x, y) else {
                panic!("{x},{y} is in range");
            };
            cells.set(at, cell(0, TerrainKind::Lake));
        }
    }
    let areas = [(0, 0, &cells)];
    let Ok(png) = render_overview_png(&areas, 1, 1, 1) else {
        panic!("render_overview_png failed");
    };
    assert_eq!(pixel_at(&png, 0, 0), LAKE_FILL);
}

#[test]
fn one_lake_cell_does_not_claim_a_whole_block() {
    // The defect this rule fixes: a single 100 m cell used to win a
    // 1.1 km² pixel, which fringed every lake with detached specks.
    let mut cells = AreaCells::flat(cell(RIVER_Q_MAX, TerrainKind::Land));
    let Some(at) = CellCoord::new(0, 0) else {
        panic!("0,0 is in range");
    };
    cells.set(at, cell(0, TerrainKind::Lake));
    let areas = [(0, 0, &cells)];
    let Ok(png) = render_overview_png(&areas, 1, 1, 1) else {
        panic!("render_overview_png failed");
    };
    assert_eq!(
        pixel_at(&png, 0, 0),
        river_band_colour(RiverBand::Dark),
        "one lake cell in 262,144 must not paint the block as lake"
    );
}

#[test]
fn a_lake_still_wins_a_block_it_genuinely_fills() {
    // px = AREA_CELLS puts one cell in each block, so a lake cell
    // covers its block entirely and must survive the area floor.
    let mut cells = AreaCells::flat(cell(0, TerrainKind::Land));
    let Some(at) = CellCoord::new(0, 0) else {
        panic!("0,0 is in range");
    };
    cells.set(at, cell(0, TerrainKind::Lake));
    let areas = [(0, 0, &cells)];
    let Ok(png) = render_overview_png(&areas, 1, 1, u32::from(AREA_CELLS)) else {
        panic!("render_overview_png failed");
    };
    assert_eq!(pixel_at(&png, 0, 0), LAKE_FILL);
}
