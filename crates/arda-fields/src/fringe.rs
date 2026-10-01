//! The woodland fringe: unclaimed ground a few squares out from farmed
//! land, lanes, roads and settlements, where the forest thins to scrub and
//! rough grass before the fields begin. Its width follows a noise field, so
//! the forest edge is organic rather than the shape of the field beyond.
//!
//! The fringe is a mask over the window: callers that know the natural
//! cover beneath decide where it applies (it only makes sense against
//! forest). Distances are measured on the plan, which extends
//! [`crate::plan::MARGIN`] squares past the window, so every window square
//! sees all the ground within the fringe's reach and windows agree.

use crate::dress::patch;
use crate::fields::FieldKind;
use crate::geom::Sq;
use crate::plan::{Cover, Plan};

/// Narrowest fringe, squares.
pub const MIN_WIDTH: f64 = 1.5;
/// Widest fringe, squares.
pub const MAX_WIDTH: f64 = 6.0;

/// Whether a square is worked ground the forest has been cleared back from.
fn worked(plan: &Plan, s: Sq) -> bool {
    match plan.at(s) {
        Cover::Field(_) => !matches!(
            plan.field_at(s).map(|(_, f)| f.kind),
            Some(FieldKind::Woodland | FieldKind::Wild)
        ),
        Cover::Compound(_) | Cover::Lane | Cover::Road(_) | Cover::Built | Cover::Apron => true,
        Cover::Wild | Cover::Rough | Cover::Water => false,
    }
}

/// The fringe's width at a square, squares.
fn width(seed: u64, s: Sq) -> f64 {
    MIN_WIDTH + (MAX_WIDTH - MIN_WIDTH) * patch(seed, 0xF41E, s, 0.07).clamp(0.0, 1.0)
}

/// Per window square, row-major: the depth into the fringe as a fraction
/// of its width (`0` beside worked ground, towards `1` at the forest), or
/// `None` outside it.
#[must_use]
pub fn fringe(plan: &Plan) -> Vec<Option<f32>> {
    let (x0, y0, w, h) = plan.win;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // a small constant
    let reach = MAX_WIDTH.ceil() as i64;
    let mut out = Vec::with_capacity(usize::try_from(w * h).unwrap_or(0));
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            let s = Sq::new(x, y);
            let unused = plan
                .field_at(s)
                .is_some_and(|(_, f)| f.kind == FieldKind::Wild);
            if !unused && !matches!(plan.at(s), Cover::Wild | Cover::Rough) {
                out.push(None);
                continue;
            }
            let mut best = f64::INFINITY;
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    #[allow(clippy::cast_precision_loss)] // tiny offsets
                    let d = ((dx * dx + dy * dy) as f64).sqrt();
                    if d < best && worked(plan, s.offset(dx, dy)) {
                        best = d;
                    }
                }
            }
            let wd = width(plan.seed, s);
            #[allow(clippy::cast_possible_truncation)] // a fraction
            out.push((best <= wd).then(|| (best / wd) as f32));
        }
    }
    out
}
