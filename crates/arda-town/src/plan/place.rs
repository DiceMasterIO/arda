//! Building footprints on assigned plots: frontage width, setback, depth,
//! doors facing the street, and barns behind farmhouses.
//!
//! Town houses fill their frontage (continuous street fronts); village
//! buildings stand detached with side gaps. Where a building has an
//! outbuilding, a side passage runs from the street to the yard, so every
//! door can be reached from a street (goal 45: every home can be visited).

use super::assign::Assigned;
use super::grid::{Kind, PlanGrid, Side, SquareRect};
use super::params::Params;
use super::plots::{Col, RawPlot};
use super::types::Door;
use crate::function::BuildingFunction as F;
use crate::rng::{hash_i, unit, Rng};
use crate::site::Tier;

/// A building before ids and wealth are final.
#[derive(Debug, Clone)]
pub struct Draft {
    /// Function.
    pub function: F,
    /// Raw plot indices it stands on (empty for off-plot buildings).
    pub plots: Vec<usize>,
    /// Footprint in global squares.
    pub rect: SquareRect,
    /// Side facing the street.
    pub front: Side,
    /// Doors, front first.
    pub doors: Vec<Door>,
    /// Not part of the building mix.
    pub ancillary: bool,
}

/// Merges a group of plots into one: concatenated columns.
#[must_use]
pub fn merged(plots: &[RawPlot], group: &[usize]) -> RawPlot {
    let mut base = plots[group[0]].clone();
    for &i in &group[1..] {
        base.cols.extend_from_slice(&plots[i].cols);
    }
    base
}

fn along(a: Side, c: (i64, i64)) -> i64 {
    let (dx, dy) = a.step();
    c.0 * dx + c.1 * dy
}

fn cell_at(a: Side, col: &Col, t: i64) -> (i64, i64) {
    let (dx, dy) = a.step();
    let k = t - along(a, col.front);
    (col.front.0 + dx * k, col.front.1 + dy * k)
}

/// Local-grid rectangle spanning columns `cols` between along-coordinates
/// `t0..t1`, as a global [`SquareRect`].
fn rect_of(g: &PlanGrid, a: Side, cols: &[Col], t0: i64, t1: i64) -> SquareRect {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for c in cols {
        for t in [t0, t1 - 1] {
            let (i, j) = cell_at(a, c, t);
            xs.push(i);
            ys.push(j);
        }
    }
    let (x0, x1) = (
        xs.iter().min().copied().unwrap_or(0),
        xs.iter().max().copied().unwrap_or(0),
    );
    let (y0, y1) = (
        ys.iter().min().copied().unwrap_or(0),
        ys.iter().max().copied().unwrap_or(0),
    );
    SquareRect {
        x0: g.gx0 + x0,
        y0: g.gy0 + y0,
        x1: g.gx0 + x1 + 1,
        y1: g.gy0 + y1 + 1,
    }
}

/// Whether a function lines the street edge to edge in towns.
fn row_building(f: F) -> bool {
    matches!(
        f,
        F::House | F::Bakery | F::Apothecary | F::Workshop | F::Tavern | F::Brewery | F::Cottage
    )
}

fn symmetric(f: F) -> bool {
    matches!(
        f,
        F::Temple
            | F::Shrine
            | F::Inn
            | F::MarketHall
            | F::Keep
            | F::Warehouse
            | F::Library
            | F::Barn
            | F::Stable
            | F::Manor
            | F::Barracks
            | F::Mill
    )
}

/// Whether a town row house leaves an eaves-drip gap (ambitus) in its
/// plot's last column: a deterministic share of street-front plots, by a
/// hash of the plot's first front square, never on the market square and
/// never below a three-square frontage. The column stays plot ground, so
/// it opens a one-square alley to the yard between two houses.
fn ambitus(g: &PlanGrid, plot: &RawPlot, params: &Params, passage: i64, wmax: i64) -> bool {
    let Some(c) = plot.cols.first() else {
        return false;
    };
    params.ambitus > 0.0
        && plot.street.is_some()
        && passage == 0
        && wmax > 3
        && i64::try_from(plot.cols.len()).unwrap_or(0) == wmax
        && unit(hash_i(AMBITUS, g.gx0 + c.front.0, g.gy0 + c.front.1)) < params.ambitus
}

/// Hash key of the ambitus draw.
const AMBITUS: u64 = 0xA3B1_7C50;

/// Places one assigned building (and its barn) on its merged plot.
#[must_use]
pub fn place(
    g: &PlanGrid,
    plots: &[RawPlot],
    a: &Assigned,
    params: &Params,
    tier: Tier,
    rng: &mut Rng,
) -> Vec<Draft> {
    let plot = merged(plots, &a.plots);
    let axis = plot.axis;
    let n = i64::try_from(plot.cols.len()).unwrap_or(0);
    let f = a.function;
    let ((w0, w1), (d0, d1)) = f.size(tier);
    let barn = f == F::Farmhouse;
    let urban = matches!(tier, Tier::Town | Tier::City);
    let gap = i64::from(params.side_gap);
    let passage = i64::from(barn && gap == 0);
    let wmax = (n - 2 * gap - passage).max(3);
    let want = i64::from(rng.range(w0, w1));
    let gap_col = urban && row_building(f) && ambitus(g, &plot, params, passage, wmax);
    let w = if urban && row_building(f) {
        wmax - i64::from(gap_col)
    } else {
        want.min(wmax)
    };
    let k0 = if symmetric(f) || gap > 0 {
        (n - w) / 2
    } else if passage > 0 && rng.chance(0.5) {
        1
    } else {
        0
    };
    let win = &plot.cols[usize::try_from(k0).unwrap_or(0)..usize::try_from(k0 + w).unwrap_or(0)];
    // Front line and depth are measured over the whole frontage, gap
    // column included, so an ambitus only takes that column away.
    let full = &plot.cols[usize::try_from(k0).unwrap_or(0)
        ..usize::try_from(k0 + w + i64::from(gap_col)).unwrap_or(0)];
    let setback = i64::from(rng.range(params.setback.0, params.setback.1));
    let front_line = full.iter().map(|c| along(axis, c.front)).max().unwrap_or(0) + setback;
    let rear = full
        .iter()
        .map(|c| along(axis, c.front) + c.len)
        .min()
        .unwrap_or(0);
    let yard_min = if barn { 9 } else { 2 };
    let avail = rear - front_line;
    let depth = i64::from(rng.range(d0, d1))
        .min(avail - yard_min)
        .max(i64::from(d0).min(avail));
    if depth < 3 {
        return Vec::new();
    }
    let rect = rect_of(g, axis, win, front_line, front_line + depth);
    let front = axis.opposite();
    // The door is drawn over the width before any ambitus, so the gap
    // never changes what the stream draws for later buildings.
    let du = if symmetric(f) {
        w / 2
    } else {
        let drawn = w + i64::from(gap_col);
        i64::from(rng.range(1, i32::try_from((drawn - 2).max(1)).unwrap_or(1))).min((w - 2).max(1))
    };
    let door_col = win[usize::try_from(du.clamp(0, w - 1)).unwrap_or(0)];
    let (di, dj) = cell_at(axis, &door_col, front_line);
    let mut doors = vec![Door {
        x: g.gx0 + di,
        y: g.gy0 + dj,
        side: front,
    }];
    if avail - depth >= 2 && !matches!(f, F::Temple | F::MarketHall) {
        let bu = usize::try_from(rng.range(0, i32::try_from(w - 1).unwrap_or(0))).unwrap_or(0);
        let (bi, bj) = cell_at(axis, &win[bu], front_line + depth - 1);
        doors.push(Door {
            x: g.gx0 + bi,
            y: g.gy0 + bj,
            side: axis,
        });
    }
    let mut out = vec![Draft {
        function: f,
        plots: a.plots.clone(),
        rect,
        front,
        doors,
        ancillary: false,
    }];
    if barn {
        let ((bw0, bw1), (bd0, bd1)) = F::Barn.size(tier);
        let bw = i64::from(rng.range(bw0, bw1)).min(n - 2 * gap.max(1));
        let bd = i64::from(rng.range(bd0, bd1));
        let b_rear = plot
            .cols
            .iter()
            .map(|c| along(axis, c.front) + c.len)
            .min()
            .unwrap_or(0);
        let b_front = b_rear - bd;
        if bw >= 5 && b_front - (front_line + depth) >= 2 {
            let bk = (n - bw) / 2;
            let bwin =
                &plot.cols[usize::try_from(bk).unwrap_or(0)..usize::try_from(bk + bw).unwrap_or(0)];
            let brect = rect_of(g, axis, bwin, b_front, b_rear);
            let (i, j) = cell_at(axis, &bwin[bwin.len() / 2], b_front);
            out.push(Draft {
                function: F::Barn,
                plots: a.plots.clone(),
                rect: brect,
                front,
                doors: vec![Door {
                    x: g.gx0 + i,
                    y: g.gy0 + j,
                    side: front,
                }],
                ancillary: true,
            });
        }
    }
    out.retain(|d| fits(g, &d.rect, &a.plots));
    out
}

/// Every cell of the rectangle lies on one of the given plots.
fn fits(g: &PlanGrid, r: &SquareRect, plots: &[usize]) -> bool {
    (r.y0..r.y1).all(|y| {
        (r.x0..r.x1).all(|x| {
            g.gidx(x, y).is_some_and(|k| {
                g.kind[k] == Kind::Plot
                    && plots
                        .iter()
                        .any(|&p| g.plot[k] == u32::try_from(p + 1).unwrap_or(0))
            })
        })
    })
}
