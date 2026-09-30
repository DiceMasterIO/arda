//! Image helpers of the tactical routes: pyramid geometry, crops, stage
//! timing and WebP encoding.

use super::dto::TacticalTilesDto;
use super::pyramid::{self, TACTICAL_TILE_PX};
use crate::error::{ServerError, ServerResult};
use arda_tactical::{Rgba, TacticalLayout};
use std::time::Instant;

/// Pyramid geometry of `layout` at `ppsq`.
#[must_use]
pub fn tiles_dto(layout: &TacticalLayout, ppsq: u32) -> TacticalTilesDto {
    tiles_dto_sized(layout.width, layout.height, ppsq)
}

/// Keeps squares `[x, y, w, h]` of a render at `ppsq`.
///
/// # Errors
/// [`ServerError::Image`] when the crop leaves the image.
pub fn crop(img: &Rgba, [x, y, w, h]: [u32; 4], ppsq: u32) -> ServerResult<Rgba> {
    let (px, py) = (x.saturating_mul(ppsq), y.saturating_mul(ppsq));
    let (pw, ph) = (w.saturating_mul(ppsq), h.saturating_mul(ppsq));
    if px.saturating_add(pw) > img.width || py.saturating_add(ph) > img.height {
        return Err(ServerError::Image("crop leaves the render".into()));
    }
    let mut out = Rgba::new(pw, ph);
    let (src_w, row) = (img.width as usize * 4, pw as usize * 4);
    for r in 0..ph as usize {
        let from = (py as usize + r) * src_w + px as usize * 4;
        let (Some(src), Some(dst)) = (
            img.data.get(from..from + row),
            out.data.get_mut(r * row..(r + 1) * row),
        ) else {
            return Err(ServerError::Image("crop leaves the render".into()));
        };
        dst.copy_from_slice(src);
    }
    Ok(out)
}

/// Pyramid geometry of a `w × h`-square render at `ppsq`.
#[must_use]
pub fn tiles_dto_sized(w_sq: u32, h_sq: u32, ppsq: u32) -> TacticalTilesDto {
    let (w, h) = (w_sq.saturating_mul(ppsq), h_sq.saturating_mul(ppsq));
    TacticalTilesDto {
        tile_px: TACTICAL_TILE_PX,
        ppsq,
        image_width_px: w,
        image_height_px: h,
        max_zoom: pyramid::max_zoom(w, h),
        format: "webp".into(),
    }
}

/// Logs one pipeline stage's duration to stderr.
pub(super) fn stage(what: &str, img: &Rgba, start: Instant) {
    let ms = start.elapsed().as_secs_f64() * 1e3;
    eprintln!(
        "arda-server tactical   {what} {}x{} px {ms:.3} ms",
        img.width, img.height
    );
}

/// Encodes a square RGBA tile as lossless WebP (image-webp, pure Rust).
///
/// # Errors
/// [`ServerError::Image`] on encoder failure or a size mismatch.
pub fn encode_webp(rgba: &[u8], edge: u32) -> ServerResult<Vec<u8>> {
    if rgba.len() != edge as usize * edge as usize * 4 {
        return Err(ServerError::Image("webp tile has the wrong size".into()));
    }
    let mut out = Vec::new();
    image_webp::WebPEncoder::new(&mut out)
        .encode(rgba, edge, edge, image_webp::ColorType::Rgba8)
        .map_err(|e| ServerError::Image(format!("webp: {e}")))?;
    Ok(out)
}
