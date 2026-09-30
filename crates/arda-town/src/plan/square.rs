//! Market square and castle ward shapes. Medieval markets are widened
//! streets (a spindle along the through road) or triangles where roads meet;
//! village greens are rough polygons between the arms.

use super::focal::Arm;
use super::grid::{PlanGrid, Side, SquareRect, SQUARE_M};
use super::params::Params;
use super::types::{CastleWard, Focal, FocalKind, MarketSquare};
use crate::geom::{self, v2, Vec2};
use crate::num::floor_i;
use crate::rng::Rng;
use crate::site::Tier;

fn heading(a: &Arm) -> Vec2 {
    let l = geom::length(&a.points);
    (geom::at(&a.points, l.min(50.0)).0 - a.points[0]).norm()
}

/// A spindle-shaped widened street centred on `c` along `axis`.
fn lens(c: Vec2, axis: Vec2, len: f64, wid: f64, street_w: f64, rng: &mut Rng) -> Vec<Vec2> {
    let n = 8;
    let side = axis.perp();
    let mut top = Vec::new();
    let mut bot = Vec::new();
    for k in 0..=n {
        let t = -1.0 + 2.0 * f64::from(k) / f64::from(n);
        let bulge = (1.0 - t * t).max(0.0).sqrt();
        let jitter = |r: &mut Rng| r.range_f(0.9, 1.1);
        let hw1 = street_w * 0.5 + (wid * 0.5 - street_w * 0.5) * bulge * jitter(rng);
        let hw2 = street_w * 0.5 + (wid * 0.5 - street_w * 0.5) * bulge * jitter(rng);
        let base = c + axis * (t * len * 0.5);
        top.push(base + side * hw1);
        bot.push(base - side * hw2);
    }
    bot.reverse();
    top.extend(bot);
    top
}

/// A green: a rough polygon whose corners sit between the arms.
fn green(c: Vec2, dirs: &[Vec2], r: f64, rng: &mut Rng) -> Vec<Vec2> {
    let mut angles: Vec<Vec2> = dirs.to_vec();
    angles.sort_by(|a, b| crate::geom::area(&[Vec2::default(), *a, *b]).total_cmp(&0.0));
    let n = 7;
    (0..n)
        .map(|k| {
            let (s, co) = geom::sin_cos(2.0 * geom::PI * f64::from(k) / f64::from(n) + 0.3);
            let d = v2(co, s);
            // Stretch towards the arms so the green follows the roads.
            let pull = angles.iter().map(|a| a.dot(d).max(0.0)).fold(0.0, f64::max);
            c + d * (r * (0.75 + 0.45 * pull) * rng.range_f(0.9, 1.1))
        })
        .collect()
}

/// The market square polygon.
#[must_use]
pub fn market(focal: &Focal, arms: &[Arm], params: &Params, rng: &mut Rng) -> MarketSquare {
    let c = focal.market;
    let (len, wid) = params.square;
    let dirs: Vec<Vec2> = arms.iter().map(heading).collect();
    if params.green {
        return MarketSquare {
            polygon: green(c, &dirs, len * 0.5, rng),
            green: true,
        };
    }
    let axis = match focal.kind {
        FocalKind::Castle => (focal.feature - c).norm(),
        FocalKind::Harbour => (c - focal.feature).norm().perp(),
        _ => {
            let mut best = (2.0, v2(1.0, 0.0));
            for i in 0..dirs.len() {
                for j in i + 1..dirs.len() {
                    let d = dirs[i].dot(dirs[j]);
                    if d < best.0 {
                        best = (d, (dirs[i] - dirs[j]).norm());
                    }
                }
            }
            if dirs.len() == 1 {
                dirs[0]
            } else {
                best.1
            }
        }
    };
    MarketSquare {
        polygon: lens(c, axis, len, wid, params.main_w, rng),
        green: false,
    }
}

/// Keep-out radius around the market for lanes, metres.
#[must_use]
pub fn radius(sq: &MarketSquare, c: Vec2) -> f64 {
    sq.polygon.iter().map(|p| p.dist(c)).fold(0.0, f64::max)
}

/// The castle ward around the keep, snapped to the grid.
#[must_use]
pub fn castle(focal: &Focal, grid: &PlanGrid, tier: Tier) -> CastleWard {
    let (w, h) = match tier {
        Tier::City => (32, 28),
        _ => (26, 24),
    };
    let (ci, cj) = grid.cell_of(focal.feature);
    let (x0, y0) = (grid.gx0 + ci - w / 2, grid.gy0 + cj - h / 2);
    CastleWard {
        bailey: SquareRect {
            x0,
            y0,
            x1: x0 + w,
            y1: y0 + h,
        },
        gate: Side::of_dir(focal.market - focal.feature),
    }
}

/// World centre of the castle gate passage.
#[must_use]
pub fn castle_gate(ward: &CastleWard) -> (i64, i64) {
    let b = ward.bailey;
    let (mx, my) = ((b.x0 + b.x1) / 2, (b.y0 + b.y1) / 2);
    match ward.gate {
        Side::North => (mx, b.y0 - 1),
        Side::South => (mx, b.y1),
        Side::East => (b.x1, my),
        Side::West => (b.x0 - 1, my),
    }
}

/// Global square of a world point.
#[must_use]
pub fn square_of(p: Vec2) -> (i64, i64) {
    (floor_i(p.x / SQUARE_M), floor_i(p.y / SQUARE_M))
}
