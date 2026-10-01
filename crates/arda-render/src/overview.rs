//! Incremental overview rendering from one saved area at a time.

use crate::carto::{river_band, RiverBand, OVERVIEW_SEA};
use crate::{AtlasTerrain, RenderError};
use arda_core::{AreaCells, AreaCoord};

mod channel_overlay;
mod oblique;
mod oblique_rows;
mod rivers;
mod sampling;
mod streaming;
pub(crate) use channel_overlay::formed_river_colour;
pub use channel_overlay::OverviewChannelContext;
pub use oblique::{ObliqueRelief, ObliqueReliefBuilder, DEFAULT_TILT_Q12};
use rivers::{atlas_river_colour, atlas_river_halo, atlas_river_size, style_river_band};
use sampling::{sample_atlas_channel_base_pixel, sample_pixel};
pub use streaming::{
    write_atlas_overview_png, write_atlas_overview_png_with_channels,
    write_atlas_overview_png_with_channels_formed, write_atlas_overview_png_with_channels_oblique,
    write_atlas_overview_png_with_channels_recipe4, write_overview_png,
};

#[derive(Clone, Copy)]
enum ChannelStyle {
    Legacy,
    Fine,
    /// Recipe 5 (logic/04 §atlas-formed rivers).
    Formed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Feature {
    Sea,
    AtlasSea,
    Land,
    AtlasLand,
    River(RiverBand),
    AtlasRiver(RiverBand, u8),
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
    channel_mode: Option<bool>,
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
            channel_mode: None,
        })
    }

    /// Incorporates one area; input order cannot affect the image.
    ///
    /// # Errors
    /// Refuses out-of-range or duplicate areas.
    pub fn push(&mut self, at: AreaCoord, cells: &AreaCells) -> Result<(), RenderError> {
        self.push_inner(at, cells, None, None)
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
        self.push_inner(at, cells, Some(terrain), None)
    }

    /// Incorporates one Atlas area with saved, neighbor-complete connected channel geometry.
    ///
    /// # Errors
    /// Refuses duplicate or out-of-range areas, incomplete channel context, and bounded geometry failures.
    pub fn push_atlas_with_channels(
        &mut self,
        at: AreaCoord,
        cells: &AreaCells,
        terrain: &AtlasTerrain,
        channels: &OverviewChannelContext,
    ) -> Result<(), RenderError> {
        self.push_inner(at, cells, Some(terrain), Some(channels))
    }

    fn push_inner(
        &mut self,
        at: AreaCoord,
        cells: &AreaCells,
        terrain: Option<&AtlasTerrain>,
        channels: Option<&OverviewChannelContext>,
    ) -> Result<(), RenderError> {
        if self
            .channel_mode
            .is_some_and(|mode| mode != channels.is_some())
        {
            return Err(RenderError::AtlasContext {
                reason: "connected Atlas overview areas cannot mix with legacy area pushes",
            });
        }
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
                let (colour, best) = match (terrain, channels) {
                    (Some(terrain), Some(_)) => sample_atlas_channel_base_pixel(
                        cells,
                        terrain,
                        tile_width,
                        tile_height,
                        pxi,
                        py,
                    )?,
                    _ => sample_pixel(cells, terrain, tile_width, tile_height, pxi, py)?,
                };

                let x = output_x0 + pxi;
                let y = output_y0 + py;
                let Ok(pixel) = usize::try_from(y * width + x) else {
                    continue;
                };
                self.rgb[pixel * 3..pixel * 3 + 3].copy_from_slice(&colour);
                self.features[pixel] = best;
            }
        }
        if let Some(channels) = channels {
            let mut tile = channel_overlay::ChannelTile::new(
                channels,
                at,
                [output_x0, output_x1, output_y0, output_y1],
                [self.width, self.height],
                ChannelStyle::Legacy,
            )?;
            for y in output_y0..output_y1 {
                let row =
                    usize::try_from(y * width).map_err(|_| RenderError::ExactOverviewDimensions)?;
                let x0 =
                    usize::try_from(output_x0).map_err(|_| RenderError::ExactOverviewDimensions)?;
                let x1 =
                    usize::try_from(output_x1).map_err(|_| RenderError::ExactOverviewDimensions)?;
                tile.blend_row(
                    y,
                    &mut self.rgb[(row + x0) * 3..(row + x1) * 3],
                    &self.features[row + x0..row + x1],
                )?;
            }
        }
        self.supplied[offset] = true;
        self.channel_mode = Some(channels.is_some());
        Ok(())
    }

    /// Applies final river styling and encodes the overview.
    ///
    /// Areas not supplied remain ocean; stored-world exports supply every area
    /// and propagate missing-layer errors before calling this method.
    ///
    /// # Errors
    /// Returns the PNG encoding error.
    pub fn finish(mut self) -> Result<Vec<u8>, RenderError> {
        let (width, height) = (self.width, self.height);
        let row_width = usize::try_from(width).map_err(|_| RenderError::ExactOverviewDimensions)?;
        let halo = atlas_river_halo(width, height);
        let mut y0 = 0;
        while y0 < height {
            let y1 = (y0 + 256).min(height);
            let data_y0 = y0.saturating_sub(halo);
            let data_y1 = (y1 + halo).min(height);
            let start = usize::try_from(data_y0)
                .map_err(|_| RenderError::ExactOverviewDimensions)?
                * row_width;
            let end = usize::try_from(data_y1).map_err(|_| RenderError::ExactOverviewDimensions)?
                * row_width;
            style_river_band(
                width,
                height,
                data_y0,
                y0,
                y1,
                &self.features[start..end],
                &mut self.rgb[start * 3..end * 3],
            )?;
            y0 = y1;
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
mod tests;
