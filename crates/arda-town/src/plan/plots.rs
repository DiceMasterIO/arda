//! Burgage plots along street frontages.
//!
//! Each street side is walked in half-square steps. The outward normal is
//! snapped to a grid axis, and every grid column along the frontage gets its
//! front cell (the first free cell beside the street). Runs of columns are
//! cut into plots of realistic width (`Params::plot_w`), then claimed in
//! order of value (nearest the market first), each column reaching back to a
//! common rear line. Curved streets give staggered fronts and slanted runs,
//! as in real towns, while every plot stays on the tactical lattice.

use super::grid::{Kind, PlanGrid, Side, SQUARE_M};
use super::params::Params;
use super::types::{MarketSquare, Street, StreetClass};
use crate::geom::{self, Vec2};
use crate::rng::Rng;

/// One grid column of a plot: a front cell and a length along the axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Col {
    /// Front cell, local grid coordinates.
    pub front: (i64, i64),
    /// Cells from the front towards the back.
    pub len: i64,
}

/// A claimed plot before function assignment.
#[derive(Debug, Clone)]
pub struct RawPlot {
    /// Street index, or `None` for the market square.
    pub street: Option<usize>,
    /// Street class (the square counts as main).
    pub class: StreetClass,
    /// Direction from the street into the plot.
    pub axis: Side,
    /// Columns ordered along the frontage.
    pub cols: Vec<Col>,
    /// Lower is more valuable.
    pub value: f64,
    /// Frontage run id (plots in one run are neighbours).
    pub run: u32,
    /// Position within the run.
    pub order: u32,
}

impl RawPlot {
    /// Every cell of the plot.
    #[must_use]
    pub fn cells(&self) -> Vec<(i64, i64)> {
        let (dx, dy) = self.axis.step();
        let mut out = Vec::new();
        for c in &self.cols {
            for k in 0..c.len {
                out.push((c.front.0 + dx * k, c.front.1 + dy * k));
            }
        }
        out
    }

    /// Mean column length in squares.
    #[must_use]
    pub fn mean_len(&self) -> f64 {
        let n = self.cols.len().max(1);
        #[allow(clippy::cast_precision_loss)]
        let s = self.cols.iter().map(|c| c.len).sum::<i64>() as f64;
        s / n as f64
    }
}

/// Whether the axis runs north–south (columns indexed by x).
const fn vertical(a: Side) -> bool {
    matches!(a, Side::North | Side::South)
}

fn frontage_u(a: Side, c: (i64, i64)) -> i64 {
    if vertical(a) {
        c.0
    } else {
        c.1
    }
}

fn find_front(g: &PlanGrid, start: (i64, i64), a: Side) -> Option<(i64, i64)> {
    let (dx, dy) = a.step();
    let mut c = start;
    for _ in 0..4 {
        let k = g.kind_at(c.0, c.1);
        if k.public() || k == Kind::Wall {
            c = (c.0 + dx, c.1 + dy);
        } else {
            break;
        }
    }
    if !g.claimable(c.0, c.1) {
        return None;
    }
    for _ in 0..3 {
        let b = (c.0 - dx, c.1 - dy);
        if g.claimable(b.0, b.1) {
            c = b;
        } else {
            break;
        }
    }
    let behind = g.kind_at(c.0 - dx, c.1 - dy);
    matches!(behind, Kind::Street | Kind::Square | Kind::Green).then_some(c)
}

/// Ordered frontage columns on one side of a polyline, split into runs.
fn frontage(g: &PlanGrid, pts: &[Vec2], hw: f64, side: f64) -> Vec<(Side, Vec<(i64, i64)>)> {
    let samples = geom::resample(pts, SQUARE_M * 0.5);
    let n = samples.len();
    let mut runs = Vec::new();
    let mut cur: Vec<(i64, i64)> = Vec::new();
    let mut axis: Option<Side> = None;
    for i in 0..n {
        let t = (samples[(i + 1).min(n - 1)] - samples[i.saturating_sub(1)]).norm();
        let nrm = t.perp() * side;
        let a = Side::of_dir(nrm);
        if axis != Some(a) {
            if let Some(ax) = axis {
                runs.push((ax, std::mem::take(&mut cur)));
            }
            axis = Some(a);
        }
        let e = samples[i] + nrm * (hw + 0.4 * SQUARE_M);
        match find_front(g, g.cell_of(e), a) {
            Some(f) => {
                let u = frontage_u(a, f);
                match cur.last() {
                    Some(&l) if frontage_u(a, l) == u => {}
                    Some(&l) if (frontage_u(a, l) - u).abs() > 1 => {
                        runs.push((a, std::mem::take(&mut cur)));
                        cur.push(f);
                    }
                    _ => cur.push(f),
                }
            }
            None => {
                if !cur.is_empty() {
                    runs.push((a, std::mem::take(&mut cur)));
                }
            }
        }
    }
    if let Some(ax) = axis {
        runs.push((ax, cur));
    }
    runs.retain(|(_, c)| c.len() >= 3);
    // A run must advance monotonically along u.
    for (a, c) in &mut runs {
        let ax = *a;
        let mut seen = std::collections::BTreeSet::new();
        c.retain(|&f| seen.insert(frontage_u(ax, f)));
    }
    runs
}

struct Cand {
    street: Option<usize>,
    class: StreetClass,
    axis: Side,
    fronts: Vec<(i64, i64)>,
    depth: i64,
    value: f64,
    run: u32,
    order: u32,
}

fn chop(fronts: &[(i64, i64)], w: (i32, i32), rng: &mut Rng) -> Vec<Vec<(i64, i64)>> {
    let mut out: Vec<Vec<(i64, i64)>> = Vec::new();
    let mut i = 0usize;
    while i < fronts.len() {
        let want = usize::try_from(rng.range(w.0, w.1)).unwrap_or(4);
        let rem = fronts.len() - i;
        let lo = usize::try_from(w.0).unwrap_or(3);
        let take = if rem < want + lo { rem } else { want };
        if take < lo.saturating_sub(1) {
            if let Some(last) = out.last_mut() {
                if last.len() + take <= usize::try_from(w.1).unwrap_or(6) + 2 {
                    last.extend_from_slice(&fronts[i..]);
                }
            }
            break;
        }
        out.push(fronts[i..i + take].to_vec());
        i += take;
    }
    out
}

/// Inputs for plot generation.
pub struct PlotInputs<'a> {
    /// Streets.
    pub streets: &'a [Street],
    /// Market square.
    pub square: Option<&'a MarketSquare>,
    /// Market centre.
    pub market: Vec2,
    /// Wall ring, if walled.
    pub ring: Option<&'a [Vec2]>,
    /// Tier parameters.
    pub params: &'a Params,
    /// Plan seed.
    pub seed: u64,
}

/// Generates and claims plots; returns them in claim order.
pub fn generate(g: &mut PlanGrid, inp: &PlotInputs<'_>) -> Vec<RawPlot> {
    let p = inp.params;
    let mut cands: Vec<Cand> = Vec::new();
    let mut run_id = 0u32;
    let mut push_runs = |runs: Vec<(Side, Vec<(i64, i64)>)>,
                         street: Option<usize>,
                         class: StreetClass,
                         cands: &mut Vec<Cand>,
                         g: &PlanGrid| {
        for (axis, fronts) in runs {
            run_id += 1;
            let mut rng = Rng::keyed(inp.seed, 0x9107, u64::from(run_id));
            let w = if street.is_none() {
                p.square_plot_w
            } else {
                p.plot_w
            };
            for (order, part) in chop(&fronts, w, &mut rng).into_iter().enumerate() {
                let mid = part[part.len() / 2];
                let c = g.centre(mid.0, mid.1);
                let penalty = match class {
                    _ if street.is_none() => -1000.0,
                    StreetClass::Main => 0.0,
                    StreetClass::Lane => 25.0,
                    StreetClass::Alley => 45.0,
                    StreetClass::Intramural => 60.0,
                };
                let outside = inp.ring.is_some_and(|r| !geom::inside(r, c));
                let d = c.dist(inp.market);
                let value = if outside { d * 1.4 + 80.0 } else { d } + penalty;
                cands.push(Cand {
                    street,
                    class,
                    axis,
                    fronts: part,
                    depth: i64::from(rng.range(p.depth.0, p.depth.1)),
                    value,
                    run: run_id,
                    order: u32::try_from(order).unwrap_or(0),
                });
            }
        }
    };
    if let Some(sq) = inp.square {
        let mut ring = sq.polygon.clone();
        ring.push(sq.polygon[0]);
        let probe = ring[0].lerp(ring[1], 0.5);
        let t = (ring[1] - ring[0]).norm();
        let side = if geom::inside(&sq.polygon, probe + t.perp() * 1.0) {
            -1.0
        } else {
            1.0
        };
        let runs = frontage(g, &ring, 0.0, side);
        push_runs(runs, None, StreetClass::Main, &mut cands, g);
    }
    for (si, s) in inp.streets.iter().enumerate() {
        for side in [-1.0, 1.0] {
            let runs = frontage(g, &s.points, s.width_m * 0.5, side);
            push_runs(runs, Some(si), s.class, &mut cands, g);
        }
    }
    cands.sort_by(|a, b| a.value.total_cmp(&b.value).then(a.run.cmp(&b.run)));
    let mut out = Vec::new();
    for c in cands {
        if let Some(rp) = claim(g, &c, p) {
            let id = u32::try_from(out.len() + 1).unwrap_or(u32::MAX);
            for (i, j) in rp.cells() {
                if let Some(k) = g.idx(i, j) {
                    g.kind[k] = Kind::Plot;
                    g.plot[k] = id;
                }
            }
            out.push(rp);
        }
    }
    out
}

fn claim(g: &PlanGrid, c: &Cand, p: &Params) -> Option<RawPlot> {
    let a = c.axis;
    let (dx, dy) = a.step();
    // Keep the longest contiguous stretch whose fronts are still free.
    let mut best: Vec<(i64, i64)> = Vec::new();
    let mut cur: Vec<(i64, i64)> = Vec::new();
    for &f in &c.fronts {
        let contiguous = cur
            .last()
            .is_none_or(|&l| (frontage_u(a, l) - frontage_u(a, f)).abs() == 1);
        if g.claimable(f.0, f.1) && contiguous {
            cur.push(f);
        } else {
            if cur.len() > best.len() {
                best = std::mem::take(&mut cur);
            }
            cur.clear();
            if g.claimable(f.0, f.1) {
                cur.push(f);
            }
        }
    }
    if cur.len() > best.len() {
        best = cur;
    }
    let min_w = usize::try_from((p.plot_w.0 - 1).max(3)).unwrap_or(3);
    let min_w = if c.street.is_none() {
        min_w.min(3)
    } else {
        min_w
    };
    if best.len() < min_w {
        return None;
    }
    let along = |f: (i64, i64)| f.0 * dx + f.1 * dy;
    let fmax = best.iter().map(|&f| along(f)).max().unwrap_or(0);
    let mut cols = Vec::with_capacity(best.len());
    for &f in &best {
        let target = fmax + c.depth - along(f);
        let mut len = 0;
        while len < target && g.claimable(f.0 + dx * len, f.1 + dy * len) {
            len += 1;
        }
        cols.push(Col { front: f, len });
    }
    let mut lens: Vec<i64> = cols.iter().map(|c| c.len).collect();
    lens.sort_unstable();
    let median = lens[lens.len() / 2];
    if median < 6 {
        return None;
    }
    // Cut at ragged stubs and keep the longest stretch of full columns.
    let mut parts: Vec<Vec<Col>> = vec![Vec::new()];
    for col in cols {
        if col.len >= 3 {
            if let Some(last) = parts.last_mut() {
                last.push(col);
            }
        } else {
            parts.push(Vec::new());
        }
    }
    let cols = parts.into_iter().max_by_key(Vec::len).unwrap_or_default();
    if cols.len() < min_w {
        return None;
    }
    Some(RawPlot {
        street: c.street,
        class: c.class,
        axis: a,
        cols,
        value: c.value,
        run: c.run,
        order: c.order,
    })
}
