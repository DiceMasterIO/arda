//! Bridges where streets cross water. Every stretch where a street's
//! centreline runs over water, between land on both sides, becomes a deck:
//! the water squares of its carriageway there, grouped into straight rows
//! along the crossing's dominant axis, each row carried on to the banks so
//! it spans the whole channel. The street's squares over water are then all
//! deck, and the street runs unbroken from bank to bank.

use super::grid::{Kind, PlanGrid, SQUARE_M};
use super::lanes::LaneSpec;
use super::raster::cells_near;
use super::types::{Bridge, Street, StreetClass};
use crate::geom::{self, Vec2};
use std::collections::BTreeMap;

/// Longest crossing a main street bridges, squares (about 100 m).
pub const MAIN_SPAN: f64 = 64.0;
/// Longest crossing a lane bridges (a footbridge), squares.
pub const LANE_SPAN: f64 = 10.0;
/// Farthest a deck row is carried on to reach a bank, squares.
const REACH: i64 = 120;

/// The longest crossing a street of `class` bridges, squares.
#[must_use]
pub fn span(class: StreetClass) -> f64 {
    if class == StreetClass::Main {
        MAIN_SPAN
    } else {
        LANE_SPAN
    }
}

/// Streets cut where they run into water they cannot bridge: a stretch
/// over the water longer than their class spans, or one they end in. The
/// pieces either side stop at the bank (pieces under 6 m are dropped).
#[must_use]
pub fn cut(g: &PlanGrid, specs: Vec<LaneSpec>) -> Vec<LaneSpec> {
    let mut out = Vec::with_capacity(specs.len());
    for s in specs {
        let pts = geom::resample(&s.points, SQUARE_M * 0.5);
        let wet: Vec<bool> = pts
            .iter()
            .map(|&p| {
                let (i, j) = g.cell_of(p);
                g.kind_at(i, j) == Kind::Water
            })
            .collect();
        let mut keep = vec![true; pts.len()];
        let mut i = 0;
        let mut any = false;
        while i < pts.len() {
            if !wet[i] {
                i += 1;
                continue;
            }
            let a = i;
            while i < pts.len() && wet[i] {
                i += 1;
            }
            #[allow(clippy::cast_precision_loss)] // sample counts are small
            let long = (i - a) as f64 * 0.5 > span(s.class);
            if a == 0 || i >= pts.len() || long {
                keep[a..i].iter_mut().for_each(|k| *k = false);
                any = true;
            }
        }
        if !any {
            out.push(s);
            continue;
        }
        let mut piece: Vec<Vec2> = Vec::new();
        for (k, &p) in pts.iter().enumerate() {
            if keep[k] {
                piece.push(p);
            }
            if (!keep[k] || k + 1 == pts.len()) && !piece.is_empty() {
                let points = std::mem::take(&mut piece);
                if geom::length(&points) >= 6.0 {
                    out.push(LaneSpec {
                        class: s.class,
                        points,
                        width_m: s.width_m,
                    });
                }
            }
        }
    }
    out
}

/// The centreline of `s`, sampled every half square.
#[must_use]
pub fn samples(s: &Street) -> Vec<Vec2> {
    geom::resample(&s.points, SQUARE_M * 0.5)
}

/// Decks every crossing of street `s` and returns them.
pub fn decks(g: &mut PlanGrid, s: &Street) -> Vec<Bridge> {
    let span = span(s.class);
    let pts = samples(s);
    let wet: Vec<bool> = pts
        .iter()
        .map(|&p| {
            let (i, j) = g.cell_of(p);
            matches!(g.kind_at(i, j), Kind::Water | Kind::Bridge)
        })
        .collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < pts.len() {
        if !wet[i] {
            i += 1;
            continue;
        }
        let a = i;
        while i < pts.len() && wet[i] {
            i += 1;
        }
        // A street ending in the water gets no deck; nor does a crossing
        // too long for its class.
        #[allow(clippy::cast_precision_loss)] // sample counts are small
        let long = (i - a) as f64 * 0.5 > span;
        if a == 0 || i >= pts.len() || long {
            continue;
        }
        if let Some(b) = deck(g, s, &pts[a - 1..=i]) {
            out.push(b);
        }
    }
    out
}

/// One deck over the stretch `run` (land sample, water samples, land
/// sample): rows along x or along y, whichever deck is smaller (so rows
/// run the short way across, square to the flow), the street's own
/// heading breaking a tie.
fn deck(g: &mut PlanGrid, s: &Street, run: &[Vec2]) -> Option<Bridge> {
    let (p0, p1) = (run[0], run[run.len() - 1]);
    let d = p1 - p0;
    let mut cells = cells_near(g, run, (s.width_m * 0.5).max(SQUARE_M * 0.5));
    for &p in run {
        let (i, j) = g.cell_of(p);
        cells.extend(g.idx(i, j));
    }
    let wet: Vec<(i64, i64)> = cells
        .into_iter()
        .filter(|&k| matches!(g.kind[k], Kind::Water | Kind::Bridge))
        .map(|k| {
            let (i, j) = g.ij(k);
            (g.gx0 + i, g.gy0 + j)
        })
        .collect();
    if wet.is_empty() {
        return None;
    }
    let (x, y) = (rows(g, &wet, true), rows(g, &wet, false));
    let area = |r: &[(i64, i64, i64)]| r.iter().map(|&(_, a, b)| b - a + 1).sum::<i64>();
    let along_x = match area(&x).cmp(&area(&y)) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => d.x.abs() >= d.y.abs(),
    };
    let rows = if along_x { x } else { y };
    let id = s.id.0 + 1;
    for &(across, from, to) in &rows {
        for along in from..=to {
            if let Some(k) = at(g, along_x, across, along) {
                if g.kind[k] == Kind::Water {
                    g.kind[k] = Kind::Bridge;
                    g.street[k] = id;
                }
            }
        }
    }
    Some(Bridge {
        street: s.id,
        along_x,
        rows,
    })
}

fn at(g: &PlanGrid, along_x: bool, across: i64, along: i64) -> Option<usize> {
    if along_x {
        g.gidx(along, across)
    } else {
        g.gidx(across, along)
    }
}

/// The deck rows over the carriageway's water squares `wet`: every row
/// covers the whole crossing, so the deck is one straight piece, is
/// carried on to its own banks, and is trimmed to the water.
fn rows(g: &PlanGrid, wet: &[(i64, i64)], along_x: bool) -> Vec<(i64, i64, i64)> {
    let mut span: BTreeMap<i64, (i64, i64)> = BTreeMap::new();
    for &(x, y) in wet {
        let (across, along) = if along_x { (y, x) } else { (x, y) };
        let e = span.entry(across).or_insert((along, along));
        e.0 = e.0.min(along);
        e.1 = e.1.max(along);
    }
    let lo = span.values().map(|r| r.0).min().unwrap_or(0);
    let hi = span.values().map(|r| r.1).max().unwrap_or(0);
    let water = |across: i64, along: i64| {
        at(g, along_x, across, along)
            .is_some_and(|k| matches!(g.kind[k], Kind::Water | Kind::Bridge))
    };
    span.keys()
        .map(|&across| {
            let (mut from, mut to) = (lo, hi);
            while lo - from < REACH && water(across, from - 1) {
                from -= 1;
            }
            while to - hi < REACH && water(across, to + 1) {
                to += 1;
            }
            // The deck is the water part of the row: land at its ends is
            // the bank it rests on.
            while from < to && !water(across, from) {
                from += 1;
            }
            while to > from && !water(across, to) {
                to -= 1;
            }
            (across, from, to)
        })
        .collect()
}
