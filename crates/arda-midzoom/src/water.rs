//! Water on relief tiles from the tactical layer's own geometry
//! (logic/17 §water): lakes, sea, shores, channels and marsh pools come
//! from `arda_refine::region`, the rules the refined blocks draw with, so a
//! river, lake or coast sits in the same place at every relief level and
//! on the tactical map.

use crate::fixed::ONE;
use crate::pyramid::Pyramid;
use crate::world::ReliefWorld;
use crate::MidzoomError;
use arda_refine::region::{RiverStyle, WaterAt, WaterRegion};
use arda_refine::terrain::Water;
use arda_render::ReliefSurface;

/// One tactical square, micrometres (100 m / 64).
pub const SQUARE_UM: i64 = 1_562_500;
/// Pixels at or below this size show marsh pools, micrometres.
const POOL_PIXEL_UM: i64 = 3_200_000;

/// Drawn over physical channel width: the overview's ×4 symbol at 25 m/px,
/// easing linearly to the true banks at 6.25 m/px and finer, so relief
/// channels meet the tactical map's banks exactly (logic/17 §rivers).
#[must_use]
#[allow(clippy::cast_precision_loss)] // pixel sizes are far below 2^52 µm
pub fn width_gain(pixel_um: i64) -> f64 {
    (pixel_um as f64 / 6_250_000.0).clamp(1.0, 4.0)
}

#[allow(clippy::cast_precision_loss)] // world micrometres are far below 2^52
fn square_units(um: i64) -> f64 {
    um as f64 / SQUARE_UM as f64
}

/// The water of one relief window.
#[derive(Debug)]
pub struct WindowWater {
    region: WaterRegion,
    pixel_sq: f64,
    pools: bool,
}

impl WindowWater {
    /// Gathers the water under the pixel window at `origin` of `size` at
    /// level `z`.
    ///
    /// # Errors
    /// A world layer failed to read.
    pub fn gather(
        rw: &ReliefWorld,
        pyramid: &Pyramid,
        z: u32,
        origin: (i64, i64),
        (w, h): (i64, i64),
    ) -> Result<Self, MidzoomError> {
        let centre = |p: i64| pyramid.centre_um(z, p);
        Self::gather_um(
            rw,
            (
                centre(origin.0),
                centre(origin.1),
                centre(origin.0 + w - 1),
                centre(origin.1 + h - 1),
            ),
            pyramid.pixel_um(z),
        )
    }

    /// Gathers the water for queries at world micrometres in
    /// `[wx0, wx1] × [wy0, wy1]`, drawn for pixels of `pixel_um` (the
    /// world-grade lattice, goal 49, samples it at its own spacing).
    ///
    /// # Errors
    /// A world layer failed to read.
    pub fn gather_um(
        rw: &ReliefWorld,
        (wx0, wy0, wx1, wy1): (i64, i64, i64, i64),
        pixel_um: i64,
    ) -> Result<Self, MidzoomError> {
        let pixel_um = pixel_um.max(1);
        let pixel_sq = square_units(pixel_um);
        let square = |um: i64| um.div_euclid(SQUARE_UM);
        let squares = (square(wx0), square(wy0), square(wx1), square(wy1));
        let style = RiverStyle {
            gain: width_gain(pixel_um),
            // Channels stay at least 1.2 px wide.
            min_half: 0.6 * pixel_sq,
        };
        Ok(Self {
            region: WaterRegion::gather(rw.water_source(), squares, style)?,
            pixel_sq,
            pools: pixel_um <= POOL_PIXEL_UM,
        })
    }

    /// The standing water (lake, sea or pool) at world micrometres, if
    /// any; channels are [`Self::river`].
    #[must_use]
    pub fn standing(&self, wx: i64, wy: i64) -> Option<WaterAt> {
        self.region
            .standing(square_units(wx), square_units(wy), self.pools)
    }

    /// The surface relief shading paints at world micrometres.
    #[must_use]
    pub fn surface(&self, wx: i64, wy: i64) -> ReliefSurface {
        #[allow(clippy::cast_possible_truncation)] // depths are under 1 km
        let mm = |m: f64| (m * 1000.0).round() as i64;
        match self.standing(wx, wy) {
            Some(WaterAt {
                kind: Water::Sea,
                depth_m,
            }) => ReliefSurface::Sea {
                depth_mm: mm(depth_m),
            },
            Some(at) => ReliefSurface::Lake {
                depth_mm: mm(at.depth_m),
            },
            None => ReliefSurface::Land,
        }
    }

    /// Channel coverage (Q12, anti-aliased over one pixel) and the
    /// channel's saved discharge at world micrometres. Coverage is at
    /// least half exactly where the pixel centre lies within the drawn
    /// banks.
    #[must_use]
    pub fn river(&self, wx: i64, wy: i64) -> Option<(i64, u64)> {
        let hit = self.region.river((square_units(wx), square_units(wy)))?;
        let cover = (0.5 - hit.d / self.pixel_sq).clamp(0.0, 1.0);
        #[allow(clippy::cast_possible_truncation)] // within [0, ONE]
        let q12 = (cover * ONE as f64).round() as i64;
        (q12 > 0).then_some((q12, hit.discharge_milli))
    }

    /// Whether a pixel centre at world micrometres is drawn as water.
    #[must_use]
    pub fn is_water(&self, wx: i64, wy: i64) -> bool {
        self.standing(wx, wy).is_some() || self.river(wx, wy).is_some_and(|(c, _)| c >= ONE / 2)
    }
}

/// Which pixels of a relief window are drawn as water (row-major), by the
/// same queries [`crate::render_window`] paints with.
///
/// # Errors
/// A world layer failed to read.
pub fn water_mask(
    rw: &ReliefWorld,
    pyramid: &Pyramid,
    z: u32,
    origin: (i64, i64),
    size: (u32, u32),
) -> Result<Vec<bool>, MidzoomError> {
    let (w, h) = (i64::from(size.0), i64::from(size.1));
    let water = WindowWater::gather(rw, pyramid, z, origin, (w, h))?;
    Ok((0..h)
        .flat_map(|py| (0..w).map(move |px| (px, py)))
        .map(|(px, py)| {
            water.is_water(
                pyramid.centre_um(z, origin.0 + px),
                pyramid.centre_um(z, origin.1 + py),
            )
        })
        .collect())
}
