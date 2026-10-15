//! Ferry landings where a way meets open water too wide to rope across
//! (review 3.5-9).
//!
//! A ferry whose open water the terrain shows within
//! [`super::crossing::MAX_FERRY_SQ`] squares is laid whole, rope and both
//! landings ([`super::crossing`]). Longer crossings (a sea route to an
//! island) are never cut to a pad and never dropped: wherever a way's
//! centreline runs from land into open water ([`Terrain::open_water`]), a
//! landing stage is laid from that shore into the water, with a boat at
//! its end. Each shore is found from the way's own geometry and the
//! terrain, both pure functions of position, so every window that sees a
//! shore lays the same landing (goal 46), however far the other shore is.

use super::crossing::{rows_for, CrossingPlan};
use super::{square_of, WayPlan, Window};
use crate::input::{CrossingKind, Terrain};

/// How far from the window a shore is looked for, metres: past the
/// landing's own reach.
const SEARCH_M: f64 = 60.0;

/// A landing stage from a shore square into open water.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Landing {
    /// Index of the way.
    pub way: usize,
    /// Whether the stage runs east–west (else north–south).
    pub east_west: bool,
    /// Along-axis index of the last land square.
    pub shore: i64,
    /// `+1` when the water lies at higher along-axis indices, else `-1`.
    pub towards: i64,
    /// Across-axis rows, inclusive.
    pub rows: (i64, i64),
}

impl Landing {
    /// Global `(gx, gy)` of along index `a` and across index `r`.
    #[must_use]
    pub fn square(&self, a: i64, r: i64) -> (i64, i64) {
        if self.east_west {
            (a, r)
        } else {
            (r, a)
        }
    }
}

/// Whether global square `(gx, gy)` lies in a laid crossing's structure.
fn in_crossing(crossings: &[CrossingPlan], gx: i64, gy: i64) -> bool {
    crossings.iter().any(|c| {
        let (a, r) = if c.east_west { (gx, gy) } else { (gy, gx) };
        let margin = super::crossing::LANDING_SQ + 1;
        (c.span.0 - margin..=c.span.1 + margin).contains(&a)
            && (c.rows.0 - 1..=c.rows.1 + 1).contains(&r)
    })
}

/// Every landing at a shore of open water within [`SEARCH_M`] of the
/// window, by way and arc length. None when the terrain cannot say where
/// open water is.
#[must_use]
pub fn plan_all(
    win: Window,
    ways: &[WayPlan],
    crossings: &[CrossingPlan],
    terrain: &dyn Terrain,
) -> Vec<Landing> {
    let mut out = Vec::new();
    for (wi, w) in ways.iter().enumerate() {
        for run in &w.dense.runs {
            let wet = |p: [f64; 2]| terrain.open_water(square_of(p[0]), square_of(p[1]));
            for pair in run.windows(2) {
                let (a, b) = (&pair[0], &pair[1]);
                if win.dist_m(a.p) > SEARCH_M && win.dist_m(b.p) > SEARCH_M {
                    continue;
                }
                let (Some(wa), Some(wb)) = (wet(a.p), wet(b.p)) else {
                    return Vec::new();
                };
                if wa == wb {
                    continue;
                }
                let (land, water) = if wa { (b.p, a.p) } else { (a.p, b.p) };
                let d = [water[0] - land[0], water[1] - land[1]];
                let east_west = d[0].abs() >= d[1].abs();
                let (shore, towards) = if east_west {
                    (square_of(land[0]), if d[0] >= 0.0 { 1 } else { -1 })
                } else {
                    (square_of(land[1]), if d[1] >= 0.0 { 1 } else { -1 })
                };
                let (gx, gy) = (square_of(land[0]), square_of(land[1]));
                if in_crossing(crossings, gx, gy) {
                    continue;
                }
                let rows = rows_for(CrossingKind::Ferry, w, east_west, land);
                let l = Landing {
                    way: wi,
                    east_west,
                    shore,
                    towards,
                    rows,
                };
                if !out.contains(&l) {
                    out.push(l);
                }
            }
        }
    }
    out
}
