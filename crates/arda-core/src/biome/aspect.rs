//! Slope aspect: the stored encoding and what it means for sun and shade.
//!
//! `Cell::aspect_deg` stores the downslope compass bearing. Up to recipe 8
//! it is one of eight directions and `0` is both "flat" and "north-facing"
//! (review 3.2-7). From recipe 9 it is a whole-degree bearing from the
//! cell's continuous gradient, `0` means flat and only flat, and a
//! north-facing slope stores `360`, the aviation convention for north.
//! [`bearing`] reads both encodings; legacy `0` reads as flat.

use super::latitude::{hemisphere, hemisphere_q12};
use super::ONE;

/// Stored aspect of flat ground.
pub const FLAT: u16 = 0;
/// Stored aspect of a north-facing slope (recipe 9 and later).
pub const NORTH: u16 = 360;

/// `cos` of whole degrees `0..=90`, Q12.
const COS_Q12: [i64; 91] = [
    4096, 4095, 4094, 4090, 4086, 4080, 4074, 4065, 4056, 4046, 4034, 4021, 4006, 3991, 3974, 3956,
    3937, 3917, 3896, 3873, 3849, 3824, 3798, 3770, 3742, 3712, 3681, 3650, 3617, 3582, 3547, 3511,
    3474, 3435, 3396, 3355, 3314, 3271, 3228, 3183, 3138, 3091, 3044, 2996, 2946, 2896, 2845, 2793,
    2741, 2687, 2633, 2578, 2522, 2465, 2408, 2349, 2290, 2231, 2171, 2110, 2048, 1986, 1923, 1860,
    1796, 1731, 1666, 1600, 1534, 1468, 1401, 1334, 1266, 1198, 1129, 1060, 991, 921, 852, 782,
    711, 641, 570, 499, 428, 357, 286, 214, 143, 71, 0,
];

/// `atan(i / 64)` for `i = 0..=64`, milli-degrees.
const ATAN_MDEG_64: [i64; 65] = [
    0, 895, 1790, 2684, 3576, 4467, 5356, 6242, 7125, 8005, 8881, 9752, 10620, 11482, 12339, 13191,
    14036, 14876, 15709, 16535, 17354, 18166, 18970, 19767, 20556, 21337, 22109, 22874, 23629,
    24376, 25115, 25844, 26565, 27277, 27979, 28673, 29358, 30033, 30700, 31357, 32005, 32645,
    33275, 33896, 34509, 35112, 35707, 36293, 36870, 37439, 37999, 38550, 39094, 39629, 40156,
    40675, 41186, 41689, 42184, 42672, 43152, 43625, 44091, 44549, 45000,
];

/// Encodes a compass bearing (any whole degree) as a stored recipe-9 aspect:
/// `1..=360`, north as `360`.
#[must_use]
pub const fn encode(bearing_deg: u16) -> u16 {
    let b = bearing_deg % 360;
    if b == 0 {
        NORTH
    } else {
        b
    }
}

/// The compass bearing `0..=359` of a stored aspect, or `None` on flat
/// ground. Reads both encodings: `360` is north, `0` is flat.
#[must_use]
pub const fn bearing(aspect: u16) -> Option<u16> {
    if aspect == FLAT {
        None
    } else {
        Some(aspect % 360)
    }
}

/// `cos` of a whole-degree angle, Q12.
#[must_use]
pub fn cos_q12(deg: i64) -> i64 {
    let d = deg.rem_euclid(360);
    let (folded, sign) = match d {
        0..=90 => (d, 1),
        91..=180 => (180 - d, -1),
        181..=270 => (d - 180, -1),
        _ => (360 - d, 1),
    };
    sign * COS_Q12[usize::try_from(folded).unwrap_or(0)]
}

/// The whole-degree compass bearing `0..=359` (clockwise from north) of the
/// horizontal vector (`east`, `south`), or `None` for the zero vector. An
/// integer `atan2` (table with linear interpolation, within 0.1°), rounded
/// to the nearest degree.
#[must_use]
pub fn compass_bearing(east: i64, south: i64) -> Option<u16> {
    if east == 0 && south == 0 {
        return None;
    }
    // Angle from north, clockwise: north component is -south.
    let (n, e) = (-i128::from(south), i128::from(east));
    let (an, ae) = (n.abs(), e.abs());
    // Angle within the first octant pair, from the larger axis.
    let (small, big, from_north) = if ae <= an {
        (ae, an, true)
    } else {
        (an, ae, false)
    };
    let scaled = small * 64 * 1_000 / big; // 0..=64_000
    let i = usize::try_from(scaled / 1_000).unwrap_or(64).min(64);
    let frac = i64::try_from(scaled % 1_000).unwrap_or(0);
    let lo = ATAN_MDEG_64[i];
    let hi = ATAN_MDEG_64[(i + 1).min(64)];
    let a = lo + (hi - lo) * frac / 1_000;
    // Within 0..=90 measured clockwise from north in the NE quadrant.
    let q = if from_north { a } else { 90_000 - a };
    let mdeg = match (n >= 0, e >= 0) {
        (true, true) => q,
        (false, true) => 180_000 - q,
        (false, false) => 180_000 + q,
        (true, false) => 360_000 - q,
    };
    u16::try_from(((mdeg + 500) / 1_000).rem_euclid(360)).ok()
}

/// How much a stored aspect faces north, Q12: `cos(bearing)`, `0` on flat
/// ground.
#[must_use]
pub fn northness_q12(aspect: u16) -> i64 {
    bearing(aspect).map_or(0, |b| cos_q12(i64::from(b)))
}

/// How much a stored aspect faces the sun's side at latitude `lat_mdeg`,
/// Q12: south-facing in the northern hemisphere, north-facing in the
/// southern, fading to zero at the equator; `0` on flat ground.
#[must_use]
pub fn sunward_q12(aspect: u16, lat_mdeg: i32) -> i64 {
    -northness_q12(aspect) * hemisphere_q12(lat_mdeg) / ONE
}

/// The `f64` form of [`sunward_q12`] from a continuous northness
/// (`-1..=1`, the slope's downhill direction's north component).
#[must_use]
pub fn sunward(north: f64, lat_deg: f64) -> f64 {
    -north * hemisphere(lat_deg)
}

/// Whether a slope faces the sun's side within 60° (orchards, vineyards).
#[must_use]
pub fn sunny(aspect: u16, lat_mdeg: i32) -> bool {
    sunward_q12(aspect, lat_mdeg) >= ONE / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn north_and_flat_are_distinct() {
        assert_eq!(encode(0), NORTH);
        assert_eq!(encode(360), NORTH);
        assert_eq!(encode(90), 90);
        assert_eq!(bearing(NORTH), Some(0));
        assert_eq!(bearing(FLAT), None);
        assert_eq!(northness_q12(NORTH), ONE);
        assert_eq!(northness_q12(FLAT), 0);
        assert_eq!(northness_q12(180), -ONE);
    }

    #[test]
    fn bearings_follow_the_compass_to_the_degree() {
        assert_eq!(compass_bearing(0, -5), Some(0));
        assert_eq!(compass_bearing(5, 0), Some(90));
        assert_eq!(compass_bearing(0, 5), Some(180));
        assert_eq!(compass_bearing(-5, 0), Some(270));
        assert_eq!(compass_bearing(3, -3), Some(45));
        assert_eq!(compass_bearing(0, 0), None);
        // Every whole degree round-trips through a long vector.
        for deg in 0..360_i64 {
            let r = 100_000.0;
            // Built from the table so the test needs no libm either.
            let (e, n) = (
                cos_q12(90 - deg) as f64 / 4_096.0,
                cos_q12(deg) as f64 / 4_096.0,
            );
            let got = compass_bearing((e * r) as i64, (-n * r) as i64).unwrap();
            let diff = (i64::from(got) - deg).rem_euclid(360);
            assert!(diff <= 1 || diff == 359, "{deg}: {got}");
        }
    }

    #[test]
    fn the_sunny_side_depends_on_the_hemisphere() {
        // South-facing is sunny in the north, north-facing in the south.
        assert!(sunny(180, 45_000) && !sunny(NORTH, 45_000));
        assert!(sunny(NORTH, -45_000) && !sunny(180, -45_000));
        assert!(!sunny(FLAT, 45_000) && !sunny(FLAT, -45_000));
        // The northern window matches the old 120..=240 rule.
        for a in 1..=360 {
            assert_eq!(sunny(a, 45_000), (120..=240).contains(&a), "{a}");
        }
        assert!((sunward(-1.0, 45.0) - 1.0).abs() < 1e-12);
        assert!((sunward(-1.0, -45.0) + 1.0).abs() < 1e-12);
    }
}
