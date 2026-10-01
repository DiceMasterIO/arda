//! Bounded-memory exact overview encoding.

use rayon::prelude::*;

use super::oblique::ObliqueRelief;
use super::oblique_rows::ObliqueRows;
use super::{
    atlas_river_halo, channel_overlay::ChannelTile, sample_atlas_channel_base_pixel, sample_pixel,
    style_river_band, validate_exact_dimensions, ChannelStyle, Feature, OverviewChannelContext,
};
use crate::{AtlasTerrain, RenderError};
use arda_core::{AreaCells, AreaCoord};
use std::io::Write;

const BAND_ROWS: u32 = 256;

fn band_height(height: u32) -> u32 {
    height.min(BAND_ROWS)
}

/// Encodes an exact-size overview without retaining the complete image.
///
/// Each axis supports 1–32,768 pixels and 1–78 areas, with at least one
/// pixel per area per axis. Image memory is limited to 256 rows of RGB and
/// feature classifications, at most 16 halo rows each side, one reusable
/// distance field, and one loaded area.
/// All saved areas are required. `load_area` may load an area more than once
/// when it intersects multiple bands and must return the same saved cells.
///
/// Pixel colours match [`super::OverviewRaster`], including river widening
/// across area and band boundaries. Streaming uses separate PNG chunks, so
/// encoded bytes may differ from the buffered API for the same pixels.
///
/// # Errors
/// Returns dimension, loading, writer or PNG errors. A failure may leave a
/// partial PNG in `output`; file callers should write to a temporary file.
pub fn write_overview_png<W, F, E>(
    areas_wide: i32,
    areas_high: i32,
    width: u32,
    height: u32,
    output: W,
    mut load_area: F,
) -> Result<(), E>
where
    W: Write,
    F: FnMut(AreaCoord) -> Result<AreaCells, E>,
    E: From<RenderError>,
{
    write_overview_png_inner(
        areas_wide,
        areas_high,
        width,
        height,
        output,
        (ChannelStyle::Legacy, None),
        move |at| load_area(at).map(|cells| (cells, None, None)),
    )
}

/// Encodes an Atlas overview in bounded 256-row bands.
///
/// The callback supplies saved cells and their validated neighboring terrain
/// context for each intersecting area; it may be called again across bands.
///
/// # Errors
/// Returns dimension, loading, atlas-context, writer or PNG errors.
pub fn write_atlas_overview_png<W, F, E>(
    areas_wide: i32,
    areas_high: i32,
    width: u32,
    height: u32,
    output: W,
    mut load_area: F,
) -> Result<(), E>
where
    W: Write,
    F: FnMut(AreaCoord) -> Result<(AreaCells, AtlasTerrain), E>,
    E: From<RenderError>,
{
    write_overview_png_inner(
        areas_wide,
        areas_high,
        width,
        height,
        output,
        (ChannelStyle::Legacy, None),
        move |at| load_area(at).map(|(cells, terrain)| (cells, Some(terrain), None)),
    )
}

/// Encodes an Atlas overview with saved, neighbor-complete channel geometry.
///
/// The callback may be called again across 256-row bands and must return the
/// same saved cells, terrain, and canonical channel context for a given area.
///
/// # Errors
/// Returns typed context, geometry, dimension, loading, writer, or PNG errors.
pub fn write_atlas_overview_png_with_channels<W, F, E>(
    areas_wide: i32,
    areas_high: i32,
    width: u32,
    height: u32,
    output: W,
    mut load_area: F,
) -> Result<(), E>
where
    W: Write,
    F: FnMut(AreaCoord) -> Result<(AreaCells, AtlasTerrain, OverviewChannelContext), E>,
    E: From<RenderError>,
{
    write_atlas_overview_png_with_channels_inner(
        areas_wide,
        areas_high,
        width,
        height,
        output,
        (ChannelStyle::Legacy, None),
        &mut load_area,
    )
}

/// Encodes the narrower, translucent channel symbol used by recipe 4 fine worlds.
///
/// Callers must select this only for a saved fine-terrain recipe 4 world.
/// The ordinary channel writer retains the original overview symbol.
///
/// # Errors
/// Returns typed context, geometry, dimension, loading, writer, or PNG errors.
pub fn write_atlas_overview_png_with_channels_recipe4<W, F, E>(
    areas_wide: i32,
    areas_high: i32,
    width: u32,
    height: u32,
    output: W,
    mut load_area: F,
) -> Result<(), E>
where
    W: Write,
    F: FnMut(AreaCoord) -> Result<(AreaCells, AtlasTerrain, OverviewChannelContext), E>,
    E: From<RenderError>,
{
    write_atlas_overview_png_with_channels_inner(
        areas_wide,
        areas_high,
        width,
        height,
        output,
        (ChannelStyle::Fine, None),
        &mut load_area,
    )
}

/// Recipe-5 Atlas overview: formed land shading and river symbols at four
/// times physical width (logic/04 §atlas-formed rivers).
///
/// # Errors
/// As [`write_atlas_overview_png_with_channels_recipe4`].
pub fn write_atlas_overview_png_with_channels_formed<W, F, E>(
    areas_wide: i32,
    areas_high: i32,
    width: u32,
    height: u32,
    output: W,
    mut load_area: F,
) -> Result<(), E>
where
    W: Write,
    F: FnMut(AreaCoord) -> Result<(AreaCells, AtlasTerrain, OverviewChannelContext), E>,
    E: From<RenderError>,
{
    write_atlas_overview_png_with_channels_inner(
        areas_wide,
        areas_high,
        width,
        height,
        output,
        (ChannelStyle::Formed, None),
        &mut load_area,
    )
}

/// The recipe-5 Atlas overview of
/// [`write_atlas_overview_png_with_channels_formed`] seen slightly obliquely
/// (goal 24, opt-in): rows are warped north by the surface height and
/// shaded with aerial perspective, valley occlusion and sky light
/// ([`ObliqueRelief`]). Memory stays bounded: only the rows the warp
/// reaches are held.
///
/// # Errors
/// As [`write_atlas_overview_png_with_channels_formed`].
pub fn write_atlas_overview_png_with_channels_oblique<W, F, E>(
    areas_wide: i32,
    areas_high: i32,
    (width, height): (u32, u32),
    output: W,
    relief: &ObliqueRelief,
    mut load_area: F,
) -> Result<(), E>
where
    W: Write,
    F: FnMut(AreaCoord) -> Result<(AreaCells, AtlasTerrain, OverviewChannelContext), E>,
    E: From<RenderError>,
{
    write_atlas_overview_png_with_channels_inner(
        areas_wide,
        areas_high,
        width,
        height,
        output,
        (ChannelStyle::Formed, Some(relief)),
        &mut load_area,
    )
}

fn write_atlas_overview_png_with_channels_inner<W, F, E>(
    areas_wide: i32,
    areas_high: i32,
    width: u32,
    height: u32,
    output: W,
    style: (ChannelStyle, Option<&ObliqueRelief>),
    mut load_area: F,
) -> Result<(), E>
where
    W: Write,
    F: FnMut(AreaCoord) -> Result<(AreaCells, AtlasTerrain, OverviewChannelContext), E>,
    E: From<RenderError>,
{
    write_overview_png_inner(
        areas_wide,
        areas_high,
        width,
        height,
        output,
        style,
        move |at| {
            load_area(at).map(|(cells, terrain, channels)| (cells, Some(terrain), Some(channels)))
        },
    )
}

fn write_overview_png_inner<W, F, E>(
    areas_wide: i32,
    areas_high: i32,
    width: u32,
    height: u32,
    output: W,
    (style, look): (ChannelStyle, Option<&ObliqueRelief>),
    mut load_area: F,
) -> Result<(), E>
where
    W: Write,
    F: FnMut(
        AreaCoord,
    ) -> Result<
        (
            AreaCells,
            Option<AtlasTerrain>,
            Option<OverviewChannelContext>,
        ),
        E,
    >,
    E: From<RenderError>,
{
    validate_exact_dimensions(areas_wide, areas_high, width, height)?;
    let world_width =
        u32::try_from(areas_wide).map_err(|_| RenderError::ExactOverviewDimensions)?;
    let world_height =
        u32::try_from(areas_high).map_err(|_| RenderError::ExactOverviewDimensions)?;
    crate::encode_png_rows(width, height, output, |stream| {
        // The oblique look (goal 24) is opt-in; without it rows go straight
        // to the encoder, exactly as before.
        let mut oblique = None;
        let writer: &mut dyn Write = match look {
            Some(relief) => oblique.insert(ObliqueRows::new(relief, width, height, stream)),
            None => stream,
        };
        let mut band = RasterBand::new(width, height)?;
        let halo = atlas_river_halo(width, height);
        let rows_per_band = band_height(height);
        let mut band_y0 = 0;
        while band_y0 < height {
            let band_y1 = (band_y0 + rows_per_band).min(height);
            band.y0 = band_y0.saturating_sub(halo);
            band.y1 = (band_y1 + halo).min(height);
            for area_y in 0..areas_high {
                let y = u32::try_from(area_y).map_err(|_| RenderError::ExactOverviewDimensions)?;
                let tile_y0 = y * height / world_height;
                let tile_y1 = (y + 1) * height / world_height;
                if tile_y1 <= band.y0 || tile_y0 >= band.y1 {
                    continue;
                }
                for area_x in 0..areas_wide {
                    let x =
                        u32::try_from(area_x).map_err(|_| RenderError::ExactOverviewDimensions)?;
                    let (cells, terrain, channels) = load_area(AreaCoord::new(area_x, area_y))?;
                    band.render_tile(
                        [
                            x * width / world_width,
                            (x + 1) * width / world_width,
                            tile_y0,
                            tile_y1,
                        ],
                        &cells,
                        terrain.as_ref(),
                        channels.as_ref(),
                        AreaCoord::new(area_x, area_y),
                        style,
                    )?;
                }
            }
            let pixels = band.pixel_count()?;
            style_river_band(
                width,
                height,
                band.y0,
                band_y0,
                band_y1,
                &band.features[..pixels],
                &mut band.rgb[..pixels * 3],
            )?;
            let start = usize::try_from(band_y0 - band.y0)
                .map_err(|_| RenderError::ExactOverviewDimensions)?
                * band.width
                * 3;
            let end = usize::try_from(band_y1 - band.y0)
                .map_err(|_| RenderError::ExactOverviewDimensions)?
                * band.width
                * 3;
            writer
                .write_all(&band.rgb[start..end])
                .map_err(|_| RenderError::Png)?;
            band_y0 = band_y1;
        }
        if let Some(rows) = oblique {
            rows.finish().map_err(|_| RenderError::Png)?;
        }
        Ok(())
    })
}

struct RasterBand {
    width: usize,
    height: u32,
    y0: u32,
    y1: u32,
    rgb: Vec<u8>,
    features: Vec<Feature>,
}

impl RasterBand {
    fn new(width: u32, height: u32) -> Result<Self, RenderError> {
        let halo = atlas_river_halo(width, height);
        let rows = (band_height(height) + halo * 2).min(height);
        let pixels =
            usize::try_from(width * rows).map_err(|_| RenderError::ExactOverviewDimensions)?;
        Ok(Self {
            width: usize::try_from(width).map_err(|_| RenderError::ExactOverviewDimensions)?,
            height,
            y0: 0,
            y1: 0,
            rgb: vec![0; pixels * 3],
            features: vec![Feature::Sea; pixels],
        })
    }

    fn render_tile(
        &mut self,
        bounds: [u32; 4],
        cells: &AreaCells,
        terrain: Option<&AtlasTerrain>,
        channels: Option<&OverviewChannelContext>,
        at: AreaCoord,
        style: ChannelStyle,
    ) -> Result<(), RenderError> {
        let [x0, x1, y0, y1] = bounds;
        let tile_width = x1 - x0;
        let tile_height = y1 - y0;
        let output_x0 = usize::try_from(x0).map_err(|_| RenderError::ExactOverviewDimensions)?;
        // Pixels are pure functions of saved data, so rows render in
        // parallel and the image is identical for any thread count.
        let ys: Vec<u32> = (y0.max(self.y0)..y1.min(self.y1)).collect();
        let rows: Vec<Vec<([u8; 3], Feature)>> = ys
            .par_iter()
            .map(|&y| {
                (0..tile_width)
                    .map(|x| match (terrain, channels) {
                        (Some(terrain), Some(_)) => sample_atlas_channel_base_pixel(
                            cells,
                            terrain,
                            tile_width,
                            tile_height,
                            x,
                            y - y0,
                        ),
                        _ => sample_pixel(cells, terrain, tile_width, tile_height, x, y - y0),
                    })
                    .collect::<Result<Vec<_>, RenderError>>()
            })
            .collect::<Result<_, RenderError>>()?;
        for (&y, samples) in ys.iter().zip(rows) {
            let row =
                usize::try_from(y - self.y0).map_err(|_| RenderError::ExactOverviewDimensions)?;
            for (pixel, (colour, feature)) in (row * self.width + output_x0..).zip(samples) {
                self.rgb[pixel * 3..pixel * 3 + 3].copy_from_slice(&colour);
                self.features[pixel] = feature;
            }
        }
        if let Some(channels) = channels {
            let mut tile = ChannelTile::new_with_terrain(
                channels,
                at,
                [x0, x1, y0, y1],
                [
                    u32::try_from(self.width).map_err(|_| RenderError::ExactOverviewDimensions)?,
                    self.height,
                ],
                style,
                terrain,
            )?;
            for y in y0.max(self.y0)..y1.min(self.y1) {
                let row = usize::try_from(y - self.y0)
                    .map_err(|_| RenderError::ExactOverviewDimensions)?
                    * self.width;
                let x0 = usize::try_from(x0).map_err(|_| RenderError::ExactOverviewDimensions)?;
                let x1 = usize::try_from(x1).map_err(|_| RenderError::ExactOverviewDimensions)?;
                tile.blend_row(
                    y,
                    &mut self.rgb[(row + x0) * 3..(row + x1) * 3],
                    &self.features[row + x0..row + x1],
                )?;
            }
        }
        Ok(())
    }

    fn pixel_count(&self) -> Result<usize, RenderError> {
        let rows =
            usize::try_from(self.y1 - self.y0).map_err(|_| RenderError::ExactOverviewDimensions)?;
        Ok(rows * self.width)
    }
}

#[cfg(test)]
mod tests;
