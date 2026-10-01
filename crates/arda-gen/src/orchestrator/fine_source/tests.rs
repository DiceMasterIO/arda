use super::*;
use crate::continent::generate_continent_attempt_fine;
use crate::spectral_composition::{forced_rim_taper_q32, FULL_GAIN_Q32};
use arda_core::{LatitudeBand, SizeKm};

fn config(width: u32, height: u32) -> GenerateConfig {
    GenerateConfig::new(SizeKm::new(width, height), LatitudeBand::new(35, 55), 15).unwrap()
}
fn limits() -> FineSourceLimits {
    FineSourceLimits {
        max_ram_bytes: u128::MAX,
        max_file_bytes: u128::MAX,
    }
}

#[test]
fn closed_world_geometry_includes_modeled_fringe_and_rejects_unsupported_axes() {
    let small = admit(config(64, 64), limits()).unwrap();
    assert_eq!((small.fine_width, small.fine_height), (1640, 1640));
    assert_eq!((small.fft_width, small.fft_height), (2048, 2048));
    assert!(small.sampled_last_x_um >= small.nominal_width_um);
    assert!(small.sampled_last_x_um - small.nominal_width_um < u64::from(FINE_SPACING_UM));
    let fringe = admit(config(102, 102), limits()).unwrap();
    assert_eq!(fringe.nominal_width_um, 102_000_000_000);
    assert_eq!(fringe.covered_width_um, 102_300_000_000);
    assert!(fringe.sampled_last_x_um >= fringe.covered_width_um);
    assert!(matches!(
        admit(config(4000, 4000), limits()),
        Err(FineSourceError::SpectralExtent)
    ));
}

#[test]
fn both_resource_envelopes_fail_before_generation() {
    let c = config(64, 64);
    let full = admit(c, limits()).unwrap();
    assert!(matches!(
        admit(
            c,
            FineSourceLimits {
                max_ram_bytes: full.admitted_peak_ram_bytes - 1,
                max_file_bytes: u128::MAX
            }
        ),
        Err(FineSourceError::ResourceLimit {
            resource: "RAM",
            ..
        })
    ));
    assert!(matches!(
        admit(
            c,
            FineSourceLimits {
                max_ram_bytes: u128::MAX,
                max_file_bytes: full.file_bytes - 1
            }
        ),
        Err(FineSourceError::ResourceLimit {
            resource: "file",
            ..
        })
    ));
    assert_eq!(
        full.file_bytes,
        88 + u128::from(full.fine_width) * u128::from(full.fine_height) * 4
    );
}

#[test]
fn cached_macro_nodes_match_qualified_100m_stencil() {
    let c = config(64, 64);
    let grid = generate_continent_attempt_fine(42, c, 0);
    let (macro_field, range_field) = macro_pair(&grid, 137, 642).unwrap();
    for &x in &[0_i32, 1, 123, 640] {
        for &y in &[137_i32, 138] {
            let point = TerrainPoint {
                x_um: i64::from(x) * MACRO_SPACING_UM,
                y_um: i64::from(y) * MACRO_SPACING_UM,
            };
            let base = HeightMm::new(coarse_height(&grid, x, y));
            let neighbors =
                OFFSETS.map(|(dx, dy)| HeightMm::new(coarse_height(&grid, x + dx, y + dy)));
            assert_eq!(macro_field.sample(point), Some(base));
            let low = neighbors
                .iter()
                .map(|h| h.raw())
                .chain(std::iter::once(base.raw()))
                .min()
                .unwrap();
            let high = neighbors
                .iter()
                .map(|h| h.raw())
                .chain(std::iter::once(base.raw()))
                .max()
                .unwrap();
            assert_eq!(
                range_field.sample(point).map(HeightMm::raw),
                Some((high - low).min(i32::try_from(FULL_RANGE_MM).unwrap()))
            );
        }
    }
}

#[test]
fn actual_macro_rim_remains_ocean_on_four_sides_and_fringe() {
    let c = config(64, 64);
    let grid = generate_continent_attempt_fine(42, c, 0);
    let plan = admit(c, limits()).unwrap();
    let macro_w = u32::try_from(
        covered_cells(
            (u64::from(plan.fine_width) - 1) * u64::from(FINE_SPACING_UM),
            100_000_000,
        )
        .unwrap(),
    )
    .unwrap();
    let edges = [
        0,
        1,
        plan.fine_width / 2,
        plan.fine_width - 2,
        plan.fine_width - 1,
    ];
    for fy in [
        0,
        1,
        plan.fine_height / 2,
        plan.fine_height - 2,
        plan.fine_height - 1,
    ] {
        let py = i64::from(fy) * i64::from(FINE_SPACING_UM);
        let my = u32::try_from(py / MACRO_SPACING_UM).unwrap();
        let (macro_field, range_field) = macro_pair(&grid, my, macro_w).unwrap();
        for fx in edges {
            let point = TerrainPoint {
                x_um: i64::from(fx) * i64::from(FINE_SPACING_UM),
                y_um: py,
            };
            let taper = forced_rim_taper_q32(point, 64, 64);
            if fx == plan.fine_width / 2 && fy == plan.fine_height / 2 {
                assert_eq!(taper, FULL_GAIN_Q32);
                continue;
            }
            assert_eq!(taper, 0, "point {fx},{fy} escaped the forced rim");
            let macro_height = macro_field.sample(point).unwrap();
            assert!(macro_height.raw() < 0, "macro forced rim became land");
            let range = u32::try_from(range_field.sample(point).unwrap().raw()).unwrap();
            assert_eq!(
                compose_smooth_relief(macro_height, HeightMm::new(1_500_000), range, taper),
                Ok(macro_height)
            );
        }
    }
}
