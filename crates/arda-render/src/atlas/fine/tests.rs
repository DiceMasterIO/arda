use arda_core::HeightMm;

use super::*;
use crate::atlas::SAMPLE_SIDE;

fn field(
    origin: TerrainPoint,
    spacing: u32,
    width: u32,
    height: u32,
    mut height_at: impl FnMut(u32, u32) -> i32,
) -> TerrainField {
    let heights = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| HeightMm::new(height_at(x, y)))
        .collect();
    TerrainField::new(origin, spacing, width, height, heights).unwrap()
}

fn all_land() -> (Vec<TerrainKind>, Vec<u8>, Vec<u8>, Vec<u8>) {
    (
        vec![TerrainKind::Land; SAMPLE_SIDE * SAMPLE_SIDE],
        vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
        vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
        vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
    )
}

#[test]
fn physical_plane_has_exact_height_and_equivalent_gradient() {
    let s = 39_062_500;
    let field = field(
        TerrainPoint {
            x_um: -2 * i64::from(s),
            y_um: -2 * i64::from(s),
        },
        s,
        5,
        5,
        |x, y| {
            1_500_000 + (i32::try_from(x).unwrap() - 2) * 5_000
                - (i32::try_from(y).unwrap() - 2) * 2_500
        },
    );
    let atlas = FineAtlas {
        formed: false,
        v6: true,
        field,
        origin_x: 0,
        origin_y: 0,
        bounds: AtlasFineWorldBounds {
            min: TerrainPoint {
                x_um: -2 * i64::from(s),
                y_um: -2 * i64::from(s),
            },
            max: TerrainPoint {
                x_um: 2 * i64::from(s),
                y_um: 2 * i64::from(s),
            },
        },
        edges: [false; 4],
        sky: None,
    };
    assert_eq!(atlas.height(0, 0).unwrap(), 1_500_000);
    let expected = (
        land_material_ecology(1_500_000, 25_600, -12_800, 0, 0, 0),
        relief_light(25_600, -12_800),
    );
    assert_eq!(
        atlas.material(0, 0, SavedMaterial::default()).unwrap(),
        expected
    );
    let (classes, wetness, moisture, canopy) = all_land();
    assert_eq!(
        atlas
            .sample(
                AxisKernel::Linear {
                    low: 0,
                    high_weight: 0,
                    denominator: 2
                },
                AxisKernel::Linear {
                    low: 0,
                    high_weight: 0,
                    denominator: 2
                },
                &classes,
                &wetness,
                &moisture,
                &canopy,
            )
            .unwrap(),
        modulate(expected.0, expected.1)
    );
}

#[test]
fn overlapping_area_queries_share_exact_seam_radiance() {
    let s = 100_000_000;
    let source = field(
        TerrainPoint { x_um: -s, y_um: -s },
        u32::try_from(s).unwrap(),
        1_027,
        3,
        |x, y| 1_000_000 + i32::try_from(x).unwrap() * 1_000 + i32::try_from(y).unwrap() * 2_000,
    );
    let bounds = AtlasFineWorldBounds {
        min: source.origin(),
        max: TerrainPoint {
            x_um: -s + 1_026 * s,
            y_um: s,
        },
    };
    let left = FineAtlas {
        formed: false,
        v6: true,
        field: source.clone(),
        origin_x: 0,
        origin_y: 0,
        bounds,
        edges: [false; 4],
        sky: None,
    };
    let right = FineAtlas {
        formed: false,
        v6: true,
        field: source,
        origin_x: AREA_UM,
        origin_y: 0,
        bounds,
        edges: [false; 4],
        sky: None,
    };
    let (classes, wetness, moisture, canopy) = all_land();
    let x_left = AxisKernel::Linear {
        low: 511,
        high_weight: 1,
        denominator: 2,
    };
    let x_right = AxisKernel::Linear {
        low: -1,
        high_weight: 1,
        denominator: 2,
    };
    let y = AxisKernel::Linear {
        low: 0,
        high_weight: 0,
        denominator: 2,
    };
    assert_eq!(
        left.sample(x_left, y, &classes, &wetness, &moisture, &canopy)
            .unwrap(),
        right
            .sample(x_right, y, &classes, &wetness, &moisture, &canopy)
            .unwrap()
    );
}

#[test]
fn fine_high_frequency_changes_box_and_linear_samples() {
    let s = 50_000_000;
    let source = field(
        TerrainPoint {
            x_um: -2 * s,
            y_um: -2 * s,
        },
        u32::try_from(s).unwrap(),
        8,
        8,
        |x, _| if x % 2 == 0 { 1_600_000 } else { 1_000_000 },
    );
    let atlas = FineAtlas {
        formed: false,
        v6: true,
        field: source,
        origin_x: 0,
        origin_y: 0,
        bounds: AtlasFineWorldBounds {
            min: TerrainPoint {
                x_um: -2 * s,
                y_um: -2 * s,
            },
            max: TerrainPoint {
                x_um: 5 * s,
                y_um: 5 * s,
            },
        },
        edges: [false; 4],
        sky: None,
    };
    let (classes, wetness, moisture, canopy) = all_land();
    let y = AxisKernel::Linear {
        low: 0,
        high_weight: 0,
        denominator: 2,
    };
    let center = atlas
        .sample(
            AxisKernel::Linear {
                low: 0,
                high_weight: 0,
                denominator: 2,
            },
            y,
            &classes,
            &wetness,
            &moisture,
            &canopy,
        )
        .unwrap();
    let half = atlas
        .sample(
            AxisKernel::Linear {
                low: 0,
                high_weight: 1,
                denominator: 2,
            },
            y,
            &classes,
            &wetness,
            &moisture,
            &canopy,
        )
        .unwrap();
    assert_ne!(center, half);
    let box_color = atlas
        .sample(
            AxisKernel::Box { start: 0, end: 1 },
            y,
            &classes,
            &wetness,
            &moisture,
            &canopy,
        )
        .unwrap();
    assert_ne!(box_color, center);
    assert_ne!(box_color, half);
}

#[test]
fn only_a_true_world_edge_clamps_outside_source_queries() {
    let source = field(
        TerrainPoint { x_um: 0, y_um: 0 },
        50_000_000,
        3,
        3,
        |x, y| 1_000_000 + i32::try_from(x + y).unwrap() * 1_000,
    );
    let bounds = AtlasFineWorldBounds {
        min: TerrainPoint { x_um: 0, y_um: 0 },
        max: TerrainPoint {
            x_um: 100_000_000,
            y_um: 100_000_000,
        },
    };
    let mut atlas = FineAtlas {
        formed: false,
        v6: true,
        field: source,
        origin_x: 0,
        origin_y: 0,
        bounds,
        edges: [false; 4],
        sky: None,
    };
    assert!(atlas.height(-1, 50_000_000).is_err());
    atlas.edges[3] = true;
    assert_eq!(
        atlas.height(-1, 50_000_000).unwrap(),
        atlas.height(0, 50_000_000).unwrap()
    );
    assert!(atlas.height(50_000_000, -1).is_err());
}

#[test]
fn flat_world_has_exactly_unity_sky_light() {
    let context = field(
        TerrainPoint {
            x_um: -10_000_000_000,
            y_um: -10_000_000_000,
        },
        1_000_000_000,
        80,
        80,
        |_, _| 1_000_000,
    );
    let sky = SkyShade::new(AreaCoord::new(0, 0), &context).unwrap();
    assert!(sky.light_q12.iter().all(|&light| light == 4_096));
    assert_eq!(
        sky.sample_q12(25_200_000_000, 25_200_000_000).unwrap(),
        4_096
    );
    let bounds = AtlasFineWorldBounds {
        min: context.origin(),
        max: TerrainPoint {
            x_um: 69_000_000_000,
            y_um: 69_000_000_000,
        },
    };
    let unshaded = FineAtlas::new(
        AreaCoord::new(0, 0),
        bounds,
        context.clone(),
        [false; 4],
        None,
        false,
    )
    .unwrap();
    let shaded = FineAtlas::new(
        AreaCoord::new(0, 0),
        bounds,
        context.clone(),
        [false; 4],
        Some(context),
        false,
    )
    .unwrap();
    assert_eq!(
        shaded
            .material(25_200_000_000, 25_200_000_000, SavedMaterial::default())
            .unwrap(),
        unshaded
            .material(25_200_000_000, 25_200_000_000, SavedMaterial::default())
            .unwrap(),
    );
}

#[test]
fn floodplain_meanders_swing_across_the_valley_and_stay_bounded() {
    // A floodplain falling 1 m per km to the north: down-valley is -y.
    let source = field(
        TerrainPoint {
            x_um: -10_000_000_000,
            y_um: -10_000_000_000,
        },
        1_000_000_000,
        80,
        80,
        |_, y| 100_000 + i32::try_from(y).unwrap() * 1_000,
    );
    let bounds = AtlasFineWorldBounds {
        min: source.origin(),
        max: TerrainPoint {
            x_um: 69_000_000_000,
            y_um: 69_000_000_000,
        },
    };
    let atlas = FineAtlas::new(
        AreaCoord::new(0, 0),
        bounds,
        source.clone(),
        [false; 4],
        Some(source),
        true,
    )
    .unwrap();
    let width_dm = 1_000; // 100 m river: wavelength 1.1 km
    let lambda = 1_100_000_000_i128;
    let at = |y: i128| {
        atlas
            .meander_offset_um(25_000_000_000, y, width_dm)
            .unwrap()
    };
    let a = at(25_000_000_000 + lambda / 4);
    let b = at(25_000_000_000 + 3 * lambda / 4);
    // Offsets lie across the valley (x) and flip sign half a wave on.
    assert!(a.0.signum() == -b.0.signum() && a.0 != 0, "{a:?} {b:?}");
    assert!(
        a.1.abs() < 1_000_000 && b.1.abs() < 1_000_000,
        "across, not along"
    );
    let amp = i64::try_from(lambda / 4).unwrap();
    assert!(a.0.abs() <= amp && b.0.abs() <= amp);
    // Narrow streams do not meander at 100 m vertex spacing.
    assert!(atlas
        .meander_offset_um(25_000_000_000, 25_000_000_000, 300)
        .is_none());
}

#[test]
fn kilometer_horizon_darks_a_valley_without_darking_its_surrounding_ridge() {
    let context = field(
        TerrainPoint {
            x_um: -10_000_000_000,
            y_um: -10_000_000_000,
        },
        1_000_000_000,
        80,
        80,
        |x, y| {
            let gx = i64::from(x) - 10;
            let gy = i64::from(y) - 10;
            let r2 = (gx - 25).pow(2) + (gy - 25).pow(2);
            if (9..=36).contains(&r2) {
                2_000_000
            } else {
                1_000_000
            }
        },
    );
    let sky = SkyShade::new(AreaCoord::new(0, 0), &context).unwrap();
    let valley = sky.sample_q12(25_000_000_000, 25_000_000_000).unwrap();
    let ridge = sky.sample_q12(29_000_000_000, 25_000_000_000).unwrap();
    assert!(
        valley < 3_700,
        "valley received too little broad shade: {valley}"
    );
    assert_eq!(ridge, 4_096);
}

#[test]
fn globally_aligned_sky_context_has_equal_radiance_at_an_area_seam() {
    let source = field(
        TerrainPoint {
            x_um: -10_000_000_000,
            y_um: -10_000_000_000,
        },
        1_000_000_000,
        130,
        80,
        |x, y| {
            let gx = i64::from(x) - 10;
            let gy = i64::from(y) - 10;
            let distance = (gx - 54).abs() + (gy - 25).abs();
            1_000_000 + i32::try_from((1_500_000 - distance * 250_000).max(0)).unwrap()
        },
    );
    let bounds = AtlasFineWorldBounds {
        min: source.origin(),
        max: TerrainPoint {
            x_um: 119_000_000_000,
            y_um: 69_000_000_000,
        },
    };
    let left = FineAtlas::new(
        AreaCoord::new(0, 0),
        bounds,
        source.clone(),
        [false; 4],
        Some(source.clone()),
        false,
    )
    .unwrap();
    let right = FineAtlas::new(
        AreaCoord::new(1, 0),
        bounds,
        source.clone(),
        [false; 4],
        Some(source),
        false,
    )
    .unwrap();
    let seam = (AREA_UM, 25_000_000_000_i128);
    let left_sky = left
        .sky
        .as_ref()
        .unwrap()
        .sample_q12(seam.0, seam.1)
        .unwrap();
    let right_sky = right
        .sky
        .as_ref()
        .unwrap()
        .sample_q12(seam.0, seam.1)
        .unwrap();
    assert!(left_sky < 4_096);
    assert_eq!(left_sky, right_sky);
    assert_eq!(
        left.material(seam.0, seam.1, SavedMaterial::default())
            .unwrap(),
        right
            .material(seam.0, seam.1, SavedMaterial::default())
            .unwrap(),
    );
}
