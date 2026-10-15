//! Switchbacks: a piece steeper than its class allows gets a lateral wave
//! whose length brings the sustained grade under the limit (goal 37,
//! `mockup-artifact.md` "Roads": "roads climb hills obliquely").

use crate::curve::{min_radius, wave_len, Piece, Shape, Zig};
use crate::input::{ClassSpec, SQUARE_M};

/// Widest switchback, metres either side of the piece.
pub const MAX_AMP_M: f64 = 54.0;
/// Leg slopes tried (lateral offset per metre along the piece), gentlest
/// first: legs from 20° to 85° off the piece's line, 5° apart.
const SLOPES_DEG: [f64; 14] = [
    20.0, 25.0, 30.0, 35.0, 40.0, 45.0, 50.0, 55.0, 60.0, 65.0, 70.0, 75.0, 80.0, 85.0,
];
/// How much wider than [`min_radius_m`] the hairpins are laid, so the
/// taper and a bending piece cannot pull a turn under it.
const RADIUS_SLACK: f64 = 1.05;

/// Tightest bend a switchback may take, metres: a hairpin a cart can
/// swing round, at least 8 m and never narrower than the carriageway with
/// its verges.
#[must_use]
pub fn min_radius_m(spec: &ClassSpec) -> f64 {
    ((f64::from(spec.width_sq) + 2.0 * spec.verge_sq) * SQUARE_M).max(8.0)
}

/// The outcome of grading one piece.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Graded {
    /// The wave, if one was needed.
    pub zig: Option<Zig>,
    /// Whether even the widest wave leaves the grade over the limit.
    pub over_grade: bool,
}

/// How far over its class grade a piece may climb as it lies: a pitch a
/// little steeper than the sustained grade is taken straight rather than
/// sawn into shallow zigzags (review 2026-10-15, gallery 07).
pub const GRADE_SLACK: f64 = 1.3;

/// Chooses the switchback that keeps `|z1 - z0| / length` within the class
/// grade with the fewest legs (so legs sit as far apart as possible), then
/// the gentlest leg slope that suffices, its hairpins never turning
/// tighter than [`min_radius_m`] nor reaching past [`MAX_AMP_M`]. A piece
/// within [`GRADE_SLACK`] of its grade stays straight; one no such wave
/// can ease takes the longest that fits, and is over grade.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // bounded counts
pub fn grade(shape: &Shape, z0: f64, z1: f64, spec: &ClassSpec) -> Graded {
    let rise = (z1 - z0).abs();
    let plain = wave_len(shape, None, 64);
    if plain <= 0.0 || rise / plain <= spec.max_grade * GRADE_SLACK {
        return Graded {
            zig: None,
            over_grade: false,
        };
    }
    let need = rise / spec.max_grade;
    let radius = min_radius_m(spec);
    // Leg spacing floor: surface, verges and two shoulder squares.
    let spacing = (f64::from(spec.width_sq) + 2.0 * spec.verge_sq + 4.0) * SQUARE_M;
    let max_legs = ((plain / spacing).floor() as u32).clamp(1, 40);
    let mut best: Option<(f64, Zig)> = None;
    for legs in 1..=max_legs {
        for deg in SLOPES_DEG {
            let (sin, cos) = arda_ids::pmath::sin_cos(deg * std::f64::consts::PI / 180.0);
            let slope = sin / cos;
            let Some(zig) = Zig::new(plain, legs, slope, radius * RADIUS_SLACK) else {
                // Steeper legs need wider hairpins still.
                break;
            };
            if zig.amp > MAX_AMP_M {
                break;
            }
            let n = 64 * legs;
            let len = wave_len(shape, Some(zig), n);
            if len < need && best.is_some_and(|(l, _)| l >= len) {
                continue;
            }
            let m = ((len / (radius / 64.0)).ceil() as u32).clamp(64, 1 << 16);
            if min_radius(shape, Some(zig), m) < radius {
                continue;
            }
            if len >= need {
                return Graded {
                    zig: Some(zig),
                    over_grade: false,
                };
            }
            best = Some((len, zig));
        }
    }
    Graded {
        zig: best.map(|(_, z)| z),
        over_grade: true,
    }
}

/// Builds a graded piece.
#[must_use]
pub fn piece(shape: Shape, z0: f64, z1: f64, spec: &ClassSpec) -> (Piece, bool) {
    let g = grade(&shape, z0, z1, spec);
    (Piece::new(shape, g.zig, z0, z1), g.over_grade)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::RoadClass;

    #[test]
    fn gentle_pieces_stay_straight_and_steep_ones_wave() {
        let spec = RoadClass::Track.spec(128);
        let s = Shape::Line([0.0, 0.0], [0.0, 300.0]);
        assert!(grade(&s, 0.0, 15.0, &spec).zig.is_none());
        let g = grade(&s, 0.0, 60.0, &spec);
        let zig = g.zig.unwrap();
        assert!(!g.over_grade);
        let len = wave_len(&s, Some(zig), 64 * zig.legs);
        assert!(60.0 / len <= spec.max_grade + 1e-9);
    }

    /// Smallest turning radius along a sampled curve, metres.
    fn tightest(shape: &Shape, zig: Option<Zig>) -> f64 {
        let n = 4096;
        let pts: Vec<[f64; 2]> = (0..=n)
            .map(|k| crate::curve::wave_at(shape, zig, f64::from(k) / f64::from(n)))
            .collect();
        let mut best = f64::INFINITY;
        for w in pts.windows(3) {
            let u = [w[1][0] - w[0][0], w[1][1] - w[0][1]];
            let v = [w[2][0] - w[1][0], w[2][1] - w[1][1]];
            let (lu, lv) = (u[0].hypot(u[1]), v[0].hypot(v[1]));
            let c = ((u[0] * v[0] + u[1] * v[1]) / (lu * lv)).clamp(-1.0, 1.0);
            let a = c.acos();
            if a > 1e-12 {
                best = best.min(0.5 * (lu + lv) / a);
            }
        }
        best
    }

    /// The angle, degrees, between the waved curve's direction at an end
    /// and the plain shape's.
    fn end_kink(shape: &Shape, zig: Option<Zig>, t: f64) -> f64 {
        let e = 1e-4;
        let (t0, t1) = if t == 0.0 { (0.0, e) } else { (1.0 - e, 1.0) };
        let (a, b) = (
            crate::curve::wave_at(shape, zig, t0),
            crate::curve::wave_at(shape, zig, t1),
        );
        let d = [b[0] - a[0], b[1] - a[1]];
        let g = shape.tangent(t);
        let c = (d[0] * g[0] + d[1] * g[1]) / (d[0].hypot(d[1]) * g[0].hypot(g[1]));
        c.clamp(-1.0, 1.0).acos().to_degrees()
    }

    #[test]
    fn switchbacks_have_no_spikes_and_meet_their_neighbours_smoothly() {
        // review 2026-10-15 (gallery 07): sawtooth legs with needle-sharp
        // tips, and petals where a wave rode a tight corner.
        for class in [RoadClass::Track, RoadClass::Road, RoadClass::Highway] {
            let spec = class.spec(128);
            let shapes = [
                Shape::Line([0.0, 0.0], [0.0, 200.0]),
                Shape::Line([0.0, 0.0], [120.0, 50.0]),
                Shape::Quad([0.0, 0.0], [70.0, 0.0], [70.0, 70.0]),
                Shape::Quad([0.0, 0.0], [100.0, 10.0], [30.0, 60.0]),
            ];
            for shape in shapes {
                let plain = wave_len(&shape, None, 256);
                for ratio in [1.2, 1.6, 2.5, 4.0] {
                    let rise = plain * spec.max_grade * ratio;
                    let g = grade(&shape, 0.0, rise, &spec);
                    let Some(z) = g.zig else { continue };
                    let r = tightest(&shape, Some(z));
                    assert!(
                        r >= min_radius_m(&spec) * 0.97,
                        "{class:?} {shape:?} x{ratio}: turns at {r:.1} m"
                    );
                    for t in [0.0, 1.0] {
                        let k = end_kink(&shape, Some(z), t);
                        assert!(
                            k < 1.0,
                            "{class:?} {shape:?} x{ratio}: kinks {k:.1} deg at {t}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_modest_excess_climbs_straight() {
        // A pitch a little over the sustained grade is taken as it lies
        // rather than sawn into zigzags (gallery 07).
        let spec = RoadClass::Track.spec(128);
        let s = Shape::Line([0.0, 0.0], [0.0, 100.0]);
        let g = grade(&s, 0.0, 100.0 * spec.max_grade * 1.25, &spec);
        assert!(g.zig.is_none() && !g.over_grade);
    }
}
