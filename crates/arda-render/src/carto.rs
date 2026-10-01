//! Saved-data cartographic palettes and area export.

use crate::channels::{AreaImageScale, ChannelInput};
pub use crate::overview::render_overview_png;
use crate::{AtlasTerrain, RenderError};
use arda_core::{AreaCells, AreaObjects, GlobalCell};

/// Hypsometric palette: millimetres of elevation to RGB.
#[must_use]
pub fn land_colour(height_mm: i32) -> [u8; 3] {
    const STOPS: [(i32, [u8; 3]); 7] = [
        (0, [86, 125, 70]),           // coastal plain
        (200_000, [122, 148, 78]),    // lowland
        (500_000, [163, 165, 92]),    // upland
        (900_000, [173, 141, 88]),    // hill
        (1_400_000, [150, 112, 78]),  // montane
        (2_000_000, [140, 130, 128]), // bare rock
        (2_800_000, [242, 242, 245]), // snow
    ];
    let h = height_mm.max(0);
    let mut i = 0;
    while i + 1 < STOPS.len() && h >= STOPS[i + 1].0 {
        i += 1;
    }
    if i + 1 >= STOPS.len() {
        return STOPS[STOPS.len() - 1].1;
    }
    let (lo, c0) = STOPS[i];
    let (hi, c1) = STOPS[i + 1];
    let span = (hi - lo).max(1);
    let t = i64::from((h - lo).clamp(0, span));
    let mix = |a: u8, b: u8| {
        let v = i64::from(a) + (i64::from(b) - i64::from(a)) * t / i64::from(span);
        u8::try_from(v.clamp(0, 255)).unwrap_or(255)
    };
    [mix(c0[0], c1[0]), mix(c0[1], c1[1]), mix(c0[2], c1[2])]
}

/// Sea colour by depth: shelf is lighter than abyss.
#[must_use]
pub fn sea_colour(height_mm: i32) -> [u8; 3] {
    let depth = (-i64::from(height_mm)).clamp(0, 3_000_000);
    let t = u8::try_from(depth / 14_000).unwrap_or(214);
    [
        26u8.saturating_sub(t / 8),
        58u8.saturating_sub(t / 5),
        110u8.saturating_sub(t / 3),
    ]
}

/// Uniform ocean tint at overview scale.
pub(crate) const OVERVIEW_SEA: [u8; 3] = [10, 30, 78];

/// Categorical standing-water tint for overview and missing surface context.
pub(crate) const LAKE_FILL: [u8; 3] = [132, 176, 205];

/// A readable blue area-water palette with deeper water shaded darker.
///
/// This is cartographic depth shading, not calibrated water optics. Depths
/// 0/50/250/1000/3000 mm interpolate 0/32/96/192/255 of the way from shallow
/// blue to deep blue. Even the shallow endpoint is water-coloured; terrain
/// colour and opacity do not enter this blend. Depth changes colour only.
pub(crate) fn lake_colour(depth_mm: u32) -> [u8; 3] {
    const RAMP: [(u32, u32); 5] = [(0, 0), (50, 32), (250, 96), (1000, 192), (3000, 255)];
    const SHALLOW: [u8; 3] = [58, 137, 180];
    const DEEP: [u8; 3] = [22, 68, 126];
    let mut alpha = 255;
    for pair in RAMP.windows(2) {
        let [(lo, a), (hi, b)] = [pair[0], pair[1]];
        if depth_mm < hi {
            alpha = a + (b - a) * (depth_mm - lo) / (hi - lo);
            break;
        }
    }
    std::array::from_fn(|i| {
        let blended =
            (u32::from(SHALLOW[i]) * (255 - alpha) + u32::from(DEEP[i]) * alpha + 127) / 255;
        // Convex combination of two bytes, rounded to the nearest byte.
        u8::try_from(blended).unwrap_or(255)
    })
}

/// Minimum lake share of an overview block; retained cartographic rule.
pub(crate) const LAKE_MIN_BLOCK_DEN: u32 = 4;

/// Overview discharge bands in litres per second.
const RIVER_Q_MIN: u64 = 4_000;
/// Mid band floor — see [`RIVER_Q_MIN`] for the calibration.
const RIVER_Q_MID: u64 = 20_000;
/// Dark band floor — see [`RIVER_Q_MIN`] for the calibration.
const RIVER_Q_MAX: u64 = 80_000;

/// A watercourse's render band, lightest to darkest.
///
/// Declaration order matters: the derived `Ord` is what makes `Dark` win
/// when one block spans several bands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RiverBand {
    /// `RIVER_Q_MIN..RIVER_Q_MID` — a stream.
    Light,
    /// `RIVER_Q_MID..RIVER_Q_MAX` — a river.
    Mid,
    /// `>= RIVER_Q_MAX` — a trunk.
    Dark,
}

/// Maps a discharge to its render band, or `None` below the floor.
#[must_use]
pub(crate) fn river_band(discharge_milli: u64) -> Option<RiverBand> {
    if discharge_milli >= RIVER_Q_MAX {
        Some(RiverBand::Dark)
    } else if discharge_milli >= RIVER_Q_MID {
        Some(RiverBand::Mid)
    } else if discharge_milli >= RIVER_Q_MIN {
        Some(RiverBand::Light)
    } else {
        None
    }
}

/// The three river band colours, shared by the overview and the area map.
///
/// All three sit well clear of [`LAKE_FILL`] in lightness so line work
/// never reads as a water body.
pub(crate) const fn river_band_colour(band: RiverBand) -> [u8; 3] {
    match band {
        RiverBand::Light => [86, 130, 190],
        RiverBand::Mid => [46, 92, 170],
        RiverBand::Dark => [16, 56, 138],
    }
}

/// Renders saved physical geometry over the area grid at the selected scale.
///
/// `origin` is the global coordinate of local cell (0,0). The supplied object
/// layer includes canonical neighboring channel edges whose envelopes touch
/// this area. Rendering performs no hydrology or terrain generation.
///
/// # Errors
/// Returns typed geometry/resource refusals or PNG encoding failure.
pub fn render_area_png(
    cells: &AreaCells,
    objects: &AreaObjects,
    origin: GlobalCell,
    scale: AreaImageScale,
) -> Result<Vec<u8>, RenderError> {
    let mut output = Vec::new();
    render_area_png_to(cells, objects, origin, scale, &mut output)?;
    Ok(output)
}

/// Streams an area PNG to a writer using one reusable RGB row.
///
/// Geometry and candidate limits are independent of the output size, including
/// 32K images. The caller owns output storage; an encoding or geometry failure
/// may leave a partial PNG in the supplied writer.
///
/// # Errors
/// Returns typed geometry/resource refusals or PNG encoding failure.
pub fn render_area_png_to<W: std::io::Write>(
    cells: &AreaCells,
    objects: &AreaObjects,
    origin: GlobalCell,
    scale: AreaImageScale,
    writer: W,
) -> Result<(), RenderError> {
    render_area_png_to_inner(cells, objects, origin, scale, None, writer)
}

/// Streams an Atlas area PNG with per-pixel saved-terrain interpolation.
///
/// Water geometry and validated lake depth retain their saved-cell ownership.
///
/// # Errors
/// Returns typed geometry, atlas-context, resource or PNG failures.
pub fn render_area_png_to_atlas<W: std::io::Write>(
    cells: &AreaCells,
    objects: &AreaObjects,
    origin: GlobalCell,
    scale: AreaImageScale,
    terrain: &AtlasTerrain,
    writer: W,
) -> Result<(), RenderError> {
    render_area_png_to_inner(cells, objects, origin, scale, Some(terrain), writer)
}

fn render_area_png_to_inner<W: std::io::Write>(
    cells: &AreaCells,
    objects: &AreaObjects,
    origin: GlobalCell,
    scale: AreaImageScale,
    terrain: Option<&AtlasTerrain>,
    writer: W,
) -> Result<(), RenderError> {
    let inputs = objects.channel_edges.iter().map(|edge| ChannelInput {
        from: edge.from,
        to: edge.to,
        from_width_dm: edge.from_width_dm,
        to_width_dm: edge.to_width_dm,
        discharge: edge.discharge.raw(),
    });
    let points = objects
        .global
        .reaches
        .iter()
        .filter(|r| r.id.is_point() && r.mean_discharge.raw() >= 40)
        .map(|r| {
            if r.from != r.to || r.id.start() != r.from {
                return Err(RenderError::ChannelGeometry {
                    reason: "invalid saved terminal point",
                });
            }
            let width = arda_core::hydrology::channel_width_dm(r.mean_discharge).ok_or(
                RenderError::ChannelGeometry {
                    reason: "terminal width overflow",
                },
            )?;
            Ok(ChannelInput {
                from: r.from,
                to: r.to,
                from_width_dm: width,
                to_width_dm: width,
                discharge: r.mean_discharge.raw(),
            })
        });
    let mut raster = crate::channels::AreaRaster::new_with_terrain(
        cells,
        terrain,
        inputs.map(Ok).chain(points),
        origin,
        scale,
        &objects.lakes,
    )?;
    crate::encode_png_rows(scale.side(), scale.side(), writer, |output| {
        for y in 0..scale.side() as usize {
            output
                .write_all(raster.row(y)?)
                .map_err(|_| RenderError::Png)?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests;
