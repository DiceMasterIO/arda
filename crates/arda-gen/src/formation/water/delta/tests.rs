use super::*;

#[test]
fn a_river_in_a_drowned_valley_builds_land_beyond_the_coast() {
    // Low coast at y = 100 (macro), sea shelf 5-30 m deep beyond; a
    // drowned valley reaches 30 cells inland; the river drains a 400 km²
    // equivalent through it (a wide sloping plain funnels to x = 128).
    let (w, h) = (257, 260);
    let mut g = Lattice::new(w, h, 39_062_500).unwrap();
    let mut macro_mm = vec![0; w * h];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            macro_mm[i] = if y >= 100 { -20_000 } else { 10_000 };
            let across = (x as i32 - 128).abs() * 400;
            g.z[i] = if y >= 100 {
                -5_000 - (y as i32 - 100) * 200
            } else if y >= 70 && (x as i32 - 128).abs() <= 2 {
                -3_000
            } else if y >= 90 {
                // A coastal ridge: the plain drains only by the valley.
                60_000
            } else {
                2_000 + (100 - y as i32) * 300 + across
            };
            if x == 0 || x == w - 1 || y == h - 1 {
                g.z[i] = -50_000;
            }
        }
    }
    let relief = vec![0_u8; w * h];
    let mut f = WaterFeatures::default();
    // Scale the threshold: this test lattice holds ~8,000 cells above
    // the coast; use a coarse spacing so that is > 150 km².
    g.spacing_um = 156_250_000;
    build(&mut g, &macro_mm, &relief, 9, &mut f, None).unwrap();
    assert!(f.stats.deltas >= 1, "{:?}", f.stats);
    // The valley head is back-filled and land now stands beyond the
    // macro coast.
    assert_eq!(f.deltas.len(), 1, "one delta per ria: {:?}", f.deltas);
    assert!(g.z[95 * w + 128] > 0, "bayhead plain");
    let beyond = (101 * w..(h - 1) * w).filter(|&i| g.z[i] > 0).count();
    assert!(
        beyond >= 25,
        "{beyond} fan cells beyond the coast: {:?}",
        f.deltas
    );
}

#[test]
fn large_rivers_build_deltas_on_moderate_coasts() {
    assert_eq!(coast_limit_q8(150), LOW_COAST_Q8);
    assert_eq!(coast_limit_q8(1_000), LOW_COAST_Q8);
    assert_eq!(coast_limit_q8(1_500), 130);
    assert_eq!(coast_limit_q8(2_000), MODERATE_COAST_Q8);
    // Seed-7 MICRO: a 4,479 km² river at relief 130/255 built none.
    assert!(130 < coast_limit_q8(4_479));
}

#[test]
fn a_large_river_splits_its_delta_into_islands() {
    // Ported from the superseded coast delta rule: a funnel of 600 x 560
    // nodes (~1,150 km² at 58.6 m) draining to one mouth at (300, 559);
    // a 5 m shelf south of y = 560.
    let (w, h) = (600, 900);
    let mut g = Lattice::new(w, h, 58_593_750).unwrap();
    let mut macro_mm = vec![0; w * h];
    for y in 0..h {
        for x in 0..w {
            let across = (x as i32 - 300).abs();
            let i = y * w + x;
            macro_mm[i] = if y >= 560 { -5_000 } else { 10_000 };
            g.z[i] = if y >= 560 {
                -5_000
            } else {
                2_000 + (560 - y as i32) * 20 + across * 60
            };
            if x == 0 || x == w - 1 || y == h - 1 {
                g.z[i] = -50_000;
            }
        }
    }
    let relief = vec![0_u8; w * h];
    let mut f = WaterFeatures::default();
    build(&mut g, &macro_mm, &relief, 9, &mut f, None).unwrap();
    assert_eq!(f.stats.deltas, 1, "{:?}", f.stats);
    assert_eq!(f.stats.delta_islands, 1, "{:?}", f.stats);
    // Land components entirely beyond the coast: the island wedge.
    let mut seen = vec![false; w * h];
    let mut islands = 0;
    for s in 561 * w..(h - 1) * w {
        if g.z[s] <= 0 || seen[s] {
            continue;
        }
        let (mut stack, mut touches) = (vec![s], false);
        seen[s] = true;
        while let Some(c) = stack.pop() {
            touches |= c / w <= 560;
            for k in 0..8 {
                if let Some(nb) = neighbour(c, w, h, k) {
                    if g.z[nb] > 0 && !seen[nb] {
                        seen[nb] = true;
                        stack.push(nb);
                    }
                }
            }
        }
        islands += usize::from(!touches);
    }
    assert!(islands >= 1, "no delta island");
    // Regression (seed-42 full size: dead-flat, planar delta plains):
    // levees and basins give the plain relief of a metre or so at the
    // 250 m scale, most of it below 3 m, and the front is lobate, so no
    // fan node is more than 1.5 m from the radial cone alone.
    let plain: Vec<usize> = (561 * w..(h - 1) * w)
        .filter(|&i| g.z[i] > 0 && g.z[i] < 6_000)
        .collect();
    assert!(plain.len() > 2_000, "fan land {}", plain.len());
    let rough = plain
        .iter()
        .filter(|&&i| {
            let (x, y) = (i % w, i / w);
            let mut lo = i32::MAX;
            let mut hi = i32::MIN;
            for yy in y - 2..=y + 2 {
                for xx in x - 2..=x + 2 {
                    let v = g.z[yy * w + xx];
                    if v > 0 {
                        lo = lo.min(v);
                        hi = hi.max(v);
                    }
                }
            }
            hi - lo >= 400
        })
        .count();
    assert!(
        rough * 10 >= plain.len(),
        "levee relief on {rough} of {} plain nodes",
        plain.len()
    );
    // The domain rim stays open sea (3 km, 51 nodes here).
    assert!(
        (0..w).all(|x| (h - 52..h).all(|y| g.z[y * w + x] <= 0)),
        "delta land at the rim"
    );
}
