//! Main streets beside rivers. A main street follows its road, and the road
//! often follows a river valley: where its carriageway would run in or
//! along the water it is moved onto the nearer bank, a square clear of the
//! water's edge, so no bridge runs along a river. Where the road really
//! crosses, the street keeps to each bank up to one point and there goes
//! straight across, at the narrowest channel the road crosses, square to
//! the flow; that stretch becomes the bridge ([`super::bridge`]).

use super::grid::SQUARE_M;
use crate::geom::{self, Vec2};
use crate::site::RiverLine;
use rayon::prelude::*;

/// Sample spacing along a street, metres.
const STEP: f64 = 2.0;
/// Pushes per sample (a sample moved off one channel may land by another).
const PUSHES: usize = 3;

/// The channel nearest a point.
#[derive(Debug, Clone, Copy)]
struct Near {
    /// Distance from the point to the channel's bank (negative inside).
    bank: f64,
    /// Closest centreline point.
    at: Vec2,
    /// Unit tangent of the centreline there.
    tangent: Vec2,
    /// Half the channel width, metres.
    half: f64,
    /// Index of the river line.
    line: usize,
}

impl Near {
    /// Which bank the point lies on: `1.0` left of the flow, `-1.0` right.
    fn side(&self, p: Vec2) -> f64 {
        if self.tangent.cross(p - self.at) >= 0.0 {
            1.0
        } else {
            -1.0
        }
    }
}

fn nearest(rivers: &[RiverLine], p: Vec2) -> Option<Near> {
    let mut best: Option<Near> = None;
    for (line, r) in rivers.iter().enumerate() {
        let half = r.width_m * 0.5;
        for w in r.points.windows(2) {
            let (d, t) = geom::seg_dist(p, w[0], w[1]);
            let bank = d - half;
            if best.is_none_or(|b| bank < b.bank) {
                best = Some(Near {
                    bank,
                    at: w[0].lerp(w[1], t),
                    tangent: (w[1] - w[0]).norm(),
                    half,
                    line,
                });
            }
        }
    }
    best
}

/// The width of water along `dir` through `at`, metres (to 80 m a side).
fn across(water: &(dyn Fn(Vec2) -> bool + Sync), at: Vec2, dir: Vec2) -> f64 {
    let mut w = 0.0;
    for sign in [1.0, -1.0] {
        let mut t = 0.0;
        while t < 80.0 && water(at + dir * (sign * (t + 0.25))) {
            t += 0.5;
        }
        w += t;
    }
    w
}

/// `p` moved onto bank `side` of `n`, `clear` metres from the water.
fn onto_bank(p: Vec2, n: &Near, side: f64, clear: f64) -> Vec2 {
    let away = p - n.at;
    let dir = if away.len() > 0.3 && n.side(p) == side {
        away.norm()
    } else {
        n.tangent.perp() * side
    };
    n.at + dir * (n.half + clear)
}

/// A main street's centreline with its carriageway (half-width `half_w`)
/// kept a square clear of the rivers, except for one straight crossing
/// wherever the street passes from one bank to the other. The first point
/// (the market) never moves.
#[must_use]
pub fn dry(
    points: &[Vec2],
    half_w: f64,
    rivers: &[RiverLine],
    water: &(dyn Fn(Vec2) -> bool + Sync),
) -> Vec<Vec2> {
    if rivers.is_empty() || points.len() < 2 {
        return points.to_vec();
    }
    let clear = half_w + SQUARE_M;
    let pts = geom::resample(points, STEP);
    // Each sample scans every river segment: in parallel (goal 50).
    let near: Vec<Option<Near>> = pts.par_iter().map(|&p| nearest(rivers, p)).collect();
    let wet: Vec<bool> = near
        .iter()
        .enumerate()
        .map(|(i, n)| i > 0 && n.is_some_and(|n| n.bank < clear))
        .collect();
    let mut out = pts.clone();
    let mut i = 0;
    while i < pts.len() {
        if !wet[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i < pts.len() && wet[i] {
            i += 1;
        }
        let run = Run {
            pts: &pts[start..i],
            near: &near[start..i],
            clear,
            rivers,
            water,
        };
        run.settle(&mut out[start..i]);
    }
    out
}

/// Moves one run of wet samples onto the banks: all onto the bank the run
/// starts on, or, when it ends on the other bank, onto the first bank up to
/// the crossing and the second after it.
struct Run<'a> {
    pts: &'a [Vec2],
    near: &'a [Option<Near>],
    clear: f64,
    rivers: &'a [RiverLine],
    water: &'a (dyn Fn(Vec2) -> bool + Sync),
}

impl Run<'_> {
    fn settle(&self, out: &mut [Vec2]) {
        let (pts, near, clear, rivers, water) =
            (self.pts, self.near, self.clear, self.rivers, self.water);
        let sides: Vec<f64> = pts
            .iter()
            .zip(near)
            .map(|(&p, n)| n.map_or(1.0, |n| n.side(p)))
            .collect();
        let (first, last) = (sides[0], sides[sides.len() - 1]);
        // The crossing: where the water square across the flow is narrowest,
        // nearest the road's own crossing on a tie (never at a confluence,
        // where the line across runs into the other channel).
        let own = (1..sides.len())
            .find(|&k| sides[k] != sides[k - 1])
            .unwrap_or(0);
        // Only the widest channel of the run is crossed: a brook beside it
        // is left to its own banks.
        let widest = near.iter().flatten().map(|n| n.half).fold(0.0, f64::max);
        let flip = (first != last)
            .then(|| {
                // Each width is measured on its own: in parallel, then the
                // minimum is taken in order, as before (goal 50).
                let widths: Vec<(f64, usize, usize)> = (1..pts.len())
                    .into_par_iter()
                    .filter_map(|k| {
                        let n = near[k].filter(|n| n.half >= widest - 1e-9)?;
                        Some((across(water, n.at, n.tangent.perp()), k.abs_diff(own), k))
                    })
                    .collect();
                widths
                    .into_iter()
                    .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
                    .map(|(_, _, k)| k)
            })
            .flatten();
        for (k, (&p, n)) in pts.iter().zip(near).enumerate() {
            let Some(n) = n else { continue };
            let side = match flip {
                Some(f) if k >= f => last,
                _ => first,
            };
            let mut q = onto_bank(p, n, side, clear);
            // Off one channel, perhaps onto another: settle against that one on
            // whichever bank the point now lies.
            for _ in 0..PUSHES {
                match nearest(rivers, q) {
                    Some(m) if m.bank < clear - 1e-6 && m.line != n.line => {
                        q = onto_bank(q, &m, m.side(q), clear);
                    }
                    _ => break,
                }
            }
            out[k] = q;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::v2;

    fn river() -> Vec<RiverLine> {
        vec![RiverLine {
            points: (0..=40).map(|k| v2(f64::from(k) * 10.0, 0.0)).collect(),
            width_m: 8.0,
        }]
    }

    #[test]
    fn a_street_along_the_river_moves_onto_the_bank() {
        let street: Vec<Vec2> = (0..=30).map(|k| v2(f64::from(k) * 10.0, 1.0)).collect();
        let d = dry(&street, 2.5, &river(), &|p: Vec2| p.y.abs() < 4.0);
        for p in &d[1..] {
            assert!(p.y >= 4.0 + 2.5 + SQUARE_M - 1e-6, "{p:?}");
        }
    }

    #[test]
    fn a_crossing_goes_straight_across_once() {
        let street = vec![v2(100.0, -40.0), v2(130.0, 40.0)];
        let d = dry(&street, 2.5, &river(), &|p: Vec2| p.y.abs() < 4.0);
        let crossings = d
            .windows(2)
            .filter(|w| (w[0].y < 0.0) != (w[1].y < 0.0))
            .collect::<Vec<_>>();
        assert_eq!(crossings.len(), 1);
        let w = crossings[0];
        assert!((w[0].x - w[1].x).abs() < 1.0, "square to the flow: {w:?}");
    }
}
