//! Incremental overview rendering from one saved area at a time.

use crate::atlas::axis_kernel;
use crate::carto::{
    land_colour, river_band, river_band_colour, RiverBand, LAKE_FILL, LAKE_MIN_BLOCK_DEN,
    OVERVIEW_SEA,
};
use crate::{AtlasTerrain, RenderError};
use arda_core::{AreaCells, AreaCoord, CellCoord, TerrainKind, AREA_CELLS};

mod channel_overlay;
mod streaming;
pub(crate) use channel_overlay::formed_river_colour;
pub use channel_overlay::OverviewChannelContext;
pub use streaming::{
    write_atlas_overview_png, write_atlas_overview_png_with_channels,
    write_atlas_overview_png_with_channels_formed, write_atlas_overview_png_with_channels_recipe4,
    write_overview_png,
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

/// Map saved channel width to a bounded cartographic radius at 32K. The floor
/// keeps narrow rivers visible when the whole world is fitted to a screen.
/// It is a symbol, not a claim that overview pixels are metres across.
fn atlas_river_size(width_dm: u32, band: RiverBand) -> u8 {
    match band {
        RiverBand::Light => 0,
        RiverBand::Mid => 4 + (width_dm / 150).min(3) as u8,
        RiverBand::Dark => 7 + (width_dm / 180).min(6) as u8,
    }
}

fn atlas_river_radius(width: u32, height: u32, size: u8) -> u32 {
    let screen_scale = width.max(height).div_ceil(2048);
    (u32::from(size) * screen_scale).div_ceil(16)
}

fn atlas_river_halo(width: u32, height: u32) -> u32 {
    atlas_river_radius(width, height, 13) + 1
}

const fn atlas_river_colour(band: RiverBand) -> [u8; 3] {
    match band {
        RiverBand::Light => [91, 153, 171],
        RiverBand::Mid => [60, 135, 162],
        RiverBand::Dark => [32, 98, 132],
    }
}

const fn atlas_river_edge_colour(band: RiverBand) -> [u8; 3] {
    match band {
        RiverBand::Light => [91, 153, 171],
        RiverBand::Mid => [80, 151, 169],
        RiverBand::Dark => [66, 141, 162],
    }
}

/// Cartographic cross-section shading from the same signed distance as the
/// bank coverage. This describes apparent water, not measured channel depth.
fn atlas_river_water_colour(band: RiverBand, distance: i16, scale: i16) -> [u8; 3] {
    let edge = atlas_river_edge_colour(band);
    let core = atlas_river_colour(band);
    let inner_pixels = match band {
        RiverBand::Light => 1,
        RiverBand::Mid => 3,
        RiverBand::Dark => 5,
    };
    let span = i32::from(scale) * inner_pixels * 3;
    let shade =
        u32::try_from(((-i32::from(distance) - 24) * 256 / span).clamp(0, 256)).unwrap_or(0);
    std::array::from_fn(|channel| {
        u8::try_from(
            (u32::from(edge[channel]) * (256 - shade) + u32::from(core[channel]) * shade + 128)
                / 256,
        )
        .unwrap_or(255)
    })
}

/// Styles a complete output band from source features with a small vertical
/// halo. The 3/4 chamfer metric keeps centered river widths nearly round while
/// requiring one reusable distance field and two linear sweeps.
fn style_river_band(
    width: u32,
    height: u32,
    data_y0: u32,
    output_y0: u32,
    output_y1: u32,
    features: &[Feature],
    rgb: &mut [u8],
) -> Result<(), RenderError> {
    let w = usize::try_from(width).map_err(|_| RenderError::ExactOverviewDimensions)?;
    if w == 0 || !features.len().is_multiple_of(w) || rgb.len() != features.len() * 3 {
        return Err(RenderError::ExactOverviewDimensions);
    }
    let data_rows = features.len() / w;
    let data_y1 = data_y0
        .checked_add(u32::try_from(data_rows).map_err(|_| RenderError::ExactOverviewDimensions)?)
        .ok_or(RenderError::ExactOverviewDimensions)?;
    if data_y1 > height || output_y0 < data_y0 || output_y1 > data_y1 || output_y0 > output_y1 {
        return Err(RenderError::ExactOverviewDimensions);
    }
    let mut distances = Vec::new();
    for band in [RiverBand::Mid, RiverBand::Dark] {
        if !features
            .iter()
            .any(|feature| matches!(feature, Feature::AtlasRiver(b, _) if *b == band))
        {
            continue;
        }
        if distances.len() != features.len() {
            distances.resize(features.len(), i16::MAX / 2);
        }
        distances.fill(i16::MAX / 2);
        // A negative seed radius turns the same chamfer transform into a
        // tapered outline: wider saved channel cells extend farther out.
        let scale = i16::try_from(width.max(height).div_ceil(2048))
            .map_err(|_| RenderError::ExactOverviewDimensions)?;
        for (i, feature) in features.iter().enumerate() {
            if let Feature::AtlasRiver(source_band, size) = *feature {
                if source_band == band {
                    // Chamfer units are 1/16 pixel, preserving width
                    // differences even in a direct 2K world overview.
                    distances[i] = -(i16::from(size) * scale * 3);
                }
            }
        }
        for y in 0..data_rows {
            for x in 0..w {
                let i = y * w + x;
                let mut d = distances[i];
                if x > 0 {
                    d = d.min(distances[i - 1].saturating_add(48));
                }
                if y > 0 {
                    d = d.min(distances[i - w].saturating_add(48));
                    if x > 0 {
                        d = d.min(distances[i - w - 1].saturating_add(64));
                    }
                    if x + 1 < w {
                        d = d.min(distances[i - w + 1].saturating_add(64));
                    }
                }
                distances[i] = d;
            }
        }
        for y in (0..data_rows).rev() {
            for x in (0..w).rev() {
                let i = y * w + x;
                let mut d = distances[i];
                if x + 1 < w {
                    d = d.min(distances[i + 1].saturating_add(48));
                }
                if y + 1 < data_rows {
                    d = d.min(distances[i + w].saturating_add(48));
                    if x > 0 {
                        d = d.min(distances[i + w - 1].saturating_add(64));
                    }
                    if x + 1 < w {
                        d = d.min(distances[i + w + 1].saturating_add(64));
                    }
                }
                distances[i] = d;
            }
        }
        for y in output_y0..output_y1 {
            let row =
                usize::try_from(y - data_y0).map_err(|_| RenderError::ExactOverviewDimensions)?;
            for x in 0..w {
                let i = row * w + x;
                let eligible = match band {
                    RiverBand::Mid => matches!(
                        features[i],
                        Feature::AtlasLand
                            | Feature::AtlasRiver(RiverBand::Light | RiverBand::Mid, _)
                    ),
                    RiverBand::Dark => {
                        matches!(features[i], Feature::AtlasLand | Feature::AtlasRiver(_, _))
                    }
                    RiverBand::Light => false,
                };
                if !eligible {
                    continue;
                }
                // A one-pixel coverage ramp softens the bank. Fractional
                // radius also changes source-pixel coverage in small maps.
                let alpha =
                    u16::try_from(((24 - i32::from(distances[i])) * 256 / 48).clamp(0, 256))
                        .unwrap_or(0);
                if alpha > 0 {
                    let colour = atlas_river_water_colour(band, distances[i], scale);
                    for (background, foreground) in rgb[i * 3..i * 3 + 3].iter_mut().zip(colour) {
                        *background = ((u32::from(*background) * u32::from(256 - alpha)
                            + u32::from(foreground) * u32::from(alpha)
                            + 128)
                            / 256)
                            .try_into()
                            .unwrap_or(255);
                    }
                }
            }
        }
    }
    // Classic retains its original right/down one-pixel trunk symbol.
    let dark = Feature::River(RiverBand::Dark);
    let colour = river_band_colour(RiverBand::Dark);
    for y in output_y0..output_y1 {
        let row = usize::try_from(y - data_y0).map_err(|_| RenderError::ExactOverviewDimensions)?;
        for x in 0..w {
            let i = row * w + x;
            if matches!(features[i], Feature::Land | Feature::Sea)
                && ((x > 0 && features[i - 1] == dark) || (row > 0 && features[i - w] == dark))
            {
                rgb[i * 3..i * 3 + 3].copy_from_slice(&colour);
            }
        }
    }
    Ok(())
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

fn sample_atlas_channel_base_pixel(
    cells: &AreaCells,
    terrain: &AtlasTerrain,
    tile_width: u32,
    tile_height: u32,
    x: u32,
    y: u32,
) -> Result<([u8; 3], Feature), RenderError> {
    let (colour, feature) = sample_pixel(cells, Some(terrain), tile_width, tile_height, x, y)?;
    if matches!(feature, Feature::AtlasRiver(_, _)) {
        let ground = terrain.sample(
            axis_kernel(x, tile_width)?,
            axis_kernel(y, tile_height)?,
            TerrainKind::Land,
        )?;
        Ok((ground, Feature::AtlasLand))
    } else {
        Ok((colour, feature))
    }
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
    let mut best_width_dm = 0u32;
    let mut best_discharge = arda_core::DischargeMilli::new(0);
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
                    if feature > best {
                        best = feature;
                        best_width_dm = cell.watercourse_width_dm;
                        best_discharge = cell.discharge;
                    } else if feature == best {
                        best_width_dm = best_width_dm.max(cell.watercourse_width_dm);
                        best_discharge = best_discharge.max(cell.discharge);
                    }
                }
            }
        }
    }
    if lake_cells * LAKE_MIN_BLOCK_DEN >= block_cells {
        best = Feature::Lake;
    }
    // Saved worlds carry width. The fallback serves synthetic Atlas callers
    // whose channel cells set discharge but leave width at zero. Overflow in
    // physical width means the largest display symbol, not the smallest.
    if terrain.is_some() && matches!(best, Feature::River(_)) && best_width_dm == 0 {
        best_width_dm = arda_core::hydrology::channel_width_dm(best_discharge).unwrap_or(u32::MAX);
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
                        Feature::River(band) => {
                            Feature::AtlasRiver(band, atlas_river_size(best_width_dm, band))
                        }
                        _ => Feature::AtlasLand,
                    },
                    TerrainKind::Lake => Feature::Lake,
                };
            } else {
                best = match best {
                    Feature::Sea => Feature::AtlasSea,
                    Feature::Land => Feature::AtlasLand,
                    Feature::River(band) => {
                        Feature::AtlasRiver(band, atlas_river_size(best_width_dm, band))
                    }
                    other => other,
                };
            }
        }
    }
    let colour = match (best, terrain) {
        (Feature::Sea | Feature::AtlasSea, Some(terrain)) => terrain.sample(
            axis_kernel(x, tile_width)?,
            axis_kernel(y, tile_height)?,
            TerrainKind::Sea,
        )?,
        (Feature::Land | Feature::AtlasLand, Some(terrain)) => terrain.sample(
            axis_kernel(x, tile_width)?,
            axis_kernel(y, tile_height)?,
            TerrainKind::Land,
        )?,
        (Feature::Sea | Feature::AtlasSea, None) => OVERVIEW_SEA,
        // Include channel cells in the land mean to preserve the ground tint.
        (Feature::Land | Feature::AtlasLand, None) => {
            land_colour(i32::try_from(height_sum / land_count.max(1)).unwrap_or(0))
        }
        (Feature::River(band), _) => river_band_colour(band),
        (Feature::AtlasRiver(RiverBand::Light, _), _) => atlas_river_colour(RiverBand::Light),
        (Feature::AtlasRiver(_, _), Some(terrain)) => terrain.sample(
            axis_kernel(x, tile_width)?,
            axis_kernel(y, tile_height)?,
            TerrainKind::Land,
        )?,
        (Feature::AtlasRiver(_, _), None) => {
            land_colour(i32::try_from(height_sum / land_count.max(1)).unwrap_or(0))
        }
        (Feature::Lake, Some(terrain)) if terrain.has_lake_depths() => terrain.sample(
            axis_kernel(x, tile_width)?,
            axis_kernel(y, tile_height)?,
            TerrainKind::Lake,
        )?,
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
                    Feature::AtlasLand
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
    fn atlas_river_symbols_taper_and_blend_at_32k_without_crossing_masks() {
        let width = 32_768usize;
        let data_y0 = 239u32;
        let data_y1 = 273u32;
        assert_eq!(
            atlas_river_radius(u32::try_from(width).unwrap(), 2048, 13),
            13
        );
        assert_eq!(
            atlas_river_radius(u32::try_from(width).unwrap(), 2048, 7),
            7
        );
        assert!(atlas_river_size(2_000, RiverBand::Dark) > atlas_river_size(360, RiverBand::Dark));
        let mut features = vec![Feature::AtlasLand; width * (data_y1 - data_y0) as usize];
        let source = (255 - data_y0) as usize * width;
        features[source + 100] = Feature::AtlasRiver(RiverBand::Dark, 13);
        features[source + 200] = Feature::AtlasRiver(RiverBand::Mid, 7);
        let guarded = (256 - data_y0) as usize * width;
        features[guarded + 101] = Feature::AtlasSea;
        features[guarded + 102] = Feature::Lake;
        features[guarded + 103] = Feature::Land;
        features[guarded + 104] = Feature::AtlasRiver(RiverBand::Light, 0);
        let mut rgb = vec![100; features.len() * 3];
        style_river_band(
            u32::try_from(width).unwrap(),
            2048,
            data_y0,
            239,
            273,
            &features,
            &mut rgb,
        )
        .unwrap();
        let at = |x: usize, y: usize| -> &[u8] {
            let i = ((y - data_y0 as usize) * width + x) * 3;
            &rgb[i..i + 3]
        };
        let dark = atlas_river_colour(RiverBand::Dark);
        assert_eq!(at(100, 255), &dark);
        assert!(at(112, 255)[0] > dark[0]);
        assert!(at(112, 255)[0] < atlas_river_edge_colour(RiverBand::Dark)[0]);
        assert_ne!(at(113, 255), &dark);
        assert_ne!(at(113, 255), &[100; 3]);
        assert_eq!(at(114, 255), &[100; 3]);
        assert_eq!(at(200, 255), &atlas_river_colour(RiverBand::Mid));
        assert_eq!(at(207, 255), &[90, 126, 135]);
        assert_eq!(at(208, 255), &[100; 3]);
        for x in [101, 102, 103] {
            assert_eq!(at(x, 256), &[100; 3], "protected surface at {x}");
        }
        assert_eq!(at(104, 256), &dark, "dark trunk wins over light stream");
    }

    #[test]
    fn zero_width_atlas_fallback_saturates_extreme_discharge() {
        let cells = AreaCells::flat(Cell {
            terrain: TerrainKind::Land,
            discharge: DischargeMilli::new(u64::MAX),
            ..Cell::default()
        });
        let terrain = standalone_atlas(&cells);
        let (_, feature) = sample_pixel(&cells, Some(&terrain), 512, 512, 0, 0).unwrap();
        assert_eq!(feature, Feature::AtlasRiver(RiverBand::Dark, 13));
    }

    #[test]
    fn saved_width_changes_coverage_in_a_direct_2k_overview() {
        let mut features = vec![Feature::AtlasLand; 32];
        features[5] = Feature::AtlasRiver(RiverBand::Mid, 5);
        features[20] = Feature::AtlasRiver(RiverBand::Mid, 7);
        let mut rgb = vec![100; 32 * 3];
        style_river_band(32, 2048, 0, 0, 1, &features, &mut rgb).unwrap();
        let narrow = &rgb[5 * 3..5 * 3 + 3];
        let wide = &rgb[20 * 3..20 * 3 + 3];
        let blue = atlas_river_colour(RiverBand::Mid);
        assert!(narrow[0] > wide[0]);
        assert!(wide[0] > blue[0]);
        assert_eq!(&rgb[2 * 3..2 * 3 + 3], &[100; 3]);
    }

    #[test]
    fn classic_trunk_still_widens_only_right_and_down() {
        let mut features = vec![Feature::Land; 12];
        features[1] = Feature::River(RiverBand::Dark);
        let mut rgb = vec![100; 36];
        style_river_band(4, 3, 0, 0, 3, &features, &mut rgb).unwrap();
        let at = |x: usize, y: usize| -> &[u8] {
            let i = (y * 4 + x) * 3;
            &rgb[i..i + 3]
        };
        let dark = river_band_colour(RiverBand::Dark);
        assert_eq!(at(2, 0), &dark);
        assert_eq!(at(1, 1), &dark);
        assert_eq!(at(2, 1), &[100; 3]);
    }

    #[test]
    fn tapered_symbols_match_chamfer_oracle_across_256_row_bands() {
        const W: usize = 64;
        const ROWS: usize = 528;
        let mut features = vec![Feature::AtlasLand; W * ROWS];
        features[255 * W + 24] = Feature::AtlasRiver(RiverBand::Dark, 13);
        features[257 * W + 48] = Feature::AtlasRiver(RiverBand::Mid, 7);
        features[256 * W + 25] = Feature::AtlasSea;
        features[258 * W + 24] = Feature::Lake;
        features[255 * W + 26] = Feature::AtlasRiver(RiverBand::Light, 0);
        let mut rgb = vec![100; W * ROWS * 3];
        for (data_y0, data_y1, output_y0, output_y1) in
            [(0usize, 270usize, 0u32, 256u32), (242, 528, 256, 512)]
        {
            style_river_band(
                u32::try_from(W).unwrap(),
                32_768,
                u32::try_from(data_y0).unwrap(),
                output_y0,
                output_y1,
                &features[data_y0 * W..data_y1 * W],
                &mut rgb[data_y0 * W * 3..data_y1 * W * 3],
            )
            .unwrap();
        }
        let chamfer = |x: usize, y: usize, sx: usize, sy: usize| {
            let dx = x.abs_diff(sx);
            let dy = y.abs_diff(sy);
            i16::try_from(3 * dx.max(dy) + dx.min(dy)).unwrap()
        };
        for y in 242..270 {
            for x in 0..W {
                let feature = features[y * W + x];
                let mut expected = [100u8; 3];
                for (band, source_x, source_y, radius) in
                    [(RiverBand::Mid, 48, 257, 7), (RiverBand::Dark, 24, 255, 13)]
                {
                    let eligible = match band {
                        RiverBand::Mid => matches!(
                            feature,
                            Feature::AtlasLand
                                | Feature::AtlasRiver(RiverBand::Light | RiverBand::Mid, _)
                        ),
                        RiverBand::Dark => {
                            matches!(feature, Feature::AtlasLand | Feature::AtlasRiver(_, _))
                        }
                        RiverBand::Light => false,
                    };
                    if !eligible {
                        continue;
                    }
                    let distance = chamfer(x, y, source_x, source_y) * 16 - radius * 48;
                    let alpha =
                        u16::try_from(((24 - i32::from(distance)) * 256 / 48).clamp(0, 256))
                            .unwrap();
                    let colour = atlas_river_water_colour(band, distance, 16);
                    for channel in 0..3 {
                        expected[channel] = ((u32::from(expected[channel])
                            * u32::from(256 - alpha)
                            + u32::from(colour[channel]) * u32::from(alpha)
                            + 128)
                            / 256)
                            .try_into()
                            .unwrap_or(255);
                    }
                }
                assert_eq!(
                    &rgb[(y * W + x) * 3..(y * W + x) * 3 + 3],
                    &expected,
                    "({x}, {y})"
                );
            }
        }
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
            assert_eq!(feature, Feature::AtlasLand);
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
    fn atlas_lake_overview_matches_area_palette_with_mixed_axis_sampling() {
        let mut cells = AreaCells::flat(Cell {
            height: HeightMm::new(100_000),
            terrain: TerrainKind::Land,
            ..Cell::default()
        });
        let locations = [
            (CellCoord::new(100, 100).unwrap(), 190_000),
            (CellCoord::new(100, 101).unwrap(), 0),
        ];
        for &(at, height) in &locations {
            cells.set(
                at,
                Cell {
                    height: HeightMm::new(height),
                    terrain: TerrainKind::Lake,
                    ..Cell::default()
                },
            );
        }
        let lake = arda_core::Lake {
            global_id: arda_core::hydrology::BasinId(1),
            id: 1,
            surface: HeightMm::new(200_000),
            depth_mm: 200_000,
            outlet: None,
            cells: locations.into_iter().map(|(at, _)| at).collect(),
        };
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
        let terrain = AtlasTerrain::new_with_lakes(&cells, &[lake], halo).unwrap();
        let (overview, feature) = sample_pixel(&cells, Some(&terrain), 512, 256, 100, 50).unwrap();
        assert_eq!(feature, Feature::Lake);
        assert_eq!(
            overview,
            terrain
                .sample(
                    axis_kernel(100, 512).unwrap(),
                    axis_kernel(50, 256).unwrap(),
                    TerrainKind::Lake
                )
                .unwrap()
        );
        assert_eq!(overview, [59, 117, 141]);

        let mut objects = arda_core::AreaObjects::default();
        objects.lakes.push(arda_core::Lake {
            global_id: arda_core::hydrology::BasinId(1),
            id: 1,
            surface: HeightMm::new(200_000),
            depth_mm: 200_000,
            outlet: None,
            cells: locations.into_iter().map(|(at, _)| at).collect(),
        });
        let mut area_png = Vec::new();
        crate::render_area_png_to_atlas(
            &cells,
            &objects,
            arda_core::GlobalCell { x: 0, y: 0 },
            crate::AreaImageScale::Preview,
            &terrain,
            &mut area_png,
        )
        .unwrap();
        let decode = |png: &[u8]| {
            let mut reader = png::Decoder::new(png).read_info().unwrap();
            let mut rgb = vec![0; reader.output_buffer_size()];
            reader.next_frame(&mut rgb).unwrap();
            rgb
        };
        let area_rgb = decode(&area_png);
        assert_eq!(
            &area_rgb[(100 * 512 + 100) * 3..(100 * 512 + 100) * 3 + 3],
            &terrain.colour(locations[0].0)
        );

        let mut buffered = OverviewRaster::new_exact(1, 1, 512, 256).unwrap();
        buffered
            .push_atlas(AreaCoord::new(0, 0), &cells, &terrain)
            .unwrap();
        let buffered_rgb = decode(&buffered.finish().unwrap());
        let mut streamed_png = Vec::new();
        let mut payload = Some((cells, terrain));
        write_atlas_overview_png(1, 1, 512, 256, &mut streamed_png, |_| {
            Ok::<_, RenderError>(payload.take().unwrap())
        })
        .unwrap();
        let streamed_rgb = decode(&streamed_png);
        assert_eq!(streamed_rgb, buffered_rgb);
        assert_eq!(
            &streamed_rgb[(50 * 512 + 100) * 3..(50 * 512 + 100) * 3 + 3],
            &overview
        );
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
