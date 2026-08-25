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
                        land_colour(i32::try_from(mean).unwrap_or(0))
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
