//! Roads and farm lanes: polylines in square units, rasterised by distance
//! from their centre lines so every window paints the same squares.

use crate::geom::{seg_dist2, Grid, Sq, SQUARE_M};
use crate::input::{Road, RoadClass};

/// Longest farm lane, in squares (about 1.5 km). Compounds farther from any
/// road get no lane and are reported as isolated.
pub const MAX_LANE_SQ: f64 = 960.0;
/// Half-width of a farm lane, in squares (a 3 m cart lane).
pub const LANE_HALF_SQ: f64 = 1.0;

/// A polyline in square units with a half-width.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// Vertices in square units.
    pub pts: Vec<[f64; 2]>,
    /// Half-width in squares.
    pub half: f64,
}

impl Line {
    /// Squared distance from `p` to the centre line, and the closest point.
    #[must_use]
    pub fn nearest(&self, p: [f64; 2]) -> Option<(f64, [f64; 2])> {
        let mut best: Option<(f64, [f64; 2])> = None;
        for w in self.pts.windows(2) {
            let (d2, t) = seg_dist2(p, w[0], w[1]);
            let q = [
                w[0][0] + t * (w[1][0] - w[0][0]),
                w[0][1] + t * (w[1][1] - w[0][1]),
            ];
            if best.is_none_or(|(b, _)| d2 < b) {
                best = Some((d2, q));
            }
        }
        if self.pts.len() == 1 {
            let q = self.pts[0];
            let d2 = (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2);
            best = Some((d2, q));
        }
        best
    }

    /// Calls `f` for every grid square whose centre lies within the band.
    #[allow(clippy::cast_possible_truncation)] // floor/ceil of map positions
    pub fn for_each_square(&self, grid_rect: (i64, i64, i64, i64), mut f: impl FnMut(Sq)) {
        let (gx0, gy0, gx1, gy1) = grid_rect;
        let r2 = self.half * self.half;
        let segs: Vec<[[f64; 2]; 2]> = if self.pts.len() == 1 {
            vec![[self.pts[0], self.pts[0]]]
        } else {
            self.pts.windows(2).map(|w| [w[0], w[1]]).collect()
        };
        for [a, b] in segs {
            let x0 = ((a[0].min(b[0]) - self.half).floor() as i64).max(gx0);
            let x1 = ((a[0].max(b[0]) + self.half).ceil() as i64).min(gx1);
            let y0 = ((a[1].min(b[1]) - self.half).floor() as i64).max(gy0);
            let y1 = ((a[1].max(b[1]) + self.half).ceil() as i64).min(gy1);
            for y in y0..y1 {
                for x in x0..x1 {
                    let s = Sq::new(x, y);
                    // Half-open band: squares exactly on an edge count on
                    // one side only, so a straight carriageway is exactly
                    // `2 * half` squares wide whatever its alignment (I4,
                    // as arda-ways paints `(-W/2, W/2]`).
                    let c = s.centre();
                    let d2 = seg_dist2(c, a, b).0;
                    let side = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
                    if d2 < r2 || (d2 == r2 && side > 0.0) {
                        f(s);
                    }
                }
            }
        }
    }
}

/// Segments per bounding box of a road's search index.
const CHUNK: usize = 16;

/// A bounding box over a run of a line's segments: `[x0, y0, x1, y1]`, and
/// the run's first vertex and one past its last.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Chunk {
    bbox: [f64; 4],
    from: usize,
    to: usize,
}

impl Chunk {
    /// Squared distance from `p` to the box, a lower bound on its
    /// segments' distances; slightly shrunk, so float rounding in
    /// [`seg_dist2`] never makes a pruned segment nearer than the bound.
    fn lower_bound(&self, p: [f64; 2]) -> f64 {
        let dx = (self.bbox[0] - p[0]).max(p[0] - self.bbox[2]).max(0.0);
        let dy = (self.bbox[1] - p[1]).max(p[1] - self.bbox[3]).max(0.0);
        (dx * dx + dy * dy) * (1.0 - 1e-9) - 1e-9
    }
}

/// Each run of [`CHUNK`] segments of `pts`, boxed (review 3.5-14: a dense
/// road centreline made every distance query scan every segment).
fn chunks(pts: &[[f64; 2]]) -> Vec<Chunk> {
    let segs = pts.len().saturating_sub(1).max(1);
    (0..segs)
        .step_by(CHUNK)
        .map(|from| {
            let to = (from + CHUNK + 1).min(pts.len());
            let mut bbox = [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ];
            for q in &pts[from..to] {
                bbox = [
                    bbox[0].min(q[0]),
                    bbox[1].min(q[1]),
                    bbox[2].max(q[0]),
                    bbox[3].max(q[1]),
                ];
            }
            Chunk { bbox, from, to }
        })
        .collect()
}

/// The input roads in square units.
#[derive(Debug, Clone, Default)]
pub struct RoadNet {
    /// Roads with their class.
    pub roads: Vec<(RoadClass, Line)>,
    /// Each road's boxed segment runs, in road order.
    index: Vec<Vec<Chunk>>,
}

impl RoadNet {
    /// Converts world-metre polylines to square units.
    #[must_use]
    pub fn new(roads: &[Road]) -> Self {
        let roads: Vec<(RoadClass, Line)> = roads
            .iter()
            .filter(|r| !r.points.is_empty() && r.class != RoadClass::None)
            .map(|r| {
                let pts = r
                    .points
                    .iter()
                    .map(|p| [p[0] / SQUARE_M, p[1] / SQUARE_M])
                    .collect();
                (
                    r.class,
                    Line {
                        pts,
                        half: r.class.half_width_sq(),
                    },
                )
            })
            .collect();
        let index = roads.iter().map(|(_, l)| chunks(&l.pts)).collect();
        Self { roads, index }
    }

    /// The nearest point of road `i`'s segments in boxes whose lower bound
    /// beats `best`, folded into `best` exactly as a scan of every segment
    /// in order would (first strict minimum wins).
    fn fold_nearest(&self, i: usize, p: [f64; 2], best: &mut Option<(f64, [f64; 2])>) {
        let line = &self.roads[i].1;
        if line.pts.len() < 2 {
            if let Some((d2, q)) = line.nearest(p) {
                if best.is_none_or(|(b, _)| d2 < b) {
                    *best = Some((d2, q));
                }
            }
            return;
        }
        for c in &self.index[i] {
            if best.is_some_and(|(b, _)| c.lower_bound(p) > b) {
                continue;
            }
            for w in line.pts[c.from..c.to].windows(2) {
                let (d2, t) = seg_dist2(p, w[0], w[1]);
                if best.is_none_or(|(b, _)| d2 < b) {
                    let q = [
                        w[0][0] + t * (w[1][0] - w[0][0]),
                        w[0][1] + t * (w[1][1] - w[0][1]),
                    ];
                    *best = Some((d2, q));
                }
            }
        }
    }

    /// The nearest point on any road centre line and its distance, in squares.
    #[must_use]
    pub fn nearest(&self, p: [f64; 2]) -> Option<([f64; 2], f64)> {
        let mut best: Option<(f64, [f64; 2])> = None;
        for i in 0..self.roads.len() {
            self.fold_nearest(i, p, &mut best);
        }
        best.map(|(d2, q)| (q, d2.sqrt()))
    }

    /// Whether `p` lies within `pad` squares of any carriageway edge.
    #[must_use]
    pub fn near(&self, p: [f64; 2], pad: f64) -> bool {
        self.roads.iter().enumerate().any(|(i, (_, l))| {
            let r2 = (l.half + pad) * (l.half + pad);
            if l.pts.len() < 2 {
                return l.nearest(p).is_some_and(|(d2, _)| d2 <= r2);
            }
            self.index[i].iter().any(|c| {
                c.lower_bound(p) <= r2
                    && line_windows(&l.pts[c.from..c.to]).any(|(a, b)| seg_dist2(p, a, b).0 <= r2)
            })
        })
    }

    /// Paints road classes into a grid; wider classes win where roads meet.
    pub fn rasterize(&self, grid: &mut Grid<Option<RoadClass>>) {
        let rect = (grid.x0, grid.y0, grid.x0 + grid.w, grid.y0 + grid.h);
        for (class, line) in &self.roads {
            line.for_each_square(rect, |s| {
                if let Some(slot) = grid.get_mut(s) {
                    if slot.is_none_or(|c| class.width_sq() > c.width_sq()) {
                        *slot = Some(*class);
                    }
                }
            });
        }
    }
}

/// The segments of a run of vertices.
fn line_windows(pts: &[[f64; 2]]) -> impl Iterator<Item = ([f64; 2], [f64; 2])> + '_ {
    pts.windows(2).map(|w| (w[0], w[1]))
}

/// The lane from a compound's gate to the nearest road, if one is in reach.
///
/// It leaves the gate square straight for three squares along `out`, then
/// heads for the nearest point on any road.
#[must_use]
#[allow(clippy::cast_precision_loss)] // small unit vectors
pub fn lane(net: &RoadNet, start: Sq, out: [i64; 2]) -> Option<Line> {
    let a = start.centre();
    let b = [a[0] + 3.0 * out[0] as f64, a[1] + 3.0 * out[1] as f64];
    let (q, d) = net.nearest(b)?;
    if d > MAX_LANE_SQ {
        return None;
    }
    let (_, d_start) = net.nearest(a)?;
    let pts = if d_start <= d {
        vec![a, net.nearest(a)?.0]
    } else {
        vec![a, b, q]
    };
    Some(Line {
        pts,
        half: LANE_HALF_SQ,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn across(class: RoadClass, x_sq: f64) -> usize {
        let net = RoadNet::new(&[Road {
            class,
            points: vec![[x_sq * SQUARE_M, 0.0], [x_sq * SQUARE_M, 64.0 * SQUARE_M]],
        }]);
        let mut g = Grid::new(0, 0, 32, 32, None);
        net.rasterize(&mut g);
        (0..32)
            .filter(|&x| g.get(Sq::new(x, 10)) != Some(&None))
            .count()
    }

    #[test]
    fn carriageways_are_exactly_their_class_width_whatever_the_alignment() {
        // I4: highway 5, road 4, track 2, footpath 1 squares, as arda-ways
        // paints them; a closed band painted W + 1 on half the alignments.
        for class in [
            RoadClass::Highway,
            RoadClass::Road,
            RoadClass::Track,
            RoadClass::Footpath,
        ] {
            for x in [10.0, 10.5, 10.25, 10.75] {
                assert_eq!(
                    across(class, x),
                    usize::from(class.width_sq()),
                    "{class:?} at x = {x}"
                );
            }
        }
    }

    #[test]
    fn a_lane_band_is_four_connected_on_a_diagonal() {
        let l = Line {
            pts: vec![[0.5, 0.5], [30.5, 30.5]],
            half: LANE_HALF_SQ,
        };
        let mut g = Grid::new(0, 0, 32, 32, false);
        l.for_each_square((0, 0, 32, 32), |s| g.set(s, true));
        let mut seen = vec![Sq::new(0, 0)];
        let mut i = 0;
        while i < seen.len() {
            let s = seen[i];
            i += 1;
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let n = s.offset(dx, dy);
                if g.get(n) == Some(&true) && !seen.contains(&n) {
                    seen.push(n);
                }
            }
        }
        assert!(seen.contains(&Sq::new(30, 30)));
    }

    #[test]
    fn the_boxed_index_answers_as_a_scan_of_every_segment() {
        // Dense wiggly roads, a single-point road and a short one.
        let mut roads = Vec::new();
        for r in 0..4_u32 {
            let points = (0..300_u32)
                .map(|k| {
                    let t = f64::from(k) * 1.25;
                    let o = f64::from(r) * 37.0;
                    [t + o, 80.0 + o + 6.0 * arda_ids::pmath::sin(t / 9.0)]
                })
                .collect();
            roads.push(Road {
                class: RoadClass::Road,
                points,
            });
        }
        roads.push(Road {
            class: RoadClass::Track,
            points: vec![[50.0, 50.0]],
        });
        roads.push(Road {
            class: RoadClass::Footpath,
            points: vec![[10.0, 0.0], [12.0, 3.0]],
        });
        let net = RoadNet::new(&roads);
        for j in 0..60_u32 {
            for i in 0..60_u32 {
                let p = [f64::from(i) * 4.1 - 20.0, f64::from(j) * 3.7 - 10.0];
                let mut best: Option<(f64, [f64; 2])> = None;
                for (_, l) in &net.roads {
                    if let Some((d2, q)) = l.nearest(p) {
                        if best.is_none_or(|(b, _)| d2 < b) {
                            best = Some((d2, q));
                        }
                    }
                }
                let scan = best.map(|(d2, q)| (q, d2.sqrt()));
                assert_eq!(net.nearest(p), scan, "nearest at {p:?}");
                for pad in [0.0, 1.5, 7.0] {
                    let scan = net.roads.iter().any(|(_, l)| {
                        l.nearest(p)
                            .is_some_and(|(d2, _)| d2 <= (l.half + pad) * (l.half + pad))
                    });
                    assert_eq!(net.near(p, pad), scan, "near at {p:?} pad {pad}");
                }
            }
        }
    }
}
