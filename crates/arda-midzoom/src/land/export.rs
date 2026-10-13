//! Worked land on area PNG exports (task 15): the parcels, roads and
//! settlement plans of relief tiles ([`super::Works`]) painted over the
//! finished rows of `arda export --area`, from the same society data and the
//! same geometry, so an area map keeps its fields, roads and buildings in
//! place across the relief and tactical zooms.
//!
//! The overlay is a [`BandOverlay`]: each group of rows is cut into chunks
//! about [`CHUNK_M`] wide, and each chunk gathers only what its box needs,
//! in parallel, so memory stays bounded whatever the output size. Detail
//! follows metres per pixel exactly as on relief tiles (see the table in
//! [`super`]); a pixel is a pure function of its world position, so a
//! building or street crossing into the next area paints the same pixels
//! there.

use super::roads::PixelGrid;
use super::tone;
use super::{Landscape, Works, CELL_TONE_Q12};
use crate::fixed::{smooth, ONE};
use crate::MidzoomError;
use arda_fields::geom::{Sq, SQUARE_M};
use arda_render::{BandOverlay, RenderError};
use rayon::prelude::*;
use std::path::Path;

/// Target width of a chunk, metres.
pub const CHUNK_M: f64 = 3_200.0;
/// Pixels held per row group (rows × width), bounding the group's memory.
const GROUP_PIXELS: usize = 4 << 20;
/// One saved area edge, metres.
const AREA_M: f64 = 51_200.0;

/// The society layer of one area export.
pub struct AreaLandOverlay {
    land: Landscape,
    /// Global pixel of the area's top-left corner.
    origin: (i64, i64),
    side: u32,
}

impl AreaLandOverlay {
    /// The overlay of area `(ax, ay)` exported `side` pixels across.
    #[must_use]
    pub fn new(land: Landscape, (ax, ay): (i32, i32), side: u32) -> Self {
        let s = i64::from(side);
        Self {
            land,
            origin: (i64::from(ax) * s, i64::from(ay) * s),
            side,
        }
    }

    /// Opens `<dir>/society/`; `None` when the world has none.
    ///
    /// # Errors
    /// Unreadable or malformed society files.
    pub fn open(dir: &Path, area: (i32, i32), side: u32) -> Result<Option<Self>, MidzoomError> {
        Ok(Landscape::open(dir)?.map(|land| Self::new(land, area, side)))
    }

    /// Metres per pixel.
    #[must_use]
    pub fn pixel_m(&self) -> f64 {
        AREA_M / f64::from(self.side)
    }

    /// Paints the `size` window whose top-left pixel is the global pixel
    /// `at` (`origin` plus the offset inside the area) onto `rgb`, tightly
    /// packed rows. Public so a window spanning two areas can be compared
    /// with the areas' own exports.
    ///
    /// # Errors
    /// A world or society read failed, or a plan could not be drawn.
    pub fn paint_window(
        &self,
        at: (i64, i64),
        size: (usize, usize),
        rgb: &mut [u8],
    ) -> Result<(), MidzoomError> {
        let pixel_m = self.pixel_m();
        let grid = PixelGrid {
            origin: at,
            size,
            pixel_m,
        };
        let (w, h) = (
            i64::try_from(size.0).unwrap_or(0),
            i64::try_from(size.1).unwrap_or(0),
        );
        let bbox = [
            grid.centre_m(at.0) - pixel_m,
            grid.centre_m(at.1) - pixel_m,
            grid.centre_m(at.0 + w - 1) + pixel_m,
            grid.centre_m(at.1 + h - 1) + pixel_m,
        ];
        let works = Works::gather(&self.land, grid, bbox)?;
        for py in 0..size.1 {
            for px in 0..size.0 {
                let k = (py * size.0 + px) * 3;
                let base = [rgb[k], rgb[k + 1], rgb[k + 2]];
                rgb[k..k + 3].copy_from_slice(&works.pixel(base, px, py));
            }
        }
        Ok(())
    }
}

impl Works {
    /// The ground tone and parcels (no relief, canopy or drainage), then
    /// roads and settlements over pixel `(px, py)`.
    fn pixel(&self, rgb: [u8; 3], px: usize, py: usize) -> [u8; 3] {
        let m = self.centre(px, py);
        let mut out = rgb;
        if let Some(cells) = &self.cells {
            #[allow(clippy::cast_possible_truncation)]
            let pixel_um = (self.grid.pixel_m * 1e6) as i64;
            let s = CELL_TONE_Q12 * (ONE - smooth(70_000, 160_000, pixel_um / 1_000)) / ONE;
            out = tone::tint(out, tone::soften(cells.at(m[0], m[1]), s));
        }
        if let Some(parcels) = &self.parcels {
            let q = [m[0] / SQUARE_M, m[1] / SQUARE_M];
            let sq = Sq::containing(q);
            let at = if self.grid.pixel_m <= 2.0 * SQUARE_M {
                sq.centre()
            } else {
                q
            };
            let (px_m, strength) = (self.grid.pixel_m, self.parcel_strength);
            match parcels.at(at) {
                Some(hit) if parcels.worked(hit.site) => {
                    out = parcels.paint_tone(out, hit, at, px_m, strength);
                    out = parcels.paint_wall(out, hit, at, px_m, strength);
                    let tree = parcels.orchard_tree(hit.site, at, px_m);
                    if tree > 0 {
                        out = tone::mix(out, [62, 84, 40], tree * 85 / 100);
                    }
                }
                Some(hit) => out = parcels.paint_wall(out, hit, at, px_m, strength),
                None => {}
            }
        }
        self.paint(out, (px, py))
    }
}

impl BandOverlay for AreaLandOverlay {
    fn paint(&self, y0: u32, rows: &mut [u8], width: u32) -> Result<(), RenderError> {
        let fail = |reason: String| RenderError::Overlay { reason };
        let w = width as usize;
        if width != self.side || w == 0 || rows.len() % (w * 3) != 0 {
            return Err(fail("rows do not match the area's image size".into()));
        }
        let h = rows.len() / (w * 3);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let chunk = ((CHUNK_M / self.pixel_m()) as usize).clamp(256, 2048);
        let starts: Vec<usize> = (0..w).step_by(chunk).collect();
        let src: &[u8] = rows;
        let done: Vec<(usize, usize, Vec<u8>)> = starts
            .par_iter()
            .map(|&x0| {
                let cw = chunk.min(w - x0);
                let mut buf = Vec::with_capacity(cw * h * 3);
                for r in 0..h {
                    buf.extend_from_slice(&src[(r * w + x0) * 3..(r * w + x0 + cw) * 3]);
                }
                let at = (
                    self.origin.0 + i64::try_from(x0).unwrap_or(0),
                    self.origin.1 + i64::from(y0),
                );
                self.paint_window(at, (cw, h), &mut buf)
                    .map(|()| (x0, cw, buf))
                    .map_err(|e| fail(e.to_string()))
            })
            .collect::<Result<_, _>>()?;
        for (x0, cw, buf) in done {
            for r in 0..h {
                rows[(r * w + x0) * 3..(r * w + x0 + cw) * 3]
                    .copy_from_slice(&buf[r * cw * 3..(r + 1) * cw * 3]);
            }
        }
        Ok(())
    }

    fn area_rows(&self, width: u32) -> usize {
        (GROUP_PIXELS / (width as usize).max(1)).clamp(16, 1024)
    }
}
