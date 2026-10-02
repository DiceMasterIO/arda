//! Parcel shape statistics on a raster of parcel ids: elongation, the
//! share of four-sided parcels, T- against Y-junctions and the spread of
//! parcel sizes with distance from a settlement. The measures read only the
//! raster, so any partition can be compared with any other.

#![allow(
    dead_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use std::collections::{BTreeMap, BTreeSet};

/// Parcel ids over a rectangle of squares, row-major (`None` unclaimed).
pub struct Ids {
    pub w: i64,
    pub h: i64,
    pub id: Vec<Option<u64>>,
}

impl Ids {
    pub fn new(w: i64, h: i64, f: impl Fn(i64, i64) -> Option<u64>) -> Self {
        let mut id = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            for x in 0..w {
                id.push(f(x, y));
            }
        }
        Self { w, h, id }
    }

    pub fn at(&self, x: i64, y: i64) -> Option<u64> {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return None;
        }
        self.id[(y * self.w + x) as usize]
    }
}

/// One parcel wholly inside the raster.
#[derive(Debug, Clone)]
pub struct Parcel {
    pub id: u64,
    pub area: f64,
    pub centroid: [f64; 2],
    /// `sqrt(λmax / λmin)` of the second moments.
    pub elongation: f64,
    /// Corners of the simplified outline.
    pub corners: usize,
    /// Each corner of the simplified outline: its position (raster
    /// squares) and interior angle, degrees.
    pub angles: Vec<([f64; 2], f64)>,
}

impl Parcel {
    /// The sharpest interior corner, degrees.
    pub fn min_corner_deg(&self) -> f64 {
        self.angles.iter().map(|a| a.1).fold(180.0, f64::min)
    }
}

/// Every parcel of at least `min_area` squares not touching the edge.
pub fn parcels(ids: &Ids, min_area: usize) -> Vec<Parcel> {
    let mut sq: BTreeMap<u64, Vec<(i64, i64)>> = BTreeMap::new();
    let mut edge = BTreeSet::new();
    for y in 0..ids.h {
        for x in 0..ids.w {
            if let Some(i) = ids.at(x, y) {
                sq.entry(i).or_default().push((x, y));
                if x == 0 || y == 0 || x == ids.w - 1 || y == ids.h - 1 {
                    edge.insert(i);
                }
            }
        }
    }
    let mut out = Vec::new();
    for (id, s) in sq {
        if edge.contains(&id) || s.len() < min_area {
            continue;
        }
        let n = s.len() as f64;
        let (mx, my) = (
            s.iter().map(|p| p.0 as f64).sum::<f64>() / n,
            s.iter().map(|p| p.1 as f64).sum::<f64>() / n,
        );
        let (mut a, mut b, mut c) = (0.0, 0.0, 0.0);
        for p in &s {
            let (dx, dy) = (p.0 as f64 - mx, p.1 as f64 - my);
            a += dx * dx;
            b += dx * dy;
            c += dy * dy;
        }
        let (a, b, c) = (a / n, b / n, c / n);
        let outline = corners(ids, id, &s);
        let t = ((a - c) * (a - c) / 4.0 + b * b).sqrt();
        let (l1, l2) = ((a + c) / 2.0 + t, ((a + c) / 2.0 - t).max(1e-9));
        out.push(Parcel {
            id,
            area: n,
            centroid: [mx + 0.5, my + 0.5],
            elongation: (l1 / l2).sqrt(),
            corners: outline.len(),
            angles: interior_angles(&outline),
        });
    }
    out
}

/// Corners of a parcel's outer outline: the boundary loop from its first
/// square, simplified (Douglas–Peucker) and with near-straight vertices
/// dropped.
fn corners(ids: &Ids, id: u64, squares: &[(i64, i64)]) -> Vec<[f64; 2]> {
    let inside = |x: i64, y: i64| ids.at(x, y) == Some(id);
    // Directed boundary edges with the parcel on the left (y down: walk
    // clockwise on screen), keyed by start vertex.
    let mut next: BTreeMap<(i64, i64), Vec<(i64, i64)>> = BTreeMap::new();
    for &(x, y) in squares {
        if !inside(x, y - 1) {
            next.entry((x + 1, y)).or_default().push((x, y));
        }
        if !inside(x, y + 1) {
            next.entry((x, y + 1)).or_default().push((x + 1, y + 1));
        }
        if !inside(x - 1, y) {
            next.entry((x, y)).or_default().push((x, y + 1));
        }
        if !inside(x + 1, y) {
            next.entry((x + 1, y + 1)).or_default().push((x + 1, y));
        }
    }
    // The outer loop starts at the top-left vertex.
    let Some(&start) = next.keys().min_by_key(|v| (v.1, v.0)) else {
        return Vec::new();
    };
    let mut lp = vec![start];
    let mut cur = start;
    for _ in 0..1_000_000 {
        let Some(n) = next.get_mut(&cur).and_then(Vec::pop) else {
            break;
        };
        if n == start {
            break;
        }
        lp.push(n);
        cur = n;
    }
    let pts: Vec<[f64; 2]> = lp.iter().map(|v| [v.0 as f64, v.1 as f64]).collect();
    let area = squares.len() as f64;
    let tol = (0.035 * area.sqrt()).clamp(1.8, 4.0);
    let simple = simplify_closed(&pts, tol);
    // Drop vertices that barely turn (kinks, staircase residue).
    let mut v = simple;
    loop {
        let n = v.len();
        if n <= 3 {
            break;
        }
        let mut drop = None;
        for i in 0..n {
            let (a, b, c) = (v[(i + n - 1) % n], v[i], v[(i + 1) % n]);
            if turn_deg(a, b, c) < 28.0 {
                drop = Some(i);
                break;
            }
        }
        match drop {
            Some(i) => {
                v.remove(i);
            }
            None => break,
        }
    }
    v
}

/// Interior angles of a simple polygon's corners, degrees, whichever way
/// it winds.
fn interior_angles(v: &[[f64; 2]]) -> Vec<([f64; 2], f64)> {
    let n = v.len();
    if n < 3 {
        return Vec::new();
    }
    let area2: f64 = (0..n)
        .map(|i| {
            let (a, b) = (v[i], v[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum();
    let orient = area2.signum();
    (0..n)
        .map(|i| {
            let (a, b, c) = (v[(i + n - 1) % n], v[i], v[(i + 1) % n]);
            let (u, w) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
            let turn = (u[0] * w[1] - u[1] * w[0]).atan2(u[0] * w[0] + u[1] * w[1]);
            (b, 180.0 - orient * turn.to_degrees())
        })
        .collect()
}

fn turn_deg(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    let (u, w) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
    let cross = u[0] * w[1] - u[1] * w[0];
    let dot = u[0] * w[0] + u[1] * w[1];
    cross.atan2(dot).abs().to_degrees()
}

fn simplify_closed(p: &[[f64; 2]], tol: f64) -> Vec<[f64; 2]> {
    let n = p.len();
    if n < 4 {
        return p.to_vec();
    }
    // Split at the vertex farthest from the first.
    let far = (1..n)
        .max_by(|&i, &j| {
            let d = |k: usize| (p[k][0] - p[0][0]).hypot(p[k][1] - p[0][1]);
            d(i).total_cmp(&d(j))
        })
        .unwrap_or(n / 2);
    let mut a = dp(&p[..=far], tol);
    let mut tail: Vec<[f64; 2]> = p[far..].to_vec();
    tail.push(p[0]);
    let b = dp(&tail, tol);
    a.pop();
    a.extend_from_slice(&b[..b.len() - 1]);
    a
}

fn dp(p: &[[f64; 2]], tol: f64) -> Vec<[f64; 2]> {
    if p.len() < 3 {
        return p.to_vec();
    }
    let (a, b) = (p[0], p[p.len() - 1]);
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = dx.hypot(dy).max(1e-9);
    let (mut best, mut k) = (0.0, 0);
    for (i, q) in p.iter().enumerate().take(p.len() - 1).skip(1) {
        let d = ((q[0] - a[0]) * dy - (q[1] - a[1]) * dx).abs() / len;
        if d > best {
            best = d;
            k = i;
        }
    }
    if best <= tol {
        return vec![a, b];
    }
    let mut l = dp(&p[..=k], tol);
    let r = dp(&p[k..], tol);
    l.pop();
    l.extend(r);
    l
}

/// Junctions where three or more parcels meet, classed by the widest gap
/// between their boundary branches: `(t, y)`.
pub fn junctions(ids: &Ids) -> (usize, usize) {
    // Lattice vertex (x, y) touches squares (x-1..=x, y-1..=y).
    let key = |x: i64, y: i64| ids.at(x, y);
    let edge_h = |x: i64, y: i64| key(x, y - 1) != key(x, y); // vertex (x,y)→(x+1,y)
    let edge_v = |x: i64, y: i64| key(x - 1, y) != key(x, y); // vertex (x,y)→(x,y+1)
    let nbrs = |x: i64, y: i64| -> Vec<(i64, i64)> {
        let mut v = Vec::new();
        if edge_h(x, y) {
            v.push((x + 1, y));
        }
        if edge_h(x - 1, y) {
            v.push((x - 1, y));
        }
        if edge_v(x, y) {
            v.push((x, y + 1));
        }
        if edge_v(x, y - 1) {
            v.push((x, y - 1));
        }
        v
    };
    let distinct = |x: i64, y: i64| {
        let mut s = BTreeSet::new();
        for (dx, dy) in [(-1, -1), (0, -1), (-1, 0), (0, 0)] {
            s.insert(key(x + dx, y + dy));
        }
        s.len()
    };
    let margin = 16;
    let mut found: Vec<(i64, i64)> = Vec::new();
    let (mut t, mut yj) = (0, 0);
    for y in margin..ids.h - margin {
        for x in margin..ids.w - margin {
            let around = [(-1, -1), (0, -1), (-1, 0), (0, 0)];
            if around.iter().any(|(dx, dy)| key(x + dx, y + dy).is_none()) {
                continue;
            }
            if distinct(x, y) < 3 || nbrs(x, y).len() < 3 {
                continue;
            }
            if found
                .iter()
                .any(|f| (f.0 - x).abs() <= 4 && (f.1 - y).abs() <= 4)
            {
                continue;
            }
            found.push((x, y));
            // Follow each branch up to 14 steps, through degree-2 vertices
            // and past the junction's own raster cluster.
            let mut dirs = Vec::new();
            for first in nbrs(x, y) {
                let (mut prev, mut cur) = ((x, y), first);
                for _ in 0..14 {
                    let n: Vec<_> = nbrs(cur.0, cur.1)
                        .into_iter()
                        .filter(|&q| q != prev)
                        .collect();
                    let near = (cur.0 - x).abs() <= 2 && (cur.1 - y).abs() <= 2;
                    if n.len() != 1 && !(near && !n.is_empty()) {
                        break;
                    }
                    (prev, cur) = (cur, n[0]);
                }
                let d = ((cur.0 - x) as f64, (cur.1 - y) as f64);
                if d.0.hypot(d.1) >= 5.0 {
                    dirs.push(d.1.atan2(d.0));
                }
            }
            if dirs.len() < 3 {
                continue;
            }
            dirs.sort_by(f64::total_cmp);
            let tau = std::f64::consts::TAU;
            let mut gap: f64 = 0.0;
            for i in 0..dirs.len() {
                let g = (dirs[(i + 1) % dirs.len()] - dirs[i]).rem_euclid(tau);
                gap = gap.max(if dirs.len() == 1 { tau } else { g });
            }
            if gap.to_degrees() >= 155.0 {
                t += 1;
            } else {
                yj += 1;
            }
        }
    }
    (t, yj)
}

/// Summary of a raster's parcels.
#[derive(Debug, Clone)]
pub struct Summary {
    pub n: usize,
    pub elong_median: f64,
    pub elong_p90: f64,
    pub four_sided: f64,
    pub t: usize,
    pub y: usize,
}

impl Summary {
    pub fn t_share(&self) -> f64 {
        self.t as f64 / (self.t + self.y).max(1) as f64
    }
}

pub fn summary(ids: &Ids, min_area: usize) -> (Summary, Vec<Parcel>) {
    let ps = parcels(ids, min_area);
    let mut el: Vec<f64> = ps.iter().map(|p| p.elongation).collect();
    el.sort_by(f64::total_cmp);
    let q = |f: f64| {
        el.get(((el.len() as f64 - 1.0) * f) as usize)
            .copied()
            .unwrap_or(0.0)
    };
    let four = ps.iter().filter(|p| p.corners == 4).count() as f64 / ps.len().max(1) as f64;
    let (t, y) = junctions(ids);
    (
        Summary {
            n: ps.len(),
            elong_median: q(0.5),
            elong_p90: q(0.9),
            four_sided: four,
            t,
            y,
        },
        ps,
    )
}

/// Mean and standard deviation of parcel area (squares) in distance bands
/// (squares) from `centre`.
pub fn size_by_distance(
    ps: &[Parcel],
    centre: [f64; 2],
    bands: &[(f64, f64)],
) -> Vec<(f64, f64, usize)> {
    bands
        .iter()
        .map(|&(lo, hi)| {
            let a: Vec<f64> = ps
                .iter()
                .filter(|p| {
                    let d = (p.centroid[0] - centre[0]).hypot(p.centroid[1] - centre[1]);
                    d >= lo && d < hi
                })
                .map(|p| p.area)
                .collect();
            let n = a.len().max(1) as f64;
            let m = a.iter().sum::<f64>() / n;
            let v = a.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / n;
            (m, v.sqrt(), a.len())
        })
        .collect()
}
