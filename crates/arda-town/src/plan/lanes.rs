//! Secondary streets: back lanes behind the main-street burgages, cross
//! alleys between them, and the intramural lane inside the wall. Back lanes
//! sit one plot depth behind the main street, as in planted medieval towns.

use super::focal::Arm;
use super::params::Params;
use super::types::StreetClass;
use crate::geom::{self, Vec2};
use crate::rng::{hash, Rng};

use crate::site::TerrainInput;

/// A proposed street before rasterisation.
#[derive(Debug, Clone)]
pub struct LaneSpec {
    /// Class.
    pub class: StreetClass,
    /// Centreline.
    pub points: Vec<Vec2>,
    /// Width in metres.
    pub width_m: f64,
}

/// Limits a lane must stay within.
pub struct Bounds<'a> {
    /// Market centre.
    pub market: Vec2,
    /// Radius of the square's keep-out zone.
    pub square_r: f64,
    /// Core radius (unwalled towns).
    pub r_core: f64,
    /// Wall ring, if any, and the clearance to keep from it.
    pub ring: Option<(&'a [Vec2], f64)>,
    /// Terrain water mask.
    pub terrain: &'a TerrainInput,
    /// A keep-out zone around the castle, if any.
    pub castle: Option<(Vec2, f64)>,
}

impl Bounds<'_> {
    fn ok(&self, q: Vec2) -> bool {
        !(self.terrain.water)(q) && self.ok_on_land(q)
    }

    /// Whether `a → b` crosses the nearest river at more than 45° to its
    /// flow (a lane crosses on a footbridge, it never runs along one).
    fn across_river(&self, a: Vec2, b: Vec2) -> bool {
        let mid = a.lerp(b, 0.5);
        let flow = self
            .terrain
            .rivers
            .iter()
            .flat_map(|r| r.points.windows(2))
            .map(|w| (geom::seg_dist(mid, w[0], w[1]).0, (w[1] - w[0]).norm()))
            .min_by(|x, y| x.0.total_cmp(&y.0));
        flow.is_some_and(|(_, t)| t.dot((b - a).norm()).abs() < 0.7)
    }

    /// [`Self::ok`] but for the water.
    fn ok_on_land(&self, q: Vec2) -> bool {
        if q.dist(self.market) < self.square_r {
            return false;
        }
        if let Some((c, r)) = self.castle {
            if q.dist(c) < r {
                return false;
            }
        }
        match self.ring {
            Some((ring, clear)) => geom::inside(ring, q) && geom::ring_dist(ring, q) > clear,
            None => q.dist(self.market) < self.r_core * 1.1,
        }
    }
}

fn runs(points: &[(Vec2, bool)]) -> Vec<Vec<Vec2>> {
    let mut out = Vec::new();
    let mut cur: Vec<Vec2> = Vec::new();
    for &(p, ok) in points {
        if ok {
            cur.push(p);
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Where an arm first reaches `r` metres from the market, and its angle.
fn at_radius(arm: &Arm, market: Vec2, r: f64) -> Option<(Vec2, f64)> {
    for w in arm.points.windows(2) {
        let (a, b) = (w[0].dist(market), w[1].dist(market));
        if a < r && b >= r {
            let t = (r - a) / (b - a).max(1e-9);
            let p = w[0].lerp(w[1], t);
            let d = p - market;
            return Some((p, geom::atan2(d.y, d.x)));
        }
    }
    None
}

fn polar(c: Vec2, r: f64, theta: f64) -> Vec2 {
    let (s, co) = geom::sin_cos(theta);
    c + Vec2 { x: co, y: s } * r
}

/// Longest gap between dry points a lane crosses on a footbridge, metres.
const FOOTBRIDGE_M: f64 = 18.0;

/// Valid stretches of a candidate lane, each at least `min_len` long. A
/// short stretch over water between valid points is kept: the lane crosses
/// there on a footbridge ([`super::bridge`]).
fn keep_valid(points: &[Vec2], b: &Bounds<'_>, min_len: f64) -> Vec<Vec<Vec2>> {
    let mut cand: Vec<(Vec2, bool)> = points.iter().map(|&q| (q, b.ok(q))).collect();
    let wet_only = |q: Vec2| (b.terrain.water)(q) && b.ok_on_land(q);
    let mut i = 1;
    while i < cand.len() {
        if cand[i].1 || !cand[i - 1].1 {
            i += 1;
            continue;
        }
        let start = i;
        while i < cand.len() && !cand[i].1 && wet_only(cand[i].0) {
            i += 1;
        }
        let bridged = i < cand.len()
            && i > start
            && cand[i].1
            && cand[start - 1].0.dist(cand[i].0) <= FOOTBRIDGE_M
            && b.across_river(cand[start - 1].0, cand[i].0);
        if bridged {
            cand[start..i].iter_mut().for_each(|c| c.1 = true);
        }
        i = i.max(start + 1);
    }
    runs(&cand)
        .into_iter()
        .filter(|r| r.len() >= 2 && geom::length(r) >= min_len)
        .collect()
}

/// The lane web: concentric ring segments between neighbouring main
/// streets, plus secondary radials in wide wedges. Organic medieval towns
/// grew this way (a radio-concentric plan), which avoids the parallel
/// back lanes of a planted grid.
#[must_use]
pub fn lanes(arms: &[Arm], params: &Params, b: &Bounds<'_>, seed: u64) -> Vec<LaneSpec> {
    let mut out = Vec::new();
    if !params.back_lanes {
        return out;
    }
    let mut rng = Rng::keyed(seed, 0x1a7e, 0);
    let spacing = 2.0 * params.depth_m() + params.lane_w;
    let limit = b.r_core * 1.05;
    let first = b.square_r + params.depth_m() + params.lane_w * 0.5;
    let mut radii = Vec::new();
    let mut r = first;
    while r < limit - params.depth_m() * 0.6 {
        radii.push(r);
        r += spacing * rng.range_f(0.9, 1.1);
    }
    let radial: Vec<&Arm> = arms.iter().filter(|a| !a.to_castle).collect();
    let keep_p = if params.green { 0.4 } else { 0.85 };
    for (k, &r) in radii.iter().enumerate() {
        let mut hits: Vec<(f64, Vec2)> = radial
            .iter()
            .filter_map(|a| at_radius(a, b.market, r).map(|(p, t)| (t, p)))
            .collect();
        hits.sort_by(|x, y| x.0.total_cmp(&y.0));
        let n = hits.len();
        if n == 0 {
            continue;
        }
        let lone = n == 1;
        for i in 0..n {
            if !rng.chance(keep_p) {
                continue;
            }
            let (t0, _) = hits[i];
            let t1 = if lone {
                t0 + 2.0 * geom::PI
            } else {
                hits[(i + 1) % n].0
            };
            let t1 = if t1 <= t0 { t1 + 2.0 * geom::PI } else { t1 };
            let gap = t1 - t0;
            let steps = ((gap * r) / 6.0).ceil().max(2.0);
            let wob = hash(seed, 0x0c1c, (k * 64 + i) as u64);
            let amp = if params.green { 0.22 } else { 0.16 };
            let bias = rng.range_f(0.86, 1.14);
            let pts: Vec<Vec2> = (0..=crate::num::clamp_u32(crate::num::round_i(steps)))
                .map(|j| {
                    let f = f64::from(j) / steps;
                    // Ends meet the main streets at radius r; the middle
                    // bows in or out, so blocks are irregular.
                    let bow = 1.0 + (bias - 1.0) * (4.0 * f * (1.0 - f));
                    let rr = r * bow * (1.0 + amp * crate::rng::fbm1(wob, f * gap * r / 110.0));
                    polar(b.market, rr, t0 + gap * f)
                })
                .collect();
            for run in keep_valid(&pts, b, 30.0) {
                out.push(LaneSpec {
                    class: StreetClass::Lane,
                    points: geom::resample(&run, 6.0),
                    width_m: params.lane_w,
                });
            }
            // Secondary radials split wide wedges into blocks.
            let wedges = (gap / 1.3).floor();
            if params.green || wedges < 1.0 || k > 0 {
                continue;
            }
            let count = crate::num::clamp_u32(crate::num::round_i(wedges));
            for s in 1..=count {
                let f = f64::from(s) / (f64::from(count) + 1.0);
                let theta = t0 + gap * (f + rng.range_f(-0.06, 0.06));
                let wob2 = hash(seed, 0x0ad1, (k * 64 + i * 8) as u64 + u64::from(s));
                let pts: Vec<Vec2> = (0..60)
                    .map(|j| {
                        let rr = r + f64::from(j) * 6.0;
                        let dt = 0.12 * crate::rng::fbm1(wob2, rr / 80.0);
                        polar(b.market, rr, theta + dt)
                    })
                    .take_while(|q| q.dist(b.market) < limit)
                    .collect();
                for run in keep_valid(&pts, b, 40.0) {
                    out.push(LaneSpec {
                        class: StreetClass::Lane,
                        points: run,
                        width_m: params.lane_w,
                    });
                }
            }
        }
    }
    out
}

/// A strand lane along the shore through a harbour market, on the landward
/// side of the water, so fishers' cottages face the beach.
#[must_use]
pub fn strand(
    market: Vec2,
    sea: Vec2,
    len: f64,
    params: &Params,
    terrain: &TerrainInput,
) -> Option<LaneSpec> {
    let inland = (market - sea).norm();
    let axis = inland.perp();
    let mut pts = Vec::new();
    let mut t = -len;
    while t <= len {
        let mut p = market + axis * t;
        // Walk seawards to the waterline, then back inland by a plot's width.
        let mut steps = 0;
        while !(terrain.water)(p) && steps < 120 {
            p = p - inland * 2.0;
            steps += 1;
        }
        if steps >= 120 {
            break;
        }
        pts.push(p + inland * (params.lane_w * 0.5 + 9.0));
        t += 8.0;
    }
    (pts.len() >= 4).then(|| LaneSpec {
        class: StreetClass::Lane,
        points: geom::resample(&geom::chaikin(&pts, 2), 6.0),
        width_m: params.lane_w,
    })
}

/// The lane just inside the wall ring.
#[must_use]
pub fn intramural(ring: &[Vec2], market: Vec2, inset: f64, width: f64) -> LaneSpec {
    let mut pts: Vec<Vec2> = ring
        .iter()
        .map(|&v| v + (market - v).norm() * inset)
        .collect();
    if let Some(&first) = pts.first() {
        pts.push(first);
    }
    LaneSpec {
        class: StreetClass::Intramural,
        points: geom::resample(&pts, 6.0),
        width_m: width,
    }
}
