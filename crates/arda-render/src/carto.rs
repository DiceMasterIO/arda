//! Cartographic area rendering (`logic/04`).

use crate::RenderError;
use arda_core::{AreaCells, CellCoord, TerrainKind, AREA_CELLS};

/// Hypsometric palette: elevation in millimetres to RGB.
///
/// Stops follow the convention of physical atlases — lowland green, upland
/// tan, montane brown, then rock and snow. A single linear ramp was tried
/// first and is useless: it saturated to white above 2,040 m and showed no
/// variation at all below that, which hid the fact that the whole continent
/// was a 118 m plateau.
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
    let depth = (-height_mm).clamp(0, 3_000_000);
    let t = u8::try_from(depth / 14_000).unwrap_or(214);
    [
        26u8.saturating_sub(t / 8),
        58u8.saturating_sub(t / 5),
        110u8.saturating_sub(t / 3),
    ]
}

/// Strahler-order floor and band cutoffs for river rendering
/// (`feature 03 §Q8`).
///
/// Feature 03 replaced the 300-cell catchment channel threshold with a
/// 40 L/s discharge threshold, which shifted the order distribution down
/// versus the plan's original guess (a floor of 4, bands at 4-5/6-7/>=8).
/// Measured on the DEFAULT continent (500x1000 km, seed 42, release build,
/// `cargo test -p arda-gen --release -- --ignored`): 1,213,091 channel
/// cells, order 1 55.15%, 2 26.91%, 3 12.37%, 4 4.13%, 5 1.16%, 6 0.24%,
/// 7 0.02%, 8 0.04% (the observed maximum order). A floor of 4 — the plan's
/// original value — keeps only the top 5.58% of channel cells,
/// undershooting the 10-25% target and risking an overview with no rivers
/// at all, exactly the regression this constant exists to avoid. A floor
/// of 3 keeps 17.94%, inside the target band, while still dropping the 82%
/// of channel cells that are order-1/2 headwater confetti. Bands keep the
/// plan's width-2/width-2/open shape, shifted down one order to match:
/// Light `{3,4}` = 16.49% of channel cells, Mid `{5,6}` = 1.40%,
/// Dark `{>=7}` = 0.05% (the trunk network).
const RIVER_BAND_MIN: u8 = 3;
/// Mid starts here — see [`RIVER_BAND_MIN`] for the calibration histogram.
const RIVER_BAND_MID: u8 = 5;
/// Dark starts here — see [`RIVER_BAND_MIN`] for the calibration histogram.
const RIVER_BAND_MAX: u8 = 7;

/// A river's overview render band, lightest (thinnest) to darkest
/// (thickest). Declaration order matters: the derived `Ord` is what makes
/// `Dark` the most prominent band when a block spans several orders
/// (`feature 03 §Q8`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RiverBand {
    /// Orders `RIVER_BAND_MIN..RIVER_BAND_MID`: 1 px, unwidened.
    Light,
    /// Orders `RIVER_BAND_MID..RIVER_BAND_MAX`: widens to 2 px.
    Mid,
    /// Orders `>= RIVER_BAND_MAX`: widens to 3 px — the trunk network.
    Dark,
}

/// Maps a Strahler order to its overview render band, or `None` below the
/// floor (`feature 03 §Q8`; see [`RIVER_BAND_MIN`] for the calibration).
#[must_use]
fn river_band(order: u8) -> Option<RiverBand> {
    if order >= RIVER_BAND_MAX {
        Some(RiverBand::Dark)
    } else if order >= RIVER_BAND_MID {
        Some(RiverBand::Mid)
    } else if order >= RIVER_BAND_MIN {
        Some(RiverBand::Light)
    } else {
        None
    }
}

/// The three river band colours, shared by the overview downsample and its
/// widening pass (`feature 03 §Q8`).
fn river_band_colour(band: RiverBand) -> [u8; 3] {
    match band {
        RiverBand::Light => [110, 150, 200],
        RiverBand::Mid => [60, 105, 185],
        RiverBand::Dark => [25, 70, 160],
    }
}

/// Per-cell channel colour for the area map, shaded by Strahler order.
///
/// Unlike the overview, the area map is one pixel per 100 m cell, so a
/// headwater stream is not confetti here — it is correctly one real pixel.
/// Orders below the overview's render floor still get a colour on this
/// map, just a paler one, so the full channel network stays visible at
/// area scale even where the overview hides it.
#[must_use]
fn area_river_colour(order: u8) -> [u8; 3] {
    river_band(order).map_or([140, 170, 210], river_band_colour)
}

/// Renders one area tile, one pixel per 100 m cell, hypsometrically tinted.
///
/// # Errors
/// [`RenderError::Png`] when encoding fails.
pub fn render_area_png(cells: &AreaCells) -> Result<Vec<u8>, RenderError> {
    let side = u32::from(AREA_CELLS);
    let mut rgb = vec![0u8; usize::try_from(side * side * 3).map_err(|_| RenderError::Png)?];

    for y in 0..AREA_CELLS {
        for x in 0..AREA_CELLS {
            let at = CellCoord::new(x, y).ok_or(RenderError::Png)?;
            let cell = cells.get(at);
            let colour = if cell.terrain == TerrainKind::Lake {
                // Inland water reads lighter than the sea, so a lake is not
                // mistaken for a bay.
                [58, 110, 190]
            } else if cell.watercourse_order > 0 {
                area_river_colour(cell.watercourse_order)
            } else if cell.terrain == TerrainKind::Land {
                land_colour(cell.height.raw())
            } else {
                sea_colour(cell.height.raw())
            };
            let i = usize::try_from((u32::from(y) * side + u32::from(x)) * 3)
                .map_err(|_| RenderError::Png)?;
            rgb[i..i + 3].copy_from_slice(&colour);
        }
    }
    crate::encode_png(side, side, &rgb)
}

/// Renders every area tile into one overview image.
///
/// Each area becomes a `px`-square block. Downsampling takes the most
/// significant feature in each block rather than its centre cell: a river is
/// one cell wide out of 512 and would vanish under nearest-neighbour
/// sampling, so water wins over land and lake wins over river.
///
/// `areas` holds `(area_x, area_y, cells)`; missing tiles render as ocean.
///
/// # Errors
/// [`RenderError::Png`] when encoding fails.
pub fn render_overview_png(
    areas: &[(i32, i32, &AreaCells)],
    areas_wide: i32,
    areas_high: i32,
    px: u32,
) -> Result<Vec<u8>, RenderError> {
    let width = u32::try_from(areas_wide).map_err(|_| RenderError::Png)? * px;
    let height = u32::try_from(areas_high).map_err(|_| RenderError::Png)? * px;
    if width == 0 || height == 0 {
        return Err(RenderError::Png);
    }
    let mut rgb = vec![8u8; usize::try_from(width * height * 3).map_err(|_| RenderError::Png)?];
    // Sea everywhere to begin with.
    for chunk in rgb.chunks_exact_mut(3) {
        chunk.copy_from_slice(&[10, 30, 78]);
    }
    // Parallel classification grid: lets the widening pass below tell land
    // and sea apart from lake without re-deriving it from RGB bytes.
    let mut features =
        vec![Feature::Sea; usize::try_from(width * height).map_err(|_| RenderError::Png)?];

    let side = u32::from(AREA_CELLS);
    let block = side / px.max(1);

    // River pixels painted this pass, widened once the whole canvas is
    // classified (`feature 03 §Q8`: banded trunks instead of confetti).
    let mut wide: Vec<(u32, u32, RiverBand)> = Vec::new();

    for &(ax, ay, cells) in areas {
        let (Ok(ox), Ok(oy)) = (u32::try_from(ax), u32::try_from(ay)) else {
            continue;
        };
        for py in 0..px {
            for pxi in 0..px {
                let mut best = Feature::Sea;
                let mut height_sum: i64 = 0;
                let mut land_count: i64 = 0;

                for cy in 0..block {
                    for cx in 0..block {
                        let sx = pxi * block + cx;
                        let sy = py * block + cy;
                        let (Ok(sxu), Ok(syu)) = (u16::try_from(sx), u16::try_from(sy)) else {
                            continue;
                        };
                        let Some(at) = CellCoord::new(sxu, syu) else {
                            continue;
                        };
                        let cell = cells.get(at);
                        let f = match cell.terrain {
                            TerrainKind::Lake => Feature::Lake,
                            TerrainKind::Sea => Feature::Sea,
                            TerrainKind::Land => match river_band(cell.watercourse_order) {
                                Some(band) => Feature::River(band),
                                None => {
                                    height_sum += i64::from(cell.height.raw());
                                    land_count += 1;
                                    Feature::Land
                                }
                            },
                        };
                        best = best.max(f);
                    }
                }

                let colour = match best {
                    Feature::Sea => [10, 30, 78],
                    Feature::Land => {
                        let mean = height_sum / land_count.max(1);
                        land_colour(i32::try_from(mean).unwrap_or(0))
                    }
                    Feature::River(band) => river_band_colour(band),
                    Feature::Lake => [58, 110, 190],
                };

                let x = ox * px + pxi;
                let y = oy * px + py;
                let Ok(pixel) = usize::try_from(y * width + x) else {
                    continue;
                };
                rgb[pixel * 3..pixel * 3 + 3].copy_from_slice(&colour);
                features[pixel] = best;
                if let Feature::River(band) = best {
                    wide.push((x, y, band));
                }
            }
        }
    }

    // feature 03 §Q8: banded trunks instead of confetti. Mid widens one
    // pixel right, Dark widens right and down, but only onto land or sea —
    // never over a lake — and never past the canvas edge.
    for (x, y, band) in wide {
        let colour = river_band_colour(band);
        for &(dx, dy) in widen_offsets(band) {
            let (tx, ty) = (x + dx, y + dy);
            if tx >= width || ty >= height {
                continue;
            }
            let Ok(pixel) = usize::try_from(ty * width + tx) else {
                continue;
            };
            if !matches!(features[pixel], Feature::Land | Feature::Sea) {
                continue;
            }
            rgb[pixel * 3..pixel * 3 + 3].copy_from_slice(&colour);
        }
    }

    crate::encode_png(width, height, &rgb)
}

/// What a downsampled block shows, in increasing order of prominence.
/// `River` carries its render band so a block spanning several orders
/// keeps the highest one, and a lake still wins over any river
/// (`feature 03 §Q8`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Feature {
    Sea,
    Land,
    River(RiverBand),
    Lake,
}

/// Extra pixel offsets a river band's base pixel widens into
/// (`feature 03 §Q8`): Light stays 1 px, Mid becomes 2 px, Dark becomes
/// 3 px.
fn widen_offsets(band: RiverBand) -> &'static [(u32, u32)] {
    match band {
        RiverBand::Light => &[],
        RiverBand::Mid => &[(1, 0)],
        RiverBand::Dark => &[(1, 0), (0, 1)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn river_bands_map_orders_to_the_three_colours() {
        // feature 03 §Q8, calibrated against the measured DEFAULT-continent
        // histogram (see RIVER_BAND_MIN's doc comment): 3-4 light, 5-6 mid,
        // >=7 dark; below 3 the overview draws no river at all.
        assert_eq!(river_band(2), None);
        assert_eq!(river_band(3), Some(RiverBand::Light));
        assert_eq!(river_band(4), Some(RiverBand::Light));
        assert_eq!(river_band(5), Some(RiverBand::Mid));
        assert_eq!(river_band(6), Some(RiverBand::Mid));
        assert_eq!(river_band(7), Some(RiverBand::Dark));
        assert_eq!(river_band(12), Some(RiverBand::Dark));
    }

    #[test]
    fn area_channels_shade_by_order() {
        assert_eq!(area_river_colour(1), [140, 170, 210]);
        assert_eq!(area_river_colour(3), [110, 150, 200]);
        assert_eq!(area_river_colour(5), [60, 105, 185]);
        assert_eq!(area_river_colour(8), [25, 70, 160]);
    }

    #[test]
    fn river_band_ordering_places_dark_above_mid_above_light() {
        // Derived Ord must keep Sea < Land < River(Light<Mid<Dark) < Lake
        // so the overview downsample's `best.max(f)` picks the highest
        // band present in a block.
        assert!(RiverBand::Light < RiverBand::Mid);
        assert!(RiverBand::Mid < RiverBand::Dark);
        assert!(Feature::Land < Feature::River(RiverBand::Light));
        assert!(Feature::River(RiverBand::Dark) < Feature::Lake);
    }

    #[test]
    fn widen_offsets_match_the_band_widths() {
        assert_eq!(widen_offsets(RiverBand::Light), &[] as &[(u32, u32)]);
        assert_eq!(widen_offsets(RiverBand::Mid), &[(1, 0)]);
        assert_eq!(widen_offsets(RiverBand::Dark), &[(1, 0), (0, 1)]);
    }
}
