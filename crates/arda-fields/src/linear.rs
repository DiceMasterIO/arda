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

/// The input roads in square units.
#[derive(Debug, Clone, Default)]
pub struct RoadNet {
    /// Roads with their class.
    pub roads: Vec<(RoadClass, Line)>,
}

impl RoadNet {
    /// Converts world-metre polylines to square units.
    #[must_use]
    pub fn new(roads: &[Road]) -> Self {
        Self {
            roads: roads
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
                .collect(),
        }
    }

    /// The nearest point on any road centre line and its distance, in squares.
    #[must_use]
    pub fn nearest(&self, p: [f64; 2]) -> Option<([f64; 2], f64)> {
        let mut best: Option<(f64, [f64; 2])> = None;
        for (_, line) in &self.roads {
            if let Some((d2, q)) = line.nearest(p) {
                if best.is_none_or(|(b, _)| d2 < b) {
                    best = Some((d2, q));
                }
            }
        }
        best.map(|(d2, q)| (q, d2.sqrt()))
    }

    /// Whether `p` lies within `pad` squares of any carriageway edge.
    #[must_use]
    pub fn near(&self, p: [f64; 2], pad: f64) -> bool {
        self.roads.iter().any(|(_, l)| {
            l.nearest(p)
                .is_some_and(|(d2, _)| d2 <= (l.half + pad) * (l.half + pad))
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
}
