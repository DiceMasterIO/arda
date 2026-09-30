//! Physical and seam checks of the mid-zoom refinement on a synthetic
//! massif: a cone cut by six V valleys, with multi-scale roughness.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    missing_docs
)]

use arda_midzoom::detail::{detail, Style};
use arda_midzoom::{refine_nodes, GridTerrain, HeightTile, Terrain, FINE_UM};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

const WIDE: i64 = 320;
const S_M: f64 = 39.0625;

fn hash(x: i64, y: i64, t: u64) -> f64 {
    let mut v = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ t.wrapping_mul(0x1656_67B1_9E37_79F9);
    v ^= v >> 29;
    v = v.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    v ^= v >> 32;
    (v % 10_000) as f64 / 10_000.0
}

fn noise(x: f64, y: f64, p: f64, t: u64) -> f64 {
    let (fx, fy) = (x / p, y / p);
    let (ix, iy) = (fx.floor() as i64, fy.floor() as i64);
    let (tx, ty) = (fx - fx.floor(), fy - fy.floor());
    let s = |t: f64| t * t * (3.0 - 2.0 * t);
    let (sx, sy) = (s(tx), s(ty));
    let a = hash(ix, iy, t) + (hash(ix + 1, iy, t) - hash(ix, iy, t)) * sx;
    let b = hash(ix, iy + 1, t) + (hash(ix + 1, iy + 1, t) - hash(ix, iy + 1, t)) * sx;
    a + (b - a) * sy - 0.5
}

/// Height (m) of the massif at lattice metres.
fn massif(x: f64, y: f64) -> f64 {
    let c = WIDE as f64 * S_M / 2.0;
    let (dx, dy) = (x - c, y - c);
    let r = (dx * dx + dy * dy).sqrt();
    let th = dy.atan2(dx);
    let valleys = 160.0 * (1.0 - (3.0 * th).sin().abs()) * (r / 1_500.0).min(1.0);
    let bumps = 8.0 * noise(x, y, 170.0, 1) + 4.0 * noise(x, y, 90.0, 2);
    (2_600.0 - 0.33 * r - valleys + bumps).max(-50.0)
}

fn world() -> GridTerrain {
    GridTerrain::from_fn(42, WIDE, WIDE, |kx, ky| {
        (massif(kx as f64 * S_M, ky as f64 * S_M) * 1_000.0) as i32
    })
}

fn centre_node(n: i64) -> i64 {
    WIDE / 2 * n
}

#[test]
fn refinement_is_deterministic_and_windows_join_exactly() {
    let w = world();
    for n in [2, 4] {
        let c = centre_node(n) + 37;
        let a = refine_nodes(&w, c, c, 96, 80, n).unwrap();
        let b = refine_nodes(&w, c + 50, c + 31, 90, 70, n).unwrap();
        assert_eq!(
            a,
            refine_nodes(&w, c, c, 96, 80, n).unwrap(),
            "deterministic"
        );
        let mut shared = 0;
        for j in b.j0..b.j0 + 70 {
            for i in b.i0..b.i0 + 90 {
                if let (Some(x), Some(y)) = (a.node(i, j), b.node(i, j)) {
                    assert_eq!(x, y, "node ({i}, {j}) differs between windows at n={n}");
                    shared += 1;
                }
            }
        }
        assert!(shared > 1_000);
    }
}

fn tile(n: i64, side: usize) -> HeightTile {
    let c = centre_node(n) - (side as i64) / 2 + 3 * n;
    refine_nodes(&world(), c, c, side, side, n).unwrap()
}

#[test]
fn refined_cell_means_keep_the_stored_heights() {
    let w = world();
    let n = 4;
    let t = tile(n, 256);
    let (kx0, kx1) = (t.i0.div_euclid(n) + 1, (t.i0 + 255).div_euclid(n) - 1);
    let stored = w
        .read_nodes(kx0, kx0, (kx1 - kx0 + 1) as usize, (kx1 - kx0 + 1) as usize)
        .unwrap();
    let (mut worst, mut sum, mut count, mut detail_energy) = (0_i64, 0_i64, 0_i64, 0_i64);
    for ky in kx0..=kx1 {
        for kx in kx0..=kx1 {
            let mut acc = 0_i64;
            for dy in -2_i64..=2 {
                for dx in -2_i64..=2 {
                    let wgt =
                        (if dx.abs() == 2 { 1 } else { 2 }) * (if dy.abs() == 2 { 1 } else { 2 });
                    acc += wgt * i64::from(t.node(kx * n + dx, ky * n + dy).unwrap());
                }
            }
            let mean = acc / 64;
            let h = i64::from(stored[((ky - kx0) * (kx1 - kx0 + 1) + kx - kx0) as usize]);
            let err = (mean - h).abs();
            worst = worst.max(err);
            sum += err;
            count += 1;
            let mid = i64::from(t.node(kx * n + 2, ky * n + 2).unwrap());
            let corner_mean = (i64::from(t.node(kx * n, ky * n).unwrap())
                + i64::from(t.node(kx * n + 4, ky * n + 4).unwrap()))
                / 2;
            detail_energy += (mid - corner_mean).abs();
        }
    }
    let mean_err = sum / count;
    assert!(mean_err <= 300, "mean |39 m mean − stored| = {mean_err} mm");
    assert!(worst <= 2_500, "worst |39 m mean − stored| = {worst} mm");
    assert!(
        detail_energy / count > 500,
        "refinement adds visible sub-39 m relief"
    );
}

/// Depth (mm) of every node below its priority-flood spill level from the
/// window edge.
fn fill_depths(t: &HeightTile) -> Vec<i64> {
    let (w, h) = (t.width, t.height);
    let z: Vec<i64> = t.heights_mm.iter().map(|&v| i64::from(v)).collect();
    let mut level = vec![i64::MAX; w * h];
    let mut heap = BinaryHeap::new();
    for y in 0..h {
        for x in 0..w {
            if x == 0 || y == 0 || x + 1 == w || y + 1 == h {
                level[y * w + x] = z[y * w + x];
                heap.push(Reverse((z[y * w + x], y * w + x)));
            }
        }
    }
    while let Some(Reverse((l, k))) = heap.pop() {
        let (x, y) = ((k % w) as i64, (k / w) as i64);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
                    continue;
                }
                let nk = ny as usize * w + nx as usize;
                if level[nk] == i64::MAX {
                    level[nk] = z[nk].max(l);
                    heap.push(Reverse((level[nk], nk)));
                }
            }
        }
    }
    level.iter().zip(&z).map(|(l, z)| l - z).collect()
}

#[test]
fn refinement_makes_no_pits_deeper_than_half_a_metre() {
    for n in [2, 4] {
        let t = tile(n, 320);
        let deep = fill_depths(&t).iter().filter(|&&d| d > 500).count();
        assert_eq!(deep, 0, "n={n}: {deep} nodes sit in pits deeper than 0.5 m");
    }
}

/// D8 flow accumulation (in nodes) of a row-major grid.
fn accumulation(z: &[i64], side: usize) -> Vec<i64> {
    let mut order: Vec<usize> = (0..side * side).collect();
    order.sort_by_key(|&k| std::cmp::Reverse((z[k], k)));
    let mut acc = vec![1_i64; side * side];
    for &k in &order {
        let (x, y) = ((k % side) as i64, (k / side) as i64);
        let mut best = (0_i64, k);
        for dy in -1_i64..=1 {
            for dx in -1_i64..=1 {
                let (nx, ny) = (x + dx, y + dy);
                if (dx, dy) == (0, 0) || nx < 0 || ny < 0 || nx >= side as i64 || ny >= side as i64
                {
                    continue;
                }
                let nk = ny as usize * side + nx as usize;
                let dist = if dx != 0 && dy != 0 { 1_414 } else { 1_000 };
                let drop = (z[k] - z[nk]) * 1_000 / dist;
                if drop > best.0 {
                    best = (drop, nk);
                }
            }
        }
        if best.1 != k {
            acc[best.1] += acc[k];
        }
    }
    acc
}

/// Share (‰) of channel nodes draining ≥ `area_m2` that lie within 60 m of
/// a trunk channel of `trunks` (lattice metres of its channel nodes).
fn on_trunks(
    z: &[i64],
    side: usize,
    origin: (f64, f64),
    s: f64,
    area_m2: f64,
    trunks: &[(f64, f64)],
) -> (usize, usize) {
    let acc = accumulation(z, side);
    let (mut all, mut near) = (0, 0);
    for y in 4..side - 4 {
        for x in 4..side - 4 {
            if (acc[y * side + x] as f64) * s * s < area_m2 {
                continue;
            }
            let p = (origin.0 + x as f64 * s, origin.1 + y as f64 * s);
            all += 1;
            if trunks.iter().any(|t| (t.0 - p.0).hypot(t.1 - p.1) < 60.0) {
                near += 1;
            }
        }
    }
    (all, near)
}

#[test]
fn refined_drainage_follows_the_stored_channels() {
    let n = 4;
    let side = 400;
    let t = tile(n, side);
    let s = (FINE_UM / n) as f64 / 1e6;
    // Stored channels: D8 on the stored lattice over the same ground.
    let cs = side / n as usize + 16;
    let k0 = t.i0.div_euclid(n) - 8;
    let stored: Vec<i64> = world()
        .read_nodes(k0, k0, cs, cs)
        .unwrap()
        .into_iter()
        .map(i64::from)
        .collect();
    let sacc = accumulation(&stored, cs);
    let mut trunks = Vec::new();
    for y in 0..cs {
        for x in 0..cs {
            if (sacc[y * cs + x] as f64) * S_M * S_M >= 100_000.0 {
                trunks.push(((k0 + x as i64) as f64 * S_M, (k0 + y as i64) as f64 * S_M));
            }
        }
    }
    let z: Vec<i64> = t.heights_mm.iter().map(|&v| i64::from(v)).collect();
    let origin = (t.i0 as f64 * s, t.j0 as f64 * s);
    let (all, near) = on_trunks(&z, side, origin, s, 300_000.0, &trunks);
    assert!(all > 200, "the refined surface drains into trunks ({all})");
    let share = near * 1_000 / all;
    assert!(
        share >= 900,
        "only {share}‰ of refined trunk channels follow stored ones"
    );
}

#[test]
fn detail_has_no_axis_bias() {
    // Energy of the detail on uniform slopes at many azimuths.
    let mut energies = Vec::new();
    for deg in (0..180).step_by(15) {
        let a = f64::from(deg).to_radians();
        let g = ((a.cos() * 2_048.0) as i64, (a.sin() * 2_048.0) as i64);
        let mut e = 0_i64;
        for j in 0..120 {
            for i in 0..120 {
                let d = detail(
                    7,
                    i * 9_765_625 + 3_000_000_000,
                    j * 9_765_625 - 1_000_000_000,
                    9_765_625,
                    g,
                    4_000,
                    Style {
                        min_wave_um: 0,
                        rib_q12: 0,
                    },
                );
                let (gx, gy) = d.gradient_q12;
                e += (gx * gx + gy * gy) / 1_000;
            }
        }
        energies.push(e);
    }
    let lo = *energies.iter().min().unwrap();
    let hi = *energies.iter().max().unwrap();
    assert!(lo > 0);
    assert!(
        hi * 100 / lo <= 110,
        "detail energy varies {lo}..{hi} with azimuth"
    );
}

#[test]
fn gentle_low_ground_stays_smooth() {
    // A 2% plain with 3 m bumps: no ribs or gullies may appear.
    let w = GridTerrain::from_fn(3, 200, 200, |kx, ky| {
        let (x, y) = (kx as f64 * S_M, ky as f64 * S_M);
        ((50.0 + 0.02 * x + 3.0 * noise(x, y, 300.0, 5)) * 1_000.0) as i32
    });
    let t = refine_nodes(&w, 300, 300, 160, 160, 4).unwrap();
    let mut worst = 0_i64;
    for j in t.j0 + 1..t.j0 + 159 {
        for i in t.i0 + 1..t.i0 + 159 {
            let c = i64::from(t.node(i, j).unwrap());
            let lap = i64::from(t.node(i - 1, j).unwrap())
                + i64::from(t.node(i + 1, j).unwrap())
                + i64::from(t.node(i, j - 1).unwrap())
                + i64::from(t.node(i, j + 1).unwrap())
                - 4 * c;
            worst = worst.max(lap.abs());
        }
    }
    assert!(worst < 250, "plain curvature {worst} mm per node²");
}

#[test]
fn a_tile_of_nodes_refines_well_within_budget() {
    let w = world();
    let c = centre_node(4) - 128;
    let _ = refine_nodes(&w, c, c, 270, 270, 4).unwrap();
    let start = std::time::Instant::now();
    let _ = refine_nodes(&w, c + 3, c, 270, 270, 4).unwrap();
    let ms = start.elapsed().as_millis();
    assert!(ms < 150, "refining a 256-px tile's nodes took {ms} ms");
}
