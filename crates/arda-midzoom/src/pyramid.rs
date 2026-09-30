//! Relief levels continue the overview pyramid past its native zoom
//! (logic/17 §pyramid). The overview maps the world's longer side to
//! `256 · 2^max_zoom` pixels, so level `z` has `longest / (256 · 2^z)`
//! metres per pixel; pixel `(px, py)` covers world
//! `[px·m, (px + 1)·m) × [py·m, (py + 1)·m)`.

use crate::world::AREA_UM;
use crate::MidzoomError;

/// Tile edge in pixels.
pub const TILE_PX: u32 = 256;
/// Deepest relief level past the native one.
pub const MAX_EXTRA_LEVELS: u32 = 8;

/// Pyramid geometry of one world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pyramid {
    /// Native (overview) deepest zoom.
    pub max_zoom: u32,
    /// Areas across.
    pub areas_wide: i32,
    /// Areas down.
    pub areas_high: i32,
}

impl Pyramid {
    /// World extent of the pyramid's square, micrometres.
    #[must_use]
    pub fn longest_um(&self) -> i128 {
        i128::from(self.areas_wide.max(self.areas_high)) * i128::from(AREA_UM)
    }

    /// Pixels across the pyramid at level `z`.
    #[must_use]
    pub fn pixels(z: u32) -> i128 {
        i128::from(TILE_PX) << z
    }

    /// Metres per pixel at `z`, micrometres (rounded).
    #[must_use]
    pub fn pixel_um(&self, z: u32) -> i64 {
        i64::try_from(self.longest_um() / Self::pixels(z)).unwrap_or(i64::MAX)
    }

    /// World micrometres of the centre of pixel `p` at `z` (either axis).
    #[must_use]
    pub fn centre_um(&self, z: u32, p: i64) -> i64 {
        let v = (2 * i128::from(p) + 1) * self.longest_um() / (2 * Self::pixels(z));
        i64::try_from(v).unwrap_or(i64::MAX)
    }

    /// Whether `z` is a relief level (past native, within the limit).
    #[must_use]
    pub const fn is_relief(&self, z: u32) -> bool {
        z > self.max_zoom && z <= self.max_zoom + MAX_EXTRA_LEVELS
    }

    /// Refinement subdivisions for level `z`: the finest lattice a pixel
    /// still resolves (39.0625 m / n no finer than about the pixel, down to
    /// 9.765625 m).
    #[must_use]
    pub fn subdivisions(&self, z: u32) -> i64 {
        let px = self.pixel_um(z);
        if px <= 16_000_000 {
            4
        } else if px <= 32_000_000 {
            2
        } else {
            1
        }
    }

    /// Checks a relief tile address.
    ///
    /// # Errors
    /// [`MidzoomError::OutOfPyramid`] outside the relief levels or grid.
    pub fn check(&self, z: u32, x: u32, y: u32) -> Result<(), MidzoomError> {
        let n = 1_u64 << z.min(40);
        if !self.is_relief(z) || u64::from(x) >= n || u64::from(y) >= n {
            return Err(MidzoomError::OutOfPyramid { z, x, y });
        }
        Ok(())
    }

    /// World width and height, micrometres.
    #[must_use]
    pub fn world_um(&self) -> (i64, i64) {
        (
            i64::from(self.areas_wide) * AREA_UM,
            i64::from(self.areas_high) * AREA_UM,
        )
    }
}
