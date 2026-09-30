//! Plan overview PNGs: fields, water, streets, plots, gardens, roofs with a
//! lit and a shaded pitch, drop shadows, walls and a scale bar.

use crate::function::BuildingFunction as F;
use crate::geom::{self, v2};
use crate::num::{clamp_u32, floor_i, round_u8};
use crate::plan::grid::{Kind, PlanGrid, SQUARE_M};
use crate::plan::{Building, TownPlan, WealthLevel};
use crate::rng::{hash_i, noise2, unit};
use arda_tactical::Rgba;

type Rgb = [f64; 3];

fn hex(c: u32) -> Rgb {
    [
        f64::from((c >> 16) & 255),
        f64::from((c >> 8) & 255),
        f64::from(c & 255),
    ]
}

fn scale(c: Rgb, k: f64) -> Rgb {
    [c[0] * k, c[1] * k, c[2] * k]
}

fn mixc(a: Rgb, b: Rgb, t: f64) -> Rgb {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn roof(b: &Building) -> Rgb {
    match b.function {
        F::Temple | F::Shrine | F::Library => hex(0x6d6b78),
        F::Keep | F::Barracks | F::Guardhouse => hex(0x7b756e),
        F::MarketHall => hex(0x8e4a32),
        F::Barn | F::Stable | F::Boathouse => hex(0xa68a58),
        F::Dock => hex(0x866440),
        _ => match b.wealth_level {
            WealthLevel::Poor => hex(0xae905a),
            WealthLevel::Modest => hex(0x9d5b3e),
            WealthLevel::Wealthy => hex(0xa84634),
        },
    }
}

/// Open-field strips: furlongs in warped, rotated tiles with hedgerows.
fn field(seed: u64, x: f64, y: f64) -> Rgb {
    let wx = x + 45.0 * noise2(seed ^ 0x33, x / 210.0, y / 210.0);
    let wy = y + 45.0 * noise2(seed ^ 0x34, x / 210.0, y / 210.0);
    let tile = 190.0;
    let (tx, ty) = (floor_i(wx / tile), floor_i(wy / tile));
    let (fx, fy) = (
        wx / tile - wx.div_euclid(tile),
        wy / tile - wy.div_euclid(tile),
    );
    let edge = fx.min(1.0 - fx).min(fy.min(1.0 - fy)) * tile;
    if edge < 1.6 {
        return hex(0x5e783e);
    }
    let ang = unit(hash_i(seed, tx, ty)) * geom::PI;
    let (s, c) = geom::sin_cos(ang);
    let u = wx * c + wy * s;
    let strip = floor_i(u / 13.0);
    let pick = unit(hash_i(seed ^ 0x51, strip, tx * 31 + ty));
    let base = if unit(hash_i(seed ^ 0x77, tx, ty)) < 0.3 {
        hex(0x8fa65c)
    } else if pick < 0.5 {
        hex(0x9daa64)
    } else if pick < 0.8 {
        hex(0xaba96a)
    } else {
        hex(0x94a25e)
    };
    scale(base, 0.96 + 0.05 * noise2(seed, x / 25.0, y / 25.0))
}

/// Squares from each cell to the nearest building, capped at 60.
fn built_distance(g: &PlanGrid) -> Vec<u8> {
    let mut d = vec![60u8; g.kind.len()];
    let mut q = std::collections::VecDeque::new();
    for (k, &b) in g.building.iter().enumerate() {
        if b != 0 {
            d[k] = 0;
            q.push_back(k);
        }
    }
    while let Some(k) = q.pop_front() {
        let (i, j) = g.ij(k);
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            if let Some(m) = g.idx(i + dx, j + dy) {
                if d[m] > d[k] + 1 {
                    d[m] = d[k] + 1;
                    q.push_back(m);
                }
            }
        }
    }
    d
}

fn ground(plan: &TownPlan, near: &[u8], i: i64, j: i64, x: f64, y: f64) -> Rgb {
    let g = &plan.grid;
    let Some(k) = g.idx(i, j) else {
        return field(plan.seed, x, y);
    };
    let n = noise2(plan.seed ^ 7, x / 6.0, y / 6.0);
    let c = match g.kind[k] {
        Kind::Open => {
            let inside = plan
                .wall
                .as_ref()
                .is_some_and(|w| geom::inside(&w.ring, v2(x, y)));
            let croft = scale(
                hex(0x8eaa5c),
                1.0 + 0.05 * noise2(plan.seed ^ 9, x / 40.0, y / 40.0),
            );
            if inside {
                croft
            } else {
                let t = ((f64::from(near[k]) - 14.0) / 30.0).clamp(0.0, 1.0);
                mixc(croft, field(plan.seed, x, y), t)
            }
        }
        Kind::Water => mixc(
            hex(0x5f9aa8),
            hex(0x2e5f7c),
            (f64::from(g.depth[k]) / 6.0).min(1.0),
        ),
        Kind::Street | Kind::Gate => {
            let paved = plan
                .streets
                .get(usize::from(g.street[k]).saturating_sub(1))
                .is_some_and(|s| s.paved);
            if paved {
                hex(0xb4aa98)
            } else {
                hex(0xc7ad84)
            }
        }
        Kind::Square => hex(0xd4c8b0),
        Kind::Green => hex(0x8cad5c),
        Kind::Bridge => hex(0x9a8f80),
        Kind::Wall | Kind::WaterGate => hex(0x5c5752),
        Kind::Front => hex(0xc5b494),
        Kind::Yard => hex(0xb8a37e),
        Kind::Garden => {
            if floor_i(y / SQUARE_M) % 2 == 0 {
                hex(0x7a9a4e)
            } else {
                hex(0x86a458)
            }
        }
        Kind::Churchyard => hex(0x86a664),
        Kind::Croft => hex(0x93b060),
        Kind::Bailey => hex(0xc3bcaa),
        Kind::Building | Kind::Plot => hex(0x998877),
    };
    scale(c, 1.0 + 0.04 * n)
}

/// Hillshade factor from the grid height field (light from the north-west).
fn shade(g: &PlanGrid, i: i64, j: i64) -> f64 {
    let h = |a: i64, b: i64| g.idx(a, b).map_or(0.0, |k| f64::from(g.height[k]));
    let dx = h(i + 1, j) - h(i - 1, j);
    let dy = h(i, j + 1) - h(i, j - 1);
    (1.0 - (dx + dy) * 0.25).clamp(0.8, 1.15)
}

/// The part of the grid worth drawing: buildings, wall and plots with a
/// field margin, in local cells `(i0, j0, i1, j1)`.
fn extent(plan: &TownPlan, margin: i64) -> (i64, i64, i64, i64) {
    let g = &plan.grid;
    let (mut i0, mut j0, mut i1, mut j1) = (g.w, g.h, 0, 0);
    for (k, (&kind, &b)) in g.kind.iter().zip(&g.building).enumerate() {
        if b != 0 || matches!(kind, Kind::Wall | Kind::Square | Kind::Green | Kind::Bailey) {
            let (i, j) = g.ij(k);
            (i0, j0, i1, j1) = (i0.min(i), j0.min(j), i1.max(i + 1), j1.max(j + 1));
        }
    }
    if i1 <= i0 {
        return (0, 0, g.w, g.h);
    }
    (
        (i0 - margin).max(0),
        (j0 - margin).max(0),
        (i1 + margin).min(g.w),
        (j1 + margin).min(g.h),
    )
}

/// Pixels per square that make the cropped plan at least `target` pixels
/// on its longer side (at least 2).
#[must_use]
pub fn auto_px(plan: &TownPlan, target: u32) -> u32 {
    let (i0, j0, i1, j1) = extent(plan, 48);
    let long = (i1 - i0).max(j1 - j0).max(1);
    clamp_u32((i64::from(target) + long - 1) / long).clamp(2, 12)
}

/// Renders the plan at `px` pixels per square, cropped to the built area
/// with a margin of fields.
#[must_use]
pub fn plan_png(plan: &TownPlan, px: u32) -> Rgba {
    let g = &plan.grid;
    let p = i64::from(px.max(1));
    let (ci0, cj0, ci1, cj1) = extent(plan, 48);
    let (w, h) = (clamp_u32((ci1 - ci0) * p), clamp_u32((cj1 - cj0) * p));
    let mut img = Rgba::new(w, h);
    let pf = f64::from(px.max(1));
    let near = built_distance(g);
    let bid = |a: i64, b: i64| g.idx(a, b).map_or(0, |k| g.building[k]);
    let plot = |a: i64, b: i64| g.idx(a, b).map_or(0, |k| g.plot[k]);
    #[allow(clippy::cast_precision_loss)]
    let (oi, oj) = (ci0 as f64, cj0 as f64);
    for py in 0..h {
        for pxx in 0..w {
            let (fx, fy) = (
                oi + (f64::from(pxx) + 0.5) / pf,
                oj + (f64::from(py) + 0.5) / pf,
            );
            let (i, j) = (floor_i(fx), floor_i(fy));
            #[allow(clippy::cast_precision_loss)]
            let (x, y) = (
                ((g.gx0 as f64) + fx) * SQUARE_M,
                ((g.gy0 as f64) + fy) * SQUARE_M,
            );
            let mut c = ground(plan, &near, i, j, x, y);
            let kind = g.kind_at(i, j);
            if matches!(
                kind,
                Kind::Open
                    | Kind::Garden
                    | Kind::Green
                    | Kind::Yard
                    | Kind::Churchyard
                    | Kind::Croft
            ) {
                c = scale(c, shade(g, i, j));
            }
            if bid(i, j) == 0 && kind != Kind::Water {
                let (sx, sy) = (floor_i(fx - 0.9), floor_i(fy - 0.9));
                if bid(sx, sy) != 0 {
                    c = scale(c, 0.68);
                }
            }
            let edge_x = (fx - fx.floor()) < 1.0 / pf;
            let edge_y = (fy - fy.floor()) < 1.0 / pf;
            let here = plot(i, j);
            let fence = (edge_x && plot(i - 1, j) != here) || (edge_y && plot(i, j - 1) != here);
            if here != 0 && bid(i, j) == 0 && fence {
                c = scale(c, 0.72);
            }
            img.set(
                pxx,
                py,
                [round_u8(c[0]), round_u8(c[1]), round_u8(c[2]), 255],
            );
        }
    }
    for b in &plan.buildings {
        draw_roof(&mut img, g, b, p, (ci0, cj0));
    }
    draw_scale_bar(&mut img, pf);
    img
}

fn roof_pixel(b: &Building, base: Rgb, t: f64, stripe: i64, ridge_w: f64) -> Rgb {
    match b.function {
        F::Stall => {
            if stripe % 2 == 0 {
                hex(0xc23b2e)
            } else {
                hex(0xe8dfc8)
            }
        }
        F::Dock => {
            if stripe % 3 == 0 {
                scale(base, 0.8)
            } else {
                base
            }
        }
        _ => {
            if (t - 0.5).abs() < ridge_w {
                base
            } else if t < 0.5 {
                scale(base, 1.12)
            } else {
                scale(base, 0.86)
            }
        }
    }
}

fn draw_roof(img: &mut Rgba, g: &PlanGrid, b: &Building, p: i64, (ci, cj): (i64, i64)) {
    let r = b.rect;
    let (x0, y0) = ((r.x0 - g.gx0 - ci) * p, (r.y0 - g.gy0 - cj) * p);
    let (x1, y1) = ((r.x1 - g.gx0 - ci) * p, (r.y1 - g.gy0 - cj) * p);
    let base = roof(b);
    let horizontal_ridge = r.w() >= r.h();
    #[allow(clippy::cast_precision_loss)]
    let span = if horizontal_ridge { y1 - y0 } else { x1 - x0 } as f64;
    let ridge_w = 0.5 / span.max(1.0);
    for y in y0..y1 {
        for x in x0..x1 {
            let (Ok(ux), Ok(uy)) = (u32::try_from(x), u32::try_from(y)) else {
                continue;
            };
            if ux >= img.width || uy >= img.height {
                continue;
            }
            let (off, stripe) = if horizontal_ridge {
                (y - y0, (y - y0) / p.max(1))
            } else {
                (x - x0, (x - x0) / p.max(1))
            };
            #[allow(clippy::cast_precision_loss)]
            let t = (off as f64 + 0.5) / span.max(1.0);
            let c = roof_pixel(b, base, t, stripe, ridge_w);
            let border = x == x0 || y == y0 || x == x1 - 1 || y == y1 - 1;
            let c = if border { scale(c, 0.55) } else { c };
            img.set(
                ux,
                uy,
                [round_u8(c[0]), round_u8(c[1]), round_u8(c[2]), 255],
            );
        }
    }
}

fn draw_scale_bar(img: &mut Rgba, pf: f64) {
    let len = crate::num::round_u32(100.0 / SQUARE_M * pf);
    let (x0, y0) = (20u32, img.height.saturating_sub(30));
    for x in 0..len {
        for y in 0..8 {
            let seg = (x * 4 / len.max(1)).is_multiple_of(2);
            let c = if seg {
                [30, 28, 26, 255]
            } else {
                [240, 236, 226, 255]
            };
            if x0 + x < img.width && y0 + y < img.height {
                img.set(x0 + x, y0 + y, c);
            }
        }
    }
}
