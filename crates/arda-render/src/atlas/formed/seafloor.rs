//! Recipe-9 sea colour (logic/04 §atlas-formed sea): the depth ramp over a
//! smoothed depth, and seafloor relief that fades out with depth.
//!
//! The saved seabed carries carved submarine channels and flat-floored pits
//! a few hundred metres wide. Drawn raw they show twice: the 1 km relief
//! gradients alias the narrow channels into arcs and ripples across the open
//! sea, and the ramp turns a 60 m pit in a 20 m shelf into a dark round blob
//! with stair-stepped walls. Here the ramp reads a smoothed depth beyond the
//! coastal band, so the waterline keeps its bright edge, and relief light fades
//! to nothing in deep water, where a few metres of channel carry no colour.

use super::{mix, rgb, sea, smooth, soft_q12, ONE};
use super::{relief_term, shore_share};

/// Depth (mm) where the smoothed depth starts to replace the raw one.
const SMOOTH_FROM_MM: i64 = 500;
/// Depth (mm) where it fully has.
const SMOOTH_FULL_MM: i64 = 12_000;
/// Depth (mm) beyond which smoothing is not needed (ramp stops are far apart).
const SMOOTH_DEEP_MM: i64 = 150_000;
/// Depth (mm) up to which seafloor relief light is at full strength.
const RELIEF_FULL_MM: i64 = 30_000;
/// Depth (mm) from which there is none.
const RELIEF_NONE_MM: i64 = 200_000;

/// Q12 weight of the smoothed depth in the ramp at raw `depth_mm`.
pub(in crate::atlas) fn smoothing_weight(depth_mm: i64) -> i64 {
    smooth(SMOOTH_FROM_MM, SMOOTH_FULL_MM, depth_mm)
        * (ONE - smooth(SMOOTH_DEEP_MM / 2, SMOOTH_DEEP_MM, depth_mm))
        / ONE
}

/// Whether the smoothed depth can matter at this raw depth, so callers
/// skip taking it elsewhere.
pub(in crate::atlas) fn wants_smoothing(depth_mm: i64) -> bool {
    depth_mm > SMOOTH_FROM_MM && depth_mm < SMOOTH_DEEP_MM
}

/// Q12 strength of seafloor relief light at `depth_mm`.
pub(in crate::atlas) fn relief_weight(depth_mm: i64) -> i64 {
    ONE - smooth(RELIEF_FULL_MM, RELIEF_NONE_MM, depth_mm)
}

/// Whether relief light has any strength at this depth.
pub(in crate::atlas) fn wants_relief(depth_mm: i64) -> bool {
    depth_mm < RELIEF_NONE_MM
}

/// Seafloor colour: the ramp at a depth blended from the raw toward
/// `smoothed_mm` away from the coast, lit by the broad gradients `broad`
/// at the strength the depth allows.
pub(in crate::atlas) fn sea_soft(depth_mm: i64, smoothed_mm: i64, broad: (i64, i64)) -> [u8; 3] {
    let raw = depth_mm.max(0);
    let w = smoothing_weight(raw);
    let depth = raw + (smoothed_mm.max(0) - raw) * w / ONE;
    let flat = sea(depth, (0, 0));
    let weight = relief_weight(raw);
    if weight == 0 {
        return flat;
    }
    let l = soft_q12(relief_term(broad.0, broad.1, 2_560) * 2);
    let k = ONE + 614 * l / ONE * weight / ONE;
    std::array::from_fn(|ch| {
        u8::try_from((i64::from(flat[ch]) * k / ONE).clamp(0, 255)).unwrap_or(255)
    })
}

/// Stored shore classes on water, feathered: tidal flats tint water
/// shallower than 0.5-4 m toward wet sand-grey, estuaries two-thirds toward
/// turbid teal. The class share is the plain bilinear weight, so the tint
/// ramps over a cell instead of stepping where the 100 m grid does.
pub(in crate::atlas) fn shore_water_soft(c: [u8; 3], depth_mm: i64, w: &[i64; 8]) -> [u8; 3] {
    let base = c.map(i64::from);
    let shallow = ONE - smooth(500, 4_000, depth_mm);
    let flat = mix(base, rgb(0xa6_a283), shore_share(w[6]) * shallow / ONE);
    let out = mix(flat, rgb(0x4f_8c86), shore_share(w[7]) * 2 / 3);
    out.map(|v| u8::try_from(v.clamp(0, 255)).unwrap_or(255))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spread(a: [u8; 3], b: [u8; 3]) -> i32 {
        (0..3)
            .map(|c| (i32::from(a[c]) - i32::from(b[c])).abs())
            .sum()
    }

    #[test]
    fn open_sea_relief_is_gone_so_channels_leave_no_arcs() {
        for depth in [250_000, 500_000, 2_000_000] {
            let flat = sea_soft(depth, depth, (0, 0));
            for broad in [(3_000, -2_500), (-4_000, 1_000), (900, 900)] {
                assert_eq!(sea_soft(depth, depth, broad), flat, "depth {depth}");
            }
        }
    }

    #[test]
    fn shelf_relief_still_reads() {
        let flat = sea_soft(10_000, 10_000, (0, 0));
        assert!(spread(sea_soft(10_000, 10_000, (3_000, -2_500)), flat) >= 6);
        assert!(spread(sea_soft(10_000, 10_000, (-3_000, 2_500)), flat) >= 6);
    }

    #[test]
    fn a_deep_pit_in_a_shelf_takes_the_shelf_colour_not_a_dark_blob() {
        let shelf = sea_soft(20_000, 20_000, (0, 0));
        let pit_raw = sea(61_000, (0, 0));
        let pit = sea_soft(61_000, 22_000, (0, 0));
        assert!(spread(pit, shelf) * 3 < spread(pit_raw, shelf), "{pit:?}");
    }

    #[test]
    fn the_coastal_strip_keeps_its_raw_depth() {
        assert_eq!(smoothing_weight(0), 0);
        assert_eq!(smoothing_weight(500), 0);
        assert_eq!(sea_soft(300, 40_000, (0, 0)), sea(300, (0, 0)));
    }

    #[test]
    fn weights_are_monotone_and_bounded() {
        let mut last = ONE;
        for d in (0..400_000).step_by(1_000) {
            let r = relief_weight(d);
            assert!((0..=ONE).contains(&r) && r <= last);
            last = r;
            assert!((0..=ONE).contains(&smoothing_weight(d)));
        }
        assert!(wants_smoothing(20_000) && !wants_smoothing(300) && !wants_smoothing(500_000));
        assert!(wants_relief(100_000) && !wants_relief(300_000));
    }

    #[test]
    fn shore_tint_ramps_instead_of_stepping() {
        let c = sea(1_000, (0, 0));
        let half = shore_water_soft(c, 1_000, &[0, 0, 0, 0, 0, 0, ONE / 2, 0]);
        let full = shore_water_soft(c, 1_000, &[0, 0, 0, 0, 0, 0, ONE, 0]);
        let none = shore_water_soft(c, 1_000, &[ONE; 8].map(|_| 0));
        assert!(spread(half, none) > 0 && spread(half, none) < spread(full, none));
    }
}
