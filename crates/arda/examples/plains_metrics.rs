//! Plains diagnostics for one saved world (goal 5).
//! Usage: cargo run -p arda --release --example plains_metrics -- WORLD
//!
//! Separates genuine plains from low hill country on the 100 m area grid
//! and prints one JSON object:
//! - `lowland`: land below 200 m, split by local relief (highest minus
//!   lowest height in a 2.1 km window) into plain (< 50 m) and hill country,
//!   with the slope percentiles of each;
//! - `plains`: all land with relief under 50 m, and the stricter under 25 m,
//!   at any height, split into valley floor (within 3 m of its river),
//!   valley side (3-40 m) and interfluve (above 40 m);
//! - `floodplains`: per river size class, the valley-floor width across
//!   river cells (shortest of four axis transects through ground at most
//!   2 m above the river cell, out to 5 km) and the floor slope, also for
//!   rivers in plain settings alone (2.1 km relief under 80 m);
//! - `hammond`: a Hammond-style landform class per land cell from a ~10 km
//!   window (relief of 1 km block extremes over ±5 blocks; share of gentle
//!   cells under 8% ≈ 4.6° within ±4.8 km): plains (gentle ≥ 80%, relief
//!   < 90 m), irregular plains (gentle 50-80%, relief < 90 m), plains with
//!   hills (gentle ≥ 50%, relief ≥ 90 m), hills (gentle < 50%, relief
//!   < 300 m) and mountains, each with its land share, lowland share and
//!   slope percentiles;
//! - `terraces`: the share of near-flat treads (< 1°) on lowland valley
//!   sides 2-40 m above their river.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use arda::{TerrainKind, World};
use arda_core::CellCoord;
use std::error::Error;
use std::path::Path;

struct Grid {
    w: usize,
    h: usize,
    z: Vec<i32>,
    land: Vec<bool>,
    slope: Vec<u16>,
    order: Vec<u8>,
    hand_dm: Vec<u16>,
    area: Vec<u32>,
}

fn pct(v: &mut [i64], p: usize) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_unstable();
    v[(v.len() - 1) * p / 100] as f64 / 1000.0
}

fn stats(v: &mut [i64]) -> String {
    let n = v.len();
    let (p25, p50, p75, p90) = (pct(v, 25), pct(v, 50), pct(v, 75), pct(v, 90));
    let lt2 = v.iter().filter(|&&s| s < 2_000).count();
    format!(
        "{{\"cells\":{n},\"slope_deg_p25_50_75_90\":[{p25:.2},{p50:.2},{p75:.2},{p90:.2}],\"under_2deg_permille\":{}}}",
        lt2 * 1000 / n.max(1)
    )
}

/// Sliding max − min over a `(2r+1)²` window, metres (separable).
fn local_relief(g: &Grid, r: usize) -> Vec<i32> {
    let (w, h) = (g.w, g.h);
    let m: Vec<i32> = (0..w * h)
        .map(|i| if g.land[i] { g.z[i].max(0) / 1000 } else { 0 })
        .collect();
    let pass = |src: &[i32], horizontal: bool, take_max: bool| -> Vec<i32> {
        let mut out = vec![0; w * h];
        for y in 0..h {
            for x in 0..w {
                let (lo, hi, at): (usize, usize, Box<dyn Fn(usize) -> usize>) = if horizontal {
                    (
                        x.saturating_sub(r),
                        (x + r + 1).min(w),
                        Box::new(move |k| y * w + k),
                    )
                } else {
                    (
                        y.saturating_sub(r),
                        (y + r + 1).min(h),
                        Box::new(move |k| k * w + x),
                    )
                };
                let it = (lo..hi).map(|k| src[at(k)]);
                out[y * w + x] = if take_max {
                    it.max().unwrap_or(0)
                } else {
                    it.min().unwrap_or(0)
                };
            }
        }
        out
    };
    let mx = pass(&pass(&m, true, true), false, true);
    let mn = pass(&pass(&m, true, false), false, false);
    mx.iter().zip(&mn).map(|(a, b)| a - b).collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = std::env::args_os()
        .nth(1)
        .ok_or("usage: plains_metrics WORLD")?;
    let world = World::load(Path::new(&root))?;
    let (w, h) = (
        usize::try_from(world.manifest().areas_wide)? * 512,
        usize::try_from(world.manifest().areas_high)? * 512,
    );
    let mut g = Grid {
        w,
        h,
        z: vec![0; w * h],
        land: vec![false; w * h],
        slope: vec![0; w * h],
        order: vec![0; w * h],
        hand_dm: vec![0; w * h],
        area: vec![0; w * h],
    };
    for (ax, ay) in world.area_coords() {
        let area = world.read_area(ax, ay)?;
        let cells = area.cells();
        for y in 0..512_u16 {
            for x in 0..512_u16 {
                let c = cells.get(CellCoord::new(x, y).ok_or("cell coordinate")?);
                let i = (usize::try_from(ay)? * 512 + usize::from(y)) * w
                    + usize::try_from(ax)? * 512
                    + usize::from(x);
                g.z[i] = c.height.raw();
                g.land[i] = c.terrain == TerrainKind::Land;
                g.slope[i] = c.slope_milli_deg;
                g.order[i] = c.watercourse_order;
                g.hand_dm[i] = c.height_above_river_dm;
                g.area[i] = c.drainage_area_cells;
            }
        }
    }
    let relief = local_relief(&g, 10);
    let s = |i: usize| i64::from(g.slope[i]);
    let land: Vec<usize> = (0..w * h).filter(|&i| g.land[i]).collect();
    let low: Vec<usize> = land.iter().copied().filter(|&i| g.z[i] < 200_000).collect();
    let mut low_all: Vec<i64> = low.iter().map(|&i| s(i)).collect();
    let mut low_plain: Vec<i64> = low
        .iter()
        .filter(|&&i| relief[i] < 50)
        .map(|&i| s(i))
        .collect();
    let mut low_hill: Vec<i64> = low
        .iter()
        .filter(|&&i| relief[i] >= 50)
        .map(|&i| s(i))
        .collect();
    let mut low_rolling: Vec<i64> = low
        .iter()
        .filter(|&&i| (50..150).contains(&relief[i]))
        .map(|&i| s(i))
        .collect();
    let plain = |lim: i32| -> String {
        let cells: Vec<usize> = land.iter().copied().filter(|&i| relief[i] < lim).collect();
        let pick = |f: &dyn Fn(u16) -> bool| -> Vec<i64> {
            cells
                .iter()
                .filter(|&&i| f(g.hand_dm[i]))
                .map(|&i| s(i))
                .collect()
        };
        let mut all: Vec<i64> = cells.iter().map(|&i| s(i)).collect();
        format!(
            "{{\"land_permille\":{},\"all\":{},\"floor\":{},\"side\":{},\"interfluve\":{}}}",
            cells.len() * 1000 / land.len().max(1),
            stats(&mut all),
            stats(&mut pick(&|d| d < 30)),
            stats(&mut pick(&|d| (30..400).contains(&d))),
            stats(&mut pick(&|d| d >= 400)),
        )
    };
    // Floodplains: transects across river cells (sampled every 4th cell).
    let classes: [(&str, u64, u64); 4] = [
        ("2-20km2", 200, 2_000),
        ("20-100km2", 2_000, 10_000),
        ("100-1000km2", 10_000, 100_000),
        ("ge1000km2", 100_000, u64::MAX),
    ];
    let mut fp = String::new();
    for (name, lo, hi) in classes {
        let (mut widths, mut floor, mut plain_w) = (Vec::new(), Vec::new(), Vec::new());
        for (k, &i) in land.iter().enumerate() {
            let a = u64::from(g.area[i]);
            if g.order[i] == 0 || a < lo || a >= hi || k % 4 != 0 || g.z[i] >= 400_000 {
                continue;
            }
            let (x, y) = ((i % w) as i64, (i / w) as i64);
            let top = g.z[i] + 2_000;
            let mut reach = |dx: i64, dy: i64| -> i64 {
                let mut n = 0;
                for step in 1..=50 {
                    let (xx, yy) = (x + dx * step, y + dy * step);
                    if xx < 0 || yy < 0 || xx >= w as i64 || yy >= h as i64 {
                        break;
                    }
                    let j = yy as usize * w + xx as usize;
                    if g.z[j] > top {
                        break;
                    }
                    n = step;
                    if g.land[j] && g.order[j] == 0 {
                        floor.push(s(j));
                    }
                }
                n
            };
            let wd = [(1, 0), (0, 1), (1, 1), (1, -1)]
                .iter()
                .map(|&(dx, dy)| {
                    let diag = if dx != 0 && dy != 0 { 141 } else { 100 };
                    (reach(dx, dy) + reach(-dx, -dy) + 1) * diag
                })
                .min()
                .unwrap_or(0);
            widths.push(wd * 1000);
            if relief[i] < 80 {
                plain_w.push(wd * 1000);
            }
        }
        let n = widths.len();
        let (w50, w90) = (pct(&mut widths, 50), pct(&mut widths, 90));
        let np = plain_w.len();
        let (p50, p90) = (pct(&mut plain_w, 50), pct(&mut plain_w, 90));
        fp.push_str(&format!(
            "{}\"{name}\":{{\"samples\":{n},\"width_m_p50_90\":[{w50:.0},{w90:.0}],\"plain_samples\":{np},\"plain_width_m_p50_90\":[{p50:.0},{p90:.0}],\"floor\":{}}}",
            if fp.is_empty() { "" } else { "," },
            stats(&mut floor),
        ));
    }
    let (mut side, mut tread) = (0_u64, 0_u64);
    for &i in &low {
        if relief[i] < 150 && (20..400).contains(&g.hand_dm[i]) {
            side += 1;
            tread += u64::from(g.slope[i] < 1_000);
        }
    }
    let hammond = hammond_json(&g, &relief);
    println!(
        "{{\"hammond\":{hammond},\"lowland_lt200m\":{{\"land_permille\":{},\"all\":{},\"plain_relief_lt50m\":{},\"hill_relief_ge50m\":{},\"rolling_relief_50_150m\":{}}},\"plains_relief_lt50m\":{},\"plains_relief_lt25m\":{},\"floodplains\":{{{fp}}},\"terraces\":{{\"valley_side_cells\":{side},\"tread_permille\":{}}}}}",
        low.len() * 1000 / land.len().max(1),
        stats(&mut low_all),
        stats(&mut low_plain),
        stats(&mut low_hill),
        stats(&mut low_rolling),
        plain(50),
        plain(25),
        tread * 1000 / side.max(1),
    );
    Ok(())
}

/// Hammond-style landform classes over a ~10 km window.
fn hammond_json(g: &Grid, _relief: &[i32]) -> String {
    let (w, h) = (g.w, g.h);
    // 1 km block extremes, then ±5 blocks.
    let (bw, bh) = (w.div_ceil(10), h.div_ceil(10));
    let (mut bmax, mut bmin) = (vec![i32::MIN; bw * bh], vec![i32::MAX; bw * bh]);
    for i in 0..w * h {
        if !g.land[i] {
            continue;
        }
        let b = (i / w / 10) * bw + (i % w) / 10;
        let m = g.z[i].max(0) / 1000;
        bmax[b] = bmax[b].max(m);
        bmin[b] = bmin[b].min(m);
    }
    let rel_b: Vec<i32> = (0..bw * bh)
        .map(|b| {
            let (bx, by) = (b % bw, b / bw);
            let (mut hi, mut lo) = (i32::MIN, i32::MAX);
            for yy in by.saturating_sub(5)..(by + 6).min(bh) {
                for xx in bx.saturating_sub(5)..(bx + 6).min(bw) {
                    hi = hi.max(bmax[yy * bw + xx]);
                    lo = lo.min(bmin[yy * bw + xx]);
                }
            }
            if hi < lo {
                0
            } else {
                hi - lo
            }
        })
        .collect();
    // Gentle share via summed-area tables over land cells.
    let sat = |f: &dyn Fn(usize) -> i64| -> Vec<i64> {
        let mut t = vec![0_i64; (w + 1) * (h + 1)];
        for y in 0..h {
            for x in 0..w {
                t[(y + 1) * (w + 1) + x + 1] =
                    f(y * w + x) + t[y * (w + 1) + x + 1] + t[(y + 1) * (w + 1) + x]
                        - t[y * (w + 1) + x];
            }
        }
        t
    };
    let land_t = sat(&|i| i64::from(g.land[i]));
    let gentle_t = sat(&|i| i64::from(g.land[i] && g.slope[i] < 4_574));
    let r = 48;
    let names = [
        "plain",
        "irregular_plain",
        "plain_with_hills",
        "hills",
        "mountains",
    ];
    let mut by: Vec<Vec<i64>> = vec![Vec::new(); 5];
    let mut low_by = [0_usize; 5];
    let (mut nland, mut nlow) = (0_usize, 0_usize);
    for i in 0..w * h {
        if !g.land[i] {
            continue;
        }
        let (x, y) = (i % w, i / w);
        let (x0, y0, x1, y1) = (
            x.saturating_sub(r),
            y.saturating_sub(r),
            (x + r + 1).min(w),
            (y + r + 1).min(h),
        );
        let boxed = |t: &[i64]| {
            t[y1 * (w + 1) + x1] - t[y0 * (w + 1) + x1] - t[y1 * (w + 1) + x0]
                + t[y0 * (w + 1) + x0]
        };
        let gentle = boxed(&gentle_t) * 100 / boxed(&land_t).max(1);
        let rel = rel_b[(y / 10) * bw + x / 10];
        let c = if gentle >= 50 && rel < 90 {
            usize::from(gentle < 80)
        } else if gentle >= 50 {
            2
        } else if rel < 300 {
            3
        } else {
            4
        };
        by[c].push(i64::from(g.slope[i]));
        nland += 1;
        if g.z[i] < 200_000 {
            nlow += 1;
            low_by[c] += 1;
        }
    }
    let parts: Vec<String> = names
        .iter()
        .zip(by.iter_mut())
        .zip(low_by)
        .map(|((name, v), low)| {
            format!(
                "\"{name}\":{{\"land_permille\":{},\"lowland_permille\":{},\"slope\":{}}}",
                v.len() * 1000 / nland.max(1),
                low * 1000 / nlow.max(1),
                stats(v)
            )
        })
        .collect();
    format!("{{{}}}", parts.join(","))
}
