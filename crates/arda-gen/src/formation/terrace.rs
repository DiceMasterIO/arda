//! Lowland plains: river terraces, bluffs and flat interfluves (logic/02
//! §fine-formation terraces, goal 5).
//!
//! Every lowland cell is compared with the first river (at least
//! [`TERRACE_RIVER_KM2`]) it drains to. Its height above that river is
//! remapped by a monotone staircase:
//! - up to three **terrace treads** (more for larger rivers), each nearly
//!   flat, separated by short steep **risers**; the top riser is the valley
//!   **bluff**;
//! - above the bluff, the **interfluve plain** is compressed toward the
//!   bluff top, so plains between valleys are broadly flat.
//!
//! The map is strictly increasing in height above the river, so every cell
//! still drains downhill along its old path. The change is then smoothed
//! over ~80 m, which removes seams where neighbours drain to different
//! rivers; the final drainage guarantee fills the few hollows that leaves. Treads are
//! scaled by the floodplain depth `D = 2.5 m x (A/km²)^¼` (§floodplain), and
//! the whole transform fades with a lowland weight, so uplands and
//! mountains keep their valleys. The weight is the larger of the macro
//! lowland mask (§masks) and a local one: a river below 100 m (fading out
//! by 300 m) in ground whose 2.5 km height deviation is under 20 m (fading
//! out by 60 m). Hill country keeps its relief (goal 4).

use super::incision::isqrt;
use super::lattice::{alloc, blur_into, Lattice};
use super::FormationError;

/// Smallest river that carries terraces, km².
pub const TERRACE_RIVER_KM2: u64 = 20;
/// Terrace base above the river, in units of the floodplain depth D (Q8).
const BASE_D_Q8: i64 = 154; // 0.6 D
/// Terrace step height, D units (Q8).
const STEP_D_Q8: i64 = 384; // 1.5 D
/// Share of each step taken by its riser, Q12.
const RISER_Q12: i64 = 1_229; // 30%
/// Tread gradient kept from the original slope, Q12 (keeps drainage).
const TREAD_KEEP_Q12: i64 = 410; // 10%
/// Interfluve plain height kept above the bluff top at full lowland, Q12.
const PLAIN_KEEP_Q12: i64 = 1_843; // 45%
/// Local plain test: 2.5 km height deviation (m) where the local lowland
/// weight is full and where it has faded out.
const PLAIN_STD_M: (i64, i64) = (20, 60);
/// River height (m) where the local lowland weight is full / faded out.
const PLAIN_RIVER_M: (i64, i64) = (100, 300);
/// Local window radius, metres.
const PLAIN_RADIUS_M: i64 = 2_500;
/// Smoothing radius of the height change, metres.
const SMOOTH_M: i64 = 40;
/// Rivers below this height (drowned or at the shore) are left alone, mm.
const MIN_RIVER_MM: i32 = 250;

fn smooth_q12(t: i64) -> i64 {
    let t = t.clamp(0, 4_096);
    t * t / 4_096 * (3 * 4_096 - 2 * t) / 4_096
}

/// Terraced height above the river (mm) for original height `h` (mm),
/// floodplain depth `d` (mm) and `steps` terrace levels. Strictly
/// increasing in `h` and equal to it at `h <= base`.
#[must_use]
pub fn staircase(h: i64, d: i64, steps: i64) -> i64 {
    let base = d * BASE_D_Q8 / 256;
    let step = (d * STEP_D_Q8 / 256).max(1);
    if h <= base {
        return h;
    }
    let top = base + step * steps;
    if h >= top {
        return top + (h - top) * PLAIN_KEEP_Q12 / 4_096;
    }
    let u_q12 = (h - base) * 4_096 / step; // steps climbed, Q12
    let (k, frac) = (u_q12 / 4_096, u_q12 % 4_096);
    let riser_start = 4_096 - RISER_Q12;
    let s = if frac < riser_start {
        0
    } else {
        smooth_q12((frac - riser_start) * 4_096 / RISER_Q12)
    };
    let stair = k * 4_096 + s;
    let mapped = stair * (4_096 - TREAD_KEEP_Q12) / 4_096 + u_q12 * TREAD_KEEP_Q12 / 4_096;
    base + step * mapped / 4_096
}

/// Riser wander at world position `(xm, ym)` metres, −1000..=1000: two
/// rotated octaves of value noise (1.8 km and 700 m).
fn riser_wander(seed: u64, xm: i64, ym: i64) -> i64 {
    let (ax, ay) = (
        ((xm * 3_271 - ym * 2_465) / 4_096) as i32,
        ((xm * 2_465 + ym * 3_271) / 4_096) as i32,
    );
    let v = i64::from(crate::noise::value_noise(seed ^ 0x7E44, ax, ay, 1_800)) * 2
        + i64::from(crate::noise::value_noise(seed ^ 0x7E45, ay, -ax, 700));
    (v * 1_000 / (2 * 32_768)).clamp(-1_000, 1_000)
}

/// Floodplain depth D (mm) for a catchment of `km2` km².
fn flood_depth_mm(km2: u64) -> i64 {
    let root4 = isqrt(isqrt(km2.max(1) << 32)); // (A)^¼ in Q8
    2_500 * i64::try_from(root4).unwrap_or(0) / 256
}

/// Terraces valley sides and flattens interfluves on lowland ground.
/// `relief_q8` is the macro relief mask on `g`'s lattice (255 = 400 m).
/// Returns the number of cells changed.
///
/// # Errors
/// Allocation failure.
pub fn apply(g: &mut Lattice, relief_q8: &[u8], seed: u64) -> Result<usize, FormationError> {
    let (w, h) = (g.width, g.height);
    let n = w * h;
    let flow = super::flow::route(g)?;
    let cell_m2 = flow.cell_m2;
    // Local height deviation over ~2.5 km (metres), for the plain test.
    let std_m: Vec<i32> = {
        let r = usize::try_from(PLAIN_RADIUS_M * 1_000_000 / g.spacing_um)
            .map_err(|_| FormationError::ArithmeticOverflow)?;
        let m: Vec<i32> = g.z.iter().map(|&z| z.max(0) / 1_000).collect();
        let sq: Vec<i32> = m.iter().map(|&v| v.saturating_mul(v)).collect();
        let (mut tmp, mut mean, mut mean_sq) = (alloc(n)?, alloc(n)?, alloc(n)?);
        blur_into(&m, w, h, r, &mut tmp, &mut mean);
        blur_into(&sq, w, h, r, &mut tmp, &mut mean_sq);
        mean.iter()
            .zip(&mean_sq)
            .map(|(&a, &b)| {
                let var = i64::from(b) - i64::from(a) * i64::from(a);
                i32::try_from(isqrt(var.max(0) as u64)).unwrap_or(i32::MAX)
            })
            .collect()
    };
    let min_cells = TERRACE_RIVER_KM2 * 1_000_000 / cell_m2;
    // First river downstream of every cell (downstream-first order).
    let mut chan: Vec<u32> = alloc(n)?;
    for &i in flow.order.iter().rev() {
        let i = i as usize;
        let r = flow.receiver(i, w, h);
        chan[i] = if u64::from(flow.area[i]) >= min_cells || r == i {
            i as u32
        } else {
            chan[r]
        };
    }
    let area = flow.area;
    let base = g.z.clone();
    let mut delta: Vec<i32> = alloc(n)?;
    for i in 0..n {
        let c = chan[i] as usize;
        if c == i || u64::from(area[c]) < min_cells || base[c] < MIN_RIVER_MM {
            continue;
        }
        // Lowland weight at the cell (256 = full lowland): macro relief
        // under 250 m, or a low river in locally flat ground.
        let rel_m = i64::from(relief_q8[i]) * 400 / 255;
        let macro_low = (256 - 256 * rel_m.min(250) / 250).clamp(0, 256);
        let ramp = |v: i64, (a, b): (i64, i64)| (256 * (b - v) / (b - a)).clamp(0, 256);
        let local_low = ramp(i64::from(std_m[i]), PLAIN_STD_M)
            * ramp(i64::from(base[c]) / 1_000, PLAIN_RIVER_M)
            / 256;
        let lowland = macro_low.max(local_low);
        let hand = i64::from(base[i]) - i64::from(base[c]);
        if lowland == 0 || hand <= 0 {
            continue;
        }
        let km2 = u64::from(area[c]) * cell_m2 / 1_000_000;
        let steps = if km2 >= 1_000 {
            3
        } else if km2 >= 100 {
            2
        } else {
            1
        };
        let d = flood_depth_mm(km2);
        // Risers wander: the staircase is read at a height shifted by up to
        // ±0.3 steps with smooth noise (1.8 km and 700 m), so a riser is a
        // scalloped scarp cut by old meanders rather than a straight line
        // along the height contour of a planar valley side (seed-3 MICRO).
        let d_m = g.spacing_um / 1_000_000;
        let shift = d * STEP_D_Q8 / 256
            * riser_wander(seed, (i % w) as i64 * d_m, (i / w) as i64 * d_m)
            * 3
            / 10_000;
        let mapped = staircase(hand + shift, d, steps) - shift;
        delta[i] = i32::try_from((mapped - hand) * lowland / 256).unwrap_or(0);
    }
    drop(chan);
    drop(area);
    drop(std_m);
    // The remap is per river, so neighbours that drain to different rivers
    // step at their divide. Smoothing the change over ~80 m removes those
    // one-cell seams and softens risers into short bluffs; the final
    // drainage guarantee fills the few shallow hollows it leaves.
    let r = usize::try_from(SMOOTH_M * 1_000_000 / g.spacing_um)
        .unwrap_or(1)
        .max(1);
    let (mut tmp, mut smooth) = (alloc(n)?, alloc(n)?);
    blur_into(&delta, w, h, r, &mut tmp, &mut smooth);
    drop(tmp);
    let mut changed = 0;
    for ((z, &b), &dz) in g.z.iter_mut().zip(&base).zip(&smooth) {
        if dz == 0 || b <= 0 {
            continue;
        }
        // Land stays land: a terraced surface never drops below 0.3 m.
        let nz = (b + dz).max(b.min(300));
        if nz != *z {
            *z = nz;
            changed += 1;
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staircase_is_strictly_increasing_with_flat_treads_and_steep_risers() {
        let d = 8_000;
        let mut prev = i64::MIN;
        let mut slopes = Vec::new();
        for h in (0..120_000).step_by(250) {
            let v = staircase(h, d, 3);
            assert!(v > prev, "not increasing at {h}: {v} <= {prev}");
            if prev != i64::MIN {
                slopes.push(v - prev);
            }
            prev = v;
        }
        let min = *slopes.iter().min().unwrap();
        let max = *slopes.iter().max().unwrap();
        // Treads keep ~15% of the original gradient, risers ~3x.
        assert!(min * 4 < 250, "treads are not flat: {min}");
        assert!(max > 600, "risers are not steep: {max}");
        assert_eq!(staircase(2_000, d, 3), 2_000, "floodplain untouched");
    }

    #[test]
    fn interfluves_are_compressed_above_the_bluff() {
        let d = 8_000;
        let top = staircase(200_000, d, 2);
        assert!(top < 200_000 * 3 / 5, "plain not flattened: {top}");
    }

    #[test]
    fn lowland_valley_side_gains_flat_treads_and_few_pits() {
        // A 2.5 km valley side (1°) sloping to a river along x = 1, draining
        // south (the rim column x = 0 is fixed and kept high).
        let (w, h) = (64, 400);
        let mut g = Lattice::new(w, h, 39_062_500).unwrap();
        for y in 0..h {
            for x in 0..w {
                let bed = 5_000 + (h - y) as i32 * 20;
                g.z[y * w + x] = match x {
                    0 => 500_000,
                    1 => bed,
                    _ => bed + (x as i32 - 1) * 700 + 100,
                };
            }
        }
        for x in 0..w {
            g.z[(h - 1) * w + x] = -1_000; // the sea along the south edge
        }
        let before = g.clone();
        assert!(apply(&mut g, &vec![0; w * h], 5).unwrap() > 0);
        // Across the lower valley side some steps become near-flat treads.
        let y = 350;
        let flat = (3..40)
            .filter(|&x| (g.z[y * w + x + 1] - g.z[y * w + x]).abs() < 300)
            .count();
        let flat_before = (3..40)
            .filter(|&x| (before.z[y * w + x + 1] - before.z[y * w + x]).abs() < 300)
            .count();
        assert!(flat > flat_before + 3, "treads {flat} vs {flat_before}");
        let mut pits = 0;
        for y in 1..h - 1 {
            for x in 2..w - 1 {
                let i = y * w + x;
                let lower = [i - 1, i + 1, i - w, i + w, i - w - 1, i + w + 1]
                    .iter()
                    .any(|&j| g.z[j] < g.z[i]);
                pits += usize::from(!lower);
            }
        }
        assert!(pits * 100 < w * h, "{pits} pits");
    }

    #[test]
    fn risers_wander_along_a_planar_valley_side() {
        // Regression (seed-3 MICRO): a riser followed the height contour of
        // a planar valley side as a straight light line. A 16 km long, 1°
        // side along a river at x = 1: the first riser's distance from the
        // river must vary along the valley.
        let (w, h) = (64, 420);
        let mut g = Lattice::new(w, h, 39_062_500).unwrap();
        for y in 0..h {
            for x in 0..w {
                let bed = 5_000 + (h - y) as i32 * 20;
                g.z[y * w + x] = match x {
                    0 => 500_000,
                    1 => bed,
                    _ => bed + (x as i32 - 1) * 700 + 100,
                };
            }
        }
        for x in 0..w {
            g.z[(h - 1) * w + x] = -1_000;
        }
        apply(&mut g, &vec![0; w * h], 11).unwrap();
        let riser_x: Vec<i64> = (20..h - 20)
            .step_by(8)
            .filter_map(|y| {
                (3..w - 2)
                    .find(|&x| g.z[y * w + x + 1] - g.z[y * w + x] > 1_400)
                    .map(|x| x as i64)
            })
            .collect();
        assert!(riser_x.len() > 30, "risers found: {}", riser_x.len());
        // The floodplain depth grows downstream, so a contour-following
        // riser only drifts away from the river; a wandering one also comes
        // back towards it.
        let back = riser_x.windows(2).filter(|p| p[1] < p[0]).count();
        assert!(back >= 3, "riser offsets {riser_x:?}");
    }
}
