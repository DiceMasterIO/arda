//! Behaviour tests for the world-water shaping passes on synthetic valleys
//! (logic/02 §world-water).

use super::super::drainage::{fill, neighbour, open_sea_flags, receiver_index, receivers};
use super::super::lattice::Lattice;
use super::*;

const D: i64 = 39_062_500;

/// A valley draining east into the sea: a flat floor `floor_half` nodes
/// either side of `y0`, sides rising `side_mm` per node, and the floor
/// falling `fall_mm` per node towards the sea at the last columns.
fn valley(
    w: usize,
    h: usize,
    floor_half: i64,
    side_mm: i64,
    fall_mm: i64,
    base_mm: i64,
) -> Lattice {
    let mut g = Lattice::new(w, h, D).unwrap();
    let y0 = (h / 2) as i64;
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let across = ((y as i64 - y0).abs() - floor_half).max(0) * side_mm;
            // A faint cross-valley tilt towards the axis keeps the thalweg
            // on the centreline of the flat floor.
            let tilt = (y as i64 - y0).abs() * 2;
            let z = base_mm + (w as i64 - 6 - x as i64) * fall_mm + across + tilt;
            g.z[i] = if x + 6 >= w || y == 0 || y + 1 == h || x == 0 {
                if x + 6 >= w {
                    -20_000
                } else {
                    i32::try_from(z + 50_000).unwrap()
                }
            } else {
                i32::try_from(z).unwrap()
            };
        }
    }
    // Rim: sea only at the outlet end; other rims are high walls.
    g
}

/// D8 path length over straight distance (‰) of the steepest-descent
/// course from `start` to the sea, after a drainage fill.
fn course_sinuosity(g: &Lattice, start: usize) -> (i64, usize) {
    let (w, h) = (g.width, g.height);
    let mut z = g.z.clone();
    let mut flags = vec![0_u8; w * h];
    open_sea_flags(&z, w, h, &mut flags);
    let (mut next, mut closed) = (vec![0; w * h], vec![0; w * h]);
    fill(&mut z, w, h, &flags, 1, &mut next, &mut closed).unwrap();
    let mut rcv = vec![0_u8; w * h];
    receivers(&z, w, h, &flags, 0, 0, &mut rcv);
    let (mut cur, mut len, mut steps) = (start, 0_i64, 0_usize);
    while g.z[cur] > 0 && steps < w * h {
        let r = receiver_index(cur, w, h, rcv[cur]);
        if r == cur {
            break;
        }
        let diag = (r % w != cur % w) && (r / w != cur / w);
        len += if diag { 1_414 } else { 1_000 };
        cur = r;
        steps += 1;
    }
    let (dx, dy) = (
        (cur % w) as i64 - (start % w) as i64,
        (cur / w) as i64 - (start / w) as i64,
    );
    let chord = i64::try_from(((dx * dx + dy * dy) * 1_000_000).unsigned_abs().isqrt()).unwrap();
    (len * 1000 / chord.max(1), steps)
}

#[test]
fn a_low_gradient_floodplain_river_meanders_and_leaves_cutoffs() {
    // 47 × 16 km: 0.3‰ floor 1.6 km wide, 2% sides.
    let (w, h) = (1_200, 400);
    let mut g = valley(w, h, 20, 800, 12, 20_000);
    let before = course_sinuosity(&g, h / 2 * w + 60).0;
    let relief = vec![0_u8; w * h];
    let mut f = WaterFeatures::default();
    shape(&mut g, &relief, 5, &mut f).unwrap();
    assert!(f.stats.meander_spans >= 1, "{:?}", f.stats);
    let valley_q8 = f.stats.meander_valley_q8.max(1);
    assert!(
        f.stats.meander_course_q8 * 1000 / valley_q8 >= 1_300,
        "carved course sinuosity {:?}",
        f.stats
    );
    let (after, steps) = course_sinuosity(&g, h / 2 * w + 60);
    assert!(steps > 100);
    assert!(
        after >= before + 150,
        "routed sinuosity {before} -> {after}"
    );
    // Cutoff crescents beyond the bends are protected sinks of oxbow origin.
    assert!(f.stats.oxbows >= 1, "{:?}", f.stats);
    assert!(f.sinks.iter().all(|s| s.kind == SinkKind::Oxbow));
    assert!(
        f.sinks.len() as u64 >= f.stats.oxbows,
        "each loop is protected whole"
    );
}

#[test]
fn a_steep_valley_keeps_its_course() {
    // 2% floor: far above the braiding threshold, and no sediment source.
    let (w, h) = (600, 200);
    let mut g = valley(w, h, 10, 800, 800, 20_000);
    let before = g.z.clone();
    let relief = vec![0_u8; w * h];
    let mut f = WaterFeatures::default();
    shape(&mut g, &relief, 5, &mut f).unwrap();
    assert_eq!(f.stats.meander_spans, 0);
    assert_eq!(f.stats.braided_reaches, 0);
    assert_eq!(g.z, before, "nothing is reshaped");
}

#[test]
fn a_piedmont_reach_below_mountains_braids() {
    // Mountains (relief mask 255) upstream of x = 300, a 1% piedmont floor
    // 1.2 km wide below it; bed load comes from the relief it has left.
    let (w, h) = (900, 300);
    let mut g = valley(w, h, 1, 1_500, 390, 20_000);
    let y0 = h / 2;
    let before = g.z[(y0 + 3) * w + 500] - g.z[y0 * w + 500];
    assert!(before >= 2_900, "a narrow floor before: {before} mm");
    let mut relief = vec![0_u8; w * h];
    for (i, r) in relief.iter_mut().enumerate() {
        if i % w < 300 {
            *r = 255;
        }
    }
    let mut f = WaterFeatures::default();
    shape(&mut g, &relief, 9, &mut f).unwrap();
    assert!(f.stats.braided_reaches >= 1, "{:?}", f.stats);
    assert!(!f.braided_cells.is_empty());
    // The belt is planed: across the floor the surface is within bar
    // relief of the bed wherever a braided cell was recorded.
    let &(cx, cy, belt_m) = &f.braided_cells[f.braided_cells.len() / 2];
    assert!(belt_m >= 230, "belt {belt_m} m");
    let x = (i64::from(cx) * 100_000_000 / D) as usize;
    let _ = cy;
    let span: Vec<i32> = (y0 - 2..=y0 + 2).map(|yy| g.z[yy * w + x]).collect();
    let (lo, hi) = (span.iter().min().unwrap(), span.iter().max().unwrap());
    assert!(hi - lo <= 1_200, "braidplain relief {span:?}");
}

#[test]
fn a_karst_valley_gets_a_closed_polje_with_an_outlet_at_its_rim() {
    // Find a seed whose carbonate proxy covers the whole test valley.
    let (w, h) = (500, 300);
    let seed = (0..4_000_u64)
        .find(|&s| {
            let k = s ^ 0x4A55;
            (0..=10).all(|i| (0..=6).all(|j| karst::is_karst(k, i * 1_950, j * 1_950)))
        })
        .expect("a karst seed");
    // 0.5% floor 0.8 km wide (too steep to meander), 45-140 m above the sea.
    let mut g = valley(w, h, 10, 600, 195, 45_000);
    let relief = vec![0_u8; w * h];
    let mut f = WaterFeatures::default();
    shape(&mut g, &relief, seed, &mut f).unwrap();
    assert!(f.stats.karst_permille > 900, "{:?}", f.stats);
    assert_eq!(f.stats.poljes, 1, "{:?}", f.stats);
    let s = f.sinks.iter().find(|s| s.kind == SinkKind::Karst).unwrap();
    // Closed: filling against the sea raises the polje centre.
    let mut z = g.z.clone();
    let mut flags = vec![0_u8; w * h];
    open_sea_flags(&z, w, h, &mut flags);
    let (mut next, mut closed) = (vec![0; w * h], vec![0; w * h]);
    fill(&mut z, w, h, &flags, 0, &mut next, &mut closed).unwrap();
    let c = (s.y_um / D) as usize * w + (s.x_um / D) as usize;
    assert!(z[c] - g.z[c] >= 3_000, "polje depth {} mm", z[c] - g.z[c]);
    // Enough floor floods for a lake of many 100 m cells.
    let flooded = (0..w * h).filter(|&i| z[i] > g.z[i]).count();
    assert!(flooded >= karst::POLJE_MIN_NODES, "{flooded} nodes");
    // The spill lies downstream (east) of the centre: the inflowing stream
    // enters at the upstream end and leaves over the old floor.
    let spill_level = z[c];
    let outlet = (0..w * h)
        .filter(|&i| z[i] == g.z[i] && g.z[i] == spill_level)
        .find(|&i| (0..8).any(|k| neighbour(i, w, h, k).is_some_and(|n| z[n] > g.z[n])));
    if let Some(o) = outlet {
        assert!(
            o % w > c % w,
            "spill at column {} left of centre {}",
            o % w,
            c % w
        );
    }
    // Dolines are recorded on the carbonate, never carved.
    assert!(!f.dolines.is_empty());
}

#[test]
fn oxbows_survive_the_drainage_guarantees_whole() {
    // Regression (seed-42 full size: 59 lakes of 1-3 cells, 62 of the 63
    // smallest oxbows): only a 75 m mid-arc disc was protected, so the fills
    // and the 100 m sampled pass lifted the rest of each crescent. After
    // the guarantees every oxbow must be one closed 100 m depression of at
    // least `oxbow::MIN_SAMPLES` samples.
    let (w, h) = (1_200, 400);
    let mut g = valley(w, h, 20, 800, 12, 20_000);
    let relief = vec![0_u8; w * h];
    let mut f = WaterFeatures::default();
    let shaped = shape_channels(&mut g, &relief, 5, &mut f).unwrap();
    let drain = |g: &mut Lattice, f: &WaterFeatures| {
        let mut flags = vec![0_u8; w * h];
        open_sea_flags(&g.z, w, h, &mut flags);
        for (i, fl) in flags.iter_mut().enumerate() {
            if f.is_sink((i % w) as i64 * D, (i / w) as i64 * D) {
                *fl = super::super::drainage::FIXED;
            }
        }
        let (mut next, mut closed) = (vec![0; w * h], vec![0; w * h]);
        fill(&mut g.z, w, h, &flags, 1, &mut next, &mut closed).unwrap();
    };
    f.index();
    drain(&mut g, &f);
    shape_basins(&mut g, &shaped, &mut f).unwrap();
    f.index();
    assert!(f.stats.oxbows >= 1, "{:?}", f.stats);
    drain(&mut g, &f);
    let sink = |x: i64, y: i64| f.is_sink(x, y);
    super::super::sampled::drain_sampled(&mut g, 100_000_000, &sink).unwrap();
    // Closed depressions of the 100 m samples (filled against the sea).
    let s = 100_000_000_i128;
    let (sw, sh) = (
        ((w as i128 - 1) * i128::from(D) / s) as usize + 1,
        ((h as i128 - 1) * i128::from(D) / s) as usize + 1,
    );
    let z: Vec<i32> = (0..sw * sh)
        .map(|i| super::super::sampled::sample(&g, (i % sw) as i128 * s, (i / sw) as i128 * s))
        .collect();
    let mut flags = vec![0_u8; sw * sh];
    open_sea_flags(&z, sw, sh, &mut flags);
    let mut filled = z.clone();
    let (mut next, mut closed) = (vec![0; sw * sh], vec![0; sw * sh]);
    fill(&mut filled, sw, sh, &flags, 0, &mut next, &mut closed).unwrap();
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
    assert!(!sizes.is_empty(), "an oxbow lake basin remains");
    assert!(
        sizes.iter().all(|&n| n >= oxbow::MIN_SAMPLES),
        "no fragments: basin sizes {sizes:?}"
    );
}
