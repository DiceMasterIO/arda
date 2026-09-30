//! A small synthetic landscape for tests and benchmarks (not a generator).
//!
//! 64 × 48 km: sea to the west with a sheltered bay, a fourth-order river
//! entering the bay, two tributaries, rolling hills, a mountain block in the
//! north-east, forest on the rough ground. It carries every field the stage
//! reads, so tests exercise the real pipeline without a generated world.
//!
//! Being a fixture rather than generation, it shapes its terrain with
//! floating point; the casts back to the grid's integer fields are bounded.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::needless_range_loop
)]

use crate::error::SettleError;
use crate::grid::{Grid, MEMORY_BUDGET};
use crate::num::{iu, sat_i32, sat_u16, ui};
use crate::rng::hash;
use arda::{Cover, TerrainKind};
use std::collections::VecDeque;

/// Grid size.
pub const WIDTH: usize = 640;
/// Grid size.
pub const HEIGHT: usize = 480;

/// A river cell with its bed height (mm), order, width (dm) and drainage.
#[derive(Clone, Copy)]
struct Bed {
    h: i64,
    order: u8,
    width_dm: u32,
    drainage: u32,
}

fn coast_x(y: i64) -> i64 {
    let wobble = (y as f64 / 37.0).sin() * 10.0;
    let bay = {
        let dy = (y - 240).abs();
        if dy < 34 {
            (((34 * 34 - dy * dy) as f64).sqrt() * 1.4) as i64
        } else {
            0
        }
    };
    40 + wobble as i64 + bay
}

fn noise(seed: u64, x: i64, y: i64, cell: i64) -> f64 {
    let (lx, ly) = (x.div_euclid(cell), y.div_euclid(cell));
    let fx = x.rem_euclid(cell) as f64 / cell as f64;
    let fy = y.rem_euclid(cell) as f64 / cell as f64;
    let v = |a: i64, b: i64| {
        (hash(seed, "synthetic", a.cast_unsigned(), b.cast_unsigned()) % 1000) as f64 / 1000.0
    };
    let s = |t: f64| t * t * (3.0 - 2.0 * t);
    let (sx, sy) = (s(fx), s(fy));
    let top = v(lx, ly) * (1.0 - sx) + v(lx + 1, ly) * sx;
    let bot = v(lx, ly + 1) * (1.0 - sx) + v(lx + 1, ly + 1) * sx;
    top * (1.0 - sy) + bot * sy
}

/// Builds the landscape for `seed` (the seed only moves the hills).
///
/// # Errors
/// Allocation errors.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]
pub fn landscape(seed: u64) -> Result<Grid, SettleError> {
    let mut g = Grid::sea(WIDTH, HEIGHT, MEMORY_BUDGET)?;
    let n = g.len();
    let mut bed: Vec<Option<Bed>> = vec![None; n];
    let put = |g: &Grid, bed: &mut Vec<Option<Bed>>, x: i64, y: i64, b: Bed| {
        if let Some(i) = g.at(x, y) {
            if x >= coast_x(y) - 1 && bed[i].is_none() {
                bed[i] = Some(b);
            }
        }
    };
    // Main river, east to west, drawn 4-connected.
    let main_y = |x: i64| 240 + ((x as f64 / 90.0).sin() * 40.0) as i64;
    let main_bed = |x: i64| 1000 + (x - 40).max(0) * 250;
    let mut prev_y = main_y(ui(WIDTH) - 1);
    for x in (coast_x(240) - 2..ui(WIDTH)).rev() {
        let y = main_y(x);
        let (order, width_dm) = if x < 350 {
            (4, 160)
        } else if x < 500 {
            (3, 90)
        } else {
            (2, 40)
        };
        let drainage = 2000 + u32::try_from(ui(WIDTH) - x).unwrap_or(0) * 60;
        for yy in prev_y.min(y)..=prev_y.max(y) {
            put(
                &g,
                &mut bed,
                x,
                yy,
                Bed {
                    h: main_bed(x),
                    order,
                    width_dm,
                    drainage,
                },
            );
        }
        prev_y = y;
    }
    let mut confluences = Vec::new();
    // Tributaries: north at x≈300 (order 3), south at x≈450 (order 2).
    for (x0, north, order, width_dm) in [(300_i64, true, 3_u8, 80_u32), (450, false, 2, 40)] {
        let jy = main_y(x0);
        let trib_x = |y: i64| x0 + ((y as f64 / 40.0).sin() * 15.0) as i64;
        let range: Vec<i64> = if north {
            (30..jy).rev().collect()
        } else {
            (jy + 1..450).collect()
        };
        let mut px = trib_x(range[0]);
        let mut first = true;
        for &y in range.iter().rev() {
            let x = trib_x(y);
            let d = (y - jy).abs();
            let b = Bed {
                h: main_bed(x0) + d * 400,
                order,
                width_dm,
                drainage: 500 + u32::try_from(450 - d).unwrap_or(0) * 20,
            };
            for xx in px.min(x)..=px.max(x) {
                put(&g, &mut bed, xx, y, b);
            }
            if first {
                if let Some(i) = g.at(x0, jy) {
                    confluences.push(i);
                }
                first = false;
            }
            px = x;
        }
    }
    // Nearest bed by multi-source BFS.
    let mut near: Vec<u32> = vec![u32::MAX; n];
    let mut dist: Vec<u32> = vec![u32::MAX; n];
    let mut q = VecDeque::new();
    for i in 0..n {
        if bed[i].is_some() {
            near[i] = u32::try_from(i).unwrap_or(0);
            dist[i] = 0;
            q.push_back(i);
        }
    }
    while let Some(i) = q.pop_front() {
        for j in g.neighbours4(i).collect::<Vec<_>>() {
            if dist[j] == u32::MAX {
                dist[j] = dist[i] + 1;
                near[j] = near[i];
                q.push_back(j);
            }
        }
    }
    for i in 0..n {
        let (x, y) = g.xy(i);
        let cx = coast_x(y);
        if x < cx {
            g.terrain[i] = TerrainKind::Sea;
            g.height_mm[i] = sat_i32(-2000 - (cx - x) * 800);
            continue;
        }
        g.terrain[i] = TerrainKind::Land;
        let src = iu(i64::from(near[i]));
        let b = bed.get(src).copied().flatten().unwrap_or(Bed {
            h: 1000,
            order: 0,
            width_dm: 0,
            drainage: 1,
        });
        let d = f64::from(dist[i]);
        let hills = noise(seed, x, y, 24) * 0.7 + noise(seed ^ 1, x, y, 9) * 0.3;
        let ramp = (d / 30.0).min(1.0);
        let mountain = if x > 430 && y < 190 {
            let m = ((x - 430) as f64 / 120.0).min(1.0) * ((190 - y) as f64 / 110.0).min(1.0);
            m * 650_000.0
        } else {
            0.0
        };
        let valley = if d == 0.0 { 0.0 } else { 2200.0 + d * 700.0 };
        let h = b.h as f64 + valley + hills * 130_000.0 * ramp + mountain * ramp;
        g.height_mm[i] = h as i32;
        g.har_dm[i] = sat_u16(((h - b.h as f64) / 100.0) as i64);
        if let Some(own) = bed[i] {
            g.order[i] = own.order;
            g.width_dm[i] = own.width_dm;
            g.drainage[i] = own.drainage;
            g.har_dm[i] = 0;
        } else {
            g.drainage[i] = 1;
        }
        g.forest[i] = if hills > 0.62 { 200 } else { 0 };
        g.moisture[i] = 160;
        g.rain_mm[i] = 800;
    }
    // Slope, aspect, temperature and cover from the finished heights.
    for i in 0..n {
        if !g.is_land(i) {
            continue;
        }
        let (x, y) = g.xy(i);
        let h = |dx: i64, dy: i64| {
            g.at(x + dx, y + dy)
                .map_or(g.height_mm[i], |j| g.height_mm[j].max(0))
        };
        let gx = f64::from(h(1, 0) - h(-1, 0)) / 200_000.0;
        let gy = f64::from(h(0, 1) - h(0, -1)) / 200_000.0;
        let slope = (gx.hypot(gy)).atan().to_degrees();
        g.slope_md[i] = (slope * 1000.0) as u16;
        let aspect = (-gx).atan2(gy).to_degrees().rem_euclid(360.0);
        g.aspect_deg[i] = aspect as u16;
        g.temp_cc[i] = (1150.0 - f64::from(g.height_mm[i]) / 1000.0 * 0.65) as i16;
        let near_mouth = x < coast_x(y) + 6 && dist[i] <= 2 && g.order[i] == 0;
        g.cover[i] = if slope > 32.0 {
            Cover::Rock
        } else if near_mouth {
            Cover::Marsh
        } else if g.forest[i] > 0 || slope > 13.0 {
            if g.forest[i] == 0 {
                g.forest[i] = 170;
            }
            Cover::Forest
        } else {
            Cover::Grass
        };
    }
    g.confluences = confluences;
    Ok(g)
}
