//! Resolution-aware sub-grid relief for recipe-5 shading (logic/04
//! §atlas-formed detail), integer Q12.
//!
//! The canonical field resolves landforms down to about 80 m (39 m nodes).
//! Once an export draws pixels much smaller than that, the interpolated field
//! alone reads as smooth and scaled up (goals 32, 33). This module adds the
//! missing wavelengths to the *light* only, never to height, ownership or
//! drainage:
//! - Each octave (160, 96, 48 and 24 m across the fall line) has a fixed world wavelength, so detail stays put across
//!   exports; it fades in only once it spans at least three pixels, and is
//!   full at six, so nothing aliases and no export is supersampled.
//! - On slopes the detail is runnels: gradient noise stretched 3:1 along the
//!   fall line (from the smooth 312 m gradient), so it reads as fluting and
//!   scree chutes rather than crumpled paper. Four fixed orientations are
//!   blended by alignment, so the pattern never swims as the frame turns.
//! - Gentle ground gets only a faint isotropic undulation.
//!
//! Noise is Perlin-style gradient noise with a quintic fade and an analytic
//! derivative, in rotated domains, never axis-aligned value noise.

const ONE: i64 = 4_096;

/// Across-fall wavelengths per octave, decimetres (160 m, 96 m, 48 m, 24 m).
/// The 160 m octave sits near the field's own limit, so it is weaker.
const OCTAVES_DM: [i64; 4] = [1_600, 960, 480, 240];
/// Relative slope amplitude per octave, Q12.
const OCTAVE_GAIN: [i64; 4] = [2_458, 4_096, 2_867, 1_638];
/// Fall-line stretch of the runnel pattern.
const STRETCH: i64 = 3;
/// Four fixed orientations (cos, sin) in Q12: 12, 57, 102 and 147 degrees.
const ORIENTATIONS: [(i64, i64); 4] =
    [(4_006, 852), (2_231, 3_435), (-852, 4_006), (-3_435, 2_231)];
/// (cos 2θ, sin 2θ) of the same orientations, Q12.
const DOUBLED: [(i64, i64); 4] = [
    (3_742, 1_666),
    (-1_666, 3_742),
    (-3_742, -1_666),
    (1_666, -3_742),
];
/// Sixteen unit gradients, offset from the axes by 11.25 degrees, Q12.
const GRADIENTS: [(i64, i64); 16] = [
    (4_017, 799),
    (3_406, 2_276),
    (2_276, 3_406),
    (799, 4_017),
    (-799, 4_017),
    (-2_276, 3_406),
    (-3_406, 2_276),
    (-4_017, 799),
    (-4_017, -799),
    (-3_406, -2_276),
    (-2_276, -3_406),
    (-799, -4_017),
    (799, -4_017),
    (2_276, -3_406),
    (3_406, -2_276),
    (4_017, -799),
];

fn smooth(e0: i64, e1: i64, x: i64) -> i64 {
    let t = ((x - e0) * ONE / (e1 - e0)).clamp(0, ONE);
    t * t / ONE * (3 * ONE - 2 * t) / ONE
}

fn gradient(ix: i64, iy: i64, salt: u64) -> (i64, i64) {
    let mut v = ix.cast_unsigned().wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ iy.cast_unsigned().wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ salt;
    v ^= v >> 31;
    v = v.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    v ^= v >> 29;
    GRADIENTS[usize::try_from(v >> 60).unwrap_or(0) & 15]
}

/// Gradient noise at lattice coordinates (Q12) with its analytic derivative
/// per lattice unit. Value and derivative are Q12, value roughly in ±0.7.
fn noise(u: i64, v: i64, salt: u64) -> (i64, i64, i64) {
    let (iu, iv) = (u.div_euclid(ONE), v.div_euclid(ONE));
    let (tu, tv) = (u.rem_euclid(ONE), v.rem_euclid(ONE));
    let fade = |t: i64| {
        let t3 = t * t / ONE * t / ONE;
        t3 * ((t * (6 * t - 15 * ONE)) / ONE + 10 * ONE) / ONE
    };
    let dfade = |t: i64| {
        let t2 = t * t / ONE;
        30 * t2 * ((t - ONE) * (t - ONE) / ONE) / ONE
    };
    let corner = |di: i64, dj: i64| {
        let g = gradient(iu + di, iv + dj, salt);
        let (du, dv) = (tu - di * ONE, tv - dj * ONE);
        (g.0 * du / ONE + g.1 * dv / ONE, g)
    };
    let (n00, g00) = corner(0, 0);
    let (n10, g10) = corner(1, 0);
    let (n01, g01) = corner(0, 1);
    let (n11, g11) = corner(1, 1);
    let (fu, fv) = (fade(tu), fade(tv));
    let (du, dv) = (dfade(tu), dfade(tv));
    let k = n00 - n10 - n01 + n11;
    let value = n00 + fu * (n10 - n00) / ONE + fv * (n01 - n00) / ONE + fu * fv / ONE * k / ONE;
    let axis = |a: i64, b: i64, c: i64, d: i64| {
        a + fu * (b - a) / ONE + fv * (c - a) / ONE + fu * fv / ONE * (a - b - c + d) / ONE
    };
    let gu = axis(g00.0, g10.0, g01.0, g11.0) + du * ((n10 - n00) + fv * k / ONE) / ONE;
    let gv = axis(g00.1, g10.1, g01.1, g11.1) + dv * ((n01 - n00) + fu * k / ONE) / ONE;
    (value, gu, gv)
}

/// One octave of runnel noise in a frame rotated to `(c, s)`: stretched
/// along the frame's first axis. Returns the world slope (Q12) of a relief
/// whose slope amplitude is one, plus the value.
fn runnels(x_dm: i64, y_dm: i64, wave_dm: i64, (c, s): (i64, i64), salt: u64) -> (i64, i64, i64) {
    let along = (x_dm * c + y_dm * s) / ONE;
    let across = (-x_dm * s + y_dm * c) / ONE;
    let (value, gu, gv) = noise(
        along * ONE / (wave_dm * STRETCH),
        across * ONE / wave_dm,
        salt,
    );
    // d/d(world) = d/d(lattice) / wavelength; height amplitude is one
    // wavelength / (2π), so slope amplitude ≈ derivative / (2π).
    let (su, sv) = (gu / STRETCH, gv);
    (
        (su * c - sv * s) * 652 / ONE / ONE,
        (su * s + sv * c) * 652 / ONE / ONE,
        value,
    )
}

/// Isotropic octave: the mean of two domains rotated by 31° and -47°.
fn isotropic(x_dm: i64, y_dm: i64, wave_dm: i64, salt: u64) -> (i64, i64) {
    let mut sum = (0, 0);
    for (c, s, k) in [(3_511_i64, 2_110_i64, 0_u64), (2_793, -2_996, 1)] {
        let u = (x_dm * c - y_dm * s) / ONE;
        let v = (x_dm * s + y_dm * c) / ONE;
        let (_, gu, gv) = noise(u * ONE / wave_dm, v * ONE / wave_dm, salt ^ (k << 40));
        // Rotate the lattice derivative back to world axes.
        sum.0 += (gu * c + gv * s) * 652 / ONE / ONE;
        sum.1 += (-gu * s + gv * c) * 652 / ONE / ONE;
    }
    (sum.0 * 2_896 / ONE, sum.1 * 2_896 / ONE)
}

/// Fade-in weight for a wavelength drawn at a pixel footprint: zero below
/// three pixels per wavelength, full from six (goal 33).
fn keep(wave_dm: i64, footprint_um: i128) -> i64 {
    let px = i64::try_from(footprint_um / 100_000)
        .unwrap_or(i64::MAX)
        .max(1);
    smooth(3 * ONE, 6 * ONE, wave_dm * ONE / px)
}

/// Sub-grid relief slope to add to the lighting gradient, Q12, and a trough
/// value (Q12, positive in runnel troughs) for ambient darkening.
///
/// `fall` is the smooth (312 m) gradient that orients runnels; `strength`
/// (Q12) sets the slope amplitude: faint on gentle turf, fluting on bare
/// rock faces.
pub(super) fn relief(
    position_dm: (i64, i64),
    footprint_um: i128,
    fall: (i64, i64),
    strength: i64,
) -> (i64, i64, i64) {
    if keep(OCTAVES_DM[0], footprint_um) == 0 {
        return (0, 0, 0);
    }
    let (x, y) = position_dm;
    let amplitude = strength.clamp(0, 3 * ONE);
    let rugged = smooth(1_229, 6_144, amplitude);
    let (fx, fy) = fall;
    let len2 = i128::from(fx) * i128::from(fx) + i128::from(fy) * i128::from(fy);
    // Runnels need a defined fall line; flats keep the isotropic term.
    // Only bare, rugged faces flute; turf stays mostly isotropic, so grass
    // never reads as combed hair.
    let directed = smooth(205, 820, i64::try_from(len2.isqrt()).unwrap_or(0))
        * (1_229 + 2_867 * rugged / ONE)
        / ONE;
    let (c2, s2) = if len2 > 0 {
        (
            i64::try_from(
                (i128::from(fx) * i128::from(fx) - i128::from(fy) * i128::from(fy))
                    * i128::from(ONE)
                    / len2,
            )
            .unwrap_or(0),
            i64::try_from(2 * i128::from(fx) * i128::from(fy) * i128::from(ONE) / len2)
                .unwrap_or(0),
        )
    } else {
        (ONE, 0)
    };
    let weights: [i64; 4] = std::array::from_fn(|k| {
        let w = ((c2 * DOUBLED[k].0 + s2 * DOUBLED[k].1) / ONE).max(0);
        w * w / ONE
    });
    // Two neighbours share weight near a bisector; restore their variance.
    let norm_sq = weights.iter().map(|w| w * w / ONE).sum::<i64>().max(1);
    let norm = i64::try_from((i128::from(norm_sq) * i128::from(ONE)).isqrt()).unwrap_or(ONE);
    let (mut gx, mut gy, mut trough) = (0, 0, 0);
    for (octave, (&wave, &gain)) in OCTAVES_DM.iter().zip(&OCTAVE_GAIN).enumerate() {
        let fade = keep(wave, footprint_um);
        if fade == 0 {
            continue;
        }
        let salt = 0x5EED_0000_u64 + u64::try_from(octave).unwrap_or(0) * 0x1_0000;
        let (mut ox, mut oy, mut ov) = (0, 0, 0);
        for k in 0..4 {
            if weights[k] == 0 {
                continue;
            }
            let (rx, ry, rv) = runnels(x, y, wave, ORIENTATIONS[k], salt + k as u64);
            ox += rx * weights[k] / norm;
            oy += ry * weights[k] / norm;
            ov += rv * weights[k] / norm;
        }
        let (ix, iy) = isotropic(x, y, wave, salt ^ 0xA5A5);
        let mix = |r: i64, i: i64| r * directed / ONE + i * (ONE - directed) / ONE;
        let scale = amplitude * gain / ONE * fade / ONE;
        gx += mix(ox, ix) * scale / ONE;
        gy += mix(oy, iy) * scale / ONE;
        trough -= ov * directed / ONE * rugged / ONE * fade / ONE / 2;
    }
    (gx, gy, trough)
}

/// Fine tonal texture (Q12, about ±1) at 40 m and 20 m for turf, canopy
/// and scree, fading in like the relief so it never aliases (goal 33).
pub(super) fn tone(position_dm: (i64, i64), footprint_um: i128) -> i64 {
    let (x, y) = position_dm;
    let mut sum = 0;
    for (wave, gain, salt) in [(400, 2_867, 0x70_4E01_u64), (200, 1_638, 0x70_4E02)] {
        let fade = keep(wave, footprint_um);
        if fade == 0 {
            continue;
        }
        let mut v = 0;
        for (c, s, k) in [(3_146_i64, 2_623_i64, 0_u64), (3_928, -1_161, 1)] {
            let u = (x * c - y * s) / ONE;
            let w = (x * s + y * c) / ONE;
            v += noise(u * ONE / wave, w * ONE / wave, salt ^ (k << 40)).0;
        }
        sum += v * gain / ONE * fade / ONE;
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_is_absent_until_its_wavelength_spans_three_pixels() {
        assert_eq!(
            relief((1_234, 5_678), 60_000_000, (2_000, 0), 8_000),
            (0, 0, 0)
        );
        let (gx, gy, _) = relief((1_234, 5_678), 6_250_000, (2_000, 0), 8_000);
        assert!(gx != 0 || gy != 0);
    }

    #[test]
    fn tone_fades_in_with_resolution() {
        assert_eq!(tone((5_000, 7_000), 20_000_000), 0);
        let v: i64 = (0..50)
            .map(|i| tone((i * 97, i * 61), 4_000_000).abs())
            .sum();
        assert!(v > 0);
    }

    #[test]
    fn noise_derivative_matches_finite_differences() {
        for (u, v) in [(1_000, 2_000), (9_999, -7_777), (-40_000, 12_345)] {
            let (_, gu, gv) = noise(u, v, 7);
            let h = 128;
            let du = (noise(u + h, v, 7).0 - noise(u - h, v, 7).0) * ONE / (2 * h);
            let dv = (noise(u, v + h, 7).0 - noise(u, v - h, 7).0) * ONE / (2 * h);
            assert!((gu - du).abs() < 200, "{gu} {du}");
            assert!((gv - dv).abs() < 200, "{gv} {dv}");
        }
    }

    #[test]
    fn bare_faces_carry_stronger_relief_than_gentle_turf() {
        let energy = |slope: i64| {
            let mut e = 0_i64;
            for i in 0..400 {
                let p = (i * 37 % 2_000, i * 91 % 2_000);
                let (gx, gy, _) = relief(p, 3_000_000, (3_000, 1_000), slope);
                e += gx.abs() + gy.abs();
            }
            e
        };
        assert!(energy(8_000) > 3 * energy(600));
    }

    #[test]
    fn runnels_vary_faster_across_the_fall_line_than_along_it() {
        // Fall line along +x: stepping across (y) changes the relief more.
        let (mut along, mut across) = (0_i64, 0_i64);
        for i in 0..300 {
            let p = (i * 53 % 3_000, i * 71 % 3_000);
            let at = |q: (i64, i64)| relief(q, 3_000_000, (3_000, 0), 8_000);
            let base = at(p);
            let a = at((p.0 + 40, p.1));
            let b = at((p.0, p.1 + 40));
            along += (a.0 - base.0).abs() + (a.1 - base.1).abs();
            across += (b.0 - base.0).abs() + (b.1 - base.1).abs();
        }
        assert!(across > 2 * along, "{across} {along}");
    }
}
