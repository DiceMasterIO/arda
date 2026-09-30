//! Submarine canyons off major rivers (logic/02 §fine-formation canyons,
//! goal 18).
//!
//! During glacial lowstands large rivers delivered sediment straight to
//! the shelf edge, and turbidity currents cut canyons down the continental
//! slope. From each major river mouth a path follows the steepest descent
//! of the smoothed seafloor as a continuous heading (not D8 steps), with
//! inertia and a slow wander, so the axis curves and never runs along a
//! lattice direction. Where the water is deeper than the shelf
//! (from [`HEAD_MM`]) a V-shaped canyon is cut along it, deepest across
//! the slope and fading out onto the abyssal fan. Only open sea is cut, so
//! land, drainage and the water ledger are untouched.

use super::lattice::{alloc, blur_into, Lattice};
use super::FormationError;

/// Smallest river that cuts a canyon, km².
pub const CANYON_MIN_KM2: u64 = 150;
/// Canyon heads where the seafloor passes this depth, millimetres.
pub const HEAD_MM: i64 = 60_000;
/// Full incision from this depth, millimetres.
const FULL_MM: i64 = 300_000;
/// Incision fades out between these depths, millimetres (the fan).
const FAN_MM: (i64, i64) = (2_500_000, 3_500_000);
/// Canyon relief at the minimum catchment, and per doubling, millimetres.
const RELIEF_BASE_MM: i64 = 120_000;
const RELIEF_PER_DOUBLING_MM: i64 = 60_000;
/// Relief cap, millimetres.
const RELIEF_MAX_MM: i64 = 500_000;
/// Longest path, metres: the shelf and slope, not the abyssal plain.
const PATH_MAX_M: i64 = 50_000;
/// The last share (‰) of the longest path fades the incision out.
const TAIL_PERMILLE: i64 = 300;
/// Heading wander: up to ±22.5° over a 7 km value-noise wavelength.
const WANDER_M: i32 = 7_000;

/// Canyon relief (mm) of a river of `km2`.
fn relief_mm(km2: u64) -> i64 {
    let doublings = 63 - i64::from((km2 / CANYON_MIN_KM2).max(1).leading_zeros());
    (RELIEF_BASE_MM + RELIEF_PER_DOUBLING_MM * doublings).min(RELIEF_MAX_MM)
}

/// Incision share (Q10) at seafloor depth `d` (mm, positive down).
fn share_q10(d: i64) -> i64 {
    if d <= HEAD_MM {
        return 0;
    }
    let ramp = ((d - HEAD_MM) * 1_024 / (FULL_MM - HEAD_MM)).min(1_024);
    let fade = ((FAN_MM.1 - d) * 1_024 / (FAN_MM.1 - FAN_MM.0)).clamp(0, 1_024);
    ramp * fade / 1_024
}

/// Cuts canyons off the given river mouths `(fine index, km²)`. Returns
/// the number of canyons cut.
///
/// # Errors
/// Allocation failure.
pub fn carve(g: &mut Lattice, mouths: &[(usize, u64)]) -> Result<usize, FormationError> {
    let (w, h) = (g.width, g.height);
    let n = w * h;
    let d_m = (g.spacing_um / 1_000_000).max(1);
    let r = usize::try_from(1_200 / d_m).unwrap_or(1).max(1);
    let (mut tmp, mut smooth) = (alloc(n)?, alloc(n)?);
    blur_into(&g.z, w, h, r, &mut tmp, &mut smooth);
    drop(tmp);
    let base = g.z.clone();
    let mut majors: Vec<(u64, usize)> = mouths
        .iter()
        .filter(|&&(_, km2)| km2 >= CANYON_MIN_KM2)
        .map(|&(i, km2)| (km2, i))
        .collect();
    majors.sort_unstable_by(|a, b| b.cmp(a));
    let mut cut = 0;
    for (km2, mouth) in majors {
        let relief = relief_mm(km2);
        let path = axis(&smooth, w, h, mouth, d_m, &base, km2);
        let mut any = false;
        let max_steps = (PATH_MAX_M / d_m * 2).max(1);
        for (s, &(px, py)) in path.iter().enumerate() {
            let (px, py) = (px.clamp(1, w as i64 - 2), py.clamp(1, h as i64 - 2));
            let p = (py as usize) * w + px as usize;
            let depth = -i64::from(base[p]);
            let left = max_steps - s as i64;
            let tail = (left * 1_000 / (max_steps * TAIL_PERMILLE / 1_000).max(1)).min(1_000);
            let dr = relief * share_q10(depth) / 1_024 * tail / 1_000;
            if dr <= 0 {
                continue;
            }
            // V cross-section with ~14° walls: half-width four times depth.
            let half_m = (dr * 4 / 1_000).max(300);
            let rc = half_m / d_m;
            let floor = i64::from(base[p]) - dr;
            for oy in -rc..=rc {
                for ox in -rc..=rc {
                    let (x, y) = (px + ox, py + oy);
                    if x < 1 || y < 1 || x >= w as i64 - 1 || y >= h as i64 - 1 {
                        continue;
                    }
                    let dist =
                        i64::try_from((ox * ox + oy * oy).unsigned_abs().isqrt()).unwrap_or(0);
                    if dist > rc {
                        continue;
                    }
                    let k = y as usize * w + x as usize;
                    if base[k] >= 0 {
                        continue;
                    }
                    let target = floor + dr * dist / rc.max(1);
                    let t = i32::try_from(target).unwrap_or(g.z[k]);
                    if t < g.z[k] {
                        g.z[k] = t;
                        any = true;
                    }
                }
            }
        }
        cut += usize::from(any);
    }
    Ok(cut)
}

/// The canyon axis from `mouth`: lattice cells, one per half cell of
/// travel. The heading follows the downhill gradient of `smooth` with
/// inertia (three parts previous heading to one part gradient), turned by a
/// slow value-noise wander. Stops on flat ground, at the fan depth, at the
/// rim or after [`PATH_MAX_M`].
fn axis(
    smooth: &[i32],
    w: usize,
    h: usize,
    mouth: usize,
    d_m: i64,
    base: &[i32],
    km2: u64,
) -> Vec<(i64, i64)> {
    use super::water::geom::{rotate, unit, CELL_Q8, ONE_Q14, TURN};
    let grad = |x: i64, y: i64| -> (i64, i64) {
        let at = |x: i64, y: i64| {
            i64::from(
                smooth[(y.clamp(0, h as i64 - 1) as usize) * w + x.clamp(0, w as i64 - 1) as usize],
            )
        };
        (at(x + 2, y) - at(x - 2, y), at(x, y + 2) - at(x, y - 2))
    };
    let mut pos = ((mouth % w) as i64 * CELL_Q8, (mouth / w) as i64 * CELL_Q8);
    let mut out = Vec::new();
    let mut heading: Option<(i64, i64)> = None;
    let seed = 0xCA_2701 ^ km2 ^ (mouth as u64).rotate_left(17);
    for _ in 0..(PATH_MAX_M / d_m * 2) {
        let (x, y) = (
            (pos.0 + CELL_Q8 / 2) / CELL_Q8,
            (pos.1 + CELL_Q8 / 2) / CELL_Q8,
        );
        if x < 3 || y < 3 || x + 3 >= w as i64 || y + 3 >= h as i64 {
            break;
        }
        if -i64::from(base[(y as usize) * w + x as usize]) > FAN_MM.1 {
            break;
        }
        let (gx, gy) = grad(x, y);
        if (gx, gy) == (0, 0) {
            break;
        }
        let down = unit(-gx, -gy);
        let hd = match heading {
            Some(p) => {
                // Never climb: an uphill blend falls back to the gradient.
                let b = unit(3 * p.0 + down.0, 3 * p.1 + down.1);
                if b.0 * down.0 + b.1 * down.1 <= 0 {
                    down
                } else {
                    b
                }
            }
            None => down,
        };
        heading = Some(hd);
        let (xm, ym) = (
            i32::try_from(x * d_m).unwrap_or(0),
            i32::try_from(y * d_m).unwrap_or(0),
        );
        let wander =
            i64::from(crate::noise::value_noise(seed, xm, ym, WANDER_M)) * (TURN / 16) / 32_768;
        let step = rotate(hd, wander);
        pos = (
            pos.0 + step.0 * (CELL_Q8 / 2) / ONE_Q14,
            pos.1 + step.1 * (CELL_Q8 / 2) / ONE_Q14,
        );
        out.push((x, y));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relief_grows_with_catchment_and_share_follows_depth() {
        assert!(relief_mm(150) < relief_mm(2_400));
        assert!(relief_mm(1_000_000) <= RELIEF_MAX_MM);
        assert_eq!(share_q10(30_000), 0, "no canyon on the inner shelf");
        assert_eq!(share_q10(1_000_000), 1_024, "full on the slope");
        assert_eq!(share_q10(4_000_000), 0, "gone on the abyssal plain");
    }

    #[test]
    fn a_major_river_cuts_a_canyon_down_the_slope() {
        // Land in the north, a 5 km shelf, then a steep slope to 3 km.
        let (w, h) = (200, 900);
        let mut g = Lattice::new(w, h, 39_062_500).unwrap();
        for y in 0..h {
            for x in 0..w {
                let z = if y < 100 {
                    10_000
                } else if y < 228 {
                    -((y as i32 - 100) * 700)
                } else {
                    -(89_600 + (y as i32 - 228) * 8_000).min(3_000_000)
                };
                g.z[y * w + x] = z;
            }
        }
        let before = g.clone();
        let mouth = 99 * w + 100;
        assert_eq!(carve(&mut g, &[(mouth, 1_000)]).unwrap(), 1);
        let row = 300; // mid-slope
        let centre = g.z[row * w + 100];
        assert!(
            centre < before.z[row * w + 100] - 100_000,
            "canyon floor {centre}"
        );
        assert_eq!(g.z[row * w + 10], before.z[row * w + 10], "walls end");
        // The axis curves: no 5 km run of it stays on one lattice column.
        let cols: Vec<usize> = (228..356)
            .map(|y| (0..w).min_by_key(|&x| g.z[y * w + x]).unwrap())
            .collect();
        assert!(
            cols.windows(128).all(|r| r.iter().any(|&c| c != r[0])),
            "grid-aligned canyon axis"
        );
        assert!(
            g.z[..100 * w].iter().all(|&z| z == 10_000),
            "land untouched"
        );
    }
}
