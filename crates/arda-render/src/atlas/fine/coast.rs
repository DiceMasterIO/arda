//! Recipe-9 coast and open water for the formed fine shading (logic/04
//! §atlas-formed shore): the shoreline drawn from the smoothing spline with
//! coverage anti-aliasing, and the sea colour over a smoothed seabed.

use super::{shore_near, FineAtlas, SavedFields, CELL_UM, KM_UM};
use crate::atlas::formed::{
    relief_weight, sea_soft, shore_water_soft, wants_relief, wants_smoothing,
};
use crate::atlas::invalid;
use crate::RenderError;

/// Q12 one.
const ONE: i64 = 4_096;
/// Slope (Q12) below which the spline has no usable direction: the coast is
/// then decided by the sign of the height alone.
const FLAT_SLOPE_Q12: i64 = 8;

/// The land share (Q12) of a pixel whose centre has spline height `h_mm`
/// and gradient `(gx, gy)` (Q12 slopes), over a pixel `footprint_um` wide:
/// a half-pixel ramp about the zero contour, so the coast is straight where
/// the spline is and soft across one pixel instead of stepping.
pub(super) fn land_share(h_mm: i64, (gx, gy): (i64, i64), footprint_um: i128) -> i64 {
    let (gx, gy) = (i128::from(gx), i128::from(gy));
    let slope = i128::try_from((gx * gx + gy * gy).unsigned_abs().isqrt()).unwrap_or(i128::MAX);
    if slope < i128::from(FLAT_SLOPE_Q12) {
        return if h_mm > 0 { ONE } else { 0 };
    }
    // Distance to the contour, micrometres: height (mm -> um) over the
    // dimensionless slope (Q12 -> 1).
    let distance_um = i128::from(h_mm) * 1_000 * i128::from(ONE) / slope;
    let t = distance_um * i128::from(ONE) / footprint_um.max(1);
    i64::try_from((i128::from(ONE) / 2 + t).clamp(0, i128::from(ONE))).unwrap_or(0)
}

/// Whether a point at bilinear height `h_mm` is near enough the waterline
/// to need the spline: within three pixel footprints of rise plus 2 m.
pub(super) fn near_waterline(h_mm: i64, footprint_um: i128) -> bool {
    let reach = footprint_um / 1_000 * 3 + 2_000;
    i128::from(h_mm).abs() <= reach
}

impl FineAtlas {
    /// The land share (Q12) at a fine point near the waterline, from the
    /// smoothing spline and its gradient.
    pub(super) fn coast_share(
        &self,
        (qx, qy): (i128, i128),
        footprint_um: i128,
    ) -> Result<i64, RenderError> {
        let (h, gx, gy) = self.cubic(qx, qy)?;
        Ok(land_share(h, (gx, gy), footprint_um))
    }

    /// Depth (mm) of the seabed around a point, averaged over two rings of
    /// eight taps at 150 m and 300 m plus the point (inside the 312 m window margin): the smoothed depth the
    /// recipe-9 ramp reads away from the coast. Land taps count as zero.
    fn smoothed_depth(&self, qx: i128, qy: i128) -> Result<i64, RenderError> {
        let tap = |dx: i128, dy: i128| -> Result<i64, RenderError> {
            let p = self.clamp_source(qx + dx, qy + dy)?;
            let h = self.height(i128::from(p.x_um), i128::from(p.y_um))?;
            Ok((-i64::from(h)).max(0))
        };
        let mut sum = tap(0, 0)?;
        let mut count = 1_i64;
        for r in [3 * CELL_UM / 2, 3 * CELL_UM] {
            // Eight directions: the axes and the unit-length diagonals.
            let d = r * 7_071 / 10_000;
            for (dx, dy) in [
                (r, 0),
                (-r, 0),
                (0, r),
                (0, -r),
                (d, d),
                (-d, d),
                (d, -d),
                (-d, -d),
            ] {
                sum += tap(dx, dy)?;
                count += 1;
            }
        }
        Ok(sum / count)
    }

    /// Recipe-9 seafloor colour at a clamped source position: ramp over the
    /// smoothed depth, relief light faded with depth, shore tints feathered.
    pub(super) fn sea_colour_soft(
        &self,
        (qx, qy): (i128, i128),
        height: i32,
        shore_mix: &[i64; 8],
    ) -> Result<[u8; 3], RenderError> {
        let sky = self
            .sky
            .as_ref()
            .ok_or_else(|| invalid("formed shading requires the 1 km context"))?;
        let depth = -i64::from(height);
        let smoothed = if wants_smoothing(depth) {
            self.smoothed_depth(qx, qy)?
        } else {
            depth
        };
        let broad = if wants_relief(depth) && relief_weight(depth) > 0 {
            let r = KM_UM;
            (
                Self::slope_q12(
                    sky.context_height_mm(qx + r, qy)?,
                    sky.context_height_mm(qx - r, qy)?,
                    r,
                )?,
                Self::slope_q12(
                    sky.context_height_mm(qx, qy + r)?,
                    sky.context_height_mm(qx, qy - r)?,
                    r,
                )?,
            )
        } else {
            (0, 0)
        };
        Ok(shore_water_soft(
            sea_soft(depth, smoothed, broad),
            depth,
            shore_mix,
        ))
    }

    /// The colour of a point at the waterline: land and sea each shaded as
    /// if they owned the point, mixed by the land `share` (Q12). Land reads
    /// its ground at least 1 mm above the water and sea at least 1 mm
    /// below, so neither shades a height it does not have.
    pub(super) fn coast_colour(
        &self,
        share: i64,
        (px, py, qx, qy): (i128, i128, i128, i128),
        (height, footprint_um): (i32, i128),
        saved: &SavedFields<'_>,
    ) -> Result<[u8; 3], RenderError> {
        let shore_mix = shore_near(px, py, (self.origin_x, self.origin_y), saved.shore)?;
        let sea = if share < ONE {
            Some(self.sea_colour_soft((qx, qy), height.min(-1), &shore_mix)?)
        } else {
            None
        };
        let land = if share > 0 {
            let ground = height.max(1);
            let inputs = self.land_inputs(px, py, ground, saved)?;
            Some(self.formed_color(qx, qy, ground, inputs, footprint_um, None)?)
        } else {
            None
        };
        Ok(match (land, sea) {
            (Some(l), Some(s)) => std::array::from_fn(|ch| {
                let v =
                    (i64::from(l[ch]) * share + i64::from(s[ch]) * (ONE - share) + ONE / 2) / ONE;
                u8::try_from(v.clamp(0, 255)).unwrap_or(255)
            }),
            (Some(l), None) => l,
            (None, Some(s)) => s,
            (None, None) => unreachable!("a share is either above zero or below one"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PX: i128 = 12_500_000;

    #[test]
    fn share_is_half_on_the_contour_and_saturates_a_pixel_away() {
        // A 0.2 slope shore: 1 m of height is 5 m of distance.
        let g = (819, 0);
        assert_eq!(land_share(0, g, PX), ONE / 2);
        assert!(land_share(1_000, g, PX) > ONE / 2);
        assert_eq!(land_share(10_000, g, PX), ONE);
        assert_eq!(land_share(-10_000, g, PX), 0);
    }

    #[test]
    fn share_ramps_monotonically_through_the_shore() {
        let mut last = 0;
        for h in (-4_000..=4_000).step_by(250) {
            let s = land_share(h, (700, 300), PX);
            assert!(s >= last && (0..=ONE).contains(&s));
            last = s;
        }
    }

    #[test]
    fn flat_ground_falls_back_to_the_sign() {
        assert_eq!(land_share(5, (0, 3), PX), ONE);
        assert_eq!(land_share(-5, (0, 3), PX), 0);
        assert_eq!(land_share(0, (0, 0), PX), 0);
    }

    #[test]
    fn the_waterline_band_scales_with_the_pixel() {
        assert!(near_waterline(30_000, PX));
        assert!(!near_waterline(60_000, PX));
        assert!(!near_waterline(30_000, 1_000_000));
    }
}
