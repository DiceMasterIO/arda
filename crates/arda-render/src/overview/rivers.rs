//! Atlas river symbols on the overview: bounded cartographic widths, band
//! colours with edge and water ramps, and the chamfer-distance band styler.

use super::Feature;
use crate::carto::{river_band_colour, RiverBand};
use crate::RenderError;

/// Map saved channel width to a bounded cartographic radius at 32K. The floor
/// keeps narrow rivers visible when the whole world is fitted to a screen.
/// It is a symbol, not a claim that overview pixels are metres across.
pub(super) fn atlas_river_size(width_dm: u32, band: RiverBand) -> u8 {
    match band {
        RiverBand::Light => 0,
        RiverBand::Mid => 4 + (width_dm / 150).min(3) as u8,
        RiverBand::Dark => 7 + (width_dm / 180).min(6) as u8,
    }
}

pub(super) fn atlas_river_radius(width: u32, height: u32, size: u8) -> u32 {
    let screen_scale = width.max(height).div_ceil(2048);
    (u32::from(size) * screen_scale).div_ceil(16)
}

pub(super) fn atlas_river_halo(width: u32, height: u32) -> u32 {
    atlas_river_radius(width, height, 13) + 1
}

pub(super) const fn atlas_river_colour(band: RiverBand) -> [u8; 3] {
    match band {
        RiverBand::Light => [91, 153, 171],
        RiverBand::Mid => [60, 135, 162],
        RiverBand::Dark => [32, 98, 132],
    }
}

pub(super) const fn atlas_river_edge_colour(band: RiverBand) -> [u8; 3] {
    match band {
        RiverBand::Light => [91, 153, 171],
        RiverBand::Mid => [80, 151, 169],
        RiverBand::Dark => [66, 141, 162],
    }
}

/// Cartographic cross-section shading from the same signed distance as the
/// bank coverage. This describes apparent water, not measured channel depth.
pub(super) fn atlas_river_water_colour(band: RiverBand, distance: i16, scale: i16) -> [u8; 3] {
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
pub(super) fn style_river_band(
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
