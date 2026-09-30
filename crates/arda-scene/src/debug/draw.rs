//! Minimal anti-alias-free drawing on an [`Rgba`] canvas for debug renders.

use arda_tactical::Rgba;

/// A canvas with blended primitives. Coordinates are pixels as `f32`.
pub struct Pen<'a>(pub &'a mut Rgba);

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
impl Pen<'_> {
    fn bbox(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> (u32, u32, u32, u32) {
        let (w, h) = (self.0.width as f32, self.0.height as f32);
        let c = |v: f32, m: f32| v.clamp(0.0, m) as u32;
        (
            c(x0.floor(), w),
            c(y0.floor(), h),
            c(x1.ceil(), w),
            c(y1.ceil(), h),
        )
    }

    /// Blends a filled axis-aligned rectangle.
    pub fn rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, px: [u8; 4]) {
        let (ax, ay, bx, by) = self.bbox(x0, y0, x1, y1);
        for y in ay..by {
            for x in ax..bx {
                self.0.blend(x, y, px);
            }
        }
    }

    /// Blends a line `t` pixels thick with square caps.
    pub fn line(&mut self, a: (f32, f32), b: (f32, f32), t: f32, px: [u8; 4]) {
        let r = t / 2.0;
        let (ax, ay, bx, by) = self.bbox(
            a.0.min(b.0) - r,
            a.1.min(b.1) - r,
            a.0.max(b.0) + r,
            a.1.max(b.1) + r,
        );
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len2 = (dx * dx + dy * dy).max(1e-6);
        for y in ay..by {
            for x in ax..bx {
                let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
                let u = (((cx - a.0) * dx + (cy - a.1) * dy) / len2).clamp(0.0, 1.0);
                let (qx, qy) = (a.0 + u * dx, a.1 + u * dy);
                if (cx - qx).abs().max((cy - qy).abs()) <= r {
                    self.0.blend(x, y, px);
                }
            }
        }
    }

    /// Blends a circle outline; `dash` pixels on, `dash` off when non-zero.
    pub fn circle(&mut self, c: (f32, f32), radius: f32, t: f32, dash: f32, px: [u8; 4]) {
        let r = radius + t;
        let (ax, ay, bx, by) = self.bbox(c.0 - r, c.1 - r, c.0 + r, c.1 + r);
        for y in ay..by {
            for x in ax..bx {
                let (dx, dy) = (x as f32 + 0.5 - c.0, y as f32 + 0.5 - c.1);
                let d = (dx * dx + dy * dy).sqrt();
                if (d - radius).abs() > t / 2.0 {
                    continue;
                }
                if dash > 0.0 {
                    let arc = dy.atan2(dx) * radius;
                    if (arc / dash).floor() as i64 % 2 != 0 {
                        continue;
                    }
                }
                self.0.blend(x, y, px);
            }
        }
    }

    /// Blends a filled disc.
    pub fn disc(&mut self, c: (f32, f32), radius: f32, px: [u8; 4]) {
        let (ax, ay, bx, by) = self.bbox(c.0 - radius, c.1 - radius, c.0 + radius, c.1 + radius);
        for y in ay..by {
            for x in ax..bx {
                let (dx, dy) = (x as f32 + 0.5 - c.0, y as f32 + 0.5 - c.1);
                if dx * dx + dy * dy <= radius * radius {
                    self.0.blend(x, y, px);
                }
            }
        }
    }

    /// Blends a polygon fill (even-odd) and outline.
    pub fn polygon(&mut self, pts: &[(f32, f32)], fill: [u8; 4], edge: [u8; 4], t: f32) {
        if pts.len() < 3 {
            return;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for p in pts {
            (x0, y0, x1, y1) = (x0.min(p.0), y0.min(p.1), x1.max(p.0), y1.max(p.1));
        }
        let (ax, ay, bx, by) = self.bbox(x0, y0, x1, y1);
        for y in ay..by {
            for x in ax..bx {
                if inside(pts, x as f32 + 0.5, y as f32 + 0.5) {
                    self.0.blend(x, y, fill);
                }
            }
        }
        for i in 0..pts.len() {
            self.line(pts[i], pts[(i + 1) % pts.len()], t, edge);
        }
    }
}

fn inside(pts: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut c = false;
    let n = pts.len();
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + n - 1) % n]);
        if (a.1 > y) != (b.1 > y) && x < (b.0 - a.0) * (y - a.1) / (b.1 - a.1) + a.0 {
            c = !c;
        }
    }
    c
}

/// Encodes an image as PNG bytes.
///
/// # Errors
/// Encoder failure, as text.
pub fn encode_png(img: &Rgba) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.width, img.height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(&img.data).map_err(|e| e.to_string())?;
    }
    Ok(out)
}
