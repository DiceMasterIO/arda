//! Effect art (`effect.<name>`): transparent textures and decals that a
//! consumer draws over tiles. Unlike every other class they keep their
//! alpha: no background matte, no shadow strip, no opaque flattening. A
//! texture is made to wrap with an alpha-aware offset blend; a decal is
//! faded to nothing before its border; light-on-black sources get their
//! alpha from brightness (`alpha = "luminance"`).

// Pixel indices are bounded by image dimensions (u32), so casts are exact.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use crate::cleanup::crop;
use crate::image_io::RawImage;
use crate::manifest::AlphaMode;
use crate::ops::{resize, to_u8};
use arda_tactical::catalog::Footprint;
use arda_tactical::validate::seam_ratio_alpha;
use arda_tactical::Rgba;

/// A prepared effect.
#[derive(Debug, Clone)]
pub struct Effect {
    /// `footprint × ppsq`, straight alpha.
    pub img: Rgba,
    /// What was done.
    pub fixes: Vec<String>,
    /// What needs a look.
    pub flags: Vec<String>,
}

/// Brightness at or below which a luminance source is fully transparent.
const FLOOR: u8 = 6;
/// Mean border brightness under which an opaque file counts as light on black.
const DARK_BORDER: f32 = 30.0;
/// Allowed seam ratio; the validator's limit.
const SEAM_LIMIT: f32 = 2.0;

/// Prepares one raw effect image. `mode` is the manifest's `alpha` key.
#[must_use]
pub fn prepare(
    raw: &RawImage,
    fp: Footprint,
    ppsq: u32,
    tileable: bool,
    mode: Option<AlphaMode>,
) -> Effect {
    let mut fixes = Vec::new();
    let mut flags = Vec::new();
    let luminance = match mode {
        Some(AlphaMode::Luminance) => true,
        Some(AlphaMode::Source) => false,
        None => !raw.has_alpha && dark_border(&raw.rgba),
    };
    let mut img = if luminance {
        let how = if mode.is_some() {
            "alpha = luminance"
        } else {
            "opaque file with a black border"
        };
        fixes.push(format!("alpha from brightness ({how}); light on black"));
        from_luminance(&raw.rgba)
    } else {
        if raw.has_alpha {
            fixes.push("kept the source alpha".into());
        } else {
            flags.push(
                "no transparency: ask for a transparent PNG, or light on black with `alpha = \"luminance\"`"
                    .into(),
            );
        }
        raw.rgba.clone()
    };
    let (tw, th) = (fp.w * ppsq, fp.h * ppsq);
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
            "resized {}x{} → {tw}x{th} (Mitchell filter, premultiplied alpha)",
            img.width, img.height
        ));
        img = resize(&img, tw, th, tileable);
    }
    if tileable {
        let before = seam_ratio_alpha(&img);
        offset_blend(&mut img);
        let after = seam_ratio_alpha(&img);
        fixes.push(format!(
            "seam fixed by alpha-aware offset-and-blend (seam ratio {before:.2} → {after:.2})"
        ));
        if after > SEAM_LIMIT {
            flags.push(format!("seam ratio {after:.2} still above {SEAM_LIMIT:.1}"));
        }
    } else {
        fade_border(&mut img);
        fixes.push("faded the border to transparent so the decal never clips".into());
    }
    let n = img.data.len() / 4;
    let clear = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[3] == 0)
        .count();
    if clear == n {
        flags.push("the effect is fully transparent".into());
    } else if clear == 0 && !img.data.as_chunks::<4>().0.iter().any(|p| p[3] < 255) {
        flags.push("the effect is fully opaque".into());
    }
    Effect { img, fixes, flags }
}

/// Whether the outer 2% of an opaque image is near black.
fn dark_border(img: &Rgba) -> bool {
    let (w, h) = (img.width, img.height);
    let band = (w.min(h) / 50).max(1);
    let (mut sum, mut n) = (0u64, 0u64);
    for y in 0..h {
        for x in 0..w {
            if x < band || y < band || x >= w - band || y >= h - band {
                let p = img.get(x, y);
                sum += u64::from(p[0].max(p[1]).max(p[2]));
                n += 1;
            }
        }
    }
    n > 0 && (sum as f32 / n as f32) < DARK_BORDER
}

/// Alpha is the brightest channel; colour is the straight (un-premultiplied)
/// colour that, drawn over black, gives the source back.
fn from_luminance(src: &Rgba) -> Rgba {
    let mut out = src.clone();
    for p in out.data.as_chunks_mut::<4>().0 {
        let a = p[0].max(p[1]).max(p[2]);
        *p = if a <= FLOOR {
            [0, 0, 0, 0]
        } else {
            let k = 255.0 / f32::from(a);
            [
                to_u8(f32::from(p[0]) * k),
                to_u8(f32::from(p[1]) * k),
                to_u8(f32::from(p[2]) * k),
                a,
            ]
        };
    }
    out
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Blends the image with a copy rolled by half a tile, in premultiplied
/// alpha, so the wrap is continuous in colour and in opacity.
pub fn offset_blend(img: &mut Rgba) {
    let (w, h) = (img.width, img.height);
    let band = (w.min(h) / 4).max(1) as f32;
    let src = img.clone();
    let pre = |p: [u8; 4]| {
        let a = f32::from(p[3]) / 255.0;
        [
            f32::from(p[0]) * a,
            f32::from(p[1]) * a,
            f32::from(p[2]) * a,
            f32::from(p[3]),
        ]
    };
    for y in 0..h {
        for x in 0..w {
            let wo = smoothstep(x.min(w - 1 - x) as f32 / band)
                * smoothstep(y.min(h - 1 - y) as f32 / band);
            let a = pre(src.get(x, y));
            let b = pre(src.get((x + w / 2) % w, (y + h / 2) % h));
            let m: [f32; 4] = std::array::from_fn(|c| wo * a[c] + (1.0 - wo) * b[c]);
            let alpha = m[3].clamp(0.0, 255.0);
            img.set(
                x,
                y,
                if alpha < 0.5 {
                    [0, 0, 0, 0]
                } else {
                    let k = 255.0 / alpha;
                    [
                        to_u8(m[0] * k),
                        to_u8(m[1] * k),
                        to_u8(m[2] * k),
                        to_u8(alpha),
                    ]
                },
            );
        }
    }
}

/// Multiplies alpha by a smooth ramp over the outer 8% of each side.
fn fade_border(img: &mut Rgba) {
    let (w, h) = (img.width, img.height);
    let band = (w.min(h) as f32 * 0.08).max(1.0);
    for y in 0..h {
        for x in 0..w {
            let d = (x.min(w - 1 - x).min(y).min(h - 1 - y)) as f32;
            let k = smoothstep(d / band);
            let mut p = img.get(x, y);
            p[3] = to_u8(f32::from(p[3]) * k);
            if p[3] == 0 {
                p = [0; 4];
            }
            img.set(x, y, p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(img: Rgba, has_alpha: bool) -> RawImage {
        RawImage {
            rgba: img,
            has_alpha,
            meta: crate::image_io::GenMeta::default(),
        }
    }

    /// A soft, non-periodic cloud: alpha rises towards the middle.
    fn cloud(side: u32, alpha: bool) -> Rgba {
        let mut img = Rgba::new(side, side);
        for y in 0..side {
            for x in 0..side {
                let (dx, dy) = (x as f32 / side as f32 - 0.5, y as f32 / side as f32 - 0.5);
                let v = (1.0 - (dx * dx + dy * dy).sqrt() * 1.6).clamp(0.0, 1.0);
                let a = if alpha { (v * 220.0) as u8 + 20 } else { 255 };
                let c = (v * 235.0) as u8;
                img.set(x, y, [c, c / 2, 20, a]);
            }
        }
        img
    }

    #[test]
    fn source_alpha_is_kept_and_the_texture_tiles() {
        let fp = Footprint { w: 2, h: 2 };
        let e = prepare(&raw(cloud(300, true), true), fp, 32, true, None);
        assert_eq!((e.img.width, e.img.height), (64, 64));
        assert!(e.flags.is_empty(), "{:?}", e.flags);
        let px = e.img.data.as_chunks::<4>().0;
        assert!(px.iter().any(|p| p[3] > 0 && p[3] < 255), "soft alpha kept");
        assert!(px.iter().all(|p| p[3] >= 15), "not flattened or matted");
        assert!(seam_ratio_alpha(&e.img) <= 2.0, "wraps");
        assert_eq!(
            prepare(&raw(cloud(300, true), true), fp, 32, true, None).img,
            e.img,
            "deterministic"
        );
    }

    #[test]
    fn luminance_mode_derives_alpha_from_brightness() {
        let mut src = Rgba::new(64, 64);
        for y in 0..64 {
            for x in 0..64 {
                let v = if (24..40).contains(&x) && (24..40).contains(&y) {
                    200
                } else {
                    0
                };
                src.set(x, y, [v, v / 2, 0, 255]);
            }
        }
        let fp = Footprint { w: 1, h: 1 };
        let e = prepare(
            &raw(src.clone(), false),
            fp,
            32,
            false,
            Some(AlphaMode::Luminance),
        );
        assert!(e.flags.is_empty(), "{:?}", e.flags);
        // The centre is bright: alpha follows the brightest channel, and the
        // colour is un-premultiplied (full-strength orange, not dim).
        let c = e.img.get(16, 16);
        assert!(
            c[3] > 150 && c[0] > 240 && (120..135).contains(&c[1]),
            "{c:?}"
        );
        assert_eq!(e.img.get(2, 2), [0, 0, 0, 0], "black is clear");
        // The same file without the manifest key is detected by its border.
        let auto = prepare(&raw(src, false), fp, 32, false, None);
        assert_eq!(auto.img, e.img);
        assert!(auto.fixes.iter().any(|f| f.contains("black border")));
    }

    #[test]
    fn opaque_files_are_flagged_not_matted() {
        let e = prepare(
            &raw(Rgba::filled(64, 64, [200, 200, 220, 255]), false),
            Footprint { w: 1, h: 1 },
            32,
            false,
            None,
        );
        assert!(
            e.flags.iter().any(|f| f.contains("no transparency")),
            "{:?}",
            e.flags
        );
        // A decal's border fades, but the middle stays opaque.
        assert_eq!(e.img.get(16, 16)[3], 255);
        assert_eq!(e.img.get(0, 0)[3], 0);
    }

    #[test]
    fn decals_fade_before_the_border() {
        let e = prepare(
            &raw(cloud(128, true), true),
            Footprint { w: 2, h: 2 },
            32,
            false,
            None,
        );
        for i in 0..e.img.width {
            assert_eq!(e.img.get(i, 0)[3], 0);
            assert_eq!(e.img.get(0, i)[3], 0);
        }
    }
}
