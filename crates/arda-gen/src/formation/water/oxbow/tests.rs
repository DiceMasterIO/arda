use super::super::super::drainage::{fill, neighbour, open_sea_flags, FIXED};
use super::super::super::sampled::{drain_sampled, sample};
use super::*;

const D: i64 = 39_062_500;

/// Closed 100 m depressions (sizes) after the drainage guarantees that
/// follow carving: a fine fill and the sampled pass, both protecting the
/// recorded sinks.
fn lakes_after_guarantees(g: &mut Lattice, f: &mut WaterFeatures) -> Vec<usize> {
    let (w, h) = (g.width, g.height);
    f.index();
    let mut flags = vec![0_u8; w * h];
    open_sea_flags(&g.z, w, h, &mut flags);
    for (i, fl) in flags.iter_mut().enumerate() {
        if f.is_sink((i % w) as i64 * D, (i / w) as i64 * D) {
            *fl = FIXED;
        }
    }
    let (mut next, mut closed) = (vec![0; w * h], vec![0; w * h]);
    fill(&mut g.z, w, h, &flags, 1, &mut next, &mut closed).unwrap();
    let sink = |x: i64, y: i64| f.is_sink(x, y);
    drain_sampled(g, 100_000_000, &sink).unwrap();
    let s = 100_000_000_i128;
    let (sw, sh) = (
        ((w as i128 - 1) * i128::from(D) / s) as usize + 1,
        ((h as i128 - 1) * i128::from(D) / s) as usize + 1,
    );
    let z: Vec<i32> = (0..sw * sh)
        .map(|i| sample(g, (i % sw) as i128 * s, (i / sw) as i128 * s))
        .collect();
    let mut sflags = vec![0_u8; sw * sh];
    open_sea_flags(&z, sw, sh, &mut sflags);
    let mut filled = z.clone();
    let (mut next, mut closed) = (vec![0; sw * sh], vec![0; sw * sh]);
    fill(&mut filled, sw, sh, &sflags, 0, &mut next, &mut closed).unwrap();
    let mut seen = vec![false; sw * sh];
    let mut sizes = Vec::new();
    for s0 in 0..sw * sh {
        if seen[s0] || filled[s0] <= z[s0] {
            continue;
        }
        let (mut stack, mut n) = (vec![s0], 0);
        seen[s0] = true;
        while let Some(c) = stack.pop() {
            n += 1;
            for k in 0..8 {
                if let Some(nb) = neighbour(c, sw, sh, k) {
                    if !seen[nb] && filled[nb] > z[nb] {
                        seen[nb] = true;
                        stack.push(nb);
                    }
                }
            }
        }
        sizes.push(n);
    }
    sizes
}

/// A floodplain pad 10 m above a sea at the west edge, rising 3‰ away
/// from a belt axis at the west (the planed pad the loop lies on), with
/// one planned cutoff crescent of radius `r_m` beyond a bend, its tips
/// pointing back west towards the course.
fn plain_with_cutoff(r_m: i64, centre: (i64, i64)) -> (Lattice, Cutoff) {
    let (w, h) = (160, 160);
    let mut g = Lattice::new(w, h, D).unwrap();
    for y in 0..h {
        for x in 0..w {
            g.z[y * w + x] = if x == 0 {
                -5_000
            } else {
                10_000 + x as i32 * 117
            };
        }
    }
    let radius = m_to_q8(&g, r_m);
    let steps = (radius * 4 / (CELL_Q8 / 2)).max(8);
    let c = (centre.0 * CELL_Q8, centre.1 * CELL_Q8);
    let arc = (0..=steps)
        .map(|k| {
            let d = rotate((ONE_Q14, 0), -TURN / 3 + (TURN * 2 / 3) * k / steps);
            (c.0 + d.0 * radius / ONE_Q14, c.1 + d.1 * radius / ONE_Q14)
        })
        .collect();
    let cut = Cutoff {
        arc,
        radius: m_to_q8(&g, 60),
        depth: 1_500,
        river_cells: 1_000_000,
    };
    (g, cut)
}

#[test]
fn a_cutoff_is_one_whole_lake_or_none_after_the_guarantees() {
    // Regression (seed-42 full size: 59 lakes of 1-3 cells, 62 of the
    // 63 smallest being oxbows): with only a 75 m mid-arc disc
    // protected, the fills and the 100 m sampled pass lifted every part
    // of a crescent that did not drain to that point at 100 m.
    for (k, &(r_m, cx, cy)) in [(160, 80, 80), (200, 77, 83), (240, 81, 76), (180, 79, 79)]
        .iter()
        .enumerate()
    {
        let (mut g, cut) = plain_with_cutoff(r_m, (cx, cy));
        let mut f = WaterFeatures::default();
        carve_cutoffs(&mut g, std::slice::from_ref(&cut), &mut f, None).unwrap();
        let sizes = lakes_after_guarantees(&mut g, &mut f);
        if f.stats.oxbows == 0 {
            assert!(
                sizes.is_empty(),
                "case {k}: an undone loop leaves {sizes:?}"
            );
            continue;
        }
        assert_eq!(
            sizes.len(),
            1,
            "case {k}: one lake, not fragments: {sizes:?}"
        );
        assert!(sizes[0] >= MIN_SAMPLES, "case {k}: {sizes:?}");
    }
}

#[test]
fn a_loop_breached_by_a_lower_channel_is_undone() {
    // Regression (seed-42 full size: five bed pits): a loop cut across a
    // delta distributary was protected as a sink but drained, a dry flat.
    let (mut g, cut) = plain_with_cutoff(200, (80, 80));
    let w = g.width;
    // A channel far below the pad runs west to the sea just north of
    // the loop, off its arc: the loop's 100 m rim is breached.
    for y in 69..72 {
        for x in 1..120 {
            g.z[y * w + x] = 1_000 + x as i32;
        }
    }
    let mut f = WaterFeatures::default();
    carve_cutoffs(&mut g, std::slice::from_ref(&cut), &mut f, None).unwrap();
    assert_eq!(f.stats.oxbows, 0, "a breached loop holds no lake");
    assert!(f.sinks.is_empty());
}
