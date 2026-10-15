//! The switchback wave (goal 37, `mockup-artifact.md` "Roads": "roads
//! climb hills obliquely"): straight legs across the slope joined by
//! hairpins of a fixed radius, laid square to a piece's chord.
//!
//! A leg climbs at a constant lateral slope; at each end it turns through
//! a circular arc into the next leg, so the tightest bend is the arc's
//! radius and never a needle-sharp tip (review 2026-10-15, gallery 07: a
//! rounded triangle wave turned in hairpins of a few decimetres and read
//! as a sawtooth). The first and last legs are half legs that leave and
//! rejoin the piece's line through an arc of the same radius, so a waved
//! piece meets its neighbours along their own direction.

/// A switchback wave across a piece.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zig {
    /// Lateral amplitude (the line to the outside of a hairpin), metres.
    pub amp: f64,
    /// Number of legs, the half legs at the ends included (at least 2).
    pub legs: u32,
    /// Lateral slope of a leg (offset per metre along the piece).
    pub slope: f64,
    /// Hairpin radius, metres.
    pub radius: f64,
    /// Length along the piece the legs share, metres (its plain length).
    pub span: f64,
}

/// The fixed geometry of a wave: arc half-width, arc rise, full and half
/// leg lengths along the piece.
#[derive(Debug, Clone, Copy)]
struct Legs {
    xa: f64,
    rise: f64,
    full: f64,
    half: f64,
}

impl Zig {
    /// The wave of `legs` legs at lateral `slope` turning on `radius` over
    /// a piece `span` metres long; `None` for fewer than two legs or where
    /// the hairpins do not fit between the legs.
    #[must_use]
    pub fn new(span: f64, legs: u32, slope: f64, radius: f64) -> Option<Self> {
        if legs < 2 || span <= 0.0 || slope <= 0.0 || radius <= 0.0 {
            return None;
        }
        let g = geometry(span, legs, slope, radius)?;
        // A full leg runs hairpin to hairpin: 2A = m (h − 2xa) + 2 rise.
        let amp = (slope * (g.full - 2.0 * g.xa) + 2.0 * g.rise) / 2.0;
        Some(Self {
            amp,
            legs,
            slope,
            radius,
            span,
        })
    }

    /// Lateral offset at parameter `t` in `0..=1` along the piece, metres.
    #[must_use]
    pub fn offset(&self, t: f64) -> f64 {
        let Some(g) = geometry(self.span, self.legs, self.slope, self.radius) else {
            return 0.0;
        };
        let x = (t * self.span).clamp(0.0, self.span);
        if x <= g.half {
            return self.half_leg(x, g);
        }
        // After the first half leg, the full legs alternate from +A.
        let inner = f64::from(self.legs - 2) * g.full;
        if x >= g.half + inner {
            let sign = if self.legs.is_multiple_of(2) { 1.0 } else { -1.0 };
            return sign * self.half_leg(self.span - x, g);
        }
        let u = x - g.half;
        let j = (u / g.full).floor();
        let q = u - j * g.full;
        #[allow(clippy::cast_possible_truncation)] // leg counts are small
        let even = (j as i64).rem_euclid(2) == 0;
        let f = self.full_leg(q, g);
        if even {
            -f
        } else {
            f
        }
    }

    /// A full leg, `q` in `0..=full`: from `-amp` (flat, a hairpin's
    /// outside) to `+amp`.
    fn full_leg(&self, q: f64, g: Legs) -> f64 {
        let (r, a) = (self.radius, self.amp);
        if q < g.xa {
            -a + r - (r * r - q * q).max(0.0).sqrt()
        } else if q > g.full - g.xa {
            let u = g.full - q;
            a - r + (r * r - u * u).max(0.0).sqrt()
        } else {
            self.slope * (q - g.full / 2.0)
        }
    }

    /// A half leg, `u` in `0..=half`: from the piece's line (flat) to
    /// `+amp` (flat).
    fn half_leg(&self, u: f64, g: Legs) -> f64 {
        let (r, a) = (self.radius, self.amp);
        if u < g.xa {
            r - (r * r - u * u).max(0.0).sqrt()
        } else if u > g.half - g.xa {
            let v = g.half - u;
            a - r + (r * r - v * v).max(0.0).sqrt()
        } else {
            g.rise + self.slope * (u - g.xa)
        }
    }
}

/// The wave's geometry, or `None` where an arc would not fit its leg.
fn geometry(span: f64, legs: u32, m: f64, r: f64) -> Option<Legs> {
    let c = 1.0 / arda_ids::pmath::hypot(1.0, m);
    let xa = r * m * c;
    let rise = r * (1.0 - c);
    // span = 2·half + (n − 2)·full with half = full/2 + xa − rise/m.
    let full = (span - 2.0 * xa + 2.0 * rise / m) / f64::from(legs - 1);
    let half = full / 2.0 + xa - rise / m;
    (full >= 2.0 * xa && half >= 2.0 * xa).then_some(Legs {
        xa,
        rise,
        full,
        half,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legs_join_their_hairpins_without_a_step() {
        let z = Zig::new(200.0, 6, 1.5, 7.0).unwrap();
        let mut prev = z.offset(0.0);
        let n = 20_000;
        for k in 1..=n {
            let o = z.offset(f64::from(k) / f64::from(n));
            assert!((o - prev).abs() < 0.03, "step at {k}: {prev} -> {o}");
            prev = o;
        }
        assert!(z.offset(0.0).abs() < 1e-9 && z.offset(1.0).abs() < 1e-9);
        let peak = (0..=n)
            .map(|k| z.offset(f64::from(k) / f64::from(n)).abs())
            .fold(0.0, f64::max);
        assert!((peak - z.amp).abs() < 1e-3);
    }

    #[test]
    fn hairpins_that_cannot_fit_refuse() {
        assert!(Zig::new(200.0, 40, 3.0, 6.0).is_none());
        assert!(Zig::new(200.0, 1, 1.0, 6.0).is_none());
    }
}
