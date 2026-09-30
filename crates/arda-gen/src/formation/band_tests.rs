//! Regression: a broad macro crest must be dissected like its flanks, not
//! left as a straight, smooth, uneroded band (logic/02 §fine-formation
//! masks). Seed-42 MICRO carried one along its main north-south divide at
//! about 70% of the width: crest roughness was a quarter of its flanks'.

use arda_core::GenerateConfig;

use super::*;
use crate::continent::generate_continent_attempt_formed;

/// 4x4 block means (~156 m): valleys and spurs, not the 39 m texture.
fn blocks(g: &Lattice) -> (Vec<i64>, usize, usize) {
    let b = 4;
    let (bw, bh) = (g.width / b, g.height / b);
    let mut m = vec![0_i64; bw * bh];
    for (i, v) in m.iter_mut().enumerate() {
        let (bx, by) = (i % bw, i / bw);
        for dy in 0..b {
            for dx in 0..b {
                *v += i64::from(g.z[(by * b + dy) * g.width + bx * b + dx]);
            }
        }
        *v /= (b * b) as i64;
    }
    (m, bw, bh)
}

/// Roughness (mean |Laplacian| of the block means) within 1.5 km of each
/// row's crest, and 4-12 km either side of it. The crest of a row is the
/// highest 2 km running mean between the permille columns `x0..x1`; rows
/// run over the permille range `y0..y1`.
fn crest_and_flank_roughness(g: &Lattice, x: (usize, usize), y: (usize, usize)) -> (i64, i64) {
    let (m, bw, bh) = blocks(g);
    // Block columns per km (6.4 at 156.25 m).
    let per_km = |km: usize| km * 1_000_000_000 / (FINE_SPACING_UM as usize * 4);
    let (mut crest, mut nc, mut flank, mut nf) = (0_i64, 0_i64, 0_i64, 0_i64);
    for by in bh * y.0 / 1000..bh * y.1 / 1000 {
        let row = &m[by * bw..(by + 1) * bw];
        let r = per_km(1);
        let xc = (bw * x.0 / 1000..bw * x.1 / 1000)
            .max_by_key(|&x| row[x - r..=x + r].iter().sum::<i64>())
            .unwrap();
        let (lo, hi) = (
            xc.saturating_sub(per_km(12)).max(1),
            (xc + per_km(12)).min(bw - 2),
        );
        for x in lo..=hi {
            let i = by * bw + x;
            let lap = (m[i - 1] + m[i + 1] + m[i - bw] + m[i + bw] - 4 * m[i]).abs();
            let off = x.abs_diff(xc);
            if off * 2 <= per_km(3) {
                crest += lap;
                nc += 1;
            } else if off >= per_km(4) {
                flank += lap;
                nf += 1;
            }
        }
    }
    (crest / nc.max(1), flank / nf.max(1))
}

#[test]
fn the_seed_42_main_divide_is_dissected_like_its_flanks() {
    let config = GenerateConfig::MICRO;
    let grid = generate_continent_attempt_formed(42, config, 0);
    let g = form(42, 0, &grid, 2_620, 5_242, u128::MAX).unwrap();
    // The divide between 60% and 80% of the width, over 55-85% of the
    // height: before the belt-relief masks its crest roughness was 0.24 of
    // its flanks' (a smooth band ~4 km wide); dissected, it is ~0.55.
    let (crest, flank) = crest_and_flank_roughness(&g, (600, 800), (550, 850));
    assert!(
        crest * 100 >= flank * 40,
        "smooth uneroded band along the divide: roughness {crest} vs flanks {flank}"
    );
}
