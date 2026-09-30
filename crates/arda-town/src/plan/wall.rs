//! The town wall ring. On a hill it follows a contour below the crest (a
//! defensible line); on flat ground it is a slightly irregular oval around
//! the core, as most walled towns were.

use super::params::Params;
use crate::geom::{self, v2, Vec2};
use crate::rng::fbm1;
use crate::site::TerrainInput;

/// Number of rays sampled around the ring.
const RAYS: u32 = 56;

/// Wall thickness in metres (two squares).
pub const THICKNESS_M: f64 = 2.0 * super::grid::SQUARE_M;

/// The closed wall centreline around `centre`, enclosing every point in
/// `must_enclose` with `margin` metres to spare.
#[must_use]
pub fn ring(
    centre: Vec2,
    params: &Params,
    terrain: &TerrainInput,
    seed: u64,
    must_enclose: &[Vec2],
    margin: f64,
    axis: Vec2,
) -> Vec<Vec2> {
    let r = params.r_core;
    let dirs: Vec<Vec2> = (0..RAYS)
        .map(|k| {
            let (s, c) = geom::sin_cos(2.0 * geom::PI * f64::from(k) / f64::from(RAYS));
            v2(c, s)
        })
        .collect();
    let at_r: Vec<f64> = dirs
        .iter()
        .map(|&d| (terrain.height)(centre + d * r))
        .collect();
    let lo = at_r.iter().copied().fold(f64::MAX, f64::min);
    let hi = at_r.iter().copied().fold(f64::MIN, f64::max);
    let h0 = (terrain.height)(centre);
    let hilly = h0 - lo > 8.0;
    let contour = at_r.iter().sum::<f64>() / f64::from(RAYS);
    let mut radii: Vec<f64> = dirs
        .iter()
        .enumerate()
        .map(|(k, &d)| {
            let t = f64::from(u32::try_from(k).unwrap_or(0)) / f64::from(RAYS);
            // Towns stretch along their main road: an ellipse with ~1.35:1
            // axes, roughened with low-frequency noise.
            let along = d.dot(axis);
            let stretch = 0.87 + 0.26 * along * along;
            let base = r * stretch * (1.0 + 0.13 * fbm1(seed, t * 6.0));
            if !hilly || hi - lo < 1.0 {
                return base;
            }
            let mut rr = r * 0.6;
            while rr < r * 1.35 && (terrain.height)(centre + d * rr) > contour {
                rr += 3.0;
            }
            0.35 * base + 0.65 * rr
        })
        .collect();
    for (k, &d) in dirs.iter().enumerate() {
        for &p in must_enclose {
            let q = p - centre;
            let along = q.dot(d);
            let across = q.cross(d).abs();
            if along > 0.0 && across < r * 0.2 {
                radii[k] = radii[k].max(along + margin);
            }
        }
    }
    for _ in 0..3 {
        let n = radii.len();
        radii = (0..n)
            .map(|k| (radii[(k + n - 1) % n] + 2.0 * radii[k] + radii[(k + 1) % n]) * 0.25)
            .collect();
    }
    dirs.iter()
        .zip(&radii)
        .map(|(&d, &rr)| centre + d * rr)
        .collect()
}
