//! Colours of worked land (logic/17 §land-tones). Land-use tones are
//! multiplicative tints against the shader's open-grassland colour, so the
//! relief light, sky and climate the formed shader put into a pixel stay
//! in it; built surfaces (roads, roofs, streets) are mixed in as paint.

use crate::fixed::{hash2, organic_q12, smooth, ONE};

/// The formed shader's mean colour of open lowland grass (measured over
/// full-size seed 42 plains at 15 and 30 m/px): the reference every tint
/// is relative to.
pub const REFERENCE: [i64; 3] = [111, 119, 62];

/// What a piece of worked land shows from above.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tone {
    /// Freshly ploughed ground.
    Ploughed,
    /// Cut cereal stubble.
    Stubble,
    /// Resting ground gone to weeds.
    Fallow,
    /// Standing hay.
    Hay,
    /// A mown hay meadow.
    Mown,
    /// Grazed turf.
    Grazed,
    /// Fruit trees in grass.
    Orchard,
    /// Managed woodland.
    Woodland,
    /// Wet floodplain meadow (unworked valley floors).
    Floodplain,
}

impl Tone {
    /// Target colour over the reference grassland.
    #[must_use]
    pub const fn target(self) -> [i64; 3] {
        match self {
            Self::Ploughed => [120, 108, 74],
            Self::Stubble => [136, 132, 86],
            Self::Fallow => [120, 124, 74],
            Self::Hay => [116, 132, 66],
            Self::Mown => [134, 142, 80],
            Self::Grazed => [104, 126, 62],
            Self::Orchard => [94, 114, 58],
            Self::Woodland => [72, 92, 48],
            Self::Floodplain => [98, 126, 66],
        }
    }

    /// The Q12 per-channel multiplier taking the reference to the target.
    #[must_use]
    pub fn multiplier(self) -> [i64; 3] {
        let t = self.target();
        std::array::from_fn(|k| t[k] * ONE / REFERENCE[k])
    }
}

/// Hedgerow canopy, seen from above.
pub const HEDGE: [i64; 3] = [62, 80, 40];
/// Drystone wall.
pub const DRYSTONE: [i64; 3] = [150, 146, 130];
/// Strip balk (a grass ridge between ploughed strips).
pub const BALK: [i64; 3] = [118, 128, 66];

/// `rgb` scaled per channel by Q12 multipliers.
#[must_use]
pub fn tint(rgb: [u8; 3], mul_q12: [i64; 3]) -> [u8; 3] {
    std::array::from_fn(|k| clamp_u8(i64::from(rgb[k]) * mul_q12[k] / ONE))
}

/// `rgb` blended toward `target` by `alpha` (Q12).
#[must_use]
pub fn mix(rgb: [u8; 3], target: [i64; 3], alpha_q12: i64) -> [u8; 3] {
    let a = alpha_q12.clamp(0, ONE);
    std::array::from_fn(|k| {
        let c = i64::from(rgb[k]);
        clamp_u8(c + (target[k] - c) * a / ONE)
    })
}

/// Multipliers blended toward one by `1 − strength` (Q12).
#[must_use]
pub fn soften(mul_q12: [i64; 3], strength_q12: i64) -> [i64; 3] {
    let s = strength_q12.clamp(0, ONE);
    mul_q12.map(|m| ONE + (m - ONE) * s / ONE)
}

/// Brightness and warmth jitter of one parcel (Q12 multipliers within
/// about ±7 % and ±3 %), so neighbouring fields of one use still differ.
#[must_use]
pub fn parcel_jitter(seed: u64, key: (i64, i64)) -> [i64; 3] {
    let h = hash2(seed, 0x7061_7263, key.0, key.1);
    let b = i64::try_from((h >> 40) & 0xFFF).unwrap_or(0) - 2_048; // ±2048
    let w = i64::try_from((h >> 20) & 0xFFF).unwrap_or(0) - 2_048;
    let lum = ONE + b * 170 / 2_048;
    let warm = w * 80 / 2_048;
    [lum + warm, lum, lum - warm]
}

/// Multiplies two Q12 multiplier triples.
#[must_use]
pub fn compose(a: [i64; 3], b: [i64; 3]) -> [i64; 3] {
    std::array::from_fn(|k| a[k] * b[k] / ONE)
}

/// Forest canopy texture (logic/17 §land canopy) at fine-lattice
/// micrometres: clumps and gaps at 40 and 18 m while pixels are coarse,
/// individual crowns lit from the north-west once pixels are 6 m or finer.
/// `weight` (Q12) is how much canopy covers the pixel. Returns a Q12
/// brightness multiplier.
#[must_use]
pub fn canopy(seed: u64, lx: i64, ly: i64, pixel_um: i64, weight_q12: i64) -> i64 {
    if weight_q12 <= 0 {
        return ONE;
    }
    // Clumps: rotated noise, fading before it aliases.
    let clump_keep = ONE - smooth(14_000_000, 30_000_000, pixel_um);
    let clump = (organic_q12(seed, 0x636c_7570, lx, ly, 40_000_000) * 2
        + organic_q12(seed, 0x636c_7571, lx, ly, 18_000_000))
        / 3
        - ONE / 2;
    let mut m = clump / 5 * clump_keep / ONE;
    // Crowns: one tree per jittered 6.5 m lattice cell.
    let crown_keep = ONE - smooth(2_500_000, 6_000_000, pixel_um);
    if crown_keep > 0 {
        m += crown(seed, lx, ly) * crown_keep / ONE;
    }
    ONE + m * weight_q12 / ONE
}

/// Crown spacing, micrometres.
const CROWN_UM: i64 = 6_500_000;

/// Brightness offset (Q12, about ±0.3) of the nearest crown at a point:
/// lit on its north-west flank, shadowed on the south-east and in gaps.
fn crown(seed: u64, x: i64, y: i64) -> i64 {
    let (ci, cj) = (x.div_euclid(CROWN_UM), y.div_euclid(CROWN_UM));
    let mut best: Option<(i64, i64, i64, i64)> = None;
    for dj in -1..=1 {
        for di in -1..=1 {
            let (i, j) = (ci + di, cj + dj);
            let h = hash2(seed, 0x7472_6565, i, j);
            let jx = i64::try_from(h & 0xFFFF).unwrap_or(0) * CROWN_UM / 0x1_0000;
            let jy = i64::try_from((h >> 16) & 0xFFFF).unwrap_or(0) * CROWN_UM / 0x1_0000;
            let r = CROWN_UM * (40 + i64::try_from((h >> 32) & 31).unwrap_or(0)) / 100;
            let (dx, dy) = (x - (i * CROWN_UM + jx), y - (j * CROWN_UM + jy));
            let d2 = (dx / 1_000) * (dx / 1_000) + (dy / 1_000) * (dy / 1_000);
            let r2 = (r / 1_000) * (r / 1_000);
            // The crown whose edge is furthest outside the point wins.
            let inside = r2 - d2;
            if best.is_none_or(|b| inside * b.3 > b.2 * r2) {
                best = Some((dx, dy, inside, r2));
            }
        }
    }
    let Some((dx, dy, inside, r2)) = best else {
        return 0;
    };
    if inside <= 0 {
        return -ONE * 28 / 100;
    }
    // Hemisphere: light from the north-west (dx, dy negative is lit).
    let r = crate::fixed::isqrt(r2).max(1);
    let lit = -(dx / 1_000 + dy / 1_000) * ONE / (2 * r);
    let edge = ONE - inside * ONE / r2;
    lit * 22 / 100 - edge * 10 / 100
}

fn clamp_u8(v: i64) -> u8 {
    u8::try_from(v.clamp(0, 255)).unwrap_or(255)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tints_take_the_reference_to_their_target() {
        for tone in [Tone::Ploughed, Tone::Grazed, Tone::Woodland] {
            let r = REFERENCE.map(|c| u8::try_from(c).unwrap());
            let out = tint(r, tone.multiplier());
            for (o, t) in out.iter().zip(tone.target()) {
                assert!((i64::from(*o) - t).abs() <= 1, "{tone:?}");
            }
        }
    }

    #[test]
    fn canopy_is_neutral_without_trees_and_bounded_with_them() {
        assert_eq!(canopy(1, 5, 5, 1_500_000, 0), ONE);
        for i in 0..200 {
            let m = canopy(1, i * 777_777, i * 333_331, 1_500_000, ONE);
            assert!((ONE * 6 / 10..=ONE * 14 / 10).contains(&m), "{m}");
        }
    }
}
