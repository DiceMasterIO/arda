//! River symbols on the overview: suppression at sea, tapering and blending
//! at 32K, saved widths, the classic trunk and the chamfer oracle.

use super::*;

#[test]
fn atlas_river_is_suppressed_where_the_contour_resolves_sea() {
    let mut cells = AreaCells::flat(Cell {
        terrain: TerrainKind::Sea,
        height: HeightMm::new(-100_000),
        ..Cell::default()
    });
    for y in 0..512 {
        for x in 101..512 {
            cells.set(
                CellCoord::new(x, y).unwrap(),
                Cell {
                    terrain: TerrainKind::Land,
                    height: HeightMm::new(1),
                    discharge: DischargeMilli::new(800_000),
                    ..Cell::default()
                },
            );
        }
    }
    let terrain = standalone_atlas(&cells);
    let (_, feature) = sample_pixel(&cells, Some(&terrain), 4096, 4096, 101 * 8, 100 * 8).unwrap();
    assert_eq!(feature, Feature::AtlasSea);
    let (_, classic) = sample_pixel(&cells, None, 4096, 4096, 101 * 8, 100 * 8).unwrap();
    assert_eq!(classic, Feature::River(RiverBand::Dark));
}

#[test]
fn atlas_river_symbols_taper_and_blend_at_32k_without_crossing_masks() {
    let width = 32_768usize;
    let data_y0 = 239u32;
    let data_y1 = 273u32;
    assert_eq!(
        atlas_river_radius(u32::try_from(width).unwrap(), 2048, 13),
        13
    );
    assert_eq!(
        atlas_river_radius(u32::try_from(width).unwrap(), 2048, 7),
        7
    );
    assert!(atlas_river_size(2_000, RiverBand::Dark) > atlas_river_size(360, RiverBand::Dark));
    let mut features = vec![Feature::AtlasLand; width * (data_y1 - data_y0) as usize];
    let source = (255 - data_y0) as usize * width;
    features[source + 100] = Feature::AtlasRiver(RiverBand::Dark, 13);
    features[source + 200] = Feature::AtlasRiver(RiverBand::Mid, 7);
    let guarded = (256 - data_y0) as usize * width;
    features[guarded + 101] = Feature::AtlasSea;
    features[guarded + 102] = Feature::Lake;
    features[guarded + 103] = Feature::Land;
    features[guarded + 104] = Feature::AtlasRiver(RiverBand::Light, 0);
    let mut rgb = vec![100; features.len() * 3];
    style_river_band(
        u32::try_from(width).unwrap(),
        2048,
        data_y0,
        239,
        273,
        &features,
        &mut rgb,
    )
    .unwrap();
    let at = |x: usize, y: usize| -> &[u8] {
        let i = ((y - data_y0 as usize) * width + x) * 3;
        &rgb[i..i + 3]
    };
    let dark = atlas_river_colour(RiverBand::Dark);
    assert_eq!(at(100, 255), &dark);
    assert!(at(112, 255)[0] > dark[0]);
    assert!(at(112, 255)[0] < atlas_river_edge_colour(RiverBand::Dark)[0]);
    assert_ne!(at(113, 255), &dark);
    assert_ne!(at(113, 255), &[100; 3]);
    assert_eq!(at(114, 255), &[100; 3]);
    assert_eq!(at(200, 255), &atlas_river_colour(RiverBand::Mid));
    assert_eq!(at(207, 255), &[90, 126, 135]);
    assert_eq!(at(208, 255), &[100; 3]);
    for x in [101, 102, 103] {
        assert_eq!(at(x, 256), &[100; 3], "protected surface at {x}");
    }
    assert_eq!(at(104, 256), &dark, "dark trunk wins over light stream");
}

#[test]
fn zero_width_atlas_fallback_saturates_extreme_discharge() {
    let cells = AreaCells::flat(Cell {
        terrain: TerrainKind::Land,
        discharge: DischargeMilli::new(u64::MAX),
        ..Cell::default()
    });
    let terrain = standalone_atlas(&cells);
    let (_, feature) = sample_pixel(&cells, Some(&terrain), 512, 512, 0, 0).unwrap();
    assert_eq!(feature, Feature::AtlasRiver(RiverBand::Dark, 13));
}

#[test]
fn saved_width_changes_coverage_in_a_direct_2k_overview() {
    let mut features = vec![Feature::AtlasLand; 32];
    features[5] = Feature::AtlasRiver(RiverBand::Mid, 5);
    features[20] = Feature::AtlasRiver(RiverBand::Mid, 7);
    let mut rgb = vec![100; 32 * 3];
    style_river_band(32, 2048, 0, 0, 1, &features, &mut rgb).unwrap();
    let narrow = &rgb[5 * 3..5 * 3 + 3];
    let wide = &rgb[20 * 3..20 * 3 + 3];
    let blue = atlas_river_colour(RiverBand::Mid);
    assert!(narrow[0] > wide[0]);
    assert!(wide[0] > blue[0]);
    assert_eq!(&rgb[2 * 3..2 * 3 + 3], &[100; 3]);
}

#[test]
fn classic_trunk_still_widens_only_right_and_down() {
    let mut features = vec![Feature::Land; 12];
    features[1] = Feature::River(RiverBand::Dark);
    let mut rgb = vec![100; 36];
    style_river_band(4, 3, 0, 0, 3, &features, &mut rgb).unwrap();
    let at = |x: usize, y: usize| -> &[u8] {
        let i = (y * 4 + x) * 3;
        &rgb[i..i + 3]
    };
    let dark = river_band_colour(RiverBand::Dark);
    assert_eq!(at(2, 0), &dark);
    assert_eq!(at(1, 1), &dark);
    assert_eq!(at(2, 1), &[100; 3]);
}

#[test]
fn tapered_symbols_match_chamfer_oracle_across_256_row_bands() {
    const W: usize = 64;
    const ROWS: usize = 528;
    let mut features = vec![Feature::AtlasLand; W * ROWS];
    features[255 * W + 24] = Feature::AtlasRiver(RiverBand::Dark, 13);
    features[257 * W + 48] = Feature::AtlasRiver(RiverBand::Mid, 7);
    features[256 * W + 25] = Feature::AtlasSea;
    features[258 * W + 24] = Feature::Lake;
    features[255 * W + 26] = Feature::AtlasRiver(RiverBand::Light, 0);
    let mut rgb = vec![100; W * ROWS * 3];
    for (data_y0, data_y1, output_y0, output_y1) in
        [(0usize, 270usize, 0u32, 256u32), (242, 528, 256, 512)]
    {
        style_river_band(
            u32::try_from(W).unwrap(),
            32_768,
            u32::try_from(data_y0).unwrap(),
            output_y0,
            output_y1,
            &features[data_y0 * W..data_y1 * W],
            &mut rgb[data_y0 * W * 3..data_y1 * W * 3],
        )
        .unwrap();
    }
    let chamfer = |x: usize, y: usize, sx: usize, sy: usize| {
        let dx = x.abs_diff(sx);
        let dy = y.abs_diff(sy);
        i16::try_from(3 * dx.max(dy) + dx.min(dy)).unwrap()
    };
    for y in 242..270 {
        for x in 0..W {
            let feature = features[y * W + x];
            let mut expected = [100u8; 3];
            for (band, source_x, source_y, radius) in
                [(RiverBand::Mid, 48, 257, 7), (RiverBand::Dark, 24, 255, 13)]
            {
                let eligible = match band {
                    RiverBand::Mid => matches!(
                        feature,
                        Feature::AtlasLand
                            | Feature::AtlasRiver(RiverBand::Light | RiverBand::Mid, _)
                    ),
                    RiverBand::Dark => {
                        matches!(feature, Feature::AtlasLand | Feature::AtlasRiver(_, _))
                    }
                    RiverBand::Light => false,
                };
                if !eligible {
                    continue;
                }
                let distance = chamfer(x, y, source_x, source_y) * 16 - radius * 48;
                let alpha =
                    u16::try_from(((24 - i32::from(distance)) * 256 / 48).clamp(0, 256)).unwrap();
                let colour = atlas_river_water_colour(band, distance, 16);
                for channel in 0..3 {
                    expected[channel] = ((u32::from(expected[channel]) * u32::from(256 - alpha)
                        + u32::from(colour[channel]) * u32::from(alpha)
                        + 128)
                        / 256)
                        .try_into()
                        .unwrap_or(255);
                }
            }
            assert_eq!(
                &rgb[(y * W + x) * 3..(y * W + x) * 3 + 3],
                &expected,
                "({x}, {y})"
            );
        }
    }
}
