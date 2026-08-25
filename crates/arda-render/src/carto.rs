//! Cartographic area rendering (`logic/04`).

use crate::RenderError;
use arda_core::{AreaCells, CellCoord, TerrainKind, AREA_CELLS};

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
                [40, 92, 170]
            } else if cell.terrain == TerrainKind::Land {
                // 0 m to about 2000 m across a green-to-pale ramp.
                let t = u8::try_from((cell.height.raw() / 8_000).clamp(0, 255)).unwrap_or(255);
                [
                    64u8.saturating_add(t),
                    120u8.saturating_add(t / 2),
                    60u8.saturating_add(t),
                ]
            } else {
                let d = u8::try_from((-cell.height.raw() / 20_000).clamp(0, 90)).unwrap_or(90);
                [10, 40u8.saturating_sub(d / 4), 110u8.saturating_sub(d)]
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

    let side = u32::from(AREA_CELLS);
    let block = side / px.max(1);

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
                            TerrainKind::Land if cell.watercourse_order >= 3 => Feature::River,
                            TerrainKind::Land => {
                                height_sum += i64::from(cell.height.raw());
                                land_count += 1;
                                Feature::Land
                            }
                        };
                        best = best.max(f);
                    }
                }

                let colour = match best {
                    Feature::Sea => [10, 30, 78],
                    Feature::Land => {
                        let mean = height_sum / land_count.max(1);
                        let t = u8::try_from((mean / 8_000).clamp(0, 255)).unwrap_or(255);
                        [
                            64u8.saturating_add(t),
                            120u8.saturating_add(t / 2),
                            60u8.saturating_add(t),
                        ]
                    }
                    Feature::River => [40, 92, 170],
                    Feature::Lake => [58, 110, 190],
                };

                let x = ox * px + pxi;
                let y = oy * px + py;
                let Ok(i) = usize::try_from((y * width + x) * 3) else {
                    continue;
                };
                rgb[i..i + 3].copy_from_slice(&colour);
            }
        }
    }

    crate::encode_png(width, height, &rgb)
}

/// What a downsampled block shows, in increasing order of prominence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Feature {
    Sea,
    Land,
    River,
    Lake,
}
