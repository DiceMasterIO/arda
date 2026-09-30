//! Switchbacks: a piece steeper than its class allows gets a lateral wave
//! whose length brings the sustained grade under the limit (goal 37,
//! `mockup-artifact.md` "Roads": "roads climb hills obliquely").

use crate::curve::{wave_len, Piece, Shape, Zig};
use crate::input::{ClassSpec, SQUARE_M};

/// Widest switchback, metres either side of the piece.
pub const MAX_AMP_M: f64 = 36.0;
/// Amplitude search step, metres.
const AMP_STEP_M: f64 = 2.0;

/// The outcome of grading one piece.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Graded {
    /// The wave, if one was needed.
    pub zig: Option<Zig>,
    /// Whether even the widest wave leaves the grade over the limit.
    pub over_grade: bool,
}

/// Chooses the switchback that keeps `|z1 - z0| / length` within the class
/// grade with the fewest legs (so legs sit as far apart as possible), then
/// the narrowest amplitude that suffices for that leg count.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // bounded counts
pub fn grade(shape: &Shape, z0: f64, z1: f64, spec: &ClassSpec) -> Graded {
    let rise = (z1 - z0).abs();
    let plain = wave_len(shape, None, 64);
    if plain <= 0.0 || rise / plain <= spec.max_grade {
        return Graded {
            zig: None,
            over_grade: false,
        };
    }
    let need = rise / spec.max_grade;
    // Leg spacing floor: surface, verges and two shoulder squares.
    let spacing = (f64::from(spec.width_sq) + 2.0 * spec.verge_sq + 4.0) * SQUARE_M;
    let max_legs = ((plain / spacing).floor() as u32).clamp(1, 40);
    let steps = (MAX_AMP_M / AMP_STEP_M) as u32;
    for legs in 1..=max_legs {
        for k in 1..=steps {
            let zig = Zig {
                amp: f64::from(k) * AMP_STEP_M,
                legs,
            };
            if wave_len(shape, Some(zig), 64 * legs) >= need {
                return Graded {
                    zig: Some(zig),
                    over_grade: false,
                };
            }
        }
    }
    Graded {
        zig: Some(Zig {
            amp: MAX_AMP_M,
            legs: max_legs,
        }),
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
        let s = Shape::Line([0.0, 0.0], [0.0, 100.0]);
        assert!(grade(&s, 0.0, 5.0, &spec).zig.is_none());
        let g = grade(&s, 0.0, 30.0, &spec);
        let zig = g.zig.unwrap();
        assert!(!g.over_grade);
        let len = wave_len(&s, Some(zig), 64 * zig.legs);
        assert!(30.0 / len <= spec.max_grade + 1e-9);
    }
}
