//! Time-stepped tectonics, coupled with erosion (`logic/01` steps 2-3).
//!
//! Boundary type follows the two plates' crust types and relative motion
//! (`logic/01` §Q5). Uplift is not applied to the boundary line itself but
//! spread across an **orogenic belt** either side of it: real mountain belts
//! are 100-300 km wide, and raising a single 4 km cell and then diffusing it
//! produces a plateau rather than a range.

use super::plates::{plate_of_warped, CrustType, Plate, SimExtent};

/// Peak uplift per step at a continent-continent collision axis, millimetres.
///
/// Sized against the target: roughly 3 km of relief accumulated over the
/// run, allowing for what the coupled diffusion passes take back.
const COLLISION_PEAK_MM: i64 = 260_000;
/// Peak uplift per step at a subduction arc axis, millimetres.
const ARC_PEAK_MM: i64 = 150_000;
/// Peak subsidence per step at a rift axis, millimetres.
const RIFT_PEAK_MM: i64 = 70_000;

/// Belt half-width in 4 km cells. 40 cells is 160 km, the scale of the Alps
/// or the Southern Uplands; arcs and rifts are narrower.
const COLLISION_BELT: i32 = 9;
const ARC_BELT: i32 = 6;
const RIFT_BELT: i32 = 14;

/// Steps between coupled erosion passes. The artifact has rivers respond as
/// the ranges rise; doing it every step flattened them faster than tectonics
/// could build them.
const DIFFUSE_EVERY: u16 = 4;

/// What kind of boundary a cell sits on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Boundary {
    None,
    Collision,
    Arc,
    Rift,
}

/// Accumulated uplift in millimetres per 4 km cell.
#[must_use]
pub fn run_tectonics(seed: u64, plates: &[Plate], sim: SimExtent, steps: u16) -> Vec<i32> {
    let w = sim.width;
    let h = sim.height;
    let count = usize::try_from(w * h).unwrap_or(0);
    let mut uplift = vec![0i32; count];

    let idx = |x: i32, y: i32| usize::try_from(y * w + x).unwrap_or(0);

    for step in 0..steps {
        // Plates drift, so the boundaries migrate and the belts they raise
        // sweep across the domain. Computing ownership once outside the loop
        // — as this did originally — pins every step to the same boundary and
        // yields a single range with a dead-flat interior; a real continent is
        // a collage of orogens of different ages and orientations.
        //
        // Drift accumulates as `centre + drift * step`, which is the
        // kinematic-lite reading of `logic/01` §Q5.
        let moved: Vec<Plate> = plates
            .iter()
            .map(|p| Plate {
                centre_x: p.centre_x + p.drift_x * i32::from(step),
                centre_y: p.centre_y + p.drift_y * i32::from(step),
                ..*p
            })
            .collect();
        let owner: Vec<u8> = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| plate_of_warped(seed, &moved, x, y))
            .collect();
        let plates = &moved;

        // Classify boundaries at the plates' current positions. Drift is
        // re-rolled inside `seed_plates`, so the belts shift a little as the
        // run proceeds and the ranges gain structure rather than being one
        // straight ridge.
        let mut kind = vec![Boundary::None; count];
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = idx(x, y);
                let mine = owner[i];
                let Some(a) = plates.iter().find(|p| p.id == mine) else {
                    continue;
                };
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let theirs = owner[idx(x + dx, y + dy)];
                    if theirs == mine {
                        continue;
                    }
                    let Some(b) = plates.iter().find(|p| p.id == theirs) else {
                        continue;
                    };
                    // Relative motion on the boundary normal; negative closes.
                    let closing = (b.drift_x - a.drift_x) * dx + (b.drift_y - a.drift_y) * dy;
                    let k = match (a.crust, b.crust, closing) {
                        (CrustType::Continental, CrustType::Continental, c) if c < 0 => {
                            Boundary::Collision
                        }
                        (CrustType::Continental, CrustType::Oceanic, c)
                        | (CrustType::Oceanic, CrustType::Continental, c)
                            if c < 0 =>
                        {
                            Boundary::Arc
                        }
                        (_, _, c) if c > 0 => Boundary::Rift,
                        _ => Boundary::None,
                    };
                    if k != Boundary::None {
                        kind[i] = k;
                    }
                }
            }
        }

        // Distance to the nearest boundary of each kind, then a belt profile
        // around it.
        let d_col = distance_to(&kind, Boundary::Collision, w, h, COLLISION_BELT);
        let d_arc = distance_to(&kind, Boundary::Arc, w, h, ARC_BELT);
        let d_rift = distance_to(&kind, Boundary::Rift, w, h, RIFT_BELT);

        for i in 0..count {
            let mut dz = 0i64;
            dz += COLLISION_PEAK_MM * belt(d_col[i], COLLISION_BELT) / 1024;
            dz += ARC_PEAK_MM * belt(d_arc[i], ARC_BELT) / 1024;
            dz -= RIFT_PEAK_MM * belt(d_rift[i], RIFT_BELT) / 1024;
            uplift[i] = i32::try_from(i64::from(uplift[i]) + dz).unwrap_or(i32::MAX);
        }

        if step % DIFFUSE_EVERY == 0 {
            diffuse(&mut uplift, w, h);
        }
    }

    normalise(&mut uplift);
    uplift
}

/// Relief the highest ground reaches once the run settles, millimetres.
///
/// Orogenic amplitude has to be pinned to something, because belt coverage
/// varies with domain size: the same plate density produced Earth-like
/// hypsometry on a 250x500 km continent and a p50 of 1,082 m on a 500x1000 km
/// one. Real ranges are height-limited anyway — erosion strips them as fast
/// as convergence builds them, which is why Earth tops out near 8-9 km rather
/// than growing without bound.
const TARGET_RELIEF_MM: i64 = 2_600_000;

/// Deepest a rift basin may be driven, millimetres.
const RIFT_FLOOR_MM: i64 = 220_000;

/// Scales the uplift field so its 99.5th percentile hits [`TARGET_RELIEF_MM`],
/// making hypsometry independent of continent size.
fn normalise(uplift: &mut [i32]) {
    let mut positive: Vec<i32> = uplift.iter().copied().filter(|&v| v > 0).collect();
    if positive.is_empty() {
        return;
    }
    positive.sort_unstable();
    let hi = i64::from(positive[positive.len() * 995 / 1000]).max(1);
    for v in uplift.iter_mut() {
        let scaled = i64::from(*v) * TARGET_RELIEF_MM / hi;
        // Floor the subsidence. Scaling is set by the positive tail, so
        // without this a rift belt is amplified along with the ranges and
        // digs the continental core down by kilometres — it put the middle of
        // the micro continent 1,050 m under water. Real rift basins are a few
        // hundred metres deep.
        *v = i32::try_from(scaled.clamp(-RIFT_FLOOR_MM, i64::from(i32::MAX))).unwrap_or(i32::MAX);
    }
}

/// Belt profile: 1024 on the axis falling to 0 at `width`, as `t^3`.
///
/// Smoothstep was tried first and is wrong here — it is flat-topped, so the
/// whole belt rises to near-peak and the continent becomes a plateau with
/// a p90 elevation of 2,500 m against Earth's ~1,000 m. A cubic falloff is
/// peaked at the axis and concave outward, which puts most of the belt's
/// area in foothills and gives the strongly right-skewed hypsometry real
/// continents have.
fn belt(dist: i32, width: i32) -> i64 {
    if dist >= width {
        return 0;
    }
    let t = 1024 - i64::from(dist) * 1024 / i64::from(width.max(1));
    let t2 = t * t / 1024;
    (t2 * t / 1024).clamp(0, 1024)
}

/// Chebyshev distance to the nearest cell of `kind`, capped at `limit`.
///
/// Two-pass chamfer transform: deterministic, integer, and O(n).
fn distance_to(kind: &[Boundary], want: Boundary, w: i32, h: i32, limit: i32) -> Vec<i32> {
    let idx = |x: i32, y: i32| usize::try_from(y * w + x).unwrap_or(0);
    let cap = limit + 1;
    let mut d: Vec<i32> = kind
        .iter()
        .map(|&k| if k == want { 0 } else { cap })
        .collect();

    for y in 0..h {
        for x in 0..w {
            let i = idx(x, y);
            let mut best = d[i];
            for (dx, dy) in [(-1, 0), (0, -1), (-1, -1), (1, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                best = best.min(d[idx(nx, ny)].saturating_add(1));
            }
            d[i] = best.min(cap);
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = idx(x, y);
            let mut best = d[i];
            for (dx, dy) in [(1, 0), (0, 1), (1, 1), (-1, 1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                best = best.min(d[idx(nx, ny)].saturating_add(1));
            }
            d[i] = best.min(cap);
        }
    }
    d
}

/// One coarse erosion/isostasy pass: a fixed-weight five-point stencil.
fn diffuse(field: &mut [i32], w: i32, h: i32) {
    let source = field.to_vec();
    let stride = usize::try_from(w).unwrap_or(1);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = usize::try_from(y * w + x).unwrap_or(0);
            // Weight the centre heavily: an even stencil flattens the ranges
            // faster than the boundaries can raise them.
            let sum = i64::from(source[i]) * 12
                + i64::from(source[i - 1])
                + i64::from(source[i + 1])
                + i64::from(source[i - stride])
                + i64::from(source[i + stride]);
            field[i] = i32::try_from(sum / 16).unwrap_or(i32::MAX);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continent::plates::seed_plates;

    fn sim() -> SimExtent {
        SimExtent {
            width: 125,
            height: 250,
        }
    }

    #[test]
    fn tectonics_is_deterministic() {
        let p = seed_plates(42, sim(), 0);
        assert_eq!(
            run_tectonics(42, &p, sim(), 20),
            run_tectonics(42, &p, sim(), 20)
        );
    }

    #[test]
    fn belts_reach_mountain_scale() {
        // A continent whose highest ground is a few hundred metres has no
        // ranges. The artifact expects mountain streams at 9% slopes.
        let p = seed_plates(42, sim(), 0);
        let u = run_tectonics(42, &p, sim(), 20);
        let peak = u.iter().copied().max().unwrap_or(0);
        assert!(peak > 1_500_000, "highest uplift is only {peak} mm");
    }

    #[test]
    fn uplift_is_a_belt_not_a_line() {
        // Measure how wide the raised ground is: a one-cell ridge would mean
        // the belt profile is not doing its job.
        let p = seed_plates(42, sim(), 0);
        let u = run_tectonics(42, &p, sim(), 20);
        let peak = u.iter().copied().max().unwrap_or(1);
        let high = u.iter().filter(|&&v| v > peak / 2).count();
        assert!(
            high > u.len() / 100,
            "only {high} of {} cells are above half peak",
            u.len()
        );
    }

    #[test]
    fn belt_profile_falls_to_zero_at_the_edge() {
        assert_eq!(belt(0, 40), 1024);
        assert_eq!(belt(40, 40), 0);
        assert_eq!(belt(41, 40), 0);
        assert!(belt(20, 40) > 0 && belt(20, 40) < 1024);
    }
}
