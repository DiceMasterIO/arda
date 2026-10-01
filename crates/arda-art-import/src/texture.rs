//! Ground and water textures: crop and resize to the tile, make them tile
//! (offset-and-blend for natural surfaces, an edge blend that keeps the
//! layout for structured ones) and flatten broad contrast, which the
//! compositor's macro tint supplies instead (README, "Ground and water").

// Pixel indices are bounded by image dimensions (u32), so casts are exact.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use crate::cleanup::crop;
use crate::ops::{box_blur, luma, resize, to_u8};
use arda_tactical::catalog::Footprint;
use arda_tactical::validate::seam_ratio;
use arda_tactical::Rgba;

/// Seam ratio the importer aims under (the validator allows 2.0).
const SEAM_TARGET: f32 = 1.5;
/// How much of the broad luminance variation to remove.
const FLATTEN: f32 = 0.8;

/// A prepared texture.
#[derive(Debug, Clone)]
pub struct Texture {
    /// The tile, `footprint × ppsq`, opaque and tileable.
    pub img: Rgba,
    /// What was done.
    pub fixes: Vec<String>,
    /// What needs a look.
    pub flags: Vec<String>,
}

/// Prepares one raw texture.
#[must_use]
pub fn prepare(raw: &Rgba, fp: Footprint, ppsq: u32, structured: bool) -> Texture {
    let (tw, th) = (fp.w * ppsq, fp.h * ppsq);
    let mut fixes = Vec::new();
    let mut flags = Vec::new();
    let mut img = opaque(raw, &mut fixes);
    // Centre-crop to the tile's aspect, then resample to the tile.
    let (sw, sh) = (img.width, img.height);
    let (cw, ch) = if u64::from(sw) * u64::from(th) > u64::from(sh) * u64::from(tw) {
        ((u64::from(sh) * u64::from(tw) / u64::from(th)) as u32, sh)
    } else {
        (sw, (u64::from(sw) * u64::from(th) / u64::from(tw)) as u32)
    };
    if (cw, ch) != (sw, sh) {
        let (x0, y0) = ((sw - cw) / 2, (sh - ch) / 2);
        img = crop(&img, (x0, y0, x0 + cw, y0 + ch));
        fixes.push(format!("centre-cropped {sw}x{sh} to {cw}x{ch}"));
    }
    if (img.width, img.height) != (tw, th) {
        fixes.push(format!(
            "resized {}x{} → {tw}x{th} (Mitchell filter)",
            img.width, img.height
        ));
        img = resize(&img, tw, th, false);
    }
    let before = seam_ratio(&img);
    // Flatten first too, so the seam fix never blends a bright part of the
    // image into a dark one (a gradient would show as a cross of blobs).
    flatten(&mut img, false);
    if structured {
        edge_blend(&mut img);
        fixes.push(format!(
            "seam fixed by edge blend, keeping the layout (seam ratio {before:.2} → {:.2})",
            seam_ratio(&img)
        ));
    } else {
        offset_blend(&mut img);
        fixes.push(format!(
            "seam fixed by offset-and-blend (seam ratio {before:.2} → {:.2})",
            seam_ratio(&img)
        ));
    }
    let (c0, c1) = flatten(&mut img, true);
    fixes.push(format!(
        "flattened broad contrast ({:.1}% → {:.1}% low-frequency luminance spread)",
        c0 * 100.0,
        c1 * 100.0
    ));
    let mut after = seam_ratio(&img);
    if after > SEAM_TARGET {
        edge_blend(&mut img);
        after = seam_ratio(&img);
        fixes.push(format!("second edge blend (seam ratio {after:.2})"));
    }
    if after > 2.0 {
        flags.push(format!("seam ratio {after:.2} still above 2.0"));
    }
    if structured && before > 3.0 {
        flags.push(format!(
            "the source did not tile (seam ratio {before:.2}); the edge blend may smear grout or board lines along the tile border, so check the 2x2 preview"
        ));
    }
    Texture { img, fixes, flags }
}

/// Composites any transparency over the mean colour.
fn opaque(raw: &Rgba, fixes: &mut Vec<String>) -> Rgba {
    let px = raw.data.as_chunks::<4>().0;
    if px.iter().all(|p| p[3] == 255) {
        return raw.clone();
    }
    let n = px.len().max(1) as u64;
    let mean: [u64; 3] =
        std::array::from_fn(|c| px.iter().map(|p| u64::from(p[c])).sum::<u64>() / n);
    let mut out = raw.clone();
    for p in out.data.as_chunks_mut::<4>().0.iter_mut() {
        let a = u64::from(p[3]);
        for c in 0..3 {
            p[c] = ((u64::from(p[c]) * a + mean[c] * (255 - a)) / 255) as u8;
        }
        p[3] = 255;
    }
    fixes.push("composited transparency over the mean colour (textures are opaque)".into());
    out
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Blends the image with a copy rolled by half a tile, whose wrap seams lie
/// in the middle. The original dominates the interior and the rolled copy
/// the borders, so the wrap is continuous; the blend is variance
/// preserving so the cross-fade does not wash out contrast.
pub fn offset_blend(img: &mut Rgba) {
    let (w, h) = (img.width, img.height);
    let band = (w.min(h) / 4).max(1) as f32;
    let px = img.data.as_chunks::<4>().0;
    let n = px.len().max(1) as f32;
    let mean: [f32; 3] =
        std::array::from_fn(|c| px.iter().map(|p| f32::from(p[c])).sum::<f32>() / n);
    let src = img.clone();
    for y in 0..h {
        for x in 0..w {
            let wx = smoothstep(x.min(w - 1 - x) as f32 / band);
            let wy = smoothstep(y.min(h - 1 - y) as f32 / band);
            let wo = wx * wy;
            let norm = (wo * wo + (1.0 - wo) * (1.0 - wo)).sqrt();
            let a = src.get(x, y);
            let b = src.get((x + w / 2) % w, (y + h / 2) % h);
            let px: [u8; 3] = std::array::from_fn(|c| {
                let m = mean[c];
                let v = (wo * (f32::from(a[c]) - m) + (1.0 - wo) * (f32::from(b[c]) - m)) / norm;
                to_u8(m + v)
            });
            img.set(x, y, [px[0], px[1], px[2], 255]);
        }
    }
}

/// Makes opposite edges meet without moving anything: the low-frequency
/// edge mismatch is spread over a wide band and the residual over 3 px,
/// half on each side (a cheap stand-in for a Poisson edge solve).
pub fn edge_blend(img: &mut Rgba) {
    for _ in 0..2 {
        blend_columns(img);
        *img = img.rotated(1);
    }
    *img = img.rotated(2);
}

/// Closes the left/right seam.
fn blend_columns(img: &mut Rgba) {
    let (w, h) = (img.width as usize, img.height as usize);
    if w < 8 {
        return;
    }
    let wide = (w / 6).max(4);
    let narrow = 3usize;
    for c in 0..3 {
        let get = |img: &Rgba, x: usize, y: usize| f32::from(img.get(x as u32, y as u32)[c]);
        let mut d: Vec<f32> = (0..h)
            .map(|y| get(img, 0, y) - get(img, w - 1, y))
            .collect();
        box_blur(&mut d, 1, h, 6, true);
        let mut plane: Vec<f32> = (0..w * h).map(|i| get(img, i % w, i / w)).collect();
        for y in 0..h {
            for k in 0..wide {
                let g = 0.5 * d[y] * (1.0 - smoothstep(k as f32 / wide as f32));
                plane[y * w + k] -= g;
                plane[y * w + w - 1 - k] += g;
            }
            let r = plane[y * w] - plane[y * w + w - 1];
            for k in 0..narrow {
                let g = 0.5 * r * (narrow - k) as f32 / narrow as f32;
                plane[y * w + k] -= g;
                plane[y * w + w - 1 - k] += g;
            }
        }
        for (i, v) in plane.iter().enumerate() {
            img.data[i * 4 + c] = to_u8(*v);
        }
    }
}

/// Divides out broad luminance variation (a blur of the luma at about a
/// sixth of the tile, wrapping once the tile tiles), keeping the fine
/// texture. Returns the low-frequency spread (std / mean) before and after.
fn flatten(img: &mut Rgba, wrap: bool) -> (f32, f32) {
    let (w, h) = (img.width as usize, img.height as usize);
    let r = (w.min(h) / 12).max(2);
    let lum: Vec<f32> = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| luma(*p))
        .collect();
    let mut low = lum.clone();
    box_blur(&mut low, w, h, r, wrap);
    let n = low.len().max(1) as f32;
    let mean = (low.iter().sum::<f32>() / n).max(1.0);
    let spread = |v: &[f32]| {
        let m = v.iter().sum::<f32>() / n;
        (v.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / n).sqrt() / m.max(1.0)
    };
    let before = spread(&low);
    for (i, p) in img.data.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let gain = (1.0 + FLATTEN * (mean / low[i].max(1.0) - 1.0)).clamp(0.6, 1.67);
        for v in p.iter_mut().take(3) {
            *v = to_u8(f32::from(*v) * gain);
        }
    }
    let mut after: Vec<f32> = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| luma(*p))
        .collect();
    box_blur(&mut after, w, h, r, wrap);
    (before, spread(&after))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A non-tileable gradient with fine noise.
    fn ramp(w: u32, h: u32) -> Rgba {
        let mut img = Rgba::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let n = arda_tactical::noise::hash2(3, i64::from(x), i64::from(y)) % 24;
                let v = (40 + x * 120 / w + y * 60 / h) as u8 + n as u8;
                img.set(x, y, [v, v / 2 + 40, 30, 255]);
            }
        }
        img
    }

    #[test]
    fn natural_and_structured_textures_end_up_tileable() {
        for structured in [false, true] {
            let t = prepare(&ramp(300, 200), Footprint { w: 2, h: 2 }, 32, structured);
            assert_eq!((t.img.width, t.img.height), (64, 64));
            assert!(seam_ratio(&t.img) < 2.0, "{structured}: {:?}", t.fixes);
        }
    }
}
