//! The built-in symbolic block style (`logic/04` §Q14).

use crate::RenderError;
use arda_core::{tile_def, Block, SquareCoord, TileGroup, TileId, BLOCK_SQUARES};

/// Pixels per five-foot square.
pub const SQUARE_PX: u32 = 8;

/// RGB for one tile, by family and id — distinct enough to read at a glance.
fn colour(id: u16) -> Option<[u8; 3]> {
    let def = tile_def(TileId::new(id))?;
    // Within a family, later ids get progressively lighter.
    let step = u8::try_from(id % 6).unwrap_or(0) * 9;
    Some(match def.group {
        TileGroup::Water => [12, 42 + step, 96 + step],
        TileGroup::Shore => [176 + step, 158 + step, 108 + step],
        TileGroup::Ground => [72 + step, 108 + step, 54 + step],
        TileGroup::Vegetation => [34 + step, 78 + step, 38 + step],
        TileGroup::Rock => [116 + step, 114 + step, 110 + step],
        TileGroup::Structure => [140 + step, 116 + step, 92 + step],
    })
}

/// Renders one block at [`SQUARE_PX`] pixels per square.
///
/// # Errors
/// [`RenderError::UnmappedTile`] when a square holds a tile this build has no
/// entry for; [`RenderError::Png`] when encoding fails.
pub fn render_block_png(block: &Block) -> Result<Vec<u8>, RenderError> {
    let side = u32::from(BLOCK_SQUARES) * SQUARE_PX;
    let mut rgb = vec![0u8; usize::try_from(side * side * 3).map_err(|_| RenderError::Png)?];

    for sy in 0..BLOCK_SQUARES {
        for sx in 0..BLOCK_SQUARES {
            let at = SquareCoord::new(sx, sy).ok_or(RenderError::Png)?;
            let id = block.square(at).raw();
            let c = colour(id).ok_or(RenderError::UnmappedTile { id })?;

            for py in 0..SQUARE_PX {
                for px in 0..SQUARE_PX {
                    let x = u32::from(sx) * SQUARE_PX + px;
                    let y = u32::from(sy) * SQUARE_PX + py;
                    let i = usize::try_from((y * side + x) * 3).map_err(|_| RenderError::Png)?;
                    rgb[i..i + 3].copy_from_slice(&c);
                }
            }
        }
    }
    crate::encode_png(side, side, &rgb)
}
