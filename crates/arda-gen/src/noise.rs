//! Hand-rolled integer value noise (`code-prefs.md` §Q2).
//!
//! Noise feeds relief, so it is determinism-critical: every operation here is
//! integer arithmetic with explicit wrapping, and nothing depends on float
//! rounding (`architecture-interview.md` §Q4).

/// Stateless lattice hash. Splitmix-style avalanche over the packed inputs.
#[must_use]
pub fn hash_2d(seed: u64, x: i32, y: i32) -> u32 {
    // Cast through u32 first so negative coordinates keep their bit pattern
    // and do not collide with their positive counterparts.
    let mut v = seed
        ^ (u64::from(x.cast_unsigned()) << 32)
        ^ u64::from(y.cast_unsigned()).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    v ^= v >> 30;
    v = v.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    v ^= v >> 27;
    v = v.wrapping_mul(0x94D0_49BB_1331_11EB);
    v ^= v >> 31;
    #[allow(clippy::cast_possible_truncation)]
    {
        v as u32
    }
}

/// The lattice value at a lattice index, in `-32768..=32767`.
fn lattice(seed: u64, lx: i32, ly: i32) -> i64 {
    i64::from(hash_2d(seed, lx, ly) % 65536) - 32768
}

/// Smoothstep weight on a `0..period` offset, scaled to `0..=65536`.
fn weight(offset: i32, period: i32) -> i64 {
    let t = i64::from(offset) * 65536 / i64::from(period);
    let t2 = (t * t) >> 16;
    let t3 = (t2 * t) >> 16;
    (3 * t2 - 2 * t3).clamp(0, 65536)
}

/// Bilinear value noise with smoothstep weights.
#[must_use]
pub fn value_noise(seed: u64, x: i32, y: i32, period: i32) -> i32 {
    let period = period.max(1);
    let lx = x.div_euclid(period);
    let ly = y.div_euclid(period);
    let fx = x.rem_euclid(period);
    let fy = y.rem_euclid(period);

    let v00 = lattice(seed, lx, ly);
    let v10 = lattice(seed, lx + 1, ly);
    let v01 = lattice(seed, lx, ly + 1);
    let v11 = lattice(seed, lx + 1, ly + 1);

    let wx = weight(fx, period);
    let wy = weight(fy, period);

    let top = v00 + (((v10 - v00) * wx) >> 16);
    let bottom = v01 + (((v11 - v01) * wx) >> 16);
    let value = top + (((bottom - top) * wy) >> 16);

    #[allow(clippy::cast_possible_truncation)]
    {
        value.clamp(-32768, 32767) as i32
    }
}

/// Fractional Brownian motion: `octaves` octaves, each half the period and
/// half the amplitude of the last.
#[must_use]
pub fn fbm(seed: u64, x: i32, y: i32, period: i32, octaves: u8) -> i32 {
    let mut total: i64 = 0;
    let mut amplitude: i64 = 65536;
    let mut normaliser: i64 = 0;
    let mut p = period.max(1);

    for octave in 0..octaves.max(1) {
        let sample = i64::from(value_noise(seed ^ (u64::from(octave) << 40), x, y, p));
        total += (sample * amplitude) >> 16;
        normaliser += amplitude;
        amplitude /= 2;
        p = (p / 2).max(1);
        if amplitude == 0 {
            break;
        }
    }

    if normaliser == 0 {
        return 0;
    }
    #[allow(clippy::cast_possible_truncation)]
    {
        (total * 65536 / normaliser).clamp(-32768, 32767) as i32
    }
}

/// Rotations of the octaves of [`rotated_noise`], `(cos, sin)` in Q12:
/// 19°, 41°, 64°, 30°, 53°, 75°, 24° and 47°. Each lies at least 15° from
/// the world axes, and successive octaves differ by at least 22°, so no
/// two octaves share lattice axes and none lines up with the world's.
const ROTATIONS_Q12: [(i64, i64); 8] = [
    (3_873, 1_334),
    (3_091, 2_687),
    (1_796, 3_681),
    (3_547, 2_048),
    (2_465, 3_271),
    (1_060, 3_956),
    (3_742, 1_666),
    (2_793, 2_996),
];

/// Value noise on a lattice rotated and shifted per `octave` (recipe 9,
/// review 3.1-4). Plain [`value_noise`] octaves share an origin and the
/// world axes, so their lattice lines coincide and their gradients lean to
/// the axes; this rotates each octave by its own angle and offsets it by a
/// seeded fraction of many periods, so octaves neither share axes nor
/// origins. `x`, `y` are world units (metres in formation).
#[must_use]
pub fn rotated_noise(seed: u64, x: i64, y: i64, period: i32, octave: usize) -> i32 {
    let (c, s) = ROTATIONS_Q12[octave % ROTATIONS_Q12.len()];
    let span = i64::from(period.max(1)) * 64;
    let ox = i64::from(hash_2d(
        seed ^ 0x0FF5_E701,
        i32::try_from(octave).unwrap_or(0),
        1,
    )) % span;
    let oy = i64::from(hash_2d(
        seed ^ 0x0FF5_E702,
        i32::try_from(octave).unwrap_or(0),
        2,
    )) % span;
    let rx = (x * c - y * s).div_euclid(4_096) + ox;
    let ry = (x * s + y * c).div_euclid(4_096) + oy;
    #[allow(clippy::cast_possible_truncation)]
    value_noise(seed, rx as i32, ry as i32, period)
}

/// [`fbm`] built from [`rotated_noise`] octaves: each octave halves the
/// period and amplitude and turns to its own angle and origin.
#[must_use]
pub fn rotated_fbm(seed: u64, x: i64, y: i64, period: i32, octaves: u8) -> i32 {
    let mut total: i64 = 0;
    let mut amplitude: i64 = 65_536;
    let mut normaliser: i64 = 0;
    let mut p = period.max(1);
    for octave in 0..octaves.max(1) {
        let sample = i64::from(rotated_noise(
            seed ^ (u64::from(octave) << 40),
            x,
            y,
            p,
            usize::from(octave),
        ));
        total += (sample * amplitude) >> 16;
        normaliser += amplitude;
        amplitude /= 2;
        p = (p / 2).max(1);
        if amplitude == 0 {
            break;
        }
    }
    if normaliser == 0 {
        return 0;
    }
    #[allow(clippy::cast_possible_truncation)]
    {
        (total * 65_536 / normaliser).clamp(-32_768, 32_767) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_for_the_same_input() {
        assert_eq!(hash_2d(42, 3, 11), hash_2d(42, 3, 11));
    }

    #[test]
    fn hash_separates_transposed_coordinates() {
        assert_ne!(hash_2d(42, 3, 11), hash_2d(42, 11, 3));
    }

    #[test]
    fn hash_separates_sign_flips() {
        assert_ne!(hash_2d(42, -3, 11), hash_2d(42, 3, 11));
        assert_ne!(hash_2d(42, 3, -11), hash_2d(42, 3, 11));
    }

    #[test]
    fn value_noise_stays_in_range() {
        for y in -40..40 {
            for x in -40..40 {
                let v = value_noise(42, x, y, 8);
                assert!(
                    (-32768..=32767).contains(&v),
                    "out of range at {x},{y}: {v}"
                );
            }
        }
    }

    #[test]
    fn value_noise_is_continuous_between_lattice_points() {
        let period = 16;
        let mut worst = 0i32;
        for x in 0..64 {
            let a = value_noise(42, x, 0, period);
            let b = value_noise(42, x + 1, 0, period);
            worst = worst.max((a - b).abs());
        }
        assert!(worst < 65536 / period, "discontinuity of {worst}");
    }

    #[test]
    fn value_noise_hits_lattice_values_exactly() {
        // At a lattice point the interpolation weight is zero, so the sample
        // is the lattice value itself — this pins the interpolation maths.
        let period = 8;
        let lattice_value = i64::from(hash_2d(42, 0, 0) % 65536) - 32768;
        assert_eq!(i64::from(value_noise(42, 0, 0, period)), lattice_value);
    }

    #[test]
    fn fbm_stays_in_range() {
        for y in -20..20 {
            for x in -20..20 {
                let v = fbm(42, x, y, 32, 4);
                assert!(
                    (-32768..=32767).contains(&v),
                    "out of range at {x},{y}: {v}"
                );
            }
        }
    }

    #[test]
    fn fbm_differs_from_single_octave() {
        assert_ne!(fbm(42, 5, 7, 32, 4), value_noise(42, 5, 7, 32));
    }

    #[test]
    fn different_seeds_give_different_fields() {
        let a: Vec<i32> = (0..32).map(|x| fbm(42, x, 0, 16, 3)).collect();
        let b: Vec<i32> = (0..32).map(|x| fbm(43, x, 0, 16, 3)).collect();
        assert_ne!(a, b);
    }
}
