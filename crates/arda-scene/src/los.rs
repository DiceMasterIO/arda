//! Line of sight and cover with exact wall-edge intersection.
//!
//! Geometry runs in integers at [`SCALE`] units per square, so a square's
//! centre is `8x + 4` and nothing is rounded. A sightline is walked through
//! every grid-line crossing in order:
//! - crossing a wall edge that blocks sight blocks it; crossing one that
//!   does not (a window) adds that edge's cover;
//! - entering a heavily obscured square other than the target blocks it;
//! - passing exactly through a grid vertex blocks it only when sight
//!   blockers (wall arms at the vertex, or opaque squares beside the line)
//!   lie on *both* sides of the line. A line may graze a wall end or the
//!   outside of a corner, but cannot slip through a closed corner or between
//!   two diagonally touching opaque squares. This vertex rule is an Arda
//!   house rule: SRD 5.1 has no grid line-of-sight procedure.

use crate::index::SceneIndex;
use crate::types::{CoverLevel, Sq};
use arda_tactical::layout::EdgeAxis;

/// Integer units per square.
pub const SCALE: i64 = 8;
/// Sample points for cover sit this far inside each square corner, so they
/// never lie on a grid line.
const INSET: i64 = 1;

/// What a sightline met.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Trace {
    /// Something blocked sight.
    pub blocked: bool,
    /// The strongest cover it passed (squares strictly between, windows).
    pub cover: CoverLevel,
}

#[derive(Debug, Clone, Copy)]
enum Event {
    /// Crosses the vertical grid line `x = k` inside row `r`.
    V(i64, i64),
    /// Crosses the horizontal grid line `y = j` inside column `c`.
    H(i64, i64),
    /// Passes exactly through grid vertex `(vx, vy)`.
    Vertex(i64, i64),
}

/// Every grid crossing of the open segment `p → q`, with its parameter
/// `t = num / den` (den > 0), in order along the segment.
fn crossings(p: (i64, i64), q: (i64, i64)) -> Vec<(i64, i64, Event)> {
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let mut ev = Vec::new();
    if dx != 0 {
        let (lo, hi) = (p.0.min(q.0), p.0.max(q.0));
        let mut k = lo.div_euclid(SCALE) + 1;
        while k * SCALE < hi {
            let x = k * SCALE;
            // y = p.y + (x - p.x) dy / dx = n / d
            let (mut n, mut d) = (p.1 * dx + (x - p.0) * dy, dx);
            if d < 0 {
                (n, d) = (-n, -d);
            }
            let (tn, td) = ((x - p.0) * dx.signum(), dx.abs());
            if n.rem_euclid(SCALE * d) == 0 {
                ev.push((tn, td, Event::Vertex(k, n / (SCALE * d))));
            } else {
                ev.push((tn, td, Event::V(k, n.div_euclid(SCALE * d))));
            }
            k += 1;
        }
    }
    if dy != 0 {
        let (lo, hi) = (p.1.min(q.1), p.1.max(q.1));
        let mut j = lo.div_euclid(SCALE) + 1;
        while j * SCALE < hi {
            let y = j * SCALE;
            let (mut n, mut d) = (p.0 * dy + (y - p.1) * dx, dy);
            if d < 0 {
                (n, d) = (-n, -d);
            }
            // Vertices were found on the vertical lines.
            if n.rem_euclid(SCALE * d) != 0 {
                let (tn, td) = ((y - p.1) * dy.signum(), dy.abs());
                ev.push((tn, td, Event::H(n.div_euclid(SCALE * d), j)));
            }
            j += 1;
        }
    }
    ev.sort_by(|a, b| {
        let l = i128::from(a.0) * i128::from(b.1);
        let r = i128::from(b.0) * i128::from(a.1);
        l.cmp(&r)
    });
    ev
}

impl SceneIndex<'_> {
    /// Walks the segment `p → q` (scaled units, never on a grid line).
    #[must_use]
    pub fn trace(&self, p: (i64, i64), q: (i64, i64)) -> Trace {
        self.walk(p, q, true)
    }

    /// Like [`Self::trace`] for cover: heavily obscured squares (dense
    /// canopy, darkness) hide a target but are not obstacles, so they do not
    /// block the line. SRD 5.1 cover comes from obstacles only; a
    /// sight-blocking prop still counts through its own square cover.
    #[must_use]
    pub fn trace_cover(&self, p: (i64, i64), q: (i64, i64)) -> Trace {
        self.walk(p, q, false)
    }

    fn walk(&self, p: (i64, i64), q: (i64, i64), obscurement_blocks: bool) -> Trace {
        let (dx, dy) = (q.0 - p.0, q.1 - p.1);
        let (sx, sy) = (dx.signum(), dy.signum());
        let end = (q.0.div_euclid(SCALE), q.1.div_euclid(SCALE));
        let mut cs = (p.0.div_euclid(SCALE), p.1.div_euclid(SCALE));
        let mut out = Trace::default();
        for (_, _, e) in crossings(p, q) {
            match e {
                Event::V(k, r) => {
                    let rule = self.edge(EdgeAxis::Vertical, k, r);
                    out.blocked |= rule.sight;
                    out.cover = out.cover.max(rule.cover);
                    cs.0 += sx;
                }
                Event::H(c, j) => {
                    let rule = self.edge(EdgeAxis::Horizontal, c, j);
                    out.blocked |= rule.sight;
                    out.cover = out.cover.max(rule.cover);
                    cs.1 += sy;
                }
                Event::Vertex(vx, vy) => {
                    out.blocked |= self.vertex_blocks(vx, vy, dx, dy);
                    cs = (cs.0 + sx, cs.1 + sy);
                }
            }
            if cs != end {
                out.blocked |= obscurement_blocks && self.opaque(cs.0, cs.1);
                out.cover = out.cover.max(self.cover(cs.0, cs.1));
            }
            if out.blocked {
                out.cover = CoverLevel::Total;
                return out;
            }
        }
        out
    }

    /// The corner rule for a line with direction `(dx, dy)` (both non-zero)
    /// passing exactly through vertex `(vx, vy)`.
    fn vertex_blocks(&self, vx: i64, vy: i64, dx: i64, dy: i64) -> bool {
        let mut sides = [false; 2];
        let mut mark = |side: i64| {
            if side > 0 {
                sides[0] = true;
            } else if side < 0 {
                sides[1] = true;
            }
        };
        // Arms N, S, E, W; the side is sign(cross(D, arm direction)).
        let arms = [
            (EdgeAxis::Vertical, vx, vy - 1, -dx),
            (EdgeAxis::Vertical, vx, vy, dx),
            (EdgeAxis::Horizontal, vx, vy, -dy),
            (EdgeAxis::Horizontal, vx - 1, vy, dy),
        ];
        for (axis, x, y, side) in arms {
            if self.edge(axis, x, y).sight {
                mark(side);
            }
        }
        // The two quadrant squares the line does not pass through.
        let (sx, sy) = (dx.signum(), dy.signum());
        for (qx, qy) in [(sx, -sy), (-sx, sy)] {
            let (x, y) = (vx + (qx - 1) / 2, vy + (qy - 1) / 2);
            if self.opaque(x, y) {
                mark(dx * qy - dy * qx);
            }
        }
        sides[0] && sides[1]
    }

    /// Line of sight between the centres of two squares.
    #[must_use]
    pub fn line_of_sight(&self, a: Sq, b: Sq) -> bool {
        let inside = |s: Sq| self.inside(i64::from(s.0), i64::from(s.1));
        if !inside(a) || !inside(b) {
            return false;
        }
        a == b || !self.trace(centre(a), centre(b)).blocked
    }

    /// Squares within `radius_ft` (grid distance under the diagonal rule)
    /// that `from` has line of sight to, row-major, `from` included.
    #[must_use]
    pub fn visible_squares(&self, from: Sq, radius_ft: u32) -> Vec<Sq> {
        let r = radius_ft / 5;
        let (w, h) = (self.scene.width, self.scene.height);
        if from.0 >= w || from.1 >= h {
            return Vec::new();
        }
        let (x0, x1) = (
            from.0.saturating_sub(r),
            from.0.saturating_add(r).min(w - 1),
        );
        let (y0, y1) = (
            from.1.saturating_sub(r),
            from.1.saturating_add(r).min(h - 1),
        );
        (y0..=y1)
            .flat_map(|y| (x0..=x1).map(move |x| Sq(x, y)))
            .filter(|&s| self.distance_ft(from, s) <= radius_ft && self.line_of_sight(from, s))
            .collect()
    }

    /// Cover of `target` against `attacker`. The degrees and "only the most
    /// protective degree applies" are SRD 5.1; how they are read off the
    /// grid is the DMG's optional grid procedure, applied as a house rule
    /// (SRD 5.1 gives none): the attacker picks the corner of its square
    /// that sees the target best; from that corner, lines run to the four
    /// corners of the target's square. Blocked lines set the degree (all
    /// four: total, three: three-quarters, one or two: half), and obstacles
    /// on the open lines (props, windows) add their own cover.
    #[must_use]
    pub fn cover_between(&self, attacker: Sq, target: Sq) -> CoverLevel {
        let inside = |s: Sq| self.inside(i64::from(s.0), i64::from(s.1));
        if !inside(attacker) || !inside(target) {
            return CoverLevel::Total;
        }
        if attacker == target {
            return CoverLevel::None;
        }
        let from = corners(attacker);
        let to = corners(target);
        from.iter()
            .map(|&p| {
                let traces: Vec<Trace> = to.iter().map(|&q| self.trace_cover(p, q)).collect();
                let blocked = traces.iter().filter(|t| t.blocked).count();
                let by_count = match blocked {
                    0 => CoverLevel::None,
                    1 | 2 => CoverLevel::Half,
                    3 => CoverLevel::ThreeQuarters,
                    _ => CoverLevel::Total,
                };
                traces
                    .iter()
                    .filter(|t| !t.blocked)
                    .map(|t| t.cover)
                    .fold(by_count, CoverLevel::max)
            })
            .min()
            .unwrap_or(CoverLevel::Total)
    }
}

fn centre(s: Sq) -> (i64, i64) {
    (
        i64::from(s.0) * SCALE + SCALE / 2,
        i64::from(s.1) * SCALE + SCALE / 2,
    )
}

fn corners(s: Sq) -> [(i64, i64); 4] {
    let (x, y) = (i64::from(s.0) * SCALE, i64::from(s.1) * SCALE);
    let (a, b) = (INSET, SCALE - INSET);
    [
        (x + a, y + a),
        (x + b, y + a),
        (x + b, y + b),
        (x + a, y + b),
    ]
}

impl crate::types::Scene {
    /// Line of sight between two squares (builds a [`SceneIndex`]; reuse one
    /// for many queries).
    #[must_use]
    pub fn line_of_sight(&self, a: Sq, b: Sq) -> bool {
        self.queries().line_of_sight(a, b)
    }

    /// Visible squares within `radius_ft`; see [`SceneIndex::visible_squares`].
    #[must_use]
    pub fn visible_squares(&self, from: Sq, radius_ft: u32) -> Vec<Sq> {
        self.queries().visible_squares(from, radius_ft)
    }

    /// SRD cover of `target` against `attacker`; see
    /// [`SceneIndex::cover_between`].
    #[must_use]
    pub fn cover_between(&self, attacker: Sq, target: Sq) -> CoverLevel {
        self.queries().cover_between(attacker, target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossings_come_in_order_and_find_vertices() {
        let ev = crossings((4, 4), (20, 20));
        assert_eq!(ev.len(), 2);
        assert!(matches!(ev[0].2, Event::Vertex(1, 1)));
        assert!(matches!(ev[1].2, Event::Vertex(2, 2)));
        let ev = crossings((4, 4), (20, 12));
        let kinds: Vec<_> = ev.iter().map(|e| format!("{:?}", e.2)).collect();
        assert_eq!(kinds, vec!["V(1, 0)", "H(1, 1)", "V(2, 1)"]);
    }
}
