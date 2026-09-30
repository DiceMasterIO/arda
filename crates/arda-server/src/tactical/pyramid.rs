//! The tactical tile pyramid: a render halved level by level, cut into
//! 512 px tiles (goal 68).
//!
//! Level `max_zoom` is the render itself; level `z − 1` is level `z` halved
//! (rounding up) with the tactical crate's integer premultiplied area
//! average, so every level is deterministic. Tiles are anchored at the
//! top-left; edge tiles are padded with transparent pixels to 512 × 512.

use crate::error::{ServerError, ServerResult};
use arda_tactical::Rgba;

/// Tile edge in pixels.
pub const TACTICAL_TILE_PX: u32 = 512;

/// Smallest zoom at which one tile covers the whole render, counted from the
/// full-resolution level: the least `z` with `512 · 2^z ≥ max(width, height)`.
#[must_use]
pub fn max_zoom(width: u32, height: u32) -> u32 {
    let edge = width.max(height).max(1);
    let tiles = edge.div_ceil(TACTICAL_TILE_PX);
    tiles.next_power_of_two().trailing_zeros()
}

/// Pixel size of level `z` of a `width × height` render.
#[must_use]
pub fn level_size(width: u32, height: u32, max_zoom: u32, z: u32) -> (u32, u32) {
    let shift = max_zoom.saturating_sub(z).min(31);
    let f = 1_u64 << shift;
    let down = |v: u32| u32::try_from(u64::from(v).div_ceil(f)).unwrap_or(u32::MAX);
    (down(width).max(1), down(height).max(1))
}

/// Tile columns and rows at level `z`.
#[must_use]
pub fn tile_grid(width: u32, height: u32, max_zoom: u32, z: u32) -> (u32, u32) {
    let (w, h) = level_size(width, height, max_zoom, z);
    (w.div_ceil(TACTICAL_TILE_PX), h.div_ceil(TACTICAL_TILE_PX))
}

/// Every level of a render, `levels[z]` for `z` in `0..=max_zoom`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pyramid {
    /// Levels from the most reduced (`z = 0`) to the render itself.
    pub levels: Vec<Rgba>,
}

impl Pyramid {
    /// Builds every level of `render`.
    #[must_use]
    pub fn build(render: Rgba) -> Self {
        let top = max_zoom(render.width, render.height);
        let mut levels = Vec::with_capacity(top as usize + 1);
        levels.push(render);
        for _ in 0..top {
            let Some(prev) = levels.last() else { break };
            let next = prev.resized(prev.width.div_ceil(2), prev.height.div_ceil(2));
            levels.push(next);
        }
        levels.reverse();
        Self { levels }
    }

    /// Deepest zoom level.
    #[must_use]
    pub fn max_zoom(&self) -> u32 {
        u32::try_from(self.levels.len().saturating_sub(1)).unwrap_or(0)
    }

    /// Heap bytes of every level.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.levels.iter().map(|l| l.data.len()).sum()
    }

    /// Cuts tile `(z, x, y)` as 512 × 512 RGBA bytes.
    ///
    /// # Errors
    /// [`ServerError::NotFound`] for tiles outside the pyramid.
    pub fn tile(&self, z: u32, x: u32, y: u32) -> ServerResult<Vec<u8>> {
        let outside = || {
            ServerError::NotFound(format!(
                "tile {z}/{x}/{y} is outside the pyramid (zoom 0..={})",
                self.max_zoom()
            ))
        };
        let level = self.levels.get(z as usize).ok_or_else(outside)?;
        let (cols, rows) = (
            level.width.div_ceil(TACTICAL_TILE_PX),
            level.height.div_ceil(TACTICAL_TILE_PX),
        );
        if x >= cols || y >= rows {
            return Err(outside());
        }
        let t = TACTICAL_TILE_PX as usize;
        let mut out = vec![0_u8; t * t * 4];
        let (x0, y0) = (x * TACTICAL_TILE_PX, y * TACTICAL_TILE_PX);
        let w = (level.width - x0).min(TACTICAL_TILE_PX) as usize;
        let h = (level.height - y0).min(TACTICAL_TILE_PX) as usize;
        let stride = level.width as usize * 4;
        for row in 0..h {
            let src = (y0 as usize + row) * stride + x0 as usize * 4;
            let dst = row * t * 4;
            out[dst..dst + w * 4].copy_from_slice(&level.data[src..src + w * 4]);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: u32, height: u32) -> Rgba {
        Rgba::filled(width, height, [10, 20, 30, 255])
    }

    #[test]
    fn zoom_depth_is_the_least_power_of_two_tile_cover() {
        assert_eq!(max_zoom(512, 300), 0);
        assert_eq!(max_zoom(513, 10), 1);
        assert_eq!(max_zoom(3840, 2176), 3);
        assert_eq!(level_size(3840, 2176, 3, 0), (480, 272));
        assert_eq!(tile_grid(3840, 2176, 3, 3), (8, 5));
    }

    #[test]
    fn levels_halve_and_match_the_geometry() {
        let p = Pyramid::build(solid(1100, 600));
        assert_eq!(p.max_zoom(), 2);
        let sizes: Vec<_> = p.levels.iter().map(|l| (l.width, l.height)).collect();
        assert_eq!(sizes, vec![(275, 150), (550, 300), (1100, 600)]);
        for (z, &(w, h)) in (0_u32..).zip(sizes.iter()) {
            assert_eq!(level_size(1100, 600, 2, z), (w, h));
        }
        assert!(p.levels[0].data.chunks(4).all(|c| c == [10, 20, 30, 255]));
    }

    #[test]
    fn tiles_are_bounded_and_edge_tiles_are_padded() {
        let p = Pyramid::build(solid(1100, 600));
        let edge = p.tile(2, 2, 1).unwrap();
        assert_eq!(edge.len(), 512 * 512 * 4);
        // Level 2 is 1100 wide: tile column 2 holds 76 image columns.
        assert_eq!(&edge[..4], &[10, 20, 30, 255]);
        assert_eq!(edge[76 * 4 + 3], 0, "padding past the image is transparent");
        for (z, x, y) in [(3, 0, 0), (2, 3, 0), (2, 0, 2), (0, 1, 0)] {
            assert!(
                matches!(p.tile(z, x, y), Err(ServerError::NotFound(_))),
                "{z}/{x}/{y}"
            );
        }
    }
}
