//! Time-stepped tectonics, coupled with erosion (`logic/01` steps 2-3).
//!
//! Boundary type follows the two plates' crust types and relative motion
//! (`logic/01` §Q5). Uplift is not applied to the boundary line itself but
//! spread across an **orogenic belt** either side of it: real mountain belts
//! are 100-300 km wide, and raising a single 4 km cell and then diffusing it
//! produces a plateau rather than a range.

use super::plates::{plate_of_warped, plate_of_warped_round, CrustType, Plate, SimExtent};
use crate::noise::value_noise;

/// Peak uplift per step at a continent-continent collision axis, millimetres.
///
/// Sized against the target: roughly 3 km of relief accumulated over the
/// run, allowing for what the coupled diffusion passes take back.
const COLLISION_PEAK_MM: i64 = 260_000;
/// Peak uplift per step at a subduction arc axis, millimetres.
const ARC_PEAK_MM: i64 = 150_000;
/// Peak subsidence per step at a rift axis, millimetres.
const RIFT_PEAK_MM: i64 = 70_000;

/// Belt half-width in 4 km cells; arcs are narrower than collision ranges.
const COLLISION_BELT: i32 = 9;
const ARC_BELT: i32 = 6;
const RIFT_BELT: i32 = 14;

/// Steps between coupled erosion passes. The artifact has rivers respond as
/// the ranges rise; doing it every step flattened them faster than tectonics
/// could build them.
const DIFFUSE_EVERY: u16 = 4;

/// What kind of boundary a cell sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Boundary {
    None = 0,
    Collision = 1,
    Arc = 2,
    Rift = 4,
}

/// A junction can source several belts; none may replace another.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct BoundaryMask(u8);

impl BoundaryMask {
    fn contains(self, kind: Boundary) -> bool {
        self.0 & kind as u8 != 0
    }
}

/// `logic/01` §Authorized boundary-classification correction: use the moved
/// plate-center normal and retain all incident kinds independently of order.
fn boundary_kinds<'a>(a: &Plate, neighbours: impl IntoIterator<Item = &'a Plate>) -> BoundaryMask {
    let mut result = BoundaryMask::default();
    for b in neighbours {
        if a.id == b.id {
            continue;
        }
        // Center separation is normal to the underlying unwarped Voronoi
        // interface. It approximates the existing warped interface; using a
        // cardinal raster face instead can turn tangential motion into both
        // collision and rift along one staircase. Only the sign is needed,
        // so no normalization or rounding can turn exact tangency into uplift.
        let nx = i128::from(b.centre_x) - i128::from(a.centre_x);
        let ny = i128::from(b.centre_y) - i128::from(a.centre_y);
        let vx = i128::from(b.drift_x) - i128::from(a.drift_x);
        let vy = i128::from(b.drift_y) - i128::from(a.drift_y);
        let closing = vx * nx + vy * ny;
        let kind = match (a.crust, b.crust, closing) {
            (CrustType::Continental, CrustType::Continental, c) if c < 0 => Boundary::Collision,
            (CrustType::Continental, CrustType::Oceanic, c)
            | (CrustType::Oceanic, CrustType::Continental, c)
                if c < 0 =>
            {
                Boundary::Arc
            }
            (_, _, c) if c > 0 => Boundary::Rift,
            _ => Boundary::None,
        };
        result.0 |= kind as u8;
    }
    result
}

/// Accumulated uplift in millimetres per 4 km cell.
#[must_use]
pub fn run_tectonics(seed: u64, plates: &[Plate], sim: SimExtent, steps: u16) -> Vec<i32> {
    run_tectonics_with_profile(seed, plates, sim, steps, false, false)
}

/// Fine-terrain macro uplift. The legacy profile remains byte-identical;
/// only the opt-in canonical source uses segmented mountain crests and a
/// continuous rift floor.
#[must_use]
pub fn run_tectonics_fine(seed: u64, plates: &[Plate], sim: SimExtent, steps: u16) -> Vec<i32> {
    run_tectonics_with_profile(seed, plates, sim, steps, true, false)
}

/// Recipe-5 macro uplift: the fine profile over curved plate margins
/// ([`plate_of_warped_round`]).
#[must_use]
pub fn run_tectonics_formed(seed: u64, plates: &[Plate], sim: SimExtent, steps: u16) -> Vec<i32> {
    run_tectonics_with_profile(seed, plates, sim, steps, true, true)
}

fn run_tectonics_with_profile(
    seed: u64,
    plates: &[Plate],
    sim: SimExtent,
    steps: u16,
    fine: bool,
    round: bool,
) -> Vec<i32> {
    let w = sim.width;
    let h = sim.height;
    let count = usize::try_from(w * h).unwrap_or(0);
    let mut uplift = vec![0i32; count];
    // Calibrate the fine variant against the unchanged tectonic accumulation.
    // Normalizing its new peaks against themselves would lower most mountain
    // slopes and make the ranges feel flatter despite adding crest structure.
    let mut calibration = if fine { vec![0i32; count] } else { Vec::new() };
    // Fixed in world coordinates across tectonic steps: a migrating belt
    // sweeps over the same crest/saddle landscape instead of re-rolling its
    // peaks each step. The two smooth scales are 64 and 24 km on this 4 km
    // lattice. They modulate only positive orogenic forcing, never the coast.
    let crest_weight: Vec<i64> = if fine {
        (0..h)
            .flat_map(|y| (0..w).map(move |x| crest_weight_q10(seed, x, y)))
            .collect()
    } else {
        Vec::new()
    };

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
            .map(|p| {
                if round {
                    // logic/02 §fine-formation continent: continuous drift
                    // direction (same speed as the shared integer drift), in
                    // 1/16-cell units. Integer axis/diagonal drift swept belts
                    // into parallelograms with axis-aligned sides.
                    let (vx, vy) = round_drift_q4(seed, p);
                    Plate {
                        centre_x: p.centre_x + (vx * i32::from(step) + 8).div_euclid(16),
                        centre_y: p.centre_y + (vy * i32::from(step) + 8).div_euclid(16),
                        // Classification needs only the sign of a dot product,
                        // so scaled drift classifies identically in kind.
                        drift_x: vx,
                        drift_y: vy,
                        ..*p
                    }
                } else {
                    Plate {
                        centre_x: p.centre_x + p.drift_x * i32::from(step),
                        centre_y: p.centre_y + p.drift_y * i32::from(step),
                        ..*p
                    }
                }
            })
            .collect();
        let owner: Vec<u8> = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| {
                if round {
                    plate_of_warped_round(seed, &moved, x, y)
                } else {
                    plate_of_warped(seed, &moved, x, y)
                }
            })
            .collect();
        let plates = &moved;

        // Classify boundaries at the plates' current positions. Drift is
        // re-rolled inside `seed_plates`, so the belts shift a little as the
        // run proceeds and the ranges gain structure rather than being one
        // straight ridge.
        let mut kind = vec![BoundaryMask::default(); count];
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = idx(x, y);
                let mine = owner[i];
                let Some(a) = plates.iter().find(|p| p.id == mine) else {
                    continue;
                };
                kind[i] = boundary_kinds(
                    a,
                    [(1, 0), (-1, 0), (0, 1), (0, -1)]
                        .into_iter()
                        .filter_map(|(dx, dy)| {
                            let theirs = owner[idx(x + dx, y + dy)];
                            if theirs == mine {
                                return None;
                            }
                            plates.iter().find(|p| p.id == theirs)
                        }),
                );
            }
        }

        // Distance to the nearest boundary of each kind, then a belt profile
        // around it.
        let d_col = distance_to(&kind, Boundary::Collision, w, h, COLLISION_BELT);
        let d_arc = distance_to(&kind, Boundary::Arc, w, h, ARC_BELT);
        let d_rift = distance_to(&kind, Boundary::Rift, w, h, RIFT_BELT);

        for i in 0..count {
            let mut dz = 0i64;
            let collision = COLLISION_PEAK_MM * belt(d_col[i], COLLISION_BELT) / 1024;
            let arc = ARC_PEAK_MM * belt(d_arc[i], ARC_BELT) / 1024;
            let rift = RIFT_PEAK_MM * belt(d_rift[i], RIFT_BELT) / 1024;
            if fine {
                dz += (collision + arc) * crest_weight[i] / 1024 - rift;
                calibration[i] = i32::try_from(i64::from(calibration[i]) + collision + arc - rift)
                    .unwrap_or(i32::MAX);
            } else {
                dz += collision + arc - rift;
            }
            uplift[i] = i32::try_from(i64::from(uplift[i]) + dz).unwrap_or(i32::MAX);
        }

        if step % DIFFUSE_EVERY == 0 {
            diffuse(&mut uplift, w, h);
            if fine {
                diffuse(&mut calibration, w, h);
            }
        }
    }

    if fine {
        normalise_fine(&mut uplift, &calibration);
    } else {
        normalise(&mut uplift);
    }
    uplift
}

/// Recipe-5 plate drift in 1/16 cell per step: the shared drift's speed in
/// a hash-chosen direction (64 headings, Q12 cosine table built from a
/// quarter-wave of integer values).
pub(super) fn round_drift_q4(seed: u64, p: &Plate) -> (i32, i32) {
    const COS_Q12: [i64; 17] = [
        4096, 4076, 4017, 3920, 3784, 3612, 3406, 3166, 2896, 2598, 2276, 1931, 1567, 1189, 799,
        401, 0,
    ];
    let speed_sq = i64::from(p.drift_x * p.drift_x + p.drift_y * p.drift_y) * 256;
    let speed = i64::try_from(speed_sq.unsigned_abs().isqrt()).unwrap_or(0);
    let k = (crate::noise::hash_2d(seed ^ 0xD21F, i32::from(p.id), 3) % 64) as usize;
    let quarter = |k: usize| -> i64 {
        // cos(k * 2pi / 64) for k in 0..64
        match k / 16 {
            0 => COS_Q12[k],
            1 => -COS_Q12[32 - k],
            2 => -COS_Q12[k - 32],
            _ => COS_Q12[64 - k],
        }
    };
    let c = quarter(k);
    let sn = quarter((k + 48) % 64);
    (
        i32::try_from(speed * c / 4096).unwrap_or(0),
        i32::try_from(speed * sn / 4096).unwrap_or(0),
    )
}

/// Bounded, smooth peak/saddle hierarchy along an orogenic axis. The 4 km
/// cells retain the tectonic belt shape; this varies its crest strength over
/// tens of kilometres so a long collision is not one wall of equal height.
fn crest_weight_q10(seed: u64, x: i32, y: i32) -> i64 {
    let broad = i64::from(value_noise(seed ^ 0xC2E5_7A11, x, y, 16));
    let secondary = i64::from(value_noise(seed ^ 0x59D1_83B4, x, y, 6));
    (1024 + broad * 640 / 32_768 + secondary * 160 / 32_768).clamp(224, 1824)
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

/// The fine profile keeps the same positive-height calibration as the
/// legacy path, while mapping negative rift uplift monotonically toward the
/// old -220 m limit. A hard clamp erased all differences between deep rift
/// cells, making a broad exact-height floor below the continental base.
fn normalise_fine(uplift: &mut [i32], calibration: &[i32]) {
    let mut positive: Vec<i32> = calibration.iter().copied().filter(|&v| v > 0).collect();
    positive.sort_unstable();
    let hi = positive
        .get(positive.len().saturating_mul(995) / 1000)
        .map(|&v| i64::from(v).max(1));
    const RIFT_SOFTNESS_MM: i64 = 550_000;
    for v in uplift.iter_mut() {
        let scaled = hi.map_or(i64::from(*v), |hi| i64::from(*v) * TARGET_RELIEF_MM / hi);
        let shaped = if scaled < 0 {
            let depth = -scaled;
            let softened = i128::from(RIFT_FLOOR_MM) * i128::from(depth)
                / i128::from(depth + RIFT_SOFTNESS_MM);
            -i64::try_from(softened).unwrap_or(RIFT_FLOOR_MM)
        } else {
            scaled
        };
        *v = i32::try_from(shaped.clamp(i64::from(i32::MIN), i64::from(i32::MAX)))
            .unwrap_or(i32::MAX);
    }
}

/// Belt profile: 1024 on the axis falling to 0 at `width` cells, as `t^3`.
///
/// Smoothstep was tried first and is wrong here — it is flat-topped, so the
/// whole belt rises to near-peak and the continent becomes a plateau with
/// a p90 elevation of 2,500 m against Earth's ~1,000 m. A cubic falloff is
/// peaked at the axis and concave outward, which puts most of the belt's
/// area in foothills and gives the strongly right-skewed hypsometry real
/// continents have.
fn belt(dist_q10: i32, width: i32) -> i64 {
    if i64::from(dist_q10) >= i64::from(width) * 1024 {
        return 0;
    }
    let t = 1024 - i64::from(dist_q10) / i64::from(width.max(1));
    let t2 = t * t / 1024;
    (t2 * t / 1024).clamp(0, 1024)
}

/// Q10 Euclidean distance to the nearest cell of `kind`, capped past `limit`.
///
/// Two separable squared-distance passes keep work linear in grid area.
fn distance_to(kind: &[BoundaryMask], want: Boundary, w: i32, h: i32, limit: i32) -> Vec<i32> {
    let width = usize::try_from(w).unwrap_or(0);
    let height = usize::try_from(h).unwrap_or(0);
    let cap_cells = i64::from(limit) + 1;
    let cap2 = cap_cells * cap_cells;
    let cap_q10 = i32::try_from(cap_cells * 1024).unwrap_or(i32::MAX);
    let mut horizontal = vec![cap2; kind.len()];

    for y in 0..height {
        let row = y * width;
        let mut nearest = None;
        for x in 0..width {
            if kind[row + x].contains(want) {
                nearest = Some(x);
            }
            if let Some(source) = nearest {
                let dx = i64::try_from(x - source).unwrap_or(i64::MAX);
                horizontal[row + x] = (dx * dx).min(cap2);
            }
        }
        nearest = None;
        for x in (0..width).rev() {
            if kind[row + x].contains(want) {
                nearest = Some(x);
            }
            if let Some(source) = nearest {
                let dx = i64::try_from(source - x).unwrap_or(i64::MAX);
                horizontal[row + x] = horizontal[row + x].min(dx * dx);
            }
        }
    }

    let mut result = vec![cap_q10; kind.len()];
    let mut sites: Vec<usize> = Vec::with_capacity(height);
    let mut starts: Vec<i128> = Vec::with_capacity(height);
    for x in 0..width {
        sites.clear();
        starts.clear();
        for q in 0..height {
            let fq = horizontal[q * width + x];
            if fq >= cap2 {
                continue;
            }
            let qi = i128::try_from(q).unwrap_or(i128::MAX);
            let mut start = i128::MIN;
            while let Some(&p) = sites.last() {
                let pi = i128::try_from(p).unwrap_or(i128::MAX);
                let fp = i128::from(horizontal[p * width + x]);
                // Integer intersections break equal-distance ties toward the
                // earlier site, without rounding the final distance.
                start = (i128::from(fq) + qi * qi - fp - pi * pi).div_euclid(2 * (qi - pi)) + 1;
                if start > starts[starts.len() - 1] {
                    break;
                }
                sites.pop();
                starts.pop();
            }
            if sites.is_empty() {
                start = i128::MIN;
            }
            sites.push(q);
            starts.push(start);
        }
        let mut active = 0;
        for y in 0..height {
            if sites.is_empty() {
                break;
            }
            while active + 1 < sites.len()
                && starts[active + 1] <= i128::try_from(y).unwrap_or(i128::MAX)
            {
                active += 1;
            }
            let dy = y.abs_diff(sites[active]);
            let dy = i64::try_from(dy).unwrap_or(i64::MAX);
            let d2 = (horizontal[sites[active] * width + x] + dy * dy).min(cap2);
            let scaled = u128::try_from(d2).unwrap_or(0) * 1024 * 1024;
            result[y * width + x] = i32::try_from(scaled.isqrt()).unwrap_or(i32::MAX);
        }
    }
    result
}

#[cfg(test)]
#[path = "tectonics_tests.rs"]
mod classification_tests;

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
mod tests;
