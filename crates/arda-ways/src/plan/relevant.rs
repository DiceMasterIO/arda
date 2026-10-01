//! Which roads a window must plan (review round 2 #36).

use super::{raw_dist, switchback, to_p, Window, DENSE_MARGIN_M};
use crate::curve::P;
use crate::input::Road;

/// How far from a kept road's endpoint another road can still move it:
/// a snap needs the endpoint within 1 m of that road's polyline, whose own
/// endpoints may have moved up to 40 m (`curve_nearest` reach) by their
/// snaps; with room to spare.
pub const SNAP_REACH_M: f64 = 64.0;

/// Distance from the window within which a road's vertices make it
/// relevant: the densify margin, the widest switchback and a snap.
pub const RELEVANT_MARGIN_M: f64 = DENSE_MARGIN_M + switchback::MAX_AMP_M + SNAP_REACH_M;

/// The roads that can shape `win` (review round 2 #36: every window
/// planned every road in the input, so its cost grew with the world's
/// network). A road is kept when its vertices come within
/// [`RELEVANT_MARGIN_M`] of the window (only pieces within
/// [`DENSE_MARGIN_M`] are densified, so only they draw, take crossings or
/// carry junction signposts; a piece reaches beyond its vertices by its
/// switchback amplitude and a snapped endpoint), and then,
/// to a fixed point, every road passing within [`SNAP_REACH_M`] of a kept
/// road's endpoint (it may snap that endpoint, which moves the whole kept
/// curve and its arc lengths). Every other road leaves the window's
/// squares, walls and props unchanged, so planning only these is
/// byte-identical.
#[must_use]
pub fn relevant_roads(win: Window, roads: &[Road]) -> Vec<bool> {
    let verts: Vec<Vec<Vec<P>>> = roads
        .iter()
        .map(|r| {
            r.segments
                .iter()
                .map(|s| s.iter().copied().map(to_p).collect())
                .collect()
        })
        .collect();
    let bounds = |segs: &[Vec<P>]| {
        let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
        for p in segs.iter().flatten() {
            b = [
                b[0].min(p[0]),
                b[1].min(p[1]),
                b[2].max(p[0]),
                b[3].max(p[1]),
            ];
        }
        b
    };
    let mut keep: Vec<bool> = verts
        .iter()
        .map(|segs| {
            segs.iter().any(|s| !s.is_empty()) && win.near_bounds(bounds(segs), RELEVANT_MARGIN_M)
        })
        .collect();
    let mut todo: Vec<usize> = (0..roads.len()).filter(|&i| keep[i]).collect();
    while let Some(i) = todo.pop() {
        let ends: Vec<P> = verts[i]
            .iter()
            .filter(|s| s.len() >= 2)
            .flat_map(|s| [s[0], s[s.len() - 1]])
            .collect();
        for (j, segs) in verts.iter().enumerate() {
            if keep[j] {
                continue;
            }
            let near = ends.iter().any(|&e| {
                segs.iter()
                    .filter(|s| s.len() >= 2)
                    .any(|s| raw_dist(s, e).0 <= SNAP_REACH_M)
            });
            if near {
                keep[j] = true;
                todo.push(j);
            }
        }
    }
    keep
}

#[cfg(test)]
#[path = "relevant_tests.rs"]
mod tests;
