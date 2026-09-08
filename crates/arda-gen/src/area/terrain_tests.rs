use super::mfd;

#[test]
fn analytic_fractional_area_incision() {
    let one = mfd::ONE;
    for (q, s, want) in [
        (one * 9 / 4, 60, 90),
        (one * 25 / 4, 60, 150),
        (one * 201 / 2, 90, 902),
        (one * 100, 90, 900),
        (one * 10000, 9, 900),
    ] {
        assert_eq!(mfd::incision(q, s, 1), want);
    }
}
#[test]
fn split_accumulation_conserves_and_repeats_on_oblique_and_curved_surfaces() {
    for field in [0, 1, 2] {
        let h: Vec<_> = (0..101 * 101)
            .map(|i| {
                let x = i % 101 - 50;
                let y = i / 101 - 50;
                match field {
                    0 => 100000 + x * 317 + y * 719,
                    1 => 100000 - x * x * 3 - y * y * 7,
                    _ => 100000 + x * x * 3 + y * y * 7,
                }
            })
            .collect();
        let a = mfd::accumulate_grid(&h, 101);
        let b = mfd::accumulate_grid(&h, 101);
        assert_eq!(a, b);
        assert!(a.iter().all(|&x| x <= 101 * 101 * mfd::ONE));
        // accumulate_grid asserts exact terminal contribution sum on every call.
    }
}
#[test]
fn exact_square_root_bounds_cover_full_area_domain() {
    for area in [0, 1, mfd::ONE, mfd::ONE * 262144] {
        let n = u128::from(area) << 32;
        let r = mfd::isqrt(n);
        assert!(r * r <= n);
        assert!((r + 1) * (r + 1) > n);
    }
}
