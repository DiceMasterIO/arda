//! Open-field strips (`04-settlements-roads.md` step 4): each furlong near
//! a village is cut into long strips a few rods wide, ploughed lengthwise,
//! with a grass balk between neighbours.
//!
//! Strips run straight along the square lattice or one of its diagonals
//! (the furlong's own bearing snapped to the nearest 45°), so a strip's
//! edge is a straight line of squares on the tactical map rather than a
//! staircase; the reverse-S of the plough team is left out for the same
//! reason (review 2026-10-15, gallery 16).

use crate::fields::Crop;
use crate::geom::{h2, h3, u01};
use crate::partition::Site;

/// Share of a furlong's strips that carry its own crop; the rest take
/// their own, so a furlong reads as strips rather than one block.
const FURLONG_CROP: f64 = 0.5;

/// Where a square falls in its furlong.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripAt {
    /// Strip index across the furlong.
    pub index: i64,
    /// Whether the square is the grass balk on the strip's edge (one row of
    /// squares).
    pub balk: bool,
    /// Unit vector along the strip (the furrow direction).
    pub along: [f64; 2],
    /// Strip width in squares.
    pub width: f64,
}

/// The furlong's strip direction and width.
#[must_use]
pub fn furlong(seed: u64, site: &Site) -> ([f64; 2], f64) {
    let (k0, k1) = site.hkey();
    let h = h2(seed, 0x57A1, k0, k1);
    // The partition already turns neighbouring furlongs to their own
    // directions (along the furlong, across it, or down the slope).
    #[allow(clippy::cast_precision_loss)] // 0..5
    let width = 7.0 + (h >> 20) as f64 % 5.0;
    (lattice_bearing(site.along), width.floor())
}

/// The unit vector of the eight lattice bearings (every 45°) nearest to
/// `v`.
#[must_use]
pub fn lattice_bearing(v: [f64; 2]) -> [f64; 2] {
    let a = arda_ids::pmath::atan2(v[1], v[0]);
    let eighth = std::f64::consts::FRAC_PI_4;
    #[allow(clippy::cast_possible_truncation)] // within -4..=4
    let k = (a / eighth).round() as i64;
    let d = std::f64::consts::FRAC_1_SQRT_2;
    match k.rem_euclid(8) {
        0 => [1.0, 0.0],
        1 => [d, d],
        2 => [0.0, 1.0],
        3 => [-d, d],
        4 => [-1.0, 0.0],
        5 => [-d, -d],
        6 => [0.0, -1.0],
        _ => [d, -d],
    }
}

/// Locates a square centre `p` within the furlong of `site`.
#[must_use]
pub fn locate(seed: u64, site: &Site, p: [f64; 2]) -> StripAt {
    let (along, width) = furlong(seed, site);
    let d = [p[0] - site.p[0], p[1] - site.p[1]];
    let v = -d[0] * along[1] + d[1] * along[0];
    let (k0, k1) = site.hkey();
    let off = u01(h2(seed, 0x57A2, k0, k1)) * width;
    let q = v + off;
    #[allow(clippy::cast_possible_truncation)] // strip counts are small
    let index = (q / width).floor() as i64;
    let rem = q - (index as f64) * width;
    // One row of squares: the lattice's spacing across the strips (a
    // square on the lattice, half a diagonal on its diagonals).
    let row = if along[0] == 0.0 || along[1] == 0.0 {
        1.0
    } else {
        std::f64::consts::FRAC_1_SQRT_2
    };
    StripAt {
        index,
        balk: rem < row,
        along,
        width,
    }
}

/// The crop of one strip: mostly the furlong's, sometimes its own.
#[must_use]
pub fn crop(seed: u64, site: &Site, furlong_crop: Crop, index: i64) -> Crop {
    let (k0, k1) = site.hkey();
    let h = h3(seed, 0x57A3, k0, k1, index);
    if u01(h) < FURLONG_CROP {
        furlong_crop
    } else {
        Crop::arable(u01(h >> 17))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fields::FieldKind;

    fn site(along: [f64; 2]) -> Site {
        Site {
            key: (3, 4, 5),
            p: [1000.3, 2000.7],
            class: None,
            along,
            slope: 0.0,
            kind: FieldKind::Strips,
            crop: Crop::Ploughed,
            area: 40_000.0,
        }
    }

    #[test]
    fn strips_run_straight_along_the_square_lattice_or_its_diagonals() {
        // review 2026-10-15 (gallery 16): strips at any bearing, curved in
        // a reverse S, stepped along every square and the tactical render
        // drew their edges as 45-degree stair-steps. On the lattice or its
        // diagonals a strip's edge is a straight line of squares.
        for deg in [0.0_f64, 7.0, 21.0, 33.0, 52.0, 80.0, 101.0, 167.0, 230.0] {
            let a = deg.to_radians();
            let s = site([a.cos(), a.sin()]);
            let (along, _) = furlong(9, &s);
            let on_lattice = |v: f64| {
                [0.0, 1.0, std::f64::consts::FRAC_1_SQRT_2]
                    .iter()
                    .any(|k| (v.abs() - k).abs() < 1e-9)
            };
            assert!(
                on_lattice(along[0]) && on_lattice(along[1]),
                "{deg}: {along:?}"
            );
            // A strip keeps its index all along its length.
            for k in 0..40 {
                let p = [1000.5 + f64::from(k) * 0.37, 2000.5 - f64::from(k) * 0.11];
                let i = locate(9, &s, p).index;
                for step in [-90.0, -30.0, 30.0, 90.0] {
                    let q = [p[0] + along[0] * step, p[1] + along[1] * step];
                    assert_eq!(locate(9, &s, q).index, i, "{deg}: bends at {step}");
                }
            }
        }
    }
}
