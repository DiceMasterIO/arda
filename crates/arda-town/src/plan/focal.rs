//! The focal point (crossing, junction, harbour or castle), the market
//! position and the main-street arms that follow the entering roads
//! (goal 36; streets grow along the roads that made the town).

use super::params::Params;
use super::types::{Focal, FocalKind};
use crate::geom::{self, v2, Vec2};
use crate::rng::{fbm1, hash};
use crate::site::{RoadClass, SettlementFunction, TerrainInput, TownSite};

/// A main street leaving the market.
#[derive(Debug, Clone)]
pub struct Arm {
    /// Centreline from the market outwards.
    pub points: Vec<Vec2>,
    /// Class of the road it follows.
    pub class: RoadClass,
    /// Leads to the castle gate rather than out of town.
    pub to_castle: bool,
}

/// Roads smoothed from coarse cell-centre polylines.
#[must_use]
pub fn smooth_roads(terrain: &TerrainInput) -> Vec<(RoadClass, Vec<Vec2>)> {
    terrain
        .roads
        .iter()
        .filter(|r| r.points.len() >= 2)
        .map(|r| {
            let p = geom::chaikin(&geom::resample(&r.points, 25.0), 3);
            (r.class, geom::resample(&p, 6.0))
        })
        .collect()
}

fn nearest_road(roads: &[(RoadClass, Vec<Vec2>)], p: Vec2) -> Option<(usize, f64, f64)> {
    roads
        .iter()
        .enumerate()
        .map(|(i, (_, pts))| {
            let (d, s) = geom::project(pts, p);
            (i, d, s)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

fn highest_point(terrain: &TerrainInput, around: Vec2, radius: f64) -> Vec2 {
    let mut best = (f64::MIN, around);
    let steps = 36;
    for j in -steps..=steps {
        for i in -steps..=steps {
            let p = around + v2(f64::from(i), f64::from(j)) * (radius / f64::from(steps));
            if p.dist(around) > radius || (terrain.water)(p) {
                continue;
            }
            let h = (terrain.height)(p);
            if h > best.0 {
                best = (h, p);
            }
        }
    }
    best.1
}

fn nearest_water(terrain: &TerrainInput, around: Vec2, radius: f64) -> Option<Vec2> {
    let mut r = 4.0;
    while r <= radius {
        let n = 64;
        for k in 0..n {
            let (s, c) = geom::sin_cos(2.0 * geom::PI * f64::from(k) / f64::from(n));
            let p = around + v2(c, s) * r;
            if (terrain.water)(p) {
                return Some(p);
            }
        }
        r += 4.0;
    }
    None
}

/// Finds the focal feature and the market position.
#[must_use]
pub fn find(site: &TownSite, terrain: &TerrainInput, params: &Params) -> Focal {
    let pos = site.position();
    let roads = smooth_roads(terrain);
    let (sq_len, sq_wid) = params.square;
    if params.castle {
        let keep = highest_point(terrain, pos, params.r_core.min(240.0));
        let dir = nearest_road(&roads, keep).map_or(v2(0.0, 1.0), |(i, _, s)| {
            (geom::at(&roads[i].1, s).0 - keep).norm()
        });
        let market = keep + dir * (38.0 + sq_len * 0.5);
        return Focal {
            kind: FocalKind::Castle,
            feature: keep,
            market,
        };
    }
    let seaside = site.coastal
        && (site.has(SettlementFunction::Fishing) || site.has(SettlementFunction::Port));
    if seaside {
        if let Some(w) = nearest_water(terrain, pos, 450.0) {
            let inland = (pos - w).norm();
            let mut shore = w;
            while (terrain.water)(shore) {
                shore = shore + inland * 1.0;
            }
            return Focal {
                kind: FocalKind::Harbour,
                feature: shore,
                market: shore + inland * (sq_wid * 0.5 + 3.0),
            };
        }
    }
    let mut crossings: Vec<(Vec2, usize)> = Vec::new();
    for (i, (_, road)) in roads.iter().enumerate() {
        for river in &terrain.rivers {
            for x in geom::crossings(road, &river.points) {
                crossings.push((x, i));
            }
        }
    }
    crossings.sort_by(|a, b| a.0.dist(pos).total_cmp(&b.0.dist(pos)));
    if let Some(&(bridge, ri)) = crossings.first() {
        if bridge.dist(pos) < 450.0 {
            let road = &roads[ri].1;
            let (_, sb) = geom::project(road, bridge);
            let len = geom::length(road);
            let d = bridge.dist(pos).clamp(70.0, 140.0);
            let fwd = geom::at(road, (sb + d).min(len)).0;
            let back = geom::at(road, (sb - d).max(0.0)).0;
            let mut s = if fwd.dist(pos) < back.dist(pos) {
                d
            } else {
                -d
            };
            let river_far = |p: Vec2| {
                terrain
                    .rivers
                    .iter()
                    .all(|r| geom::dist_to(&r.points, p) > r.width_m * 0.5 + sq_wid * 0.5 + 12.0)
            };
            let mut market = geom::at(road, (sb + s).clamp(0.0, len)).0;
            for _ in 0..40 {
                if river_far(market) && !(terrain.water)(market) {
                    break;
                }
                s += s.signum() * 6.0;
                market = geom::at(road, (sb + s).clamp(0.0, len)).0;
            }
            return Focal {
                kind: FocalKind::Crossing,
                feature: bridge,
                market,
            };
        }
    }
    let mut junctions: Vec<Vec2> = Vec::new();
    for i in 0..roads.len() {
        for j in i + 1..roads.len() {
            junctions.extend(geom::crossings(&roads[i].1, &roads[j].1));
        }
    }
    junctions.sort_by(|a, b| a.dist(pos).total_cmp(&b.dist(pos)));
    if let Some(&j) = junctions.first() {
        if j.dist(pos) < 400.0 {
            return Focal {
                kind: FocalKind::Crossroads,
                feature: j,
                market: j,
            };
        }
    }
    let p = nearest_road(&roads, pos).map_or(pos, |(i, _, s)| geom::at(&roads[i].1, s).0);
    Focal {
        kind: FocalKind::Roadside,
        feature: p,
        market: p,
    }
}

fn clip_to(points: &[Vec2], centre: Vec2, r: f64) -> Vec<Vec2> {
    let mut out = Vec::new();
    for &p in points {
        out.push(p);
        if p.dist(centre) > r {
            break;
        }
    }
    out
}

fn wobble(points: &[Vec2], seed: u64, amp: f64) -> Vec<Vec2> {
    let pts = geom::resample(points, 6.0);
    let n = pts.len();
    let mut s = 0.0;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        if i > 0 {
            s += pts[i].dist(pts[i - 1]);
        }
        let t = (pts[(i + 1).min(n - 1)] - pts[i.saturating_sub(1)]).norm();
        let taper = (s / 45.0).clamp(0.0, 1.0);
        let taper = taper * taper * (3.0 - 2.0 * taper);
        out.push(pts[i] + t.perp() * (amp * taper * fbm1(seed, s / 75.0)));
    }
    out
}

/// Builds the main-street arms from the market outwards.
#[must_use]
pub fn arms(
    terrain: &TerrainInput,
    focal: &Focal,
    params: &Params,
    seed: u64,
    r_ext: f64,
) -> Vec<Arm> {
    let m = focal.market;
    let roads = smooth_roads(terrain);
    let mut out: Vec<Arm> = Vec::new();
    for (class, road) in &roads {
        let (d, s) = geom::project(road, m);
        if d > r_ext {
            continue;
        }
        let len = geom::length(road);
        let junction = geom::at(road, s).0;
        let mut halves = Vec::new();
        let mut back = geom::slice(road, 0.0, s);
        back.reverse();
        halves.push(back);
        halves.push(geom::slice(road, s, len));
        for mut h in halves {
            if d > 6.0 {
                h[0] = junction;
            } else {
                h[0] = m;
            }
            let h = clip_to(&h, m, r_ext);
            if geom::length(&h) > 20.0 {
                out.push(Arm {
                    points: h,
                    class: *class,
                    to_castle: false,
                });
            }
        }
        if d > 6.0 {
            out.push(Arm {
                points: geom::resample(&[m, junction], 6.0),
                class: *class,
                to_castle: false,
            });
        }
    }
    if focal.kind == FocalKind::Castle {
        let dir = (focal.feature - m).norm();
        let end = focal.feature - dir * 22.0;
        out.push(Arm {
            points: geom::resample(&[m, end], 6.0),
            class: RoadClass::Road,
            to_castle: true,
        });
    }
    if out.is_empty() {
        let (s, c) = geom::sin_cos(crate::rng::unit(hash(seed, 1, 1)) * geom::PI);
        let d = v2(c, s);
        for dir in [d, -d] {
            out.push(Arm {
                points: geom::resample(&[m, m + dir * r_ext], 6.0),
                class: RoadClass::Track,
                to_castle: false,
            });
        }
    }
    dedupe(&mut out);
    let amp = (params.main_w * 0.45).clamp(1.5, 4.5);
    out.iter()
        .enumerate()
        .map(|(i, a)| Arm {
            points: if a.to_castle || geom::length(&a.points) < 40.0 {
                a.points.clone()
            } else {
                wobble(&a.points, hash(seed, 77, i as u64), amp)
            },
            class: a.class,
            to_castle: a.to_castle,
        })
        .collect()
}

fn heading(a: &Arm) -> Vec2 {
    let l = geom::length(&a.points);
    (geom::at(&a.points, l.min(60.0)).0 - a.points[0]).norm()
}

fn dedupe(arms: &mut Vec<Arm>) {
    let mut keep: Vec<Arm> = Vec::new();
    let mut sorted = arms.clone();
    sorted.sort_by(|a, b| {
        b.class
            .rank()
            .cmp(&a.class.rank())
            .then(geom::length(&b.points).total_cmp(&geom::length(&a.points)))
    });
    for a in sorted {
        let h = heading(&a);
        let same_start = |k: &Arm| k.points[0].dist(a.points[0]) < 10.0;
        if keep
            .iter()
            .any(|k| same_start(k) && heading(k).dot(h) > 0.9)
        {
            continue;
        }
        keep.push(a);
    }
    *arms = keep;
}
