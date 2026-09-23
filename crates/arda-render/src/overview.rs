//! Incremental overview rendering from one saved area at a time.

use crate::atlas::axis_kernel;
use crate::carto::{
    land_colour, river_band, river_band_colour, RiverBand, LAKE_FILL, LAKE_MIN_BLOCK_DEN,
    OVERVIEW_SEA,
};
use crate::{AtlasTerrain, RenderError};
use arda_core::{AreaCells, AreaCoord, CellCoord, TerrainKind, AREA_CELLS};

mod streaming;
pub use streaming::{write_atlas_overview_png, write_overview_png};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Feature {
    Sea,
    AtlasSea,
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
        Self::allocate(areas_wide, areas_high, width, height)
    }

    /// Allocates an exact-size overview; Classic repeats saved cells when enlarged.
    ///
    /// Each axis supports 1–32,768 pixels, with at most 134,217,728 pixels
    /// total. There must be at least one pixel per area along each axis;
    /// unequal area pixel widths or heights cover the image without gaps.
    /// This changes image resolution, not the saved terrain resolution.
    /// Use [`write_overview_png`] for larger images without a full raster buffer.
    ///
    /// # Errors
    /// Returns a typed error for invalid dimensions or the pixel budget.
    pub fn new_exact(
        areas_wide: i32,
        areas_high: i32,
        width: u32,
        height: u32,
    ) -> Result<Self, RenderError> {
        validate_exact_dimensions(areas_wide, areas_high, width, height)?;
        if u64::from(width) * u64::from(height) > 134_217_728 {
            return Err(RenderError::ExactOverviewDimensions);
        }
        Self::allocate(areas_wide, areas_high, width, height)
    }

    fn allocate(
        areas_wide: i32,
        areas_high: i32,
        width: u32,
        height: u32,
    ) -> Result<Self, RenderError> {
        let pixels = usize::try_from(u64::from(width) * u64::from(height))
            .map_err(|_| RenderError::ExactOverviewDimensions)?;
        let mut rgb = vec![0; pixels * 3];
        for pixel in rgb.as_chunks_mut::<3>().0 {
            pixel.copy_from_slice(&OVERVIEW_SEA);
        }
        Ok(Self {
            width,
            height,
            areas_wide,
            areas_high,
            rgb,
            features: vec![Feature::Sea; pixels],
            supplied: vec![
                false;
                usize::try_from(areas_wide * areas_high)
                    .map_err(|_| RenderError::OverviewDimensions)?
            ],
        })
    }

    /// Incorporates one area; input order cannot affect the image.
    ///
    /// # Errors
    /// Refuses out-of-range or duplicate areas.
    pub fn push(&mut self, at: AreaCoord, cells: &AreaCells) -> Result<(), RenderError> {
        self.push_inner(at, cells, None)
    }

    /// Incorporates one Atlas area with reconstructed linear shoreline pixels.
    ///
    /// # Errors
    /// Refuses out-of-range or duplicate areas and invalid atlas context.
    pub fn push_atlas(
        &mut self,
        at: AreaCoord,
        cells: &AreaCells,
        terrain: &AtlasTerrain,
    ) -> Result<(), RenderError> {
        self.push_inner(at, cells, Some(terrain))
    }

    fn push_inner(
        &mut self,
        at: AreaCoord,
        cells: &AreaCells,
        terrain: Option<&AtlasTerrain>,
    ) -> Result<(), RenderError> {
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
        let areas_wide =
            u32::try_from(self.areas_wide).map_err(|_| RenderError::OverviewDimensions)?;
        let areas_high =
            u32::try_from(self.areas_high).map_err(|_| RenderError::OverviewDimensions)?;
        let width = self.width;
        let output_x0 = ox * width / areas_wide;
        let output_x1 = (ox + 1) * width / areas_wide;
        let output_y0 = oy * self.height / areas_high;
        let output_y1 = (oy + 1) * self.height / areas_high;
        let tile_width = output_x1 - output_x0;
        let tile_height = output_y1 - output_y0;
        for py in 0..tile_height {
            for pxi in 0..tile_width {
                let (colour, best) =
                    sample_pixel(cells, terrain, tile_width, tile_height, pxi, py)?;

                let x = output_x0 + pxi;
                let y = output_y0 + py;
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
        // visible weight against the streams. Saved Classic sea may receive
        // this mark, while reconstructed Atlas sea remains protected.
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

fn validate_exact_dimensions(
    areas_wide: i32,
    areas_high: i32,
    width: u32,
    height: u32,
) -> Result<(), RenderError> {
    if !(1..=78).contains(&areas_wide)
        || !(1..=78).contains(&areas_high)
        || !(1..=32_768).contains(&width)
        || !(1..=32_768).contains(&height)
        || i64::from(width) < i64::from(areas_wide)
        || i64::from(height) < i64::from(areas_high)
    {
        return Err(RenderError::ExactOverviewDimensions);
    }
    Ok(())
}

fn sample_pixel(
    cells: &AreaCells,
    terrain: Option<&AtlasTerrain>,
    tile_width: u32,
    tile_height: u32,
    x: u32,
    y: u32,
) -> Result<([u8; 3], Feature), RenderError> {
    // Half-open bounds retain the rightmost/bottom cells even when 512 is
    // not divisible by the output tile size. These bounds own feature masks.
    let side = u32::from(AREA_CELLS);
    let x0 = x * side / tile_width;
    let x1 = ((x + 1) * side / tile_width).max(x0 + 1);
    let y0 = y * side / tile_height;
    let y1 = ((y + 1) * side / tile_height).max(y0 + 1);
    // Lake coverage, rather than feature precedence, determines lake fill.
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
                TerrainKind::Sea => {}
                TerrainKind::Land => {
                    height_sum += i64::from(cell.height.raw());
                    land_count += 1;
                    let feature = match river_band(cell.discharge.raw()) {
                        Some(band) => Feature::River(band),
                        None => Feature::Land,
                    };
                    best = best.max(feature);
                }
            }
        }
    }
    if lake_cells * LAKE_MIN_BLOCK_DEN >= block_cells {
        best = Feature::Lake;
    }
    if let Some(terrain) = terrain {
        if best != Feature::Lake {
            let x_kernel = axis_kernel(x, tile_width)?;
            let y_kernel = axis_kernel(y, tile_height)?;
            if matches!(
                (x_kernel, y_kernel),
                (
                    crate::atlas::AxisKernel::Linear { .. },
                    crate::atlas::AxisKernel::Linear { .. }
                )
            ) {
                let owner = CellCoord::new(
                    u16::try_from(x0).map_err(|_| RenderError::ExactOverviewDimensions)?,
                    u16::try_from(y0).map_err(|_| RenderError::ExactOverviewDimensions)?,
                )
                .ok_or(RenderError::ExactOverviewDimensions)?;
                let saved = cells.get(owner).terrain;
                best = match terrain.contour_class(x_kernel, y_kernel, owner, saved)? {
                    TerrainKind::Sea => Feature::AtlasSea,
                    TerrainKind::Land => match best {
                        Feature::River(band) => Feature::River(band),
                        _ => Feature::Land,
                    },
                    TerrainKind::Lake => Feature::Lake,
                };
            } else if best == Feature::Sea {
                best = Feature::AtlasSea;
            }
        }
    }
    let colour = match (best, terrain) {
        (Feature::Sea | Feature::AtlasSea, Some(terrain)) => terrain.sample(
            axis_kernel(x, tile_width)?,
            axis_kernel(y, tile_height)?,
            TerrainKind::Sea,
        )?,
        (Feature::Land, Some(terrain)) => terrain.sample(
            axis_kernel(x, tile_width)?,
            axis_kernel(y, tile_height)?,
            TerrainKind::Land,
        )?,
        (Feature::Sea | Feature::AtlasSea, None) => OVERVIEW_SEA,
        // Include channel cells in the land mean to preserve the ground tint.
        (Feature::Land, None) => {
            land_colour(i32::try_from(height_sum / land_count.max(1)).unwrap_or(0))
        }
        (Feature::River(band), _) => river_band_colour(band),
        (Feature::Lake, _) => LAKE_FILL,
    };
    Ok((colour, best))
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
    use crate::{AtlasHalo, AtlasNeighbor, AtlasTerrain};
    use arda_core::{Cell, DischargeMilli, HeightMm};

    fn standalone_atlas(cells: &AreaCells) -> AtlasTerrain {
        let mut halo = AtlasHalo::new();
        for direction in [
            AtlasNeighbor::North,
            AtlasNeighbor::NorthEast,
            AtlasNeighbor::East,
            AtlasNeighbor::SouthEast,
            AtlasNeighbor::South,
            AtlasNeighbor::SouthWest,
            AtlasNeighbor::West,
            AtlasNeighbor::NorthWest,
        ] {
            halo.mark_world_edge(direction).unwrap();
        }
        AtlasTerrain::new(cells, halo).unwrap()
    }

    #[test]
    fn atlas_overview_mixed_axes_and_exact_centers() {
        let mut cells = AreaCells::flat(Cell {
            terrain: TerrainKind::Land,
            ..Cell::default()
        });
        for y in 0..AREA_CELLS {
            for x in 0..AREA_CELLS {
                cells.set(
                    CellCoord::new(x, y).unwrap(),
                    Cell {
                        height: HeightMm::new(i32::from(x) * 2_000 + i32::from(y) * 3_000),
                        terrain: TerrainKind::Land,
                        ..Cell::default()
                    },
                );
            }
        }
        let terrain = standalone_atlas(&cells);
        let mut mixed = OverviewRaster::new_exact(1, 1, 511, 513).unwrap();
        mixed
            .push_atlas(AreaCoord::new(0, 0), &cells, &terrain)
            .unwrap();
        for y in [0, 1, 255, 512] {
            for x in [0, 255, 510] {
                let pixel = usize::try_from((y * 511 + x) * 3).unwrap();
                assert_eq!(
                    &mixed.rgb[pixel..pixel + 3],
                    &terrain
                        .sample(
                            crate::atlas::axis_kernel(x, 511).unwrap(),
                            crate::atlas::axis_kernel(y, 513).unwrap(),
                            TerrainKind::Land
                        )
                        .unwrap()
                );
            }
        }
        let mut enlarged = OverviewRaster::new_exact(1, 1, 1536, 1536).unwrap();
        enlarged
            .push_atlas(AreaCoord::new(0, 0), &cells, &terrain)
            .unwrap();
        for (x, y) in [(0, 0), (255, 255), (511, 511)] {
            let pixel = ((y * 3 + 1) * 1536 + (x * 3 + 1)) * 3;
            assert_eq!(
                &enlarged.rgb[pixel..pixel + 3],
                &terrain.colour(
                    CellCoord::new(u16::try_from(x).unwrap(), u16::try_from(y).unwrap()).unwrap()
                )
            );
        }
    }

    #[test]
    fn atlas_overview_resolves_linear_shore_and_propagates_context_errors() {
        let mut cells = AreaCells::flat(Cell {
            height: HeightMm::new(-2000),
            terrain: TerrainKind::Sea,
            ..Cell::default()
        });
        for y in 0..AREA_CELLS {
            for x in 256..AREA_CELLS {
                cells.set(
                    CellCoord::new(x, y).unwrap(),
                    Cell {
                        height: HeightMm::new(200_000),
                        terrain: TerrainKind::Land,
                        ..Cell::default()
                    },
                );
            }
        }
        let terrain = standalone_atlas(&cells);
        let mut classic = OverviewRaster::new_exact(1, 1, 513, 513).unwrap();
        classic.push(AreaCoord::new(0, 0), &cells).unwrap();
        let mut atlas = OverviewRaster::new_exact(1, 1, 513, 513).unwrap();
        atlas
            .push_atlas(AreaCoord::new(0, 0), &cells, &terrain)
            .unwrap();
        assert_eq!(classic.features[256 * 513 + 255], Feature::Sea);
        for x in [255, 256, 257] {
            let owner = CellCoord::new(u16::try_from(x * 512 / 513).unwrap(), 255).unwrap();
            let x_kernel = crate::atlas::axis_kernel(u32::try_from(x).unwrap(), 513).unwrap();
            let y_kernel = crate::atlas::axis_kernel(256, 513).unwrap();
            let class = terrain
                .contour_class(x_kernel, y_kernel, owner, cells.get(owner).terrain)
                .unwrap();
            let i = (256 * 513 + x) * 3;
            assert_eq!(
                atlas.features[256 * 513 + x],
                if class == TerrainKind::Sea {
                    Feature::AtlasSea
                } else {
                    Feature::Land
                }
            );
            assert_eq!(
                &atlas.rgb[i..i + 3],
                &terrain.sample(x_kernel, y_kernel, class).unwrap()
            );
        }
        let wrong = AreaCells::flat(Cell {
            terrain: TerrainKind::Land,
            ..Cell::default()
        });
        let mut rejected = OverviewRaster::new_exact(1, 1, 512, 512).unwrap();
        assert!(matches!(
            rejected.push_atlas(AreaCoord::new(0, 0), &wrong, &terrain),
            Err(RenderError::AtlasContext { .. })
        ));
    }

    #[test]
    fn atlas_river_is_suppressed_where_the_contour_resolves_sea() {
        let mut cells = AreaCells::flat(Cell {
            terrain: TerrainKind::Sea,
            height: HeightMm::new(-100_000),
            ..Cell::default()
        });
        for y in 0..512 {
            for x in 101..512 {
                cells.set(
                    CellCoord::new(x, y).unwrap(),
                    Cell {
                        terrain: TerrainKind::Land,
                        height: HeightMm::new(1),
                        discharge: DischargeMilli::new(800_000),
                        ..Cell::default()
                    },
                );
            }
        }
        let terrain = standalone_atlas(&cells);
        let (_, feature) =
            sample_pixel(&cells, Some(&terrain), 4096, 4096, 101 * 8, 100 * 8).unwrap();
        assert_eq!(feature, Feature::AtlasSea);
        let (_, classic) = sample_pixel(&cells, None, 4096, 4096, 101 * 8, 100 * 8).unwrap();
        assert_eq!(classic, Feature::River(RiverBand::Dark));
    }

    #[test]
    fn linear_overview_and_area_use_the_same_shoreline_decision() {
        use crate::channels::{AreaImageScale, AreaRaster};
        use crate::{GlobalCell, ImageQuality};
        let mut cells = AreaCells::flat(Cell {
            terrain: TerrainKind::Sea,
            height: HeightMm::new(0),
            ..Cell::default()
        });
        for y in 0..512 {
            for x in 101..512 {
                cells.set(
                    CellCoord::new(x, y).unwrap(),
                    Cell {
                        terrain: TerrainKind::Land,
                        height: HeightMm::new(100_000),
                        ..Cell::default()
                    },
                );
            }
        }
        let terrain = standalone_atlas(&cells);
        for side in [2048, 8192] {
            let py = 100 * (side / 512) + 1;
            let px = 101 * (side / 512) - 1;
            let (overview_colour, feature) =
                sample_pixel(&cells, Some(&terrain), side, side, px, py).unwrap();
            assert_eq!(feature, Feature::Land);
            let mut area = AreaRaster::new_with_terrain(
                &cells,
                Some(&terrain),
                std::iter::empty(),
                GlobalCell { x: 0, y: 0 },
                AreaImageScale::Custom(ImageQuality::new(side).unwrap()),
                &[],
            )
            .unwrap();
            let row = area.row(usize::try_from(py).unwrap()).unwrap();
            let start = usize::try_from(px).unwrap() * 3;
            assert_eq!(&row[start..start + 3], &overview_colour);
        }
    }

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

    #[test]
    fn exact_dimensions_refuse_invalid_axes_and_pixel_budgets() {
        for (aw, ah, width, height) in [
            (0, 1, 1, 1),
            (1, -1, 1, 1),
            (79, 1, 79, 1),
            (1, 1, 0, 1),
            (1, 1, 1, 32_769),
            (1, 1, u32::MAX, 1),
            (3, 1, 2, 1),
            (1, 3, 1, 2),
            (1, 1, 16_384, 16_384),
        ] {
            assert!(matches!(
                OverviewRaster::new_exact(aw, ah, width, height),
                Err(RenderError::ExactOverviewDimensions)
            ));
        }
    }

    #[test]
    fn exact_long_axis_supports_32k_within_the_buffer_budget() {
        let raster = OverviewRaster::new_exact(1, 1, 32_768, 1).unwrap();
        assert_eq!((raster.width, raster.height), (32_768, 1));
    }

    #[test]
    fn exact_uneven_aspect_covers_every_pixel_without_area_gaps() {
        let mut canvas = OverviewRaster::new_exact(3, 2, 8, 5).unwrap();
        let heights = [0, 200_000, 500_000, 900_000, 1_400_000, 2_800_000];
        for (index, height) in heights.into_iter().enumerate().rev() {
            let cells = AreaCells::flat(Cell {
                height: HeightMm::new(height),
                terrain: TerrainKind::Land,
                ..Cell::default()
            });
            canvas
                .push(
                    AreaCoord::new(
                        i32::try_from(index % 3).unwrap(),
                        i32::try_from(index / 3).unwrap(),
                    ),
                    &cells,
                )
                .unwrap();
        }
        let png = canvas.finish().unwrap();
        let mut reader = png::Decoder::new(png.as_slice()).read_info().unwrap();
        let mut rgb = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut rgb).unwrap();
        assert_eq!((info.width, info.height), (8, 5));
        // Three areas occupy widths 2/3/3; two rows occupy heights 2/3.
        let columns = [0, 0, 1, 1, 1, 2, 2, 2];
        let rows = [0, 0, 1, 1, 1];
        for (y, row) in rows.into_iter().enumerate() {
            for (x, column) in columns.into_iter().enumerate() {
                let pixel = (y * 8 + x) * 3;
                assert_eq!(
                    &rgb[pixel..pixel + 3],
                    &land_colour(heights[row * 3 + column])
                );
            }
        }
    }

    #[test]
    fn exact_upscale_retains_last_source_row_and_column() {
        let mut cells = AreaCells::flat(Cell {
            height: HeightMm::new(200_000),
            terrain: TerrainKind::Land,
            ..Cell::default()
        });
        for offset in 0..AREA_CELLS {
            cells.set(
                CellCoord::new(AREA_CELLS - 1, offset).unwrap(),
                Cell {
                    terrain: TerrainKind::Lake,
                    ..Cell::default()
                },
            );
            cells.set(
                CellCoord::new(offset, AREA_CELLS - 1).unwrap(),
                Cell {
                    terrain: TerrainKind::Sea,
                    ..Cell::default()
                },
            );
        }
        let mut canvas = OverviewRaster::new_exact(1, 1, 513, 515).unwrap();
        canvas.push(AreaCoord::new(0, 0), &cells).unwrap();
        for y in 0..515 {
            for x in 0..513 {
                let want = if y == 514 {
                    OVERVIEW_SEA
                } else if x == 512 {
                    LAKE_FILL
                } else {
                    land_colour(200_000)
                };
                let pixel = (y * 513 + x) * 3;
                assert_eq!(&canvas.rgb[pixel..pixel + 3], &want);
            }
        }
    }

    #[test]
    fn exact_uniform_scale_preserves_existing_png_bytes() {
        let cells = AreaCells::flat(Cell {
            height: HeightMm::new(321_000),
            terrain: TerrainKind::Land,
            discharge: DischargeMilli::new(80_000),
            ..Cell::default()
        });
        let mut normal = OverviewRaster::new(2, 1, 48).unwrap();
        let mut exact = OverviewRaster::new_exact(2, 1, 96, 48).unwrap();
        for x in 0..2 {
            normal.push(AreaCoord::new(x, 0), &cells).unwrap();
            exact.push(AreaCoord::new(x, 0), &cells).unwrap();
        }
        assert_eq!(normal.finish().unwrap(), exact.finish().unwrap());
    }
}
