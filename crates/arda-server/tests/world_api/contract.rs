//! Contract checks on real data: stored fields, derived coast, sample_point.

use crate::support::{state, world_dir};
use arda_core::{TerrainKind, TerrainPoint};
use arda_server::contract::TerrainKindDto;
use arda_server::fine::FINE_READER_BYTES;

#[test]
fn cell_samples_carry_the_stored_fields_in_plain_units() {
    let s = state();
    let area = s.query.area(1, 2).unwrap();
    for (cx, cy) in [(0_u16, 12_u16), (100, 300), (511, 511)] {
        let raw = area.cell(cx, cy).unwrap();
        let sample = s
            .query
            .cell(512 + u32::from(cx), 1024 + u32::from(cy))
            .unwrap();
        assert_eq!((sample.ax, sample.ay, sample.cx, sample.cy), (1, 2, cx, cy));
        assert_eq!(sample.height_m, f64::from(raw.height.raw()) / 1000.0);
        assert_eq!(
            sample.temperature_c,
            f64::from(raw.temperature.raw()) / 100.0
        );
        assert_eq!(sample.slope_deg, f64::from(raw.slope_milli_deg) / 1000.0);
        assert_eq!(sample.watercourse_order, raw.watercourse_order);
        assert_eq!(
            sample.drainage_area_km2,
            f64::from(raw.drainage_area_cells) * 0.01
        );
        let fine = sample.fine.expect("MICRO fine world has fine heights");
        assert!(fine.min_m <= fine.centre_m && fine.centre_m <= fine.max_m);
    }
}

#[test]
fn coast_cells_are_land_beside_sea_and_distances_grow_inland() {
    let s = state();
    let (ax, ay) = (1, 2);
    let area = s.query.area(ax, ay).unwrap();
    let samples = s.query.area_samples(ax, ay).unwrap();
    let mut coast = 0;
    for (i, sample) in samples.iter().enumerate() {
        let (cx, cy) = (i % 512, i / 512);
        if sample.coast.is_coast {
            coast += 1;
            assert_eq!(sample.terrain, TerrainKindDto::Land);
            assert_eq!(sample.coast.distance_m, Some(0.0));
            if (1..511).contains(&cx) && (1..511).contains(&cy) {
                let sea_near = (-1_i32..=1).any(|dy| {
                    (-1_i32..=1).any(|dx| {
                        let x = u16::try_from(i32::try_from(cx).unwrap() + dx).unwrap();
                        let y = u16::try_from(i32::try_from(cy).unwrap() + dy).unwrap();
                        area.cell(x, y).unwrap().terrain == TerrainKind::Sea
                    })
                });
                assert!(sea_near, "coast cell {cx},{cy} has no sea neighbour");
            }
        } else if let Some(d) = sample.coast.distance_m {
            assert!(d > 0.0 && d <= 5000.0);
        }
    }
    assert!(coast > 100, "area 1,2 of seed 42 has a coastline");
}

#[test]
fn snow_proxy_is_bounded_and_ice_or_cold_implies_snow() {
    let s = state();
    for sample in s.query.area_samples(0, 1).unwrap() {
        assert!((0.0..=1.0).contains(&sample.snow.fraction));
        let peak = sample.snow.peak_fraction.unwrap();
        assert!(peak >= sample.snow.fraction && peak <= 1.0);
        if sample.terrain != TerrainKindDto::Land {
            continue;
        }
        if sample.temperature_c <= -7.0 && sample.slope_deg < 35.0 {
            assert!(sample.snow.perennial);
        }
        if sample.height_m >= sample.snow.snowline_m + 1.0 && sample.slope_deg < 35.0 {
            assert!(sample.snow.fraction > 0.5);
        }
    }
}

#[test]
fn sample_point_matches_the_fine_file_reader_bilinearly() {
    let s = state();
    let world = arda::World::load(world_dir()).unwrap();
    let mut reader = world.fine_terrain(FINE_READER_BYTES).unwrap().unwrap();
    for (x_m, y_m) in [
        (51_234.5, 103_617.25),
        (0.0, 0.0),
        (102_300.0, 204_700.0),
        (7_777.777, 12_345.678),
    ] {
        let p = s.query.sample_point(x_m, y_m).unwrap();
        // World metres read the lattice through the I1 cell frame, clamped
        // to its coverage (the outer half cells).
        #[allow(clippy::cast_possible_truncation)]
        let point = TerrainPoint {
            x_um: (x_m * 1e6).round() as i64,
            y_um: (y_m * 1e6).round() as i64,
        }
        .fine_of_world();
        let step = i64::from(reader.spacing_um());
        let (xmax, ymax) = (
            i64::from(reader.width() - 1) * step,
            i64::from(reader.height() - 1) * step,
        );
        let point = TerrainPoint {
            x_um: point.x_um.clamp(0, xmax),
            y_um: point.y_um.clamp(0, ymax),
        };
        let expect = reader.sample(point).unwrap().unwrap();
        assert_eq!(p.height_m, f64::from(expect.raw()) / 1000.0, "{x_m},{y_m}");
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let containing = ((x_m / 100.0).floor() as u32, (y_m / 100.0).floor() as u32);
        assert_eq!((p.cell.gx, p.cell.gy), containing);
    }
    // At a cell centre the point height is the cell's fine centre.
    let at_centre = s.query.sample_point(30_050.0, 70_050.0).unwrap();
    assert_eq!((at_centre.cell.gx, at_centre.cell.gy), (300, 700));
    assert_eq!(
        Some(at_centre.height_m),
        at_centre.cell.fine.map(|f| f.centre_m)
    );
    // I1 frame: the fine surface at a land cell's centre is the stored
    // cell's own height, which the generator sampled from that lattice point.
    let mut agree = 0;
    for (gx, gy) in [
        (300, 700),
        (512, 1036),
        (450, 900),
        (600, 1200),
        (380, 1500),
    ] {
        let c = s.query.cell(gx, gy).unwrap();
        let centre = c.fine.map(|f| f.centre_m);
        if c.terrain == arda_server::contract::TerrainKindDto::Land && centre == Some(c.height_m) {
            agree += 1;
        }
    }
    assert!(
        agree >= 3,
        "{agree} of 5 cells agree with their fine centre"
    );
}

#[test]
fn cells_cover_their_hundred_metre_square_from_the_north_west_corner() {
    // Regression (vocabulary I1): x = 151 m used to round to cell 2.
    let s = state();
    for (x_m, gx) in [
        (0.0, 0),
        (99.999, 0),
        (100.0, 1),
        (149.0, 1),
        (151.0, 1),
        (199.9, 1),
    ] {
        let p = s.query.sample_point(x_m, 50.0).unwrap();
        assert_eq!(p.cell.gx, gx, "x = {x_m}");
        assert_eq!(p.cell.x_m, f64::from(gx) * 100.0);
        assert_eq!(p.cell.centre_x_m, f64::from(gx) * 100.0 + 50.0);
        assert_eq!(p.cell.centre_y_m, 50.0);
    }
    let world = arda::World::load(world_dir()).unwrap();
    let end_x = f64::from(world.manifest().areas_wide) * 512.0 * 100.0;
    let last = s.query.sample_point(end_x - 0.001, 50.0).unwrap();
    assert_eq!(f64::from(last.cell.gx), end_x / 100.0 - 1.0);
    assert!(s.query.sample_point(end_x, 50.0).is_err());
    let cell = s.query.cell(3, 4).unwrap();
    assert_eq!(cell.contract_version, arda_server::CONTRACT_VERSION);
    assert_eq!((cell.centre_x_m, cell.centre_y_m), (350.0, 450.0));
}

#[test]
fn area_and_cell_queries_agree() {
    let s = state();
    let samples = s.query.area_samples(0, 3).unwrap();
    for i in [0, 777, 131_072, 262_143] {
        let from_area = &samples[i];
        let from_cell = s.query.cell(from_area.gx, from_area.gy).unwrap();
        assert_eq!(from_area, &from_cell);
    }
}
