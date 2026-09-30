//! Burns the vector plan into the grid: square, castle ward, wall band,
//! gates, water gates and streets (bridges in [`super::bridge`]).

use super::grid::{Kind, PlanGrid, SQUARE_M};
use super::square::castle_gate;
use super::types::{Bridge, CastleWard, Gate, MarketSquare, Street, StreetClass, StreetId};
use crate::geom::{self, Vec2};
use crate::num::floor_i;
use crate::site::RiverLine;

/// Cells whose centre lies within `r` metres of a polyline, sorted.
#[must_use]
pub fn cells_near(g: &PlanGrid, pts: &[Vec2], r: f64) -> Vec<usize> {
    let mut out = Vec::new();
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let lo_x = floor_i((a.x.min(b.x) - r) / SQUARE_M) - g.gx0;
        let hi_x = floor_i((a.x.max(b.x) + r) / SQUARE_M) - g.gx0;
        let lo_y = floor_i((a.y.min(b.y) - r) / SQUARE_M) - g.gy0;
        let hi_y = floor_i((a.y.max(b.y) + r) / SQUARE_M) - g.gy0;
        for j in lo_y.max(0)..=hi_y.min(g.h - 1) {
            for i in lo_x.max(0)..=hi_x.min(g.w - 1) {
                let d = geom::seg_dist(g.centre(i, j), a, b).0;
                if d <= r {
                    if let Some(k) = g.idx(i, j) {
                        out.push(k);
                    }
                }
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Paints the market square or green.
pub fn square(g: &mut PlanGrid, sq: &MarketSquare) {
    let (lo, hi) = bbox(&sq.polygon);
    let (i0, j0) = g.cell_of(lo);
    let (i1, j1) = g.cell_of(hi);
    let kind = if sq.green { Kind::Green } else { Kind::Square };
    for j in j0.max(0)..=j1.min(g.h - 1) {
        for i in i0.max(0)..=i1.min(g.w - 1) {
            if let Some(k) = g.idx(i, j) {
                if g.kind[k] == Kind::Open && geom::inside(&sq.polygon, g.centre(i, j)) {
                    g.kind[k] = kind;
                }
            }
        }
    }
}

fn bbox(p: &[Vec2]) -> (Vec2, Vec2) {
    let mut lo = Vec2 {
        x: f64::MAX,
        y: f64::MAX,
    };
    let mut hi = Vec2 {
        x: f64::MIN,
        y: f64::MIN,
    };
    for v in p {
        lo = Vec2 {
            x: lo.x.min(v.x),
            y: lo.y.min(v.y),
        };
        hi = Vec2 {
            x: hi.x.max(v.x),
            y: hi.y.max(v.y),
        };
    }
    (lo, hi)
}

/// Paints the castle: bailey, a two-square curtain and a gate passage.
pub fn castle(g: &mut PlanGrid, ward: &CastleWard) {
    let b = ward.bailey;
    let outer = b.grown(2);
    for y in outer.y0..outer.y1 {
        for x in outer.x0..outer.x1 {
            if let Some(k) = g.gidx(x, y) {
                g.kind[k] = if b.contains(x, y) {
                    Kind::Bailey
                } else {
                    Kind::Wall
                };
            }
        }
    }
    let (gx, gy) = castle_gate(ward);
    let horizontal = matches!(
        ward.gate,
        super::grid::Side::North | super::grid::Side::South
    );
    for d in -1..=1 {
        for t in 0..2 {
            let (x, y) = if horizontal {
                let y = if gy < b.y0 { b.y0 - 1 - t } else { b.y1 + t };
                (gx + d, y)
            } else {
                let x = if gx < b.x0 { b.x0 - 1 - t } else { b.x1 + t };
                (x, gy + d)
            };
            if let Some(k) = g.gidx(x, y) {
                g.kind[k] = Kind::Gate;
            }
        }
    }
}

/// Paints the wall band along a closed ring.
pub fn wall(g: &mut PlanGrid, ring: &[Vec2], thickness: f64) {
    let mut closed = ring.to_vec();
    if let Some(&f) = ring.first() {
        closed.push(f);
    }
    for k in cells_near(g, &closed, thickness * 0.5) {
        if !matches!(g.kind[k], Kind::Bailey | Kind::Gate) {
            g.kind[k] = Kind::Wall;
        }
    }
}

/// Paints streets in order and decks their crossings ([`super::bridge`]):
/// main streets bridge the water they cross, lanes only narrow channels.
pub fn streets(g: &mut PlanGrid, streets: &[Street]) -> Vec<Bridge> {
    let mut out = Vec::new();
    for s in streets {
        let id = s.id.0 + 1;
        for k in cells_near(g, &s.points, s.width_m * 0.5) {
            if g.kind[k] == Kind::Open {
                g.kind[k] = Kind::Street;
                g.street[k] = id;
            }
        }
        out.extend(super::bridge::decks(g, s));
    }
    out
}

/// An axis-aligned gatehouse where a street crosses the band at `x`: a
/// straight three-square passage through the band, flanked by two-square
/// towers, so the gate is clean even where the wall runs diagonally.
fn gatehouse(g: &mut PlanGrid, x: Vec2, tangent: Vec2, street: u16) {
    let horizontal = tangent.x.abs() >= tangent.y.abs();
    let (ci, cj) = g.cell_of(x);
    let at = |a: i64, c: i64| {
        if horizontal {
            (ci + a, cj + c)
        } else {
            (ci + c, cj + a)
        }
    };
    let (mut lo, mut hi, mut found) = (0, 0, false);
    for a in -8..=8 {
        for c in -1..=1 {
            let (i, j) = at(a, c);
            if g.kind_at(i, j) == Kind::Wall {
                lo = lo.min(a);
                hi = hi.max(a);
                found = true;
            }
        }
    }
    if !found {
        return;
    }
    for a in lo..=hi {
        for c in -3..=3 {
            let (i, j) = at(a, c);
            let Some(k) = g.idx(i, j) else { continue };
            if c.abs() <= 1 {
                g.kind[k] = Kind::Gate;
                g.street[k] = street;
            } else if matches!(g.kind[k], Kind::Open | Kind::Street | Kind::Wall) {
                g.kind[k] = Kind::Wall;
                g.street[k] = 0;
            }
        }
    }
}

/// Cuts gate passages where main streets cross the wall band, and water
/// gates where rivers do. Returns the gates.
pub fn gates(
    g: &mut PlanGrid,
    ring: &[Vec2],
    streets: &[Street],
    rivers: &[RiverLine],
) -> Vec<Gate> {
    let mut closed = ring.to_vec();
    if let Some(&f) = ring.first() {
        closed.push(f);
    }
    let mut out = Vec::new();
    for s in streets.iter().filter(|s| s.class == StreetClass::Main) {
        for x in geom::crossings(&s.points, &closed) {
            let (_, at) = geom::project(&s.points, x);
            gatehouse(g, x, geom::at(&s.points, at).1, s.id.0 + 1);
            out.push(Gate {
                point: x,
                street: Some(StreetId(s.id.0)),
            });
        }
    }
    for r in rivers {
        for x in geom::crossings(&r.points, &closed) {
            let (_, at) = geom::project(&r.points, x);
            let piece = geom::slice(&r.points, (at - 8.0).max(0.0), at + 8.0);
            let hw = (r.width_m * 0.5 - SQUARE_M).max(SQUARE_M);
            let mut any = false;
            for k in cells_near(g, &piece, hw) {
                if g.kind[k] == Kind::Wall {
                    g.kind[k] = Kind::WaterGate;
                    any = true;
                }
            }
            if any {
                out.push(Gate {
                    point: x,
                    street: None,
                });
            }
        }
    }
    out
}

/// Marks wall-band cells that lie over water, so the depth map treats
/// them as river bed (the wall stands on arches there).
#[must_use]
pub fn wall_over_water(g: &PlanGrid, water: &dyn Fn(Vec2) -> bool) -> Vec<usize> {
    (0..g.kind.len())
        .filter(|&k| {
            let (i, j) = g.ij(k);
            g.kind[k] == Kind::Wall && water(g.centre(i, j))
        })
        .collect()
}
