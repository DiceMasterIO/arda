//! Integer geometry for river and lake shaping (logic/02 §world-water).
//!
//! Positions are in Q8 fine-cell units (1/256 of a lattice cell), angles in
//! Q16 turns (65,536 = one full turn), unit vectors and trigonometric values
//! in Q14. Everything is integer so the result is platform-independent.

use super::super::lattice::Lattice;

/// One fine cell in Q8 position units.
pub const CELL_Q8: i64 = 256;
/// One in Q14.
pub const ONE_Q14: i64 = 16_384;
/// A full turn in Q16 angle units.
pub const TURN: i64 = 65_536;

/// Sine of a Q16-turn angle, Q14 (7-term Taylor on the reduced quarter
/// turn; error below 3 × 2^-14).
#[must_use]
pub fn sin_q14(angle: i64) -> i64 {
    let a = angle.rem_euclid(TURN);
    let (quarter, sign) = match a / (TURN / 4) {
        0 => (a, 1),
        1 => (TURN / 2 - a, 1),
        2 => (a - TURN / 2, -1),
        _ => (TURN - a, -1),
    };
    // r = quarter × 2π / TURN in Q30.
    let r = i128::from(quarter) * 6_746_518_852 / i128::from(TURN);
    let one = 1_i128 << 30;
    let r2 = r * r / one;
    // sin r = r (1 - r²/6 (1 - r²/20 (1 - r²/42))).
    let mut s = one - r2 / 42;
    s = one - r2 * s / one / 20;
    s = one - r2 * s / one / 6;
    let v = r * s / one;
    sign * i64::try_from(v >> 16).unwrap_or(0)
}

/// Cosine of a Q16-turn angle, Q14.
#[must_use]
pub fn cos_q14(angle: i64) -> i64 {
    sin_q14(angle + TURN / 4)
}

/// Integer square root of a non-negative `i128`, as `i64` (saturating).
#[must_use]
pub fn isqrt_i(v: i128) -> i64 {
    i64::try_from(v.max(0).unsigned_abs().isqrt()).unwrap_or(i64::MAX)
}

/// Unit vector (Q14) along `(dx, dy)`, or `(ONE_Q14, 0)` for a zero vector.
#[must_use]
pub fn unit(dx: i64, dy: i64) -> (i64, i64) {
    let len = isqrt_i(i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy));
    if len == 0 {
        return (ONE_Q14, 0);
    }
    (dx * ONE_Q14 / len, dy * ONE_Q14 / len)
}

/// Rotates a Q14 unit vector by a Q16-turn angle.
#[must_use]
pub fn rotate(v: (i64, i64), angle: i64) -> (i64, i64) {
    let (s, c) = (sin_q14(angle), cos_q14(angle));
    ((v.0 * c - v.1 * s) / ONE_Q14, (v.0 * s + v.1 * c) / ONE_Q14)
}

/// Lattice height at the node nearest a Q8 position, clamped to the grid.
#[must_use]
pub fn height_at(g: &Lattice, p: (i64, i64)) -> i32 {
    let x = ((p.0 + CELL_Q8 / 2) / CELL_Q8).clamp(0, g.width as i64 - 1);
    let y = ((p.1 + CELL_Q8 / 2) / CELL_Q8).clamp(0, g.height as i64 - 1);
    g.z[(y as usize) * g.width + x as usize]
}

/// Q8 centre of lattice node `i`.
#[must_use]
pub fn node_q8(i: usize, width: usize) -> (i64, i64) {
    ((i % width) as i64 * CELL_Q8, (i / width) as i64 * CELL_Q8)
}

/// Absolute micrometres of a Q8 position on `g`.
#[must_use]
pub fn q8_to_um(g: &Lattice, p: (i64, i64)) -> (i64, i64) {
    (p.0 * g.spacing_um / CELL_Q8, p.1 * g.spacing_um / CELL_Q8)
}

/// Q8 units per metre-denominated length on `g`: `m` metres as Q8.
#[must_use]
pub fn m_to_q8(g: &Lattice, m: i64) -> i64 {
    m * 1_000_000 * CELL_Q8 / g.spacing_um.max(1)
}

/// Lowers every interior land node within `radius` (Q8) of `centre` to at
/// most `bed + rise × (d / radius)²`, a parabolic channel cross-section
/// meeting the surrounding surface at `bed + rise`. Only lowers.
pub fn carve_disc(g: &mut Lattice, centre: (i64, i64), radius: i64, bed: i64, rise: i64) {
    let (w, h) = (g.width as i64, g.height as i64);
    let r = radius.max(1);
    let (x0, x1) = ((centre.0 - r) / CELL_Q8, (centre.0 + r) / CELL_Q8 + 1);
    let (y0, y1) = ((centre.1 - r) / CELL_Q8, (centre.1 + r) / CELL_Q8 + 1);
    for y in y0.max(1)..=y1.min(h - 2) {
        for x in x0.max(1)..=x1.min(w - 2) {
            let (dx, dy) = (x * CELL_Q8 - centre.0, y * CELL_Q8 - centre.1);
            let d2 = dx * dx + dy * dy;
            if d2 > r * r {
                continue;
            }
            let j = (y as usize) * g.width + x as usize;
            if g.z[j] <= 0 {
                continue;
            }
            let target = bed + rise * d2 / (r * r);
            if i64::from(g.z[j]) > target {
                g.z[j] = i32::try_from(target.max(1)).unwrap_or(g.z[j]);
            }
        }
    }
}

/// Deterministic 64-bit mix of a seed and two integers.
#[must_use]
pub fn hash3(seed: u64, a: i64, b: i64) -> u64 {
    let mut h = seed ^ 0x9E37_79B9_7F4A_7C15;
    for v in [a as u64, b as u64] {
        h ^= v.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h = (h ^ (h >> 31)).wrapping_mul(0x94D0_49BB_1331_11EB);
        h ^= h >> 29;
    }
    h
}

/// Uniform value in `0..n` from a hash.
#[must_use]
pub fn pick(h: u64, n: u64) -> u64 {
    ((u128::from(h) * u128::from(n.max(1))) >> 64) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_trig_matches_the_unit_circle() {
        assert_eq!(sin_q14(0), 0);
        assert!((sin_q14(TURN / 4) - ONE_Q14).abs() <= 3);
        assert!((cos_q14(TURN / 2) + ONE_Q14).abs() <= 3);
        // sin 30° = 0.5.
        assert!((sin_q14(TURN / 12) - ONE_Q14 / 2).abs() <= 3);
        for a in (0..TURN).step_by(997) {
            let (s, c) = (sin_q14(a), cos_q14(a));
            let r = s * s + c * c;
            assert!((r - ONE_Q14 * ONE_Q14).abs() < ONE_Q14 * 8, "{a}: {r}");
        }
        let v = rotate((ONE_Q14, 0), TURN / 4);
        assert!(v.0.abs() <= 3 && (v.1 - ONE_Q14).abs() <= 3);
    }

    #[test]
    fn a_carved_disc_is_a_parabolic_channel_and_only_lowers() {
        let mut g = Lattice::new(21, 21, 39_062_500).unwrap();
        g.z.fill(10_000);
        carve_disc(
            &mut g,
            (10 * CELL_Q8, 10 * CELL_Q8),
            4 * CELL_Q8,
            8_000,
            2_000,
        );
        assert_eq!(g.z[10 * 21 + 10], 8_000);
        assert_eq!(g.z[10 * 21 + 12], 8_500);
        assert_eq!(g.z[10 * 21 + 16], 10_000);
        let before = g.z.clone();
        carve_disc(&mut g, (10 * CELL_Q8, 10 * CELL_Q8), 4 * CELL_Q8, 12_000, 0);
        assert_eq!(g.z, before, "carving never raises");
    }
}
