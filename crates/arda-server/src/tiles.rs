//! Pure image helpers for the overview slippy pyramid.
//!
//! Pyramid: the overview image sits at the top-left of a square of
//! `tile_px << max_zoom` pixels; the rest is transparent. Zoom `z` has
//! `2^z × 2^z` tiles, each a box-filtered `2^(max_zoom − z)` reduction.

use crate::error::{ServerError, ServerResult};

/// Tile edge in pixels.
pub const TILE_PX: u32 = 256;

/// Decoded 8-bit RGB raster.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgb {
    /// Columns.
    pub width: u32,
    /// Rows.
    pub height: u32,
    /// Row-major RGB triples.
    pub pixels: Vec<u8>,
}

impl Rgb {
    /// Heap bytes.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.pixels.len()
    }
}

/// Deepest zoom for a square base of `base_px`, which must be `TILE_PX · 2^k` with k ≥ 1.
///
/// # Errors
/// [`ServerError::BadRequest`] for any other size.
pub fn max_zoom(base_px: u32) -> ServerResult<u32> {
    if base_px < 2 * TILE_PX
        || !base_px.is_multiple_of(TILE_PX)
        || !(base_px / TILE_PX).is_power_of_two()
    {
        return Err(ServerError::BadRequest(format!(
            "tile base {base_px} px must be 256 × a power of two, at least 512"
        )));
    }
    Ok((base_px / TILE_PX).trailing_zeros())
}

/// Decodes an 8-bit RGB or RGBA PNG to RGB.
///
/// # Errors
/// [`ServerError::Image`] for undecodable or unsupported images.
pub fn decode_rgb(png_bytes: &[u8]) -> ServerResult<Rgb> {
    let image = |e: String| ServerError::Image(e);
    let mut reader = png::Decoder::new(png_bytes)
        .read_info()
        .map_err(|e| image(e.to_string()))?;
    let mut buf = Vec::new();
    buf.try_reserve_exact(reader.output_buffer_size())
        .map_err(|_| ServerError::ResourceLimit("overview decode".into()))?;
    buf.resize(reader.output_buffer_size(), 0);
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| image(e.to_string()))?;
    if info.bit_depth != png::BitDepth::Eight {
        return Err(image("overview is not 8-bit".into()));
    }
    buf.truncate(info.buffer_size());
    let pixels = match info.color_type {
        png::ColorType::Rgb => buf,
        png::ColorType::Rgba => buf
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect(),
        other => return Err(image(format!("unsupported overview colour type {other:?}"))),
    };
    Ok(Rgb {
        width: info.width,
        height: info.height,
        pixels,
    })
}

/// Encodes an RGBA tile with fixed settings, so equal pixels give equal bytes.
///
/// # Errors
/// [`ServerError::Image`] on encoder failure.
pub fn encode_rgba(width: u32, height: u32, rgba: &[u8]) -> ServerResult<Vec<u8>> {
    encode_rgba_with(
        width,
        height,
        rgba,
        png::Compression::Default,
        png::FilterType::NoFilter,
    )
}

/// As [`encode_rgba`], fast: `fdeflate` with the Paeth filter. Painted
/// tactical maps of 4096² px take about 2 s at the default settings; the
/// fast path is several times quicker and smaller (deterministic either way).
///
/// # Errors
/// [`ServerError::Image`] on encoder failure.
pub fn encode_rgba_fast(width: u32, height: u32, rgba: &[u8]) -> ServerResult<Vec<u8>> {
    encode_rgba_with(
        width,
        height,
        rgba,
        png::Compression::Fast,
        png::FilterType::Paeth,
    )
}

fn encode_rgba_with(
    width: u32,
    height: u32,
    rgba: &[u8],
    compression: png::Compression,
    filter: png::FilterType,
) -> ServerResult<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(compression);
        encoder.set_filter(filter);
        let mut writer = encoder
            .write_header()
            .map_err(|e| ServerError::Image(e.to_string()))?;
        writer
            .write_image_data(rgba)
            .map_err(|e| ServerError::Image(e.to_string()))?;
    }
    Ok(out)
}

/// Cuts tile `(z, x, y)` from `base` in a pyramid of depth `max_zoom`, as RGBA.
///
/// Each output pixel averages the image pixels of its `f × f` source block
/// (integer, round half up); alpha is the covered fraction of the block.
///
/// # Errors
/// [`ServerError::NotFound`] for tiles outside the pyramid.
pub fn cut(base: &Rgb, max_zoom: u32, z: u32, x: u32, y: u32) -> ServerResult<Vec<u8>> {
    if z > max_zoom || x >= (1 << z) || y >= (1 << z) {
        return Err(ServerError::NotFound(format!(
            "tile {z}/{x}/{y} is outside the pyramid (zoom 0..={max_zoom})"
        )));
    }
    let f = 1_u64 << (max_zoom - z);
    let block = f * f;
    let tile = u64::from(TILE_PX);
    let (w, h) = (u64::from(base.width), u64::from(base.height));
    let mut out = Vec::with_capacity((TILE_PX * TILE_PX * 4) as usize);
    for py in 0..tile {
        let sy0 = (u64::from(y) * tile + py) * f;
        for px in 0..tile {
            let sx0 = (u64::from(x) * tile + px) * f;
            let (mut sum, mut count) = ([0_u64; 3], 0_u64);
            for sy in sy0..(sy0 + f).min(h) {
                let row = sy * w;
                for sx in sx0..(sx0 + f).min(w) {
                    let i = usize::try_from((row + sx) * 3).unwrap_or(usize::MAX);
                    if let Some(p) = base.pixels.get(i..i + 3) {
                        sum[0] += u64::from(p[0]);
                        sum[1] += u64::from(p[1]);
                        sum[2] += u64::from(p[2]);
                        count += 1;
                    }
                }
            }
            let avg = |s: u64| {
                (s + count / 2)
                    .checked_div(count)
                    .map_or(0, |v| u8::try_from(v).unwrap_or(u8::MAX))
            };
            let alpha = u8::try_from((255 * count + block / 2) / block).unwrap_or(u8::MAX);
            out.extend_from_slice(&[avg(sum[0]), avg(sum[1]), avg(sum[2]), alpha]);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient(width: u32, height: u32) -> Rgb {
        let pixels = (0..width * height)
            .flat_map(|i| [u8::try_from(i % 256).unwrap(), 100, 200])
            .collect();
        Rgb {
            width,
            height,
            pixels,
        }
    }

    #[test]
    fn zoom_depth_requires_power_of_two_multiples_of_256() {
        assert_eq!(max_zoom(512).unwrap(), 1);
        assert_eq!(max_zoom(4096).unwrap(), 4);
        assert!(max_zoom(256).is_err() && max_zoom(768).is_err());
    }

    #[test]
    fn pyramid_bounds_are_enforced_per_zoom() {
        let base = gradient(512, 256);
        assert!(cut(&base, 1, 0, 0, 0).is_ok());
        assert!(cut(&base, 1, 1, 1, 1).is_ok());
        for (z, x, y) in [(2, 0, 0), (0, 1, 0), (1, 2, 0), (1, 0, 2)] {
            assert!(
                matches!(cut(&base, 1, z, x, y), Err(ServerError::NotFound(_))),
                "{z}/{x}/{y}"
            );
        }
    }

    #[test]
    fn deepest_zoom_is_a_pixel_copy_and_padding_is_transparent() {
        let base = gradient(512, 256);
        let t = cut(&base, 1, 1, 0, 0).unwrap();
        assert_eq!(&t[..8], &[0, 100, 200, 255, 1, 100, 200, 255]);
        let below = cut(&base, 1, 1, 0, 1).unwrap();
        assert!(
            below.iter().all(|&v| v == 0),
            "rows past the image are padding"
        );
    }

    #[test]
    fn zoom_zero_averages_blocks_and_marks_half_covered_alpha() {
        let base = gradient(512, 256);
        let t = cut(&base, 1, 0, 0, 0).unwrap();
        // Pixel (0,0) averages columns 0,1 of rows 0,1: red (0+1+0+1)/4 rounds to 1.
        assert_eq!(&t[..4], &[1, 100, 200, 255]);
        // Row 128 of zoom 0 samples base rows 256–257: outside the image.
        assert_eq!(t[128 * 256 * 4 + 3], 0);
    }

    #[test]
    fn encoding_round_trips_and_is_deterministic() {
        let rgba: Vec<u8> = (0..4_u8 * 4 * 4).collect();
        let a = encode_rgba(4, 4, &rgba).unwrap();
        assert_eq!(a, encode_rgba(4, 4, &rgba).unwrap());
        let back = decode_rgb(&a).unwrap();
        assert_eq!((back.width, back.height), (4, 4));
        assert_eq!(&back.pixels[..3], &[0, 1, 2]);
    }
}
