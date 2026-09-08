//! Incremental overview rendering from one saved area at a time.

use crate::carto::{
    land_colour, river_band, river_band_colour, RiverBand, LAKE_FILL, LAKE_MIN_BLOCK_DEN,
    OVERVIEW_SEA,
};
use crate::RenderError;
use arda_core::{AreaCells, AreaCoord, CellCoord, TerrainKind, AREA_CELLS};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Feature {
    Sea,
    Land,
    River(RiverBand),
    Lake,
}

/// Overview-sized buffer that never retains an area's cell grid.
pub struct OverviewRaster {
    width: u32,
    height: u32,
    areas_wide: i32,
    areas_high: i32,
    px: u32,
    rgb: Vec<u8>,
    features: Vec<Feature>,
    supplied: Vec<bool>,
}

impl OverviewRaster {
    /// Allocates the final raster; at most 64 million pixels are supported.
    ///
    /// # Errors
    /// Returns a typed error for invalid dimensions or the pixel budget.
    pub fn new(areas_wide: i32, areas_high: i32, px: u32) -> Result<Self, RenderError> {
        let invalid = || RenderError::OverviewDimensions;
        if !(1..=78).contains(&areas_wide)
            || !(1..=78).contains(&areas_high)
            || !(1..=512).contains(&px)
        {
            return Err(invalid());
        }
        let width = u32::try_from(areas_wide).map_err(|_| invalid())? * px;
        let height = u32::try_from(areas_high).map_err(|_| invalid())? * px;
        let pixels = u64::from(width) * u64::from(height);
        if pixels > 64_000_000 {
            return Err(invalid());
        }
        let pixels = usize::try_from(pixels).map_err(|_| invalid())?;
        let mut rgb = vec![0; pixels * 3];
        for pixel in rgb.chunks_exact_mut(3) {
            pixel.copy_from_slice(&OVERVIEW_SEA);
        }
        Ok(Self {
            width,
            height,
            areas_wide,
            areas_high,
            px,
            rgb,
            features: vec![Feature::Sea; pixels],
            supplied: vec![false; usize::try_from(areas_wide * areas_high).map_err(|_| invalid())?],
        })
    }

    /// Incorporates one area; input order cannot affect the image.
    ///
    /// # Errors
    /// Refuses out-of-range or duplicate areas.
    pub fn push(&mut self, at: AreaCoord, cells: &AreaCells) -> Result<(), RenderError> {
        if at.x < 0 || at.y < 0 || at.x >= self.areas_wide || at.y >= self.areas_high {
            return Err(RenderError::OverviewDimensions);
        }
        let offset = usize::try_from(at.y * self.areas_wide + at.x)
            .map_err(|_| RenderError::OverviewDimensions)?;
        if self.supplied[offset] {
            return Err(RenderError::DuplicateOverviewArea { area: at });
        }
        let ox = u32::try_from(at.x).map_err(|_| RenderError::OverviewDimensions)?;
        let oy = u32::try_from(at.y).map_err(|_| RenderError::OverviewDimensions)?;
        let (px, width, side) = (self.px, self.width, u32::from(AREA_CELLS));
        for py in 0..px {
            for pxi in 0..px {
                // Half-open cell bounds, derived per pixel so the blocks
                // tile the whole 512 exactly. The previous `block = side /
                // px` was 512/48 = 10, and `pxi * block + cx` therefore
                // topped out at 479: cells 480..=511 of every tile — a
                // 3.2 km strip down the right edge and along the bottom of
                // all 171 tiles — were never read, which truncated courses
                // at tile edges. Uneven blocks (here 10 and 11 cells) are
                // the correct answer when px does not divide 512.
                let x0 = pxi * side / px;
                let x1 = ((pxi + 1) * side / px).max(x0 + 1);
                let y0 = py * side / px;
                let y1 = ((py + 1) * side / px).max(y0 + 1);

                // `best` ranks only sea, land and river. A lake is
                // decided by area below, not by winning a max.
                let mut best = Feature::Sea;
                let mut height_sum: i64 = 0;
                let mut land_count: i64 = 0;
                let mut lake_cells: u32 = 0;
                let mut block_cells: u32 = 0;

                for sy in y0..y1 {
                    for sx in x0..x1 {
                        let (Ok(sxu), Ok(syu)) = (u16::try_from(sx), u16::try_from(sy)) else {
                            continue;
                        };
                        let Some(at) = CellCoord::new(sxu, syu) else {
                            continue;
                        };
                        let cell = cells.get(at);
                        block_cells += 1;
                        match cell.terrain {
                            TerrainKind::Lake => lake_cells += 1,
                            // Feature::Sea is already the floor.
                            TerrainKind::Sea => {}
                            TerrainKind::Land => {
                                height_sum += i64::from(cell.height.raw());
                                land_count += 1;
                                let f = match river_band(cell.discharge.raw()) {
                                    Some(band) => Feature::River(band),
                                    None => Feature::Land,
                                };
                                best = best.max(f);
                            }
                        }
                    }
                }
                if lake_cells * LAKE_MIN_BLOCK_DEN >= block_cells {
                    best = Feature::Lake;
                }

                let colour = match best {
                    Feature::Sea => OVERVIEW_SEA,
                    // Mean over every land cell in the block, including
                    // the channel cells: excluding them made the tint jump
                    // wherever a river crossed a block.
                    Feature::Land | Feature::River(_) => {
                        let mean = height_sum / land_count.max(1);
                        let base = land_colour(i32::try_from(mean).unwrap_or(0));
                        match best {
                            Feature::River(band) => river_band_colour(band),
                            _ => base,
                        }
                    }
                    Feature::Lake => LAKE_FILL,
                };

                let x = ox * px + pxi;
                let y = oy * px + py;
                let Ok(pixel) = usize::try_from(y * width + x) else {
                    continue;
                };
                self.rgb[pixel * 3..pixel * 3 + 3].copy_from_slice(&colour);
                self.features[pixel] = best;
            }
        }
        self.supplied[offset] = true;
        Ok(())
    }

    /// Applies final trunk styling and encodes the overview.
    ///
    /// Areas not supplied remain ocean; stored-world exports supply every area
    /// and propagate missing-layer errors before calling this method.
    ///
    /// # Errors
    /// Returns the PNG encoding error.
    pub fn finish(mut self) -> Result<Vec<u8>, RenderError> {
        let (width, height) = (self.width, self.height);
        // The Dark band widens by one pixel so the few real trunks carry
        // visible weight against the streams. Guarded both ways: never over a
        // lake, and never over another river pixel, so the pass cannot change
        // a classification the block pass already made.
        let mut widened = vec![false; self.features.len()];
        for y in 0..height {
            for x in 0..width {
                let Ok(src) = usize::try_from(y * width + x) else {
                    continue;
                };
                if self.features.get(src) != Some(&Feature::River(RiverBand::Dark)) {
                    continue;
                }
                for (dx, dy) in [(1u32, 0u32), (0, 1)] {
                    let (tx, ty) = (x + dx, y + dy);
                    if tx >= width || ty >= height {
                        continue;
                    }
                    let Ok(dst) = usize::try_from(ty * width + tx) else {
                        continue;
                    };
                    if !matches!(self.features.get(dst), Some(Feature::Land | Feature::Sea)) {
                        continue;
                    }
                    if widened.get(dst) == Some(&true) {
                        continue;
                    }
                    if let Some(slot) = widened.get_mut(dst) {
                        *slot = true;
                    }
                    if let Some(slot) = self.rgb.get_mut(dst * 3..dst * 3 + 3) {
                        slot.copy_from_slice(&river_band_colour(RiverBand::Dark));
                    }
                }
            }
        }

        crate::encode_png(width, height, &self.rgb)
    }
}

/// Renders supplied areas as one overview, preserving the borrowed batch API.
///
/// # Errors
/// Returns dimension, duplicate area, resource or PNG errors.
pub fn render_overview_png(
    areas: &[(i32, i32, &AreaCells)],
    areas_wide: i32,
    areas_high: i32,
    px: u32,
) -> Result<Vec<u8>, RenderError> {
    let mut raster = OverviewRaster::new(areas_wide, areas_high, px)?;
    for &(x, y, cells) in areas {
        raster.push(AreaCoord::new(x, y), cells)?;
    }
    raster.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::{Cell, DischargeMilli, HeightMm};

    #[test]
    fn incremental_order_and_batch_wrapper_have_identical_bytes() {
        let dry = AreaCells::flat(Cell {
            height: HeightMm::new(321_000),
            terrain: TerrainKind::Land,
            ..Cell::default()
        });
        let river = AreaCells::flat(Cell {
            discharge: DischargeMilli::new(80_000),
            terrain: TerrainKind::Land,
            ..Cell::default()
        });
        let mut forward = OverviewRaster::new(2, 1, 48).unwrap();
        forward.push(AreaCoord::new(0, 0), &dry).unwrap();
        forward.push(AreaCoord::new(1, 0), &river).unwrap();
        let mut reverse = OverviewRaster::new(2, 1, 48).unwrap();
        reverse.push(AreaCoord::new(1, 0), &river).unwrap();
        reverse.push(AreaCoord::new(0, 0), &dry).unwrap();
        let bytes = forward.finish().unwrap();
        assert_eq!(bytes, reverse.finish().unwrap());
        assert_eq!(
            bytes,
            crate::carto::render_overview_png(&[(0, 0, &dry), (1, 0, &river)], 2, 1, 48).unwrap()
        );
    }

    #[test]
    fn invalid_scale_budget_and_duplicate_area_are_typed_errors() {
        for (w, h, px) in [
            (0, 1, 48),
            (1, 1, 0),
            (79, 1, 48),
            (78, 78, 512),
            (1, 1, 513),
        ] {
            assert!(matches!(
                OverviewRaster::new(w, h, px),
                Err(RenderError::OverviewDimensions)
            ));
        }
        let mut canvas = OverviewRaster::new(1, 1, 48).unwrap();
        let cells = AreaCells::flat(Cell::default());
        canvas.push(AreaCoord::new(0, 0), &cells).unwrap();
        assert!(matches!(
            canvas.push(AreaCoord::new(0, 0), &cells),
            Err(RenderError::DuplicateOverviewArea { .. })
        ));
        assert!(matches!(
            canvas.push(AreaCoord::new(1, 0), &cells),
            Err(RenderError::OverviewDimensions)
        ));
    }
}
