//! Buildings that do not stand on burgage plots: the keep and barracks in
//! the castle bailey, the market hall and stalls in the square, and docks
//! and boathouses on the shore.

use super::grid::{Kind, PlanGrid, Side, SquareRect};
use super::place::Draft;
use super::types::{CastleWard, Door};
use crate::function::BuildingFunction as F;
use crate::geom::Vec2;
use crate::rng::Rng;
use crate::site::Tier;

fn all_kind(g: &PlanGrid, r: &SquareRect, margin: i64, ok: &[Kind]) -> bool {
    let rr = r.grown(margin);
    (rr.y0..rr.y1).all(|y| {
        (rr.x0..rr.x1).all(|x| {
            g.gidx(x, y)
                .is_some_and(|k| ok.contains(&g.kind[k]) && g.building[k] == 0)
        })
    })
}

fn mark(g: &mut PlanGrid, r: &SquareRect, tag: u32) {
    for y in r.y0..r.y1 {
        for x in r.x0..r.x1 {
            if let Some(k) = g.gidx(x, y) {
                g.building[k] = tag;
            }
        }
    }
}

/// Middle square of the rectangle's `side` row, and the door on it.
fn door_on(r: &SquareRect, side: Side) -> Door {
    let (mx, my) = ((r.x0 + r.x1) / 2, (r.y0 + r.y1) / 2);
    let (x, y) = match side {
        Side::North => (mx, r.y0),
        Side::South => (mx, r.y1 - 1),
        Side::East => (r.x1 - 1, my),
        Side::West => (r.x0, my),
    };
    Door { x, y, side }
}

/// A draft with a door on `front` and a temporary grid mark.
fn draft(g: &mut PlanGrid, f: F, rect: SquareRect, front: Side, ancillary: bool) -> Draft {
    mark(g, &rect, u32::MAX);
    let doors = if f.walled() {
        vec![door_on(&rect, front)]
    } else {
        Vec::new()
    };
    Draft {
        function: f,
        plots: Vec::new(),
        rect,
        front,
        doors,
        ancillary,
    }
}

/// Keep at the far end of the bailey, barracks along one side.
pub fn castle(g: &mut PlanGrid, ward: &CastleWard, want_barracks: bool) -> Vec<Draft> {
    let b = ward.bailey;
    let gate = ward.gate;
    let (kw, kd) = (12, 10);
    let mut out = Vec::new();
    let keep = match gate {
        Side::North => SquareRect {
            x0: b.x0 + (b.w() - kw) / 2,
            y0: b.y1 - 1 - kd,
            x1: b.x0 + (b.w() + kw) / 2,
            y1: b.y1 - 1,
        },
        Side::South => SquareRect {
            x0: b.x0 + (b.w() - kw) / 2,
            y0: b.y0 + 1,
            x1: b.x0 + (b.w() + kw) / 2,
            y1: b.y0 + 1 + kd,
        },
        Side::West => SquareRect {
            x0: b.x1 - 1 - kd,
            y0: b.y0 + (b.h() - kw) / 2,
            x1: b.x1 - 1,
            y1: b.y0 + (b.h() + kw) / 2,
        },
        Side::East => SquareRect {
            x0: b.x0 + 1,
            y0: b.y0 + (b.h() - kw) / 2,
            x1: b.x0 + 1 + kd,
            y1: b.y0 + (b.h() + kw) / 2,
        },
    };
    out.push(draft(g, F::Keep, keep, gate, false));
    if want_barracks {
        // Along the wall to the left of the gate, clear of the keep.
        let bar = match gate {
            Side::North | Side::South => {
                let (y0, y1) = if gate == Side::North {
                    (b.y0 + 1, keep.y0 - 2)
                } else {
                    (keep.y1 + 2, b.y1 - 1)
                };
                SquareRect {
                    x0: b.x0 + 1,
                    y0,
                    x1: b.x0 + 7,
                    y1,
                }
            }
            Side::East | Side::West => {
                let (x0, x1) = if gate == Side::West {
                    (b.x0 + 1, keep.x0 - 2)
                } else {
                    (keep.x1 + 2, b.x1 - 1)
                };
                SquareRect {
                    x0,
                    y0: b.y0 + 1,
                    x1,
                    y1: b.y0 + 7,
                }
            }
        };
        let facing = match gate {
            Side::North | Side::South => Side::East,
            _ => Side::South,
        };
        if bar.w() >= 5 && bar.h() >= 5 {
            out.push(draft(g, F::Barracks, bar, facing, false));
        }
    }
    out
}

/// Market hall near the square's centre and stalls in aisled rows.
pub fn square(
    g: &mut PlanGrid,
    centre: Vec2,
    halls: u32,
    stalls: u32,
    tier: Tier,
    rng: &mut Rng,
) -> Vec<Draft> {
    let mut out = Vec::new();
    let (ci, cj) = g.cell_of(centre);
    let (cx, cy) = (g.gx0 + ci, g.gy0 + cj);
    let ok = [Kind::Square, Kind::Green];
    for _ in 0..halls {
        let ((w0, w1), (d0, d1)) = F::MarketHall.size(tier);
        let (w, d) = (i64::from(rng.range(w0, w1)), i64::from(rng.range(d0, d1)));
        let mut placed = false;
        'search: for r in 0..30i64 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    for (ww, dd) in [(w, d), (d, w)] {
                        let rect = SquareRect {
                            x0: cx + dx - ww / 2,
                            y0: cy + dy - dd / 2,
                            x1: cx + dx - ww / 2 + ww,
                            y1: cy + dy - dd / 2 + dd,
                        };
                        if all_kind(g, &rect, 2, &ok) {
                            let front = if ww >= dd { Side::South } else { Side::East };
                            let mut dr = draft(g, F::MarketHall, rect, front, false);
                            dr.doors.push(door_on(&rect, front.opposite()));
                            out.push(dr);
                            placed = true;
                            break 'search;
                        }
                    }
                }
            }
        }
        if !placed {
            break;
        }
    }
    let mut spots: Vec<(i64, SquareRect)> = Vec::new();
    for j in 0..g.h {
        for i in 0..g.w {
            let (x, y) = (g.gx0 + i, g.gy0 + j);
            if x.rem_euclid(3) != 0 || y.rem_euclid(4) != 0 {
                continue;
            }
            let rect = SquareRect {
                x0: x,
                y0: y,
                x1: x + 2,
                y1: y + 2,
            };
            if all_kind(g, &rect, 1, &ok) {
                let d = (x - cx).pow(2) + (y - cy).pow(2);
                spots.push((d, rect));
            }
        }
    }
    spots.sort_by_key(|s| (s.0, s.1.x0, s.1.y0));
    for (_, rect) in spots.into_iter().take(usize::try_from(stalls).unwrap_or(0)) {
        if all_kind(g, &rect, 1, &ok) {
            let front = if rect.x0 < cx { Side::East } else { Side::West };
            out.push(draft(g, F::Stall, rect, front, false));
        }
    }
    out
}

/// Docks and boathouses on the shore nearest `target`.
pub fn shore(
    g: &mut PlanGrid,
    target: Vec2,
    docks: u32,
    boathouses: u32,
    rng: &mut Rng,
) -> Vec<Draft> {
    let mut out = Vec::new();
    // Shore cells: open land with water on exactly one side.
    let mut shore: Vec<(f64, i64, i64, Side)> = Vec::new();
    for j in 1..g.h - 1 {
        for i in 1..g.w - 1 {
            if g.kind_at(i, j) != Kind::Open {
                continue;
            }
            for s in [Side::North, Side::East, Side::South, Side::West] {
                let (dx, dy) = s.step();
                if g.kind_at(i + dx, j + dy) == Kind::Water {
                    shore.push((g.centre(i, j).dist(target), i, j, s));
                }
            }
        }
    }
    shore.sort_by(|a, b| a.0.total_cmp(&b.0).then((a.1, a.2).cmp(&(b.1, b.2))));
    let mut want: Vec<F> = Vec::new();
    for k in 0..docks.max(boathouses) {
        if k < docks {
            want.push(F::Dock);
        }
        if k < boathouses {
            want.push(F::Boathouse);
        }
    }
    for f in want {
        let ((w0, w1), (d0, d1)) = f.size(Tier::Village);
        let (w, d) = (i64::from(rng.range(w0, w1)), i64::from(rng.range(d0, d1)));
        let placed = shore.iter().find_map(|&(_, i, j, s)| {
            let (x, y) = (g.gx0 + i, g.gy0 + j);
            // Docks reach out over water; boathouses stand inland of the shore.
            let dir = if f == F::Dock { s } else { s.opposite() };
            let (dx, dy) = dir.step();
            let start = if f == F::Dock {
                (x + dx, y + dy)
            } else {
                (x, y)
            };
            let end = (start.0 + dx * (d - 1), start.1 + dy * (d - 1));
            let rect = if dx == 0 {
                SquareRect {
                    x0: x - w / 2,
                    x1: x - w / 2 + w,
                    y0: start.1.min(end.1),
                    y1: start.1.max(end.1) + 1,
                }
            } else {
                SquareRect {
                    x0: start.0.min(end.0),
                    x1: start.0.max(end.0) + 1,
                    y0: y - w / 2,
                    y1: y - w / 2 + w,
                }
            };
            let ok: &[Kind] = if f == F::Dock {
                &[Kind::Water]
            } else {
                &[Kind::Open]
            };
            let margin = i64::from(f != F::Dock);
            all_kind(g, &rect, 0, ok)
                .then_some(())
                .filter(|()| margin == 0 || all_kind(g, &rect, 1, &[Kind::Open, Kind::Water]))
                .map(|()| (rect, s))
        });
        if let Some((rect, s)) = placed {
            // Docks face the land; boathouses face inland with the gate on the water.
            let front = s.opposite();
            let mut dr = draft(g, f, rect, front, false);
            if f == F::Boathouse {
                dr.doors.push(door_on(&rect, s));
            }
            out.push(dr);
        }
    }
    out
}
