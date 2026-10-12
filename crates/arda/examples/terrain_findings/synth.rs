//! Synthetic controls: flagged fields must be flagged, isotropic ones not.

use crate::basins::{lakes, MIN_CELLS};
use crate::coast::measure;
use crate::drainage::{descent_hist, runs, spectrum};
use crate::grid::{shares, Grid, LAKE, LAND, SEA};
use crate::trace::trace;

fn hash(x: i64, y: i64, s: u64) -> f64 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ s.wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 31;
    (h >> 11) as f64 / (1_u64 << 53) as f64
}

fn value_noise(x: f64, y: f64, cell: f64, seed: u64) -> f64 {
    let (fx, fy) = (x / cell, y / cell);
    let (x0, y0) = (fx.floor(), fy.floor());
    let sm = |t: f64| t * t * (3.0 - 2.0 * t);
    let (tx, ty) = (sm(fx - x0), sm(fy - y0));
    let v = |dx: i64, dy: i64| hash(x0 as i64 + dx, y0 as i64 + dy, seed);
    (v(0, 0) * (1.0 - tx) + v(1, 0) * tx) * (1.0 - ty) + (v(0, 1) * (1.0 - tx) + v(1, 1) * tx) * ty
}

fn fractal(x: f64, y: f64, seed: u64) -> f64 {
    let mut s = 0.0;
    let mut amp = 1.0;
    let mut cell = 128.0;
    for o in 0..6 {
        s += amp * value_noise(x, y, cell, seed + o);
        amp *= 0.55;
        cell /= 2.0;
    }
    s / 2.0
}

fn mountains(w: usize, f: impl Fn(f64, f64) -> f64) -> Grid {
    let mut g = Grid::flat(w, w);
    for y in 0..w {
        for x in 0..w {
            g.z[y * w + x] = (1_500_000.0 + 400_000.0 * f(x as f64, y as f64)) as i32;
        }
    }
    g
}

#[test]
fn lattice_of_parallel_valleys_is_flagged() {
    let g = mountains(768, |x, y| {
        (std::f64::consts::TAU * (0.8 * x + 0.6 * y) / 25.0).cos()
    });
    let s = spectrum(&g);
    assert!(s.lattice(), "{:?}", s.ratios);
}

#[test]
fn white_and_fractal_noise_are_not_flagged() {
    for g in [
        mountains(768, |x, y| hash(x as i64, y as i64, 1)),
        mountains(768, |x, y| fractal(x, y, 5)),
    ] {
        let s = spectrum(&g);
        assert!(!s.lattice(), "{:?}", s.ratios);
        assert!(s.ratios.iter().all(|&r| r < 20.0), "{:?}", s.ratios);
    }
}

#[test]
fn descent_directions_follow_the_grid_only_for_axis_slopes() {
    let tilt = mountains(300, |x, _| x * 0.05);
    assert!(shares(&descent_hist(&tilt)).ratio > 1.5);
    let noise = mountains(300, |x, y| fractal(x, y, 9) * 8.0);
    assert!(shares(&descent_hist(&noise)).ratio < 1.2);
}

fn river(len: usize, f: impl Fn(usize) -> f64) -> Grid {
    let mut g = Grid::flat(len + 20, 300);
    for x in 0..len {
        let y = (150.0 + f(x)).round() as usize;
        g.order[y * g.w + x] = 3;
        g.z[y * g.w + x] = 100_000 - x as i32 * 10;
        // keep the chain 8-connected when the line steps rows
        if x > 0 {
            let py = (150.0 + f(x - 1)).round() as usize;
            if py != y {
                g.order[py * g.w + x] = 3;
                g.z[py * g.w + x] = 100_000 - x as i32 * 10 + 5;
            }
        }
    }
    g
}

#[test]
fn near_straight_rivers_are_found_and_meanders_are_not() {
    let line = crate::drainage::straight_probes(&river(1500, |_| 0.0), 3.0);
    assert!(line.straight > 50 && shares(&line.hist).ratio > 1.4);
    let wavy = river(1500, |x| 15.0 * (x as f64 / 20.0).sin());
    let m = crate::drainage::straight_probes(&wavy, 3.0);
    assert!(
        m.tested > 50 && m.straight == 0,
        "meander straight {}/{}",
        m.straight,
        m.tested
    );
}

#[test]
fn straight_channel_runs_are_counted() {
    let mut g = Grid::flat(400, 50);
    for x in 10..360 {
        g.order[20 * 400 + x] = 3;
        g.z[20 * 400 + x] = 1000 - x as i32;
    }
    let r = runs(&g, 20);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].cells, 350);
}

fn coast_of(w: usize, sea: impl Fn(f64, f64) -> bool) -> crate::coast::Acc {
    let m: Vec<bool> = (0..w * w)
        .map(|i| sea((i % w) as f64, (i / w) as f64))
        .collect();
    measure(&trace(w, w, &m), 40)
}

#[test]
fn stair_step_coasts_are_flagged() {
    // A 3:1 staircase is not axis or 45 degrees: only its stair share shows it.
    let tri = coast_of(1000, |x, y| x < 700.0 && y < 300.0 && 3.0 * y > x);
    assert!(tri.stair_share() > 0.4, "stair {}", tri.stair_share());
    let diag = coast_of(2000, |x, y| x < 1800.0 && y < 1800.0 && y > x);
    assert!(diag.angular(None), "diag {}", diag.stair_share());
    let boxy = coast_of(2000, |x, y| {
        (100.0..1900.0).contains(&x) && (100.0..1800.0).contains(&y)
    });
    assert!(boxy.angular(None));
}

#[test]
fn fractal_and_round_coasts_are_not_flagged() {
    let f = coast_of(1500, |x, y| fractal(x, y, 3) > 0.62);
    assert!(f.edges > 2000);
    assert!(
        !f.angular(None),
        "ratio {} stair {} long {}",
        f.ratio(1),
        f.stair_share(),
        f.long_share()
    );
    let c = coast_of(1000, |x, y| (x - 500.0).hypot(y - 500.0) < 400.0);
    assert!(
        !c.angular(None),
        "circle ratio {} stair {}",
        c.ratio(1),
        c.stair_share()
    );
}

fn lake_grid(shape: impl Fn(usize, usize) -> bool) -> Grid {
    let mut g = Grid::flat(700, 600);
    for y in 0..600 {
        for x in 0..700 {
            if shape(x, y) {
                g.kind[y * 700 + x] = LAKE;
            }
        }
    }
    g
}

#[test]
fn rectangle_lake_is_flagged_and_circle_is_not() {
    let r = lakes(
        &lake_grid(|x, y| (100..400).contains(&x) && (100..300).contains(&y)),
        MIN_CELLS,
    );
    assert_eq!(r.len(), 1);
    assert!(r[0].rectangular() && r[0].bbox_fill > 0.99);
    let c = lakes(
        &lake_grid(|x, y| ((x as f64 - 350.0).hypot(y as f64 - 300.0)) < 140.0),
        MIN_CELLS,
    );
    assert_eq!(c.len(), 1);
    assert!(!c[0].rectangular(), "fill {}", c[0].rect_fill);
    assert!((c[0].iso - 1.0).abs() < 0.3, "iso {}", c[0].iso);
}

#[test]
fn rectangle_on_area_lines_is_reported() {
    let r = lakes(
        &lake_grid(|x, y| (100..512).contains(&x) && (100..400).contains(&y)),
        MIN_CELLS,
    );
    assert!(r[0].on_area_lines());
}

#[test]
fn analysis_is_deterministic_and_handles_a_small_world() {
    let mut g = mountains(600, |x, y| fractal(x, y, 2));
    for i in 0..g.kind.len() {
        let (x, y) = (i % 600, i / 600);
        if x < 120 {
            g.kind[i] = SEA;
        } else if (x as i64 - 400).pow(2) + (y as i64 - 300).pow(2) < 90 * 90 {
            g.kind[i] = LAKE;
        }
    }
    let a = crate::analyse(&g, None).render();
    let b = crate::analyse(&g, None).render();
    assert_eq!(a, b);
    assert!(a.contains("\"lakes_over_500km2\""));
    assert!(g.kind.contains(&LAND));
}

#[test]
fn saved_micro_world_smoke_test_when_present() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../out/recheck/micro42-r7");
    let Ok(world) = arda::World::load(&dir) else {
        eprintln!("skipped: {} not generated", dir.display());
        return;
    };
    let Ok(g) = Grid::load(&world) else {
        panic!("load grid")
    };
    let Ok(shore) = world.shore() else {
        panic!("load shore")
    };
    let a = crate::analyse(&g, shore.as_ref()).render();
    assert_eq!(a, crate::analyse(&g, shore.as_ref()).render());
    assert!(a.contains("\"flags\""));
}
