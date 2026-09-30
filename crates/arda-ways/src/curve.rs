//! Smooth centrelines: world polylines become quadratic B-spline pieces
//! (optionally with a switchback wave), sampled at a fixed step into dense
//! stations that carry arc length and design height.
//!
//! Every piece depends only on three consecutive polyline vertices and is
//! sampled at a resolution fixed by its own geometry, so a window never
//! changes the curve: two windows sharing a road sample it identically
//! (goal 46, `mockup-artifact.md` "Inside a cell": linear features must
//! leave one block exactly where they enter the next).

use std::f64::consts::PI;

/// A world point in metres (x east, y south).
pub type P = [f64; 2];

/// Dense sampling step along a piece, metres.
pub const STEP_M: f64 = 0.5;

/// `a + (b - a) t`.
#[must_use]
pub fn lerp(a: P, b: P, t: f64) -> P {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

/// Euclidean distance.
#[must_use]
pub fn dist(a: P, b: P) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// Unit vector of `v`, or east for a zero vector.
#[must_use]
pub fn unit(v: P) -> P {
    let l = v[0].hypot(v[1]);
    if l > 1e-12 {
        [v[0] / l, v[1] / l]
    } else {
        [1.0, 0.0]
    }
}

/// The geometric shape of a piece before any switchback wave.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// A straight run.
    Line(P, P),
    /// A quadratic Bézier from edge midpoint to edge midpoint, with the
    /// polyline vertex as control point (a uniform quadratic B-spline).
    Quad(P, P, P),
}

impl Shape {
    /// Position at `t` in `0..=1`.
    #[must_use]
    pub fn at(&self, t: f64) -> P {
        match *self {
            Self::Line(a, b) => lerp(a, b, t),
            Self::Quad(a, c, b) => {
                let u = 1.0 - t;
                [
                    u * u * a[0] + 2.0 * u * t * c[0] + t * t * b[0],
                    u * u * a[1] + 2.0 * u * t * c[1] + t * t * b[1],
                ]
            }
        }
    }

    /// Derivative at `t` (not normalised).
    #[must_use]
    pub fn tangent(&self, t: f64) -> P {
        match *self {
            Self::Line(a, b) => [b[0] - a[0], b[1] - a[1]],
            Self::Quad(a, c, b) => [
                2.0 * (1.0 - t) * (c[0] - a[0]) + 2.0 * t * (b[0] - c[0]),
                2.0 * (1.0 - t) * (c[1] - a[1]) + 2.0 * t * (b[1] - c[1]),
            ],
        }
    }

    /// End points.
    #[must_use]
    pub fn ends(&self) -> (P, P) {
        match *self {
            Self::Line(a, b) | Self::Quad(a, _, b) => (a, b),
        }
    }
}

/// Peak rounding of the switchback wave: 1 would give sharp hairpins.
const HAIRPIN: f64 = 0.985;

/// A switchback wave laid across a piece: a rounded triangle wave, so legs
/// run straight across the slope and turn in tight hairpins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zig {
    /// Lateral amplitude, metres.
    pub amp: f64,
    /// Number of half-waves (legs).
    pub legs: u32,
}

/// One piece of a centreline with its design heights and arc length.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    /// Geometry.
    pub shape: Shape,
    /// Optional switchback wave.
    pub zig: Option<Zig>,
    /// Design height at the start, metres.
    pub z0: f64,
    /// Design height at the end, metres.
    pub z1: f64,
    /// Arc length at the start of the piece, metres.
    pub s0: f64,
    /// Arc length of the piece, metres.
    pub len: f64,
    /// Sample count (fixed by the geometry).
    pub samples: u32,
}

/// A dense centreline sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Station {
    /// Position.
    pub p: P,
    /// Arc length from the start of the way, metres.
    pub s: f64,
    /// Design height, metres.
    pub z: f64,
}

/// Position on a (possibly waved) shape.
#[must_use]
pub fn wave_at(shape: &Shape, zig: Option<Zig>, t: f64) -> P {
    let base = shape.at(t);
    match zig {
        None => base,
        Some(z) => {
            let d = unit(shape.tangent(t));
            let x = (PI * f64::from(z.legs) * t).sin();
            let o = z.amp * (HAIRPIN * x).asin() / HAIRPIN.asin();
            [base[0] - d[1] * o, base[1] + d[0] * o]
        }
    }
}

/// Polyline length of a waved shape at `n` samples.
#[must_use]
pub fn wave_len(shape: &Shape, zig: Option<Zig>, n: u32) -> f64 {
    let mut prev = wave_at(shape, zig, 0.0);
    let mut len = 0.0;
    for k in 1..=n {
        let q = wave_at(shape, zig, f64::from(k) / f64::from(n));
        len += dist(prev, q);
        prev = q;
    }
    len
}

impl Piece {
    /// Builds a piece, fixing its sample count and length.
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // bounded counts
    pub fn new(shape: Shape, zig: Option<Zig>, z0: f64, z1: f64) -> Self {
        let rough = wave_len(&shape, zig, 64 * zig.map_or(1, |z| z.legs.max(1)));
        let samples = ((rough / STEP_M).ceil() as u32).clamp(2, 1 << 20);
        let len = wave_len(&shape, zig, samples);
        Self {
            shape,
            zig,
            z0,
            z1,
            s0: 0.0,
            len,
            samples,
        }
    }

    /// Axis-aligned bounds, padded by the wave amplitude.
    #[must_use]
    pub fn bounds(&self) -> [f64; 4] {
        let pts: Vec<P> = match self.shape {
            Shape::Line(a, b) => vec![a, b],
            Shape::Quad(a, c, b) => vec![a, c, b],
        };
        let pad = self.zig.map_or(0.0, |z| z.amp.abs());
        let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
        for p in pts {
            b = [
                b[0].min(p[0]),
                b[1].min(p[1]),
                b[2].max(p[0]),
                b[3].max(p[1]),
            ];
        }
        [b[0] - pad, b[1] - pad, b[2] + pad, b[3] + pad]
    }

    /// Dense stations, heights linear in arc length (a constant grade).
    #[must_use]
    pub fn stations(&self) -> Vec<Station> {
        let mut out = Vec::with_capacity(self.samples as usize + 1);
        let mut prev = wave_at(&self.shape, self.zig, 0.0);
        let mut s = 0.0;
        for k in 0..=self.samples {
            let t = f64::from(k) / f64::from(self.samples);
            let p = wave_at(&self.shape, self.zig, t);
            s += dist(prev, p);
            prev = p;
            let f = if self.len > 0.0 { s / self.len } else { t };
            out.push(Station {
                p,
                s: self.s0 + s,
                z: self.z0 + (self.z1 - self.z0) * f,
            });
        }
        out
    }
}

/// The shapes of a polyline: a straight lead-in to the first edge midpoint,
/// a quadratic per interior vertex, and a straight lead-out.
#[must_use]
pub fn shapes(v: &[P]) -> Vec<Shape> {
    let mut pts: Vec<P> = Vec::with_capacity(v.len());
    for &p in v {
        if pts.last().is_none_or(|&q| dist(p, q) > 1e-9) {
            pts.push(p);
        }
    }
    match pts.len() {
        0 | 1 => Vec::new(),
        2 => vec![Shape::Line(pts[0], pts[1])],
        n => {
            let mid = |i: usize| lerp(pts[i], pts[i + 1], 0.5);
            let mut out = vec![Shape::Line(pts[0], mid(0))];
            out.extend((1..n - 1).map(|i| Shape::Quad(mid(i - 1), pts[i], mid(i))));
            out.push(Shape::Line(mid(n - 2), pts[n - 1]));
            out
        }
    }
}

/// Cubic Hermite from `a` (direction `ta`) to `b` (direction `tb`),
/// sampled every [`STEP_M`], excluding `a` and including `b`.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // bounded counts
pub fn hermite(a: P, ta: P, b: P, tb: P) -> Vec<P> {
    let l = dist(a, b);
    let (ta, tb) = (unit(ta), unit(tb));
    let n = ((l * 1.3 / STEP_M).ceil() as u32).max(1);
    (1..=n)
        .map(|k| {
            let t = f64::from(k) / f64::from(n);
            let (t2, t3) = (t * t, t * t * t);
            let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
            let h10 = t3 - 2.0 * t2 + t;
            let h01 = -2.0 * t3 + 3.0 * t2;
            let h11 = t3 - t2;
            [
                h00 * a[0] + h10 * l * ta[0] + h01 * b[0] + h11 * l * tb[0],
                h00 * a[1] + h10 * l * ta[1] + h01 * b[1] + h11 * l * tb[1],
            ]
        })
        .collect()
}

/// The nearest point of a dense run to a query.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    /// Unsigned distance, metres.
    pub d: f64,
    /// Signed distance, positive to the right of travel (x east, y south).
    pub side: f64,
    /// Arc length at the foot point.
    pub s: f64,
    /// Design height at the foot point.
    pub z: f64,
    /// Unit direction of travel.
    pub dir: P,
    /// The foot point.
    pub foot: P,
}

/// Station runs chunked with bounds for fast nearest queries.
#[derive(Debug, Clone, Default)]
pub struct Dense {
    /// Contiguous runs of stations.
    pub runs: Vec<Vec<Station>>,
    chunks: Vec<(usize, usize, [f64; 4])>,
}

const CHUNK: usize = 32;

impl Dense {
    /// Builds the chunk index over `runs`.
    #[must_use]
    pub fn new(runs: Vec<Vec<Station>>) -> Self {
        let mut chunks = Vec::new();
        for (r, run) in runs.iter().enumerate() {
            let mut i = 0;
            while i + 1 < run.len() {
                let j = (i + CHUNK).min(run.len() - 1);
                let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
                for st in &run[i..=j] {
                    b = [
                        b[0].min(st.p[0]),
                        b[1].min(st.p[1]),
                        b[2].max(st.p[0]),
                        b[3].max(st.p[1]),
                    ];
                }
                chunks.push((r, i, b));
                i = j;
            }
        }
        Self { runs, chunks }
    }

    /// The nearest foot point within `reach` metres, if any.
    #[must_use]
    pub fn nearest(&self, q: P, reach: f64) -> Option<Hit> {
        let mut best: Option<Hit> = None;
        for &(r, i0, b) in &self.chunks {
            let lim = best.map_or(reach, |h| h.d.min(reach));
            if q[0] < b[0] - lim || q[0] > b[2] + lim || q[1] < b[1] - lim || q[1] > b[3] + lim {
                continue;
            }
            let run = &self.runs[r];
            let i1 = (i0 + CHUNK).min(run.len() - 1);
            for i in i0..i1 {
                if let Some(h) = seg_hit(&run[i], &run[i + 1], q) {
                    if h.d <= reach && best.is_none_or(|b| h.d < b.d) {
                        best = Some(h);
                    }
                }
            }
        }
        best
    }
}

fn seg_hit(a: &Station, b: &Station, q: P) -> Option<Hit> {
    let v = [b.p[0] - a.p[0], b.p[1] - a.p[1]];
    let l2 = v[0] * v[0] + v[1] * v[1];
    if l2 <= 1e-18 {
        return None;
    }
    let w = [q[0] - a.p[0], q[1] - a.p[1]];
    let t = ((w[0] * v[0] + w[1] * v[1]) / l2).clamp(0.0, 1.0);
    let foot = lerp(a.p, b.p, t);
    let d = dist(q, foot);
    let dir = unit(v);
    // Right of travel in a y-south frame is the sign of dir × w.
    let cross = dir[0] * (q[1] - foot[1]) - dir[1] * (q[0] - foot[0]);
    Some(Hit {
        d,
        side: if cross >= 0.0 { d } else { -d },
        s: a.s + (b.s - a.s) * t,
        z: a.z + (b.z - a.z) * t,
        dir,
        foot,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadratic_pieces_join_at_midpoints() {
        let s = shapes(&[[0.0, 0.0], [100.0, 0.0], [100.0, 100.0]]);
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].ends().1, [50.0, 0.0]);
        assert_eq!(s[1].ends(), ([50.0, 0.0], [100.0, 50.0]));
    }

    #[test]
    fn nearest_reports_side_and_arc() {
        let p = Piece::new(Shape::Line([0.0, 0.0], [10.0, 0.0]), None, 0.0, 1.0);
        let d = Dense::new(vec![p.stations()]);
        let h = d.nearest([5.0, 2.0], 10.0).unwrap();
        assert!((h.d - 2.0).abs() < 1e-9);
        assert!(h.side > 0.0, "south of an eastbound road is its right");
        assert!((h.s - 5.0).abs() < 1e-9 && (h.z - 0.5).abs() < 1e-9);
    }
}
