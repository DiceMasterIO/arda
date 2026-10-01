//! RGBA8 rasters: PNG I/O, quarter-turn rotation, mirroring and resampling.
//!
//! Everything here is integer maths so results are byte-identical on every
//! platform (goal 62).

use crate::error::TacticalError;
use std::path::Path;

/// Alpha-composites `px` over the RGBA pixel `dst` ("source over"), in
/// integer /255 fixed point.
pub fn blend_px(dst: &mut [u8], px: [u8; 4]) {
    let a = u32::from(px[3]);
    if a == 0 || dst.len() < 4 {
        return;
    }
    if a == 255 {
        dst[..4].copy_from_slice(&px);
        return;
    }
    let da = u32::from(dst[3]);
    // out_a = a + da (1 - a), all in /255 fixed point.
    let out_a = a * 255 + da * (255 - a);
    if out_a == 0 {
        return;
    }
    for (c, &src) in px.iter().take(3).enumerate() {
        let s = u32::from(src) * a * 255;
        let d = u32::from(dst[c]) * da * (255 - a);
        dst[c] = to_u8((s + d + out_a / 2) / out_a);
    }
    dst[3] = to_u8((out_a + 127) / 255);
}

/// A straight-alpha RGBA8 image, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes.
    pub data: Vec<u8>,
}

impl Rgba {
    /// A fully transparent image.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![0; width as usize * height as usize * 4],
        }
    }

    /// An image filled with one colour.
    #[must_use]
    pub fn filled(width: u32, height: u32, px: [u8; 4]) -> Self {
        let mut img = Self::new(width, height);
        for chunk in img.data.as_chunks_mut::<4>().0.iter_mut() {
            chunk.copy_from_slice(&px);
        }
        img
    }

    fn index(&self, x: u32, y: u32) -> usize {
        (y as usize * self.width as usize + x as usize) * 4
    }

    /// The pixel at `(x, y)`; callers keep coordinates in bounds.
    #[must_use]
    pub fn get(&self, x: u32, y: u32) -> [u8; 4] {
        let i = self.index(x, y);
        [
            self.data[i],
            self.data[i + 1],
            self.data[i + 2],
            self.data[i + 3],
        ]
    }

    /// Sets the pixel at `(x, y)`.
    pub fn set(&mut self, x: u32, y: u32, px: [u8; 4]) {
        let i = self.index(x, y);
        self.data[i..i + 4].copy_from_slice(&px);
    }

    /// Alpha-composites `px` over the pixel at `(x, y)` ("source over").
    pub fn blend(&mut self, x: u32, y: u32, px: [u8; 4]) {
        let i = self.index(x, y);
        blend_px(&mut self.data[i..i + 4], px);
    }

    /// Mirrors left to right.
    #[must_use]
    pub fn mirrored(&self) -> Self {
        let mut out = Self::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                out.set(self.width - 1 - x, y, self.get(x, y));
            }
        }
        out
    }

    /// Rotates clockwise by `quarter_turns` × 90°.
    #[must_use]
    pub fn rotated(&self, quarter_turns: u8) -> Self {
        match quarter_turns % 4 {
            0 => self.clone(),
            q => {
                let mut img = self.clone();
                for _ in 0..q {
                    img = img.rotated_cw();
                }
                img
            }
        }
    }

    fn rotated_cw(&self) -> Self {
        let mut out = Self::new(self.height, self.width);
        for y in 0..self.height {
            for x in 0..self.width {
                // (x, y) moves to (h-1-y, x).
                out.set(self.height - 1 - y, x, self.get(x, y));
            }
        }
        out
    }

    /// Resamples to `width × height` with premultiplied-alpha area averaging
    /// when shrinking and bilinear interpolation when enlarging. Premultiplying
    /// keeps transparent pixels from bleeding dark fringes into edges.
    #[must_use]
    pub fn resized(&self, width: u32, height: u32) -> Self {
        if width == self.width && height == self.height {
            return self.clone();
        }
        let mut out = Self::new(width, height);
        for oy in 0..height {
            for ox in 0..width {
                let px = if width <= self.width && height <= self.height {
                    self.area_average(ox, oy, width, height)
                } else {
                    self.bilinear(ox, oy, width, height)
                };
                out.set(ox, oy, px);
            }
        }
        out
    }

    fn area_average(&self, ox: u32, oy: u32, width: u32, height: u32) -> [u8; 4] {
        let x0 = u64::from(ox) * u64::from(self.width) / u64::from(width);
        let x1 = (u64::from(ox + 1) * u64::from(self.width)).div_ceil(u64::from(width));
        let y0 = u64::from(oy) * u64::from(self.height) / u64::from(height);
        let y1 = (u64::from(oy + 1) * u64::from(self.height)).div_ceil(u64::from(height));
        let mut acc = [0u64; 4];
        let mut n = 0u64;
        for y in y0..y1 {
            for x in x0..x1 {
                let p = self.get(to_u32(x), to_u32(y));
                let a = u64::from(p[3]);
                for c in 0..3 {
                    acc[c] += u64::from(p[c]) * a;
                }
                acc[3] += a;
                n += 1;
            }
        }
        unpremultiply(acc, n)
    }

    fn bilinear(&self, ox: u32, oy: u32, width: u32, height: u32) -> [u8; 4] {
        // Source position in 1/256 pixel, sampling at pixel centres.
        let sx = (i64::from(2 * ox + 1) * i64::from(self.width) * 128 / i64::from(width)) - 128;
        let sy = (i64::from(2 * oy + 1) * i64::from(self.height) * 128 / i64::from(height)) - 128;
        let clamp_x = |v: i64| clamp_index(v, self.width);
        let clamp_y = |v: i64| clamp_index(v, self.height);
        let (ix, fx) = (
            sx.div_euclid(256),
            u64::try_from(sx.rem_euclid(256)).unwrap_or(0),
        );
        let (iy, fy) = (
            sy.div_euclid(256),
            u64::try_from(sy.rem_euclid(256)).unwrap_or(0),
        );
        let mut acc = [0u64; 4];
        for (dy, wy) in [(0, 256 - fy), (1, fy)] {
            for (dx, wx) in [(0, 256 - fx), (1, fx)] {
                let p = self.get(clamp_x(ix + dx), clamp_y(iy + dy));
                let w = wx * wy;
                let a = u64::from(p[3]) * w;
                for c in 0..3 {
                    acc[c] += u64::from(p[c]) * a;
                }
                acc[3] += a;
            }
        }
        unpremultiply(acc, 65536)
    }

    /// Reads an 8-bit RGBA or RGB PNG.
    ///
    /// # Errors
    /// I/O and decode failures, or an unsupported colour type.
    pub fn read_png(path: &Path) -> Result<Self, TacticalError> {
        let err = |e: String| TacticalError::Image {
            path: path.to_path_buf(),
            message: e,
        };
        let file = std::fs::File::open(path).map_err(|e| err(e.to_string()))?;
        let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().map_err(|e| err(e.to_string()))?;
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader
            .next_frame(&mut buf)
            .map_err(|e| err(e.to_string()))?;
        let (w, h) = (info.width, info.height);
        let pixels = w as usize * h as usize;
        let data = match info.color_type {
            png::ColorType::Rgba => buf[..pixels * 4].to_vec(),
            png::ColorType::Rgb => buf[..pixels * 3]
                .as_chunks::<3>()
                .0
                .iter()
                .flat_map(|p| [p[0], p[1], p[2], 255])
                .collect(),
            png::ColorType::GrayscaleAlpha => buf[..pixels * 2]
                .as_chunks::<2>()
                .0
                .iter()
                .flat_map(|p| [p[0], p[0], p[0], p[1]])
                .collect(),
            png::ColorType::Grayscale => {
                buf[..pixels].iter().flat_map(|&g| [g, g, g, 255]).collect()
            }
            png::ColorType::Indexed => return Err(err("unexpanded indexed PNG".into())),
        };
        Ok(Self {
            width: w,
            height: h,
            data,
        })
    }

    /// Writes an RGBA PNG. Encoding is deterministic for identical pixels.
    ///
    /// # Errors
    /// I/O and encode failures.
    pub fn write_png(&self, path: &Path) -> Result<(), TacticalError> {
        let err = |e: String| TacticalError::Image {
            path: path.to_path_buf(),
            message: e,
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| err(e.to_string()))?;
        }
        let file = std::fs::File::create(path).map_err(|e| err(e.to_string()))?;
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| err(e.to_string()))?;
        writer
            .write_image_data(&self.data)
            .map_err(|e| err(e.to_string()))?;
        writer.finish().map_err(|e| err(e.to_string()))
    }
}

fn unpremultiply(acc: [u64; 4], n: u64) -> [u8; 4] {
    if acc[3] == 0 || n == 0 {
        return [0; 4];
    }
    let mut px = [0u8; 4];
    for c in 0..3 {
        px[c] = to_u8_64((acc[c] + acc[3] / 2) / acc[3]);
    }
    px[3] = to_u8_64((acc[3] + n / 2) / n);
    px
}

/// Saturating conversion to `u8`.
#[must_use]
pub fn to_u8(v: u32) -> u8 {
    u8::try_from(v).unwrap_or(u8::MAX)
}

fn to_u8_64(v: u64) -> u8 {
    u8::try_from(v).unwrap_or(u8::MAX)
}

fn clamp_index(v: i64, len: u32) -> u32 {
    u32::try_from(v.clamp(0, i64::from(len) - 1)).unwrap_or(0)
}

fn to_u32(v: u64) -> u32 {
    u32::try_from(v).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient() -> Rgba {
        let mut img = Rgba::new(3, 2);
        for y in 0..2 {
            for x in 0..3 {
                img.set(x, y, [to_u8(x * 10), to_u8(y * 10), 0, 255]);
            }
        }
        img
    }

    #[test]
    fn four_quarter_turns_are_identity() {
        let img = gradient();
        assert_eq!(img.rotated(4), img);
        let r = img.rotated(1);
        assert_eq!((r.width, r.height), (2, 3));
        // Top-left moves to top-right under a clockwise turn.
        assert_eq!(r.get(1, 0), img.get(0, 0));
        assert_eq!(img.rotated(1).rotated(3), img);
    }

    #[test]
    fn mirror_twice_is_identity() {
        let img = gradient();
        assert_eq!(img.mirrored().get(0, 0), img.get(2, 0));
        assert_eq!(img.mirrored().mirrored(), img);
    }

    #[test]
    fn blend_over_opaque_is_a_lerp() {
        let mut img = Rgba::filled(1, 1, [0, 0, 0, 255]);
        img.blend(0, 0, [255, 255, 255, 128]);
        let p = img.get(0, 0);
        assert!((127..=129).contains(&p[0]) && p[3] == 255);
    }

    #[test]
    fn downscale_premultiplies_so_edges_do_not_darken() {
        let mut img = Rgba::new(2, 2);
        img.set(0, 0, [200, 100, 50, 255]);
        let small = img.resized(1, 1);
        assert_eq!(&small.get(0, 0)[..3], &[200, 100, 50]);
        assert_eq!(small.get(0, 0)[3], 64);
    }

    #[test]
    fn upscale_keeps_flat_colour_exact() {
        let img = Rgba::filled(2, 2, [10, 20, 30, 255]);
        let big = img.resized(5, 5);
        assert!(big
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [10, 20, 30, 255]));
    }
}
