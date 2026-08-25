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
            let colour = if cell.watercourse_order > 0 {
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
