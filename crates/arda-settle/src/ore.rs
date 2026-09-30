//! The ore proxy (see the README, "Ore proxy").
//!
//! The world has no geology, so ore stands in for it in two steps. First,
//! mineral districts: one 8 km lattice block in seven holds a district, a
//! disc of 2.5 km radius around a seeded point in the block, which makes
//! deposits cluster the way real ore fields do. Second, inside a district
//! only rough, rocky ground carries ore: bare rock over a tenth of the
//! surrounding kilometre and a mean slope of 15° or more. Only the few
//! settlements nearest a district's ore work it ([`MINERS_PER_DISTRICT`]),
//! so mining stays a local trade.

use crate::rng::hash;

/// District lattice edge, cells (8 km).
pub const DISTRICT_LATTICE: i64 = 80;
/// One block in this many holds a district.
pub const DISTRICT_ONE_IN: u64 = 7;
/// District radius, cells (2.5 km).
pub const DISTRICT_R: i64 = 25;
/// Settlements that mine one district, nearest the ore first.
pub const MINERS_PER_DISTRICT: usize = 3;
/// Least bare-rock share of the surrounding kilometre, per mille.
pub const ROCK_PM: i64 = 100;
/// Least mean slope over the surrounding kilometre, millidegrees.
pub const SLOPE_MD: i64 = 15_000;

/// The district centre of lattice block `(bx, by)`, if it holds one.
#[must_use]
pub fn district_centre(seed: u64, bx: i64, by: i64) -> Option<(i64, i64)> {
    let h = hash(seed, "ore-district", bx.cast_unsigned(), by.cast_unsigned());
    if !h.is_multiple_of(DISTRICT_ONE_IN) {
        return None;
    }
    let off = |shift: u32| i64::try_from((h >> shift) % 80).unwrap_or(40);
    Some((
        bx * DISTRICT_LATTICE + off(16),
        by * DISTRICT_LATTICE + off(32),
    ))
}

/// The lattice block whose district holds cell `(x, y)`, if any (the
/// first in a fixed scan order when districts overlap).
#[must_use]
pub fn district_of(seed: u64, x: i64, y: i64) -> Option<(i64, i64)> {
    let (bx, by) = (
        x.div_euclid(DISTRICT_LATTICE),
        y.div_euclid(DISTRICT_LATTICE),
    );
    (-1..=1_i64)
        .flat_map(|oy| (-1..=1_i64).map(move |ox| (bx + ox, by + oy)))
        .find(|&(kx, ky)| {
            district_centre(seed, kx, ky).is_some_and(|(cx, cy)| {
                (cx - x).pow(2) + (cy - y).pow(2) <= DISTRICT_R * DISTRICT_R
            })
        })
}

/// Whether cell `(x, y)` lies in a mineral district.
#[must_use]
pub fn in_district(seed: u64, x: i64, y: i64) -> bool {
    district_of(seed, x, y).is_some()
}

/// Whether a cell in a district carries ore, given its surroundings.
#[must_use]
pub const fn deposit(rock_pm: i64, mean_slope_md: i64) -> bool {
    rock_pm >= ROCK_PM && mean_slope_md >= SLOPE_MD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn districts_are_rare_and_round() {
        let mut blocks = 0;
        for by in 0..40 {
            for bx in 0..40 {
                blocks += u32::from(district_centre(42, bx, by).is_some());
            }
        }
        assert!((150..=320).contains(&blocks), "{blocks} of 1600");
        let (bx, by) = (0..40)
            .flat_map(|y| (0..40).map(move |x| (x, y)))
            .find(|&(x, y)| district_centre(42, x, y).is_some())
            .unwrap();
        let (cx, cy) = district_centre(42, bx, by).unwrap();
        assert!(in_district(42, cx, cy) && in_district(42, cx + DISTRICT_R - 1, cy));
        assert!(deposit(150, 20_000) && !deposit(50, 20_000) && !deposit(150, 10_000));
    }
}
