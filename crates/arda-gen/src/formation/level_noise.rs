//! The noise fields of the level masks (logic/02 §fine-formation levels and
//! masks): along-range uplift and erodibility variation, the level-0 base
//! perturbation and the band-limited relief of levels 1 and 2.
//!
//! Recipes 5-8 read plain value noise in world metres. Its octaves share
//! the world axes and origin, so 24 km lattice lines are invariant flow
//! lines and gradients lean to the axes: 40.7% of slope-weighted
//! directions within ±10° of an axis against an isotropic 22.2% (review
//! 3.1-4, goal-prompt §7, a source of OI-03's axis lean). Recipe 9 turns
//! every octave to its own angle and origin ([`crate::noise::rotated_noise`]).
#![allow(clippy::cast_possible_truncation)]

use crate::noise::{fbm, rotated_fbm, rotated_noise, value_noise};

/// Along-range uplift variation, Q16 (about 0.2..1.6 before clamping).
#[must_use]
pub fn uplift_variation(seed: u64, xm: i64, ym: i64, v9: bool) -> i64 {
    let (a, b) = if v9 {
        (
            rotated_noise(seed ^ 0x4242, xm, ym, 45_000, 0),
            rotated_noise(seed ^ 0x4243, xm, ym, 18_000, 1),
        )
    } else {
        let (xi, yi) = (xm as i32, ym as i32);
        (
            value_noise(seed ^ 0x4242, xi, yi, 45_000),
            value_noise(seed ^ 0x4243, xi, yi, 18_000),
        )
    };
    36_045 + (i64::from(a) + i64::from(b) / 2) * 29_491 / 32_768
}

/// Erodibility, 64 = 1, before clamping.
#[must_use]
pub fn erodibility(seed: u64, xm: i64, ym: i64, v9: bool) -> i64 {
    let (a, b) = if v9 {
        (
            rotated_noise(seed ^ 0x9191, xm, ym, 14_000, 2),
            rotated_noise(seed ^ 0x9192, xm, ym, 5_000, 3),
        )
    } else {
        let (xi, yi) = (xm as i32, ym as i32);
        (
            value_noise(seed ^ 0x9191, xi, yi, 14_000),
            value_noise(seed ^ 0x9192, xi, yi, 5_000),
        )
    };
    64 + (i64::from(a) * 32 + i64::from(b) * 16) / 32_768
}

/// Level-0 base perturbation: four octaves from 24 km, ±32768.
#[must_use]
pub fn base(seed: u64, xm: i64, ym: i64, v9: bool) -> i32 {
    if v9 {
        rotated_fbm(seed, xm, ym, 24_000, 4)
    } else {
        fbm(seed, xm as i32, ym as i32, 24_000, 4)
    }
}

/// Band-limited relief of levels 1 and 2 at `period` metres, ±32768.
#[must_use]
pub fn level(seed: u64, xm: i64, ym: i64, period: i32, v9: bool) -> i32 {
    if v9 {
        rotated_noise(seed, xm, ym, period, 4)
    } else {
        value_noise(seed, xm as i32, ym as i32, period)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Slope-weighted share (‰) of gradient directions within ±10° of an
    /// axis, from central differences on a `step`-metre grid. An isotropic
    /// field gives 222‰.
    fn axis_share(field: &dyn Fn(i64, i64) -> i64, step: i64) -> i64 {
        let (mut near, mut total) = (0.0_f64, 0.0_f64);
        for y in 1..399_i64 {
            for x in 1..399_i64 {
                let gx = (field((x + 1) * step, y * step) - field((x - 1) * step, y * step)) as f64;
                let gy = (field(x * step, (y + 1) * step) - field(x * step, (y - 1) * step)) as f64;
                let mag = gx.hypot(gy);
                if mag == 0.0 {
                    continue;
                }
                let deg = gy.atan2(gx).to_degrees().rem_euclid(90.0);
                total += mag;
                if !(10.0..=80.0).contains(&deg) {
                    near += mag;
                }
            }
        }
        (near * 1_000.0 / total) as i64
    }

    /// Review 3.1-4: the recipe-7 mask noise leans to the axes; recipe 9's
    /// rotated octaves are within a few points of isotropic.
    #[test]
    fn recipe_9_mask_noise_does_not_lean_to_the_axes() {
        for (name, step, f7, f9) in [
            (
                "uplift",
                900,
                Box::new(|x, y| uplift_variation(5, x, y, false)) as Box<dyn Fn(i64, i64) -> i64>,
                Box::new(|x, y| uplift_variation(5, x, y, true)) as Box<dyn Fn(i64, i64) -> i64>,
            ),
            (
                "erodibility",
                1_000,
                Box::new(|x, y| erodibility(5, x, y, false) * 1_000),
                Box::new(|x, y| erodibility(5, x, y, true) * 1_000),
            ),
            (
                "base",
                500,
                Box::new(|x, y| i64::from(base(5, x, y, false))),
                Box::new(|x, y| i64::from(base(5, x, y, true))),
            ),
            (
                "level",
                60,
                Box::new(|x, y| i64::from(level(5, x, y, 2_000, false))),
                Box::new(|x, y| i64::from(level(5, x, y, 2_000, true))),
            ),
        ] {
            let (a7, a9) = (axis_share(&*f7, step), axis_share(&*f9, step));
            eprintln!("{name}: axis share recipe 7 {a7}‰, recipe 9 {a9}‰");
            assert!(a7 >= 260, "{name}: recipe 7 leans to the axes ({a7}‰)");
            assert!(a9 <= 240, "{name}: recipe 9 is near isotropic ({a9}‰)");
        }
    }

    #[test]
    fn recipe_7_values_are_the_published_ones() {
        let (xm, ym) = (12_345, 67_890);
        assert_eq!(
            base(9, xm, ym, false),
            fbm(9, xm as i32, ym as i32, 24_000, 4)
        );
        assert_eq!(
            level(9, xm, ym, 700, false),
            value_noise(9, xm as i32, ym as i32, 700)
        );
    }
}
