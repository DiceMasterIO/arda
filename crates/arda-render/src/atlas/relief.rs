//! Public close-zoom relief entry points on [`AtlasTerrain`]
//! (logic/17 §render): colour one pixel from refined geometry and the
//! surface the tactical water geometry decides.

use super::{fine, invalid, AtlasTerrain};
use crate::RenderError;

pub use fine::{ReliefGeometry, ReliefSurface};

impl AtlasTerrain {
    /// Formed-overview colour of one point at absolute fine-lattice
    /// micrometres (world position minus `arda_core::FINE_FRAME_OFFSET_UM`),
    /// lit with caller-supplied refined geometry, showing the surface the
    /// caller decided (logic/17 §render, §water): relief tiles take lakes,
    /// sea and shores from the tactical layer's water geometry, so both
    /// zooms put every shore in the same place. The point must lie in this
    /// area's saved cells (or their one-cell halo); `footprint_um` is the
    /// pixel size, which fades texture before it aliases.
    ///
    /// # Errors
    /// Not a recipe-5 formed area, or the point lies outside its context.
    pub fn relief_surface_colour(
        &self,
        (x_um, y_um): (i64, i64),
        footprint_um: i64,
        geometry: ReliefGeometry,
        surface: ReliefSurface,
    ) -> Result<[u8; 3], RenderError> {
        let fine = self
            .fine
            .as_ref()
            .filter(|f| f.is_formed())
            .ok_or_else(|| invalid("relief shading requires recipe-5 formed terrain"))?;
        fine.relief_surface_colour(
            (i128::from(x_um), i128::from(y_um)),
            i128::from(footprint_um.max(1)),
            geometry,
            surface,
            &fine::SavedFields {
                classes: &self.classes,
                wetness: &self.wetness,
                moisture: &self.moisture,
                forest_density: &self.forest_density,
                temperature: &self.temperature,
                heights: &self.heights,
                lake_surface: &self.lake_surface,
                shore: &self.shore,
                salt: &self.salt,
            },
        )
    }
}

/// Recipe-5 river water colour for a saved discharge (goal 28), the same
/// blue-teal the formed overview uses (`formed_river_colour` in the channel
/// overlay); `None` below the drawn threshold.
#[must_use]
pub fn formed_river_rgb(discharge_milli: u64) -> Option<[u8; 3]> {
    crate::carto::river_band(discharge_milli).map(crate::overview::formed_river_colour)
}
