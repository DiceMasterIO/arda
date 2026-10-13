//! Map labels on relief tiles: the gazetteer's names laid out once per
//! level by [`arda_settle::render::tilelabels`] and cut into tiles, so a
//! label that crosses a tile edge is the same pixels on both sides
//! (logic/17 §labels).
//!
//! Labels are drawn after the relief and its worked land, never inside
//! them, and are a pure function of the world, the level and the pixel
//! position.

use crate::pyramid::{Pyramid, TILE_PX};
use crate::tile::Rgba;
use crate::world::AREA_UM;
use crate::MidzoomError;
use arda_settle::gazetteer::GazetteerFile;
use arda_settle::model::Settlement;

pub use arda_settle::render::tilelabels::Labeler;

/// Labels for a world: its gazetteer file and settlements, on this
/// pyramid.
///
/// # Errors
/// [`MidzoomError::Society`] if the embedded fonts fail to parse.
pub fn labeler(
    pyramid: &Pyramid,
    file: &GazetteerFile,
    settlements: &[Settlement],
) -> Result<Labeler, MidzoomError> {
    let um_to_m = |um: i128| um as f64 / 1_000_000.0;
    let world = [
        um_to_m(i128::from(pyramid.areas_wide) * i128::from(AREA_UM)),
        um_to_m(i128::from(pyramid.areas_high) * i128::from(AREA_UM)),
    ];
    Labeler::new(file, settlements, world, um_to_m(pyramid.longest_um()))
        .map_err(|e| MidzoomError::Society(e.to_string()))
}

/// Draws the labels of level `z` into the window at `origin` (pixels of the
/// level) held by `rgba`.
pub fn draw_window(labeler: &Labeler, z: u32, origin: (i64, i64), rgba: &mut Rgba) {
    let Ok(z8) = u8::try_from(z) else {
        return;
    };
    let level = labeler.level(z8);
    let size = (
        usize::try_from(rgba.width).unwrap_or(0),
        usize::try_from(rgba.height).unwrap_or(0),
    );
    labeler.draw_rgba(&level, origin, size, &mut rgba.pixels);
}

/// Draws the labels of tile `(z, x, y)` into its render.
pub fn draw_tile(labeler: &Labeler, z: u32, x: u32, y: u32, rgba: &mut Rgba) {
    let origin = (
        i64::from(x) * i64::from(TILE_PX),
        i64::from(y) * i64::from(TILE_PX),
    );
    draw_window(labeler, z, origin, rgba);
}
