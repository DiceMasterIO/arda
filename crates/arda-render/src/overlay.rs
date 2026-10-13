//! The overlay hook: painting on rows the exporters have finished.
//!
//! Overview and area exporters stream finished RGB rows to the PNG encoder.
//! A [`BandOverlay`] sees each group of rows once, just before encoding, and
//! may paint on it. The renderer knows nothing of what is painted (the
//! society layer lives in crates above this one), and `None` is exactly the
//! path without overlay: same pixels, same bytes.

use crate::RenderError;

/// A painter of finished pixel rows.
///
/// Rows arrive in increasing `y0` order, each group exactly once. The slice
/// holds `rows.len() / (3 * width)` whole rows of tightly packed RGB. An
/// overlay must paint pixels as a pure function of their position in the
/// full image, so any grouping of rows gives the same image; a symbol
/// that straddles two groups paints its share in each.
pub trait BandOverlay: Sync {
    /// Paints rows `y0 ..` of an image `width` pixels across.
    ///
    /// # Errors
    /// An overlay-specific failure, usually [`RenderError::Overlay`].
    fn paint(&self, y0: u32, rows: &mut [u8], width: u32) -> Result<(), RenderError>;

    /// Rows per [`Self::paint`] call the area exporter should hand over for
    /// an image `width` pixels across (overviews always use 256-row bands).
    /// The group is held in memory, so keep `rows * width` bounded.
    fn area_rows(&self, _width: u32) -> usize {
        AREA_OVERLAY_ROWS
    }
}

/// Default rows per group the area exporter hands to an overlay.
pub const AREA_OVERLAY_ROWS: usize = 64;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        render_area_png_to, render_area_png_to_with_overlay, write_overview_png,
        write_overview_png_with_overlay, AreaImageScale, ImageQuality, OverviewRaster,
    };
    use arda_core::{AreaCells, AreaCoord, AreaObjects, Cell, GlobalCell, TerrainKind};
    use std::sync::Mutex;

    /// Paints a red pixel on the diagonal of the full image and records its
    /// calls.
    struct Diagonal {
        calls: Mutex<Vec<(u32, usize, u32)>>,
    }

    impl Diagonal {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl BandOverlay for Diagonal {
        fn paint(&self, y0: u32, rows: &mut [u8], width: u32) -> Result<(), RenderError> {
            let w = width as usize;
            self.calls.lock().unwrap().push((y0, rows.len(), width));
            for (i, row) in rows.chunks_exact_mut(w * 3).enumerate() {
                let y = y0 as usize + i;
                if y < w {
                    row[y * 3..y * 3 + 3].copy_from_slice(&[255, 0, 0]);
                }
            }
            Ok(())
        }
    }

    fn land() -> AreaCells {
        AreaCells::flat(Cell {
            terrain: TerrainKind::Land,
            ..Cell::default()
        })
    }

    fn decode(png_bytes: &[u8]) -> (u32, u32, Vec<u8>) {
        let mut reader = png::Decoder::new(png_bytes).read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        (info.width, info.height, buf)
    }

    #[test]
    fn overview_without_overlay_is_the_current_stream_and_calls_come_once_per_band() {
        let cells = land();
        let (w, h) = (700_u32, 600_u32);
        let mut plain = Vec::new();
        write_overview_png(1, 1, w, h, &mut plain, |_| {
            Ok::<_, RenderError>(cells.clone())
        })
        .unwrap();
        let mut none = Vec::new();
        write_overview_png_with_overlay(1, 1, (w, h), &mut none, None, |_| {
            Ok::<_, RenderError>(cells.clone())
        })
        .unwrap();
        assert_eq!(plain, none);

        let overlay = Diagonal::new();
        let mut painted = Vec::new();
        write_overview_png_with_overlay(1, 1, (w, h), &mut painted, Some(&overlay), |_| {
            Ok::<_, RenderError>(cells.clone())
        })
        .unwrap();
        let calls = overlay.calls.lock().unwrap().clone();
        let starts: Vec<u32> = calls.iter().map(|c| c.0).collect();
        assert_eq!(
            starts,
            [0, 256, 512],
            "one call per 256-row band, increasing"
        );
        assert_eq!(calls[2].1, 88 * 700 * 3);
        let (_, _, rgb) = decode(&painted);
        let (_, _, base) = decode(&plain);
        for y in 0..h as usize {
            for x in 0..w as usize {
                let p = (y * w as usize + x) * 3;
                if x == y {
                    assert_eq!(&rgb[p..p + 3], &[255, 0, 0]);
                } else {
                    assert_eq!(&rgb[p..p + 3], &base[p..p + 3]);
                }
            }
        }
    }

    #[test]
    fn the_buffered_raster_finishes_with_the_same_bands() {
        let cells = land();
        let build = || {
            let mut raster = OverviewRaster::new_exact(1, 1, 300, 600).unwrap();
            raster.push(AreaCoord::new(0, 0), &cells).unwrap();
            raster
        };
        assert_eq!(
            build().finish().unwrap(),
            build().finish_with_overlay(None).unwrap()
        );
        let overlay = Diagonal::new();
        build().finish_with_overlay(Some(&overlay)).unwrap();
        let starts: Vec<u32> = overlay.calls.lock().unwrap().iter().map(|c| c.0).collect();
        assert_eq!(starts, [0, 256, 512]);
    }

    #[test]
    fn area_rows_reach_the_overlay_in_groups_and_none_is_the_old_stream() {
        let cells = land();
        let objects = AreaObjects::default();
        let origin = GlobalCell { x: 0, y: 0 };
        let scale = AreaImageScale::Custom(ImageQuality::new(1024).unwrap());
        let mut plain = Vec::new();
        render_area_png_to(&cells, &objects, origin, scale, &mut plain).unwrap();
        let mut none = Vec::new();
        render_area_png_to_with_overlay(&cells, &objects, origin, scale, (None, None), &mut none)
            .unwrap();
        assert_eq!(plain, none);

        let overlay = Diagonal::new();
        let mut painted = Vec::new();
        render_area_png_to_with_overlay(
            &cells,
            &objects,
            origin,
            scale,
            (None, Some(&overlay)),
            &mut painted,
        )
        .unwrap();
        let calls = overlay.calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 1024 / AREA_OVERLAY_ROWS);
        assert!(calls.windows(2).all(|p| p[0].0 < p[1].0));
        assert!(calls
            .iter()
            .all(|c| c.2 == 1024 && c.1 == AREA_OVERLAY_ROWS * 1024 * 3));
        let (_, _, rgb) = decode(&painted);
        assert_eq!(&rgb[(700 * 1024 + 700) * 3..][..3], &[255, 0, 0]);
    }
}
