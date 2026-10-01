use super::*;

fn integral(polygons: &[Polygon]) -> (f64, f64) {
    let mut total = 0_u64;
    let mut roundings = 0_u64;
    let mut touched = 0_u64;
    for y in 0..8 {
        for x in 0..8 {
            let c = coverage(
                polygons.iter().map(|p| (p, 1000)),
                x,
                y,
                &mut WorkBudget::new(250_000_000),
            )
            .unwrap();
            total += u64::from(c.alpha);
            roundings += c.roundings;
            touched += u64::from(c.alpha > 0);
        }
    }
    (
        total as f64 / 65_535.,
        4. * roundings as f64 / Q as f64 + touched as f64 / (2. * 65_535.),
    )
}

#[test]
fn terminal_point_numeric_area_and_union_have_no_fictitious_step() {
    for width in [Q / 125, Q / 10, Q, 2 * Q] {
        let p = terminal_footprint((4 * Q, 4 * Q), width).unwrap();
        assert_eq!(p.len(), 32);
        let exact = area2(&p) as f64 / (2. * Q as f64 * Q as f64);
        let circle = std::f64::consts::PI * (width as f64 / Q as f64).powi(2) / 4.;
        assert!(exact > circle * 0.9934 && exact < circle * 0.9938);
        let (covered, tolerance) = integral(std::slice::from_ref(&p));
        assert!((covered - exact).abs() <= tolerance);
        let (duplicate, duplicate_tolerance) = integral(&[p.clone(), p]);
        assert!((duplicate - exact).abs() <= duplicate_tolerance);
    }
    let p = terminal_footprint((4 * Q, 4 * Q), Q).unwrap();
    let edge = strip((4 * Q, 4 * Q), (6 * Q, 4 * Q), Q, Q).unwrap().0;
    let (union, tolerance) = integral(&[p.clone(), edge]);
    let disk = area2(&p) as f64 / (2. * Q as f64 * Q as f64);
    assert!((union - (2. + disk / 2.)).abs() <= tolerance);
    assert!(terminal_footprint((0, 0), 0).is_err());
    assert!(terminal_footprint((COORD_LIMIT, 0), Q).is_err());
}

#[test]
fn physical_subpixel_width_survives_without_a_visibility_mark() {
    for dm in [8, 10, 20, 100, 1000, 65_535, 230_650] {
        let width = dm * Q / 1000;
        let p = rectangle(Q, 4 * Q - width / 2, 7 * Q, 4 * Q + width / 2);
        let (got, tolerance) = integral(&[p]);
        let expected = 6. * (width as f64 / Q as f64).min(8.);
        assert!(got > 0.);
        assert!((got - expected).abs() <= tolerance + 0.00001);
    }
}

#[test]
fn variable_width_and_diagonal_area_match_physical_geometry() {
    for (wa, wb) in [(Q / 100, Q / 100), (Q / 10, Q / 2), (Q / 2, Q)] {
        for (b, length) in [((7 * Q, Q), 6.), ((7 * Q, 7 * Q), 6. * 2_f64.sqrt())] {
            let (p, _, _) = strip((Q, Q), b, wa, wb).unwrap();
            let exact = area2(&p) as f64 / (2. * Q as f64 * Q as f64);
            let physical = length * (wa + wb) as f64 / (2. * Q as f64);
            let (got, tolerance) = integral(&[p]);
            assert!((exact - physical).abs() < 0.00003);
            assert!((got - exact).abs() <= tolerance);
        }
    }
}

#[test]
fn widest_diagonal_channels_have_an_explicit_normal_approximation_bound() {
    let root_two = 2_f64.sqrt();
    let relative_normal_error = (root_two * 741_455. / Q as f64 - 1.).abs();
    for dm in [10_i64, 400, 65_535, 230_650] {
        for scale in [1_i64, 8] {
            let width = dm * scale * Q / 1000;
            let length = scale as f64 * root_two;
            let ideal = length * dm as f64 * scale as f64 / 1000.;
            let bound = ideal * relative_normal_error + (2. * root_two + 1.) * length / Q as f64;
            for sign in [-1_i64, 1] {
                let (polygon, _, _) =
                    strip((0, 0), (scale * Q, sign * scale * Q), width, width).unwrap();
                let actual = area2(&polygon) as f64 / (2. * Q as f64 * Q as f64);
                assert!(
                    (actual - ideal).abs() <= bound,
                    "width={dm}dm scale={scale} error={} bound={bound}",
                    (actual - ideal).abs()
                );
            }
        }
    }
}

#[test]
fn confluence_union_counts_overlap_once() {
    let a = rectangle(Q, 3 * Q, 7 * Q, 4 * Q);
    let b = rectangle(3 * Q, Q, 4 * Q, 7 * Q);
    let (got, tolerance) = integral(&[a.clone(), b.clone()]);
    assert!((got - 11.).abs() <= tolerance);
    assert_eq!(integral(&[b, a]).0, got);
    let (p, _, _) = strip((Q, Q), (7 * Q, 7 * Q), Q / 10, Q / 10).unwrap();
    assert_eq!(
        integral(std::slice::from_ref(&p)).0,
        integral(&[p.clone(), p]).0
    );
}

#[test]
fn diagonal_crossing_and_bevel_have_independent_analytic_areas() {
    let width = Q / 2;
    let (a, _, _) = strip((Q, Q), (7 * Q, 7 * Q), width, width).unwrap();
    let (b, _, _) = strip((Q, 7 * Q), (7 * Q, Q), width, width).unwrap();
    let (got, tolerance) = integral(&[a, b]);
    assert!((got - (6. * 2_f64.sqrt() - 0.25)).abs() <= tolerance + 0.00003);
    let (a, _, a_cap) = strip((Q, 4 * Q), (4 * Q, 4 * Q), width, width).unwrap();
    let (b, b_cap, _) = strip((4 * Q, 4 * Q), (4 * Q, 7 * Q), width, width).unwrap();
    let join = hull(a_cap.into_iter().chain(b_cap).collect()).unwrap();
    let (got, tolerance) = integral(&[a, b, join]);
    assert!((got - (3. - 0.25 / 8.)).abs() <= tolerance);
}

#[test]
fn seam_clips_partition_one_global_channel() {
    let (p, _, _) = strip((Q, Q), (7 * Q, 7 * Q), Q / 10, Q / 10).unwrap();
    let mut rounds = 0;
    let left = intersection(
        &p,
        &rectangle(0, 0, 4 * Q, 8 * Q),
        &mut rounds,
        &mut WorkBudget::new(250_000_000),
    )
    .unwrap();
    let right = intersection(
        &p,
        &rectangle(4 * Q, 0, 8 * Q, 8 * Q),
        &mut rounds,
        &mut WorkBudget::new(250_000_000),
    )
    .unwrap();
    assert_eq!(area2(&left) + area2(&right), area2(&p));
}

#[test]
fn colour_uses_only_geometry_that_intersects_the_pixel() {
    let low = rectangle(0, 0, Q / 2, Q);
    let absent = rectangle(2 * Q, 2 * Q, 3 * Q, 3 * Q);
    let c = coverage(
        [(&low, 1000), (&absent, 1_000_000)],
        0,
        0,
        &mut WorkBudget::new(250_000_000),
    )
    .unwrap();
    assert_eq!(c.maximum_discharge, 1000);
    assert_eq!(c.alpha, 32_768);
}

#[test]
fn last_32k_pixel_has_the_same_exact_coverage_as_the_origin() {
    let low = rectangle(0, 0, Q / 2, Q);
    let offset = 32_767 * Q;
    let high: Polygon = low.iter().map(|&(x, y)| (x + offset, y + offset)).collect();
    let base = coverage([(&low, 1000)], 0, 0, &mut WorkBudget::new(100_000)).unwrap();
    let last = coverage(
        [(&high, 1000)],
        32_767,
        32_767,
        &mut WorkBudget::new(100_000),
    )
    .unwrap();
    assert_eq!(last.alpha, 32_768);
    assert_eq!(last.alpha, base.alpha);
    assert_eq!(last.maximum_discharge, base.maximum_discharge);
    for (x, y) in [(32_768, 0), (0, 32_768), (u32::MAX, u32::MAX)] {
        assert!(coverage([(&high, 1000)], x, y, &mut WorkBudget::new(100_000)).is_err());
    }
}

#[test]
fn bounds_and_fragment_limits_are_errors() {
    let too_far = rectangle(COORD_LIMIT, 0, COORD_LIMIT + 1, Q);
    assert!(coverage([(&too_far, 1)], 0, 0, &mut WorkBudget::new(250_000_000)).is_err());
    assert!(strip((0, 0), (Q, Q / 2), Q, Q).is_err());
    let polygons: Vec<_> = (0..4)
        .map(|i| rectangle(i * Q / 4, 0, (i * 2 + 1) * Q / 8, Q))
        .collect();
    assert!(coverage_limited(
        polygons.iter().map(|p| (p, 1)),
        0,
        0,
        3,
        &mut WorkBudget::new(250_000_000)
    )
    .is_err());
    assert!(coverage_limited(
        polygons.iter().map(|p| (p, 1)),
        0,
        0,
        4,
        &mut WorkBudget::new(250_000_000)
    )
    .is_ok());
}
#[test]
fn exact_fraction_oracle_is_within_one_alpha_quantum() {
    #[derive(serde::Deserialize)]
    struct Case {
        polygons: Vec<Polygon>,
        nearest_alpha: u16,
        exact_area: f64,
    }
    let cases: Vec<Case> = serde_json::from_str(include_str!("../coverage_fixtures.json")).unwrap();
    assert_eq!(cases.len(), 96);
    for (i, case) in cases.iter().enumerate() {
        let c = coverage(
            case.polygons.iter().map(|p| (p, 0)),
            0,
            0,
            &mut WorkBudget::new(250_000_000),
        )
        .unwrap();
        assert!(c.alpha.abs_diff(case.nearest_alpha) <= 1, "fixture {i}");
        assert!(
            (f64::from(c.alpha) / 65_535. - case.exact_area).abs() <= 1. / 65_535.,
            "fixture {i}"
        );
    }
}

#[test]
fn disjoint_comparisons_stop_inside_the_coverage_budget() {
    let polygons: Vec<_> = (0..128)
        .map(|i| rectangle(i * Q / 128, 0, (i * 2 + 1) * Q / 256, Q))
        .collect();
    let mut work = WorkBudget::new(500);
    assert!(coverage(polygons.iter().map(|p| (p, 1)), 0, 0, &mut work).is_err());
    assert!(work.remaining < 100);
}
