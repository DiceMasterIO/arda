//! Crossing geometry: where a way meets its channel, the axis-aligned span
//! a bridge, ford or ferry occupies, and the straightened approach the way
//! takes onto it (`mockup-artifact.md` "Roads": bridges, fords and ferries
//! at real crossings; goal 37).
//!
//! Spans are axis-aligned so parapets fall on square edges and deck planks
//! run true. The axis is the one nearest the river's normal; the way is bent
//! onto it with Hermite blends that start and end on its own curve.

use super::{square_centre, square_of, ChannelPlan, WayPlan, Window};
use crate::curve::{self, dist, hermite, unit, Dense, Station, P};
use crate::input::{m_to_ft, Crossing, CrossingKind, RiverChannel, RoadClass, Terrain, SQUARE_M};

/// Straight run beyond each end of the span before the blend starts, metres.
pub const APPROACH_M: f64 = 4.0 * SQUARE_M;
/// Abutment length at each end of a bridge, in squares.
pub const ABUTMENT_SQ: i64 = 2;
/// Squares of landing stage a ferry pushes into the water on each bank.
pub const LANDING_SQ: i64 = 3;

/// A resolved crossing on the global square lattice.
#[derive(Debug, Clone, PartialEq)]
pub struct CrossingPlan {
    /// Crossing id.
    pub id: u64,
    /// Bridge, ford or ferry.
    pub kind: CrossingKind,
    /// Index of the way.
    pub way: usize,
    /// Index of the channel.
    pub channel: usize,
    /// Whether the span runs east–west (else north–south).
    pub east_west: bool,
    /// Across-axis square range (rows for east–west spans), inclusive.
    pub rows: (i64, i64),
    /// Along-axis squares holding water in any row, inclusive.
    pub water: (i64, i64),
    /// Along-axis squares of the whole structure, inclusive.
    pub span: (i64, i64),
    /// Deck level (bridges) in feet.
    pub deck_ft: i16,
    /// Stone arch (else timber) for bridges.
    pub stone: bool,
    /// Road class.
    pub class: RoadClass,
    /// Channel width, metres.
    pub width_m: f64,
}

impl CrossingPlan {
    /// Global `(gx, gy)` of along index `a` and across index `r`.
    #[must_use]
    pub fn square(&self, a: i64, r: i64) -> (i64, i64) {
        if self.east_west {
            (a, r)
        } else {
            (r, a)
        }
    }
}

pub(super) fn along(ew: bool, p: P) -> f64 {
    if ew {
        p[0]
    } else {
        p[1]
    }
}

pub(super) fn from_axes(ew: bool, a: f64, c: f64) -> P {
    if ew {
        [a, c]
    } else {
        [c, a]
    }
}

/// Resolves every crossing that can reach `win`, straightening its way.
#[allow(clippy::cast_precision_loss)] // widths and indices are small
pub fn plan_all(
    win: Window,
    crossings: &[Crossing],
    ways: &mut [WayPlan],
    channels: &mut Vec<ChannelPlan>,
    terrain: &dyn Terrain,
    standing: &crate::standing::Standing,
) -> (Vec<CrossingPlan>, Vec<u64>) {
    let mut sorted: Vec<&Crossing> = crossings.iter().collect();
    sorted.sort_by_key(|c| c.id);
    let (mut out, mut orphans) = (Vec::new(), Vec::new());
    for c in sorted {
        let pos = [c.x_m as f64, c.y_m as f64];
        if win.dist_m(pos) > reach_m(c) {
            continue;
        }
        match plan_one(c, pos, ways, channels, terrain, standing) {
            Some(p) => out.push(p),
            None => orphans.push(c.id),
        }
    }
    for w in ways.iter_mut() {
        w.dense = Dense::new(std::mem::take(&mut w.dense.runs));
    }
    (out, orphans)
}

fn pick_way(c: &Crossing, pos: P, ways: &[WayPlan]) -> Option<(usize, curve::Hit)> {
    let mut best: Option<(f64, usize, curve::Hit)> = None;
    for (i, w) in ways.iter().enumerate() {
        if let Some(h) = w.dense.nearest(pos, 150.0) {
            let score = h.d + if w.class == c.road_class { 0.0 } else { 60.0 };
            if best.is_none_or(|b| score < b.0) {
                best = Some((score, i, h));
            }
        }
    }
    best.map(|(_, i, h)| (i, h))
}

/// Half-length of a synthetic channel of width `w` metres.
fn synthetic_half(w: f64) -> f64 {
    (3.0 * w).max(60.0)
}

/// How far from a crossing record its work can reach, metres: the foot on
/// its way lies within 150 m of it, and a synthetic channel runs its
/// half-length plus half its width (and a bank square) from the foot.
/// A window this far away must plan it, or a neighbour that does paints
/// water this one lacks (review round 2 #34: `1.5 w + 150` fell short for
/// crossings wider than 75 m).
#[must_use]
pub fn reach_m(c: &Crossing) -> f64 {
    let w = f64::from(c.width_m).max(3.0);
    let structure = f64::from(c.width_m) * 1.5 + 150.0;
    structure.max(150.0 + synthetic_half(w) + w / 2.0 + SQUARE_M)
}

fn synthetic_channel(c: &Crossing, foot: P, dir: P) -> ChannelPlan {
    let w = f64::from(c.width_m).max(3.0);
    let half = synthetic_half(w);
    let n = [-dir[1], dir[0]];
    let depth = match c.kind {
        CrossingKind::Ford => 0.5,
        _ => 0.8 + 0.4 * f64::from(c.order),
    };
    ChannelPlan::new(&RiverChannel {
        id: c.id,
        centreline: vec![
            [foot[0] - n[0] * half, foot[1] - n[1] * half],
            [foot[0] + n[0] * half, foot[1] + n[1] * half],
        ],
        width_m: w,
        depth_m: depth,
    })
}

/// The intersection of two dense curves nearest `pos`, if any.
fn intersect(a: &Dense, b: &Dense, pos: P, reach: f64) -> Option<P> {
    let near = |s: &Station| dist(s.p, pos) <= reach;
    let mut best: Option<(f64, P)> = None;
    for ra in &a.runs {
        for wa in ra.windows(2).filter(|w| near(&w[0])) {
            for rb in &b.runs {
                for wb in rb.windows(2).filter(|w| near(&w[0])) {
                    if let Some(x) = seg_x(wa[0].p, wa[1].p, wb[0].p, wb[1].p) {
                        let d = dist(x, pos);
                        if best.is_none_or(|bb| d < bb.0) {
                            best = Some((d, x));
                        }
                    }
                }
            }
        }
    }
    best.map(|b| b.1)
}

fn seg_x(p: P, p2: P, q: P, q2: P) -> Option<P> {
    let r = [p2[0] - p[0], p2[1] - p[1]];
    let s = [q2[0] - q[0], q2[1] - q[1]];
    let den = r[0] * s[1] - r[1] * s[0];
    if den.abs() < 1e-12 {
        return None;
    }
    let qp = [q[0] - p[0], q[1] - p[1]];
    let t = (qp[0] * s[1] - qp[1] * s[0]) / den;
    let u = (qp[0] * r[1] - qp[1] * r[0]) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then(|| curve::lerp(p, p2, t))
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)] // small indices
pub(super) fn plan_one(
    c: &Crossing,
    pos: P,
    ways: &mut [WayPlan],
    channels: &mut Vec<ChannelPlan>,
    terrain: &dyn Terrain,
    standing: &crate::standing::Standing,
) -> Option<CrossingPlan> {
    let (wi, hit) = pick_way(c, pos, ways)?;
    let ci = channels
        .iter()
        .enumerate()
        .filter(|(_, ch)| !ch.guide)
        .filter_map(|(i, ch)| ch.dense.nearest(hit.foot, 150.0).map(|h| (h.d, i)))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|x| x.1);
    let ci = match ci {
        Some(i) => i,
        None => {
            let mut ch = synthetic_channel(c, hit.foot, hit.dir);
            // A crossing over a lake or the sea already has its water: the
            // synthetic channel only orients it and paints nothing.
            ch.guide = standing.at(square_of(hit.foot[0]), square_of(hit.foot[1]));
            channels.push(ch);
            channels.len() - 1
        }
    };
    let ch = &channels[ci];
    let centre = intersect(&ways[wi].dense, &ch.dense, pos, 200.0).unwrap_or(hit.foot);
    let flow = ch.dense.nearest(centre, 1e9)?.dir;
    let east_west = flow[1].abs() >= flow[0].abs();
    let rows = rows_for(c.kind, &ways[wi], east_west, centre);
    let mid = square_of(along(east_west, centre));
    let reach = ((ch.width_m * 2.0 + 60.0) / SQUARE_M).ceil() as i64;
    let mut water: Option<(i64, i64)> = None;
    for r in rows.0..=rows.1 {
        for a in mid - reach..=mid + reach {
            let (gx, gy) = if east_west { (a, r) } else { (r, a) };
            if ch.is_water(square_centre(gx, gy)) {
                water = Some(water.map_or((a, a), |(lo, hi)| (lo.min(a), hi.max(a))));
            }
        }
    }
    let lay = Lay {
        id: c.id,
        kind: c.kind,
        way: wi,
        channel: ci,
        centre,
        east_west,
        rows,
        water: water?,
        width_m: ch.width_m,
    };
    let out = finish(&lay, ways, terrain).map(|x| x.0);
    // `straighten` spliced stations into this way: rebuild its chunk index
    // now, so the next crossing's nearest-way query sees the moved stations
    // (review round 2 #35).
    if let Some(w) = ways.get_mut(wi) {
        w.dense = Dense::new(std::mem::take(&mut w.dense.runs));
    }
    out
}

/// The across-axis rows a crossing of `kind` on `way` occupies around
/// `centre`.
#[allow(clippy::cast_possible_truncation)] // world squares are far below 2^63
pub(super) fn rows_for(
    kind: CrossingKind,
    way: &WayPlan,
    east_west: bool,
    centre: P,
) -> (i64, i64) {
    let spec = way.spec;
    let width = match kind {
        CrossingKind::Bridge => spec.deck_sq.min(spec.width_sq).max(2),
        CrossingKind::Ferry => spec.width_sq.min(3),
        CrossingKind::Ford => spec.width_sq,
    };
    let across_m = along(!east_west, centre);
    let h = i64::from(width / 2);
    if width % 2 == 1 {
        let r = square_of(across_m);
        (r - h, r + h)
    } else {
        let b = (across_m / SQUARE_M).round() as i64;
        (b - h, b + h - 1)
    }
}

/// A crossing placed on the lattice, before its structure and approach.
pub(super) struct Lay {
    /// Crossing id.
    pub id: u64,
    /// Bridge, ford or ferry.
    pub kind: CrossingKind,
    /// Index of the way.
    pub way: usize,
    /// Index of the channel.
    pub channel: usize,
    /// Where the way meets the water.
    pub centre: P,
    /// Span axis.
    pub east_west: bool,
    /// Across-axis rows.
    pub rows: (i64, i64),
    /// Along-axis squares holding water.
    pub water: (i64, i64),
    /// Channel width, metres.
    pub width_m: f64,
}

/// Completes a laid crossing: abutments, deck level, the plan, and the
/// way straightened onto the span. Also returns the arc range of the way
/// the approach replaced, if any.
#[allow(clippy::cast_precision_loss)] // widths and indices are small
pub(super) fn finish(
    l: &Lay,
    ways: &mut [WayPlan],
    terrain: &dyn Terrain,
) -> Option<(CrossingPlan, Option<(f64, f64)>)> {
    let (wi, east_west, rows, water) = (l.way, l.east_west, l.rows, l.water);
    let road = ways[wi].dense.nearest(l.centre, 1e9)?;
    let sign = if along(east_west, road.dir) >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let across_c = (rows.0 + rows.1 + 1) as f64 * SQUARE_M / 2.0;
    let abut = if l.kind == CrossingKind::Bridge {
        ABUTMENT_SQ
    } else {
        0
    };
    let span = (water.0 - abut, water.1 + abut);
    let stone = l.kind == CrossingKind::Bridge
        && (ways[wi].class == RoadClass::Highway
            || (ways[wi].class == RoadClass::Road && ways[wi].wealth >= 80));
    let bank = |a: i64| {
        let p = from_axes(east_west, (a as f64 + 0.5) * SQUARE_M, across_c);
        terrain.height_m(p[0], p[1])
    };
    let deck_m = bank(span.0).max(bank(span.1)) + if stone { 0.6 } else { 0.3 };
    let plan = CrossingPlan {
        id: l.id,
        kind: l.kind,
        way: wi,
        channel: l.channel,
        east_west,
        rows,
        water,
        span,
        deck_ft: m_to_ft(deck_m),
        stone,
        class: ways[wi].class,
        width_m: l.width_m,
    };
    let (lo_m, hi_m) = (
        span.0 as f64 * SQUARE_M - APPROACH_M,
        (span.1 + 1) as f64 * SQUARE_M + APPROACH_M,
    );
    let (pa, pb) = if sign > 0.0 {
        (lo_m, hi_m)
    } else {
        (hi_m, lo_m)
    };
    let pa = from_axes(east_west, pa, across_c);
    let pb = from_axes(east_west, pb, across_c);
    let axis = from_axes(east_west, sign, 0.0);
    let bridge = (l.kind == CrossingKind::Bridge).then_some(deck_m);
    let arcs = straighten(&mut ways[wi].dense, road, pa, pb, axis, bridge, terrain);
    Some((plan, arcs))
}

/// Replaces the way's stations around `hit` with blend → straight → blend.
/// Arc lengths outside the replaced stretch are untouched, so ruts and
/// milestones elsewhere do not move.
fn straighten(
    dense: &mut Dense,
    hit: curve::Hit,
    pa: P,
    pb: P,
    axis: P,
    deck_m: Option<f64>,
    terrain: &dyn Terrain,
) -> Option<(f64, f64)> {
    let span = dist(pa, pb);
    let blend = 10.0 + 0.25 * span;
    let back = dist(hit.foot, pa);
    let fwd = dist(hit.foot, pb);
    let (s_lo, s_hi) = (hit.s - back - blend, hit.s + fwd + blend);
    let run = dense.runs.iter_mut().find(|r| {
        r.first().is_some_and(|a| a.s <= hit.s) && r.last().is_some_and(|b| b.s >= hit.s)
    })?;
    let lo = run.iter().rposition(|st| st.s <= s_lo).unwrap_or(0);
    let hi = run
        .iter()
        .position(|st| st.s >= s_hi)
        .unwrap_or(run.len() - 1);
    if hi <= lo + 1 {
        return None;
    }
    let tan = |i: usize| {
        let (a, b) = (
            run[i.saturating_sub(1)].p,
            run[(i + 1).min(run.len() - 1)].p,
        );
        unit([b[0] - a[0], b[1] - a[1]])
    };
    let (a, b) = (run[lo], run[hi]);
    let mut pts: Vec<(P, u8)> = hermite(a.p, tan(lo), pa, axis)
        .into_iter()
        .map(|p| (p, 0))
        .collect();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // bounded counts
    let n = ((span / curve::STEP_M).ceil() as u32).max(1);
    for k in 1..=n {
        pts.push((curve::lerp(pa, pb, f64::from(k) / f64::from(n)), 1));
    }
    pts.extend(hermite(pb, axis, b.p, tan(hi)).into_iter().map(|p| (p, 2)));
    let mut chord = vec![0.0];
    let mut prev = a.p;
    for (p, _) in &pts {
        let last = chord.last().copied().unwrap_or(0.0);
        chord.push(last + dist(prev, *p));
        prev = *p;
    }
    let total = chord.last().copied().unwrap_or(1.0).max(1e-9);
    let first_blend = chord
        .get(pts.iter().filter(|x| x.1 == 0).count())
        .copied()
        .unwrap_or(0.0);
    let second_blend = chord
        .get(pts.iter().filter(|x| x.1 < 2).count())
        .copied()
        .unwrap_or(total);
    let mut fresh = vec![a];
    for (i, (p, part)) in pts.iter().enumerate() {
        let f = chord[i + 1] / total;
        let straight_z = deck_m.unwrap_or_else(|| terrain.height_m(p[0], p[1]));
        let z = match part {
            0 => {
                let t = chord[i + 1] / first_blend.max(1e-9);
                a.z + (deck_m.unwrap_or(a.z) - a.z) * t.min(1.0)
            }
            1 => straight_z,
            _ => {
                let t = (chord[i + 1] - second_blend) / (total - second_blend).max(1e-9);
                let from = deck_m.unwrap_or(b.z);
                from + (b.z - from) * t.clamp(0.0, 1.0)
            }
        };
        fresh.push(Station {
            p: *p,
            s: a.s + (b.s - a.s) * f,
            z: if deck_m.is_none() && *part != 1 {
                terrain.height_m(p[0], p[1])
            } else {
                z
            },
        });
    }
    run.splice(lo..=hi, fresh);
    Some((a.s, b.s))
}

#[cfg(test)]
#[path = "crossing_tests.rs"]
mod tests;
