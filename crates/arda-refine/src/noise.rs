//! Rotated multi-octave gradient noise in global square units.
//!
//! Every octave is rotated by a different fixed angle before sampling, so no
//! lattice axis of any octave lines up with the square grid or with another
//! octave (goal 43: no axis-aligned bias). Only `+ - * /` and `floor` are
//! used, which IEEE-754 makes bit-identical on every platform.

use crate::hash::{hash3, mix};

/// Cosine and sine of the per-octave rotations: 29.0, 67.5, 113.3, 151.7,
/// 197.1 and 241.9 degrees.
const ROTATIONS: [(f64, f64); 6] = [
    (0.8746197071393957, 0.48480962024633706),
    (0.38268343236508984, 0.9238795325112867),
    (-0.3955455025629649, 0.9184463813430872),
    (-0.880477353509162, 0.4740882090471163),
    (-0.9557930147983301, -0.2940403252323039),
    (-0.4710118812194099, -0.8821268660176679),
];

/// Sixteen unit gradients, 22.5 degrees apart, offset by 11.25 degrees so
/// none is axis-aligned.
const GRADIENTS: [(f64, f64); 16] = [
    (0.9807852804032304, 0.19509032201612825),
    (0.8314696123025452, 0.5555702330196022),
    (0.5555702330196023, 0.8314696123025452),
    (0.19509032201612833, 0.9807852804032304),
    (-0.1950903220161282, 0.9807852804032304),
    (-0.5555702330196023, 0.8314696123025451),
    (-0.8314696123025453, 0.5555702330196022),
    (-0.9807852804032304, 0.19509032201612816),
    (-0.9807852804032304, -0.19509032201612836),
    (-0.8314696123025452, -0.5555702330196023),
    (-0.5555702330196022, -0.8314696123025452),
    (-0.19509032201612866, -0.9807852804032303),
    (0.1950903220161283, -0.9807852804032304),
    (0.5555702330196018, -0.8314696123025455),
    (0.8314696123025452, -0.5555702330196022),
    (0.9807852804032303, -0.19509032201612872),
];

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn grad(seed: u64, tag: u64, ix: i64, iy: i64, dx: f64, dy: f64) -> f64 {
    let h = hash3(seed, tag, ix, iy, 0x6E);
    // The top four bits pick the gradient; the index is always < 16.
    let (gx, gy) = GRADIENTS[usize::try_from(h >> 60).unwrap_or(0) & 15];
    gx * dx + gy * dy
}

/// Single-octave gradient noise, roughly in `[-1, 1]`, unit lattice.
#[must_use]
pub fn gradient(seed: u64, tag: u64, x: f64, y: f64) -> f64 {
    let fx = x.floor();
    let fy = y.floor();
    #[allow(clippy::cast_possible_truncation)]
    let (ix, iy) = (fx as i64, fy as i64);
    let (dx, dy) = (x - fx, y - fy);
    let n00 = grad(seed, tag, ix, iy, dx, dy);
    let n10 = grad(seed, tag, ix + 1, iy, dx - 1.0, dy);
    let n01 = grad(seed, tag, ix, iy + 1, dx, dy - 1.0);
    let n11 = grad(seed, tag, ix + 1, iy + 1, dx - 1.0, dy - 1.0);
    let (u, v) = (fade(dx), fade(dy));
    let a = n00 + (n10 - n00) * u;
    let b = n01 + (n11 - n01) * u;
    (a + (b - a) * v) * 1.414
}

/// Fractal sum of rotated octaves. `wavelength` is in squares; each octave
/// halves it and scales amplitude by `gain`. The result is normalised to
/// roughly `[-1, 1]`.
#[must_use]
pub fn fbm(seed: u64, tag: u64, x: f64, y: f64, wavelength: f64, octaves: u32, gain: f64) -> f64 {
    let mut sum = 0.0;
    let mut norm = 0.0;
    let mut amp = 1.0;
    let mut freq = 1.0 / wavelength;
    let salt = mix(seed ^ tag);
    for o in 0..octaves.min(6) {
        let (c, s) = ROTATIONS[o as usize];
        // Per-octave offset breaks the shared lattice origin as well.
        let ox = crate::hash::unit(mix(salt ^ u64::from(o))) * 97.0;
        let oy = crate::hash::unit(mix(salt ^ u64::from(o) ^ 0xABCD)) * 97.0;
        let rx = (c * x - s * y) * freq + ox;
        let ry = (s * x + c * y) * freq + oy;
        sum += amp * gradient(seed, tag.wrapping_add(u64::from(o)), rx, ry);
        norm += amp;
        amp *= gain;
        freq *= 2.0;
    }
    if norm > 0.0 {
        sum / norm
    } else {
        0.0
    }
}

/// Smooth cubic step from `a` to `b`.
#[must_use]
pub fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    if (b - a).abs() < 1e-12 {
        return if x < a { 0.0 } else { 1.0 };
    }
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotations_and_gradients_are_unit_vectors() {
        for (c, s) in ROTATIONS.iter().chain(GRADIENTS.iter()) {
            assert!((c * c + s * s - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn noise_is_bounded_and_centred() {
        let mut sum = 0.0;
        let mut max: f64 = 0.0;
        for i in 0..20_000 {
            let x = f64::from(i % 200) * 0.37;
            let y = f64::from(i / 200) * 0.41;
            let n = fbm(7, 3, x, y, 8.0, 3, 0.5);
            sum += n;
            max = max.max(n.abs());
        }
        assert!(max <= 1.2, "{max}");
        assert!((sum / 20_000.0).abs() < 0.08, "{}", sum / 20_000.0);
    }

    #[test]
    fn noise_is_a_function_of_position_only() {
        assert_eq!(
            fbm(1, 2, 1234.5, -77.25, 11.0, 3, 0.5).to_bits(),
            fbm(1, 2, 1234.5, -77.25, 11.0, 3, 0.5).to_bits()
        );
    }
}
