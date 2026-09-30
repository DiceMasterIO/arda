//! Hashing and rotated value noise.
//!
//! Lesson §7 of the master plan: never use raw axis-aligned value noise for
//! anything visible. Every octave here is sampled through an integer
//! rotation-and-scale matrix (±26.57°, ×√5), which keeps the integer lattice
//! so periodic (tileable) noise stays periodic, yet no octave lines up with
//! the grid. All arithmetic is IEEE single precision with only `+ - * /` and
//! `floor`, so it is deterministic on every platform.

/// SplitMix64 finaliser: a strong 64-bit mix.
#[must_use]
pub fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Hashes a seed and two signed coordinates.
#[must_use]
pub fn hash2(seed: u64, x: i64, y: i64) -> u64 {
    mix(mix(seed ^ x.cast_unsigned().wrapping_mul(0x632B_E59B_D9B4_E019)) ^ y.cast_unsigned())
}

/// Hashes a seed and a string key (FNV-1a folded through [`mix`]).
#[must_use]
pub fn hash_str(seed: u64, key: &str) -> u64 {
    let mut h = 0xCBF2_9CE4_8422_2325_u64 ^ seed;
    for b in key.bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01B3);
    }
    mix(h)
}

/// A uniform value in `[0, 1)` from a hash (24 exact mantissa bits).
#[must_use]
pub fn unit(h: u64) -> f32 {
    (h >> 40) as f32 / 16_777_216.0
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn lattice(seed: u64, x: i64, y: i64, period: Option<i64>) -> f32 {
    let (x, y) = match period {
        Some(p) if p > 0 => (x.rem_euclid(p), y.rem_euclid(p)),
        _ => (x, y),
    };
    unit(hash2(seed, x, y))
}

/// Plain value noise in `[0, 1)` on the unit lattice; `period` wraps it.
#[must_use]
// Lattice coordinates are floors of bounded map positions; no truncation.
#[allow(clippy::cast_possible_truncation)]
pub fn value(seed: u64, x: f32, y: f32, period: Option<i64>) -> f32 {
    let (fx, fy) = (x.floor(), y.floor());
    let (ix, iy) = (fx as i64, fy as i64);
    let (tx, ty) = (smooth(x - fx), smooth(y - fy));
    let a = lattice(seed, ix, iy, period);
    let b = lattice(seed, ix + 1, iy, period);
    let c = lattice(seed, ix, iy + 1, period);
    let d = lattice(seed, ix + 1, iy + 1, period);
    let top = a + (b - a) * tx;
    let bottom = c + (d - c) * tx;
    top + (bottom - top) * ty
}

/// Integer rotation-scale matrices, alternated per octave.
const ROTATIONS: [[i64; 4]; 2] = [[2, -1, 1, 2], [1, 2, -2, 1]];

/// Rotated fractal value noise in `[0, 1)`.
///
/// `x, y` are in *cells* of the first octave. With `period = Some(p)` the
/// result is periodic with period `p` cells in both axes (`p` ≥ 1).
#[must_use]
pub fn fbm(seed: u64, x: f32, y: f32, octaves: u32, period: Option<i64>) -> f32 {
    let mut sum = 0.0;
    let mut norm = 0.0;
    let mut amp = 1.0;
    let mut scale = 1_i64;
    for o in 0..octaves {
        let m = ROTATIONS[(o % 2) as usize];
        let (sx, sy) = (x * scale as f32, y * scale as f32);
        let rx = m[0] as f32 * sx + m[1] as f32 * sy;
        let ry = m[2] as f32 * sx + m[3] as f32 * sy;
        // The matrix maps a shift of `p` cells to a lattice shift that is a
        // multiple of `p`, so wrapping the lattice at `p·scale` keeps the
        // octave periodic.
        let per = period.map(|p| p * scale);
        sum += amp * value(mix(seed ^ u64::from(o)), rx, ry, per);
        norm += amp;
        amp *= 0.5;
        scale *= 2;
    }
    sum / norm
}

/// Sine and cosine of an angle given in *turns* (1.0 = 360°).
///
/// Range-reduced Taylor series with only `+ - * floor`, so the result is
/// identical on every platform (unlike `f32::sin`). Error < 1e-6.
#[must_use]
pub fn sincos(turns: f32) -> (f32, f32) {
    let t = turns - turns.floor(); // [0, 1)
                                   // Map to [-0.5, 0.5) turns, then to radians in [-π, π).
    let t = if t >= 0.5 { t - 1.0 } else { t };
    let x = t * core::f32::consts::TAU;
    // Reduce further to [-π/2, π/2] with sin(π - x) = sin x, cos(π - x) = -cos x.
    let half_pi = core::f32::consts::FRAC_PI_2;
    let (r, flip) = if x > half_pi {
        (core::f32::consts::PI - x, true)
    } else if x < -half_pi {
        (-core::f32::consts::PI - x, true)
    } else {
        (x, false)
    };
    let r2 = r * r;
    let sin = r
        * (1.0
            - r2 / 6.0
                * (1.0 - r2 / 20.0 * (1.0 - r2 / 42.0 * (1.0 - r2 / 72.0 * (1.0 - r2 / 110.0)))));
    let cos = 1.0
        - r2 / 2.0 * (1.0 - r2 / 12.0 * (1.0 - r2 / 30.0 * (1.0 - r2 / 56.0 * (1.0 - r2 / 90.0))));
    (sin, if flip { -cos } else { cos })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sincos_matches_the_standard_library() {
        for i in -40..=40 {
            let t = i as f32 * 0.0371;
            let (s, c) = sincos(t);
            let a = t * core::f32::consts::TAU;
            assert!((s - a.sin()).abs() < 1e-5, "sin {t}");
            assert!((c - a.cos()).abs() < 1e-5, "cos {t}");
        }
    }

    #[test]
    fn periodic_fbm_wraps_exactly() {
        for i in 0..20 {
            let (x, y) = (i as f32 * 0.37, i as f32 * 0.21);
            let a = fbm(7, x, y, 4, Some(3));
            assert!((a - fbm(7, x + 3.0, y, 4, Some(3))).abs() < 1e-4);
            assert!((a - fbm(7, x, y + 3.0, 4, Some(3))).abs() < 1e-4);
        }
    }

    #[test]
    fn noise_is_in_unit_range_and_seeded() {
        let mut differs = false;
        for i in 0..200 {
            let (x, y) = (i as f32 * 0.13, i as f32 * 0.29);
            let v = fbm(1, x, y, 3, None);
            assert!((0.0..1.0).contains(&v));
            differs |= v != fbm(2, x, y, 3, None);
        }
        assert!(differs);
    }
}
