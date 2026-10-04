//! Auto-crop, centre and fit cut-outs to `footprint × ppsq`.

// Sizes are bounded by image dimensions (u32), so casts are exact.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use crate::cleanup::{bbox, crop};
use crate::ops::resize;
use arda_tactical::catalog::Footprint;
use arda_tactical::Rgba;

/// Transparent margin kept inside the footprint, px (at least).
const MIN_PAD: u32 = 3;
/// Padding added around a crop before resampling, so the filter fades out.
const FILTER_PAD: u32 = 4;

/// A fitted image and what happened to it.
#[derive(Debug, Clone)]
pub struct Fitted {
    /// The image, exactly `footprint × ppsq`.
    pub img: Rgba,
    /// What was done.
    pub fixes: Vec<String>,
    /// What needs a look.
    pub flags: Vec<String>,
}

/// How far an aspect ratio is from a target, as a ratio ≥ 1.
fn mismatch(a: f32, target: f32) -> f32 {
    (a / target).max(target / a)
}

/// Crops a cut-out to its silhouette, turns it a quarter if that matches
/// the footprint's aspect much better, and scales it (aspect kept) to fill
/// the footprint less a small margin, centred.
#[must_use]
pub fn fit_cutout(img: &Rgba, fp: Footprint, ppsq: u32, allow_turn: bool) -> Fitted {
    let (tw, th) = (fp.w * ppsq, fp.h * ppsq);
    let mut fixes = Vec::new();
    let mut flags = Vec::new();
    let Some(b) = bbox(img) else {
        flags.push("nothing left after background removal".into());
        return Fitted {
            img: Rgba::new(tw, th),
            fixes,
            flags,
        };
    };
    let mut obj = crop(img, b);
    if (b.2 - b.0, b.3 - b.1) != (img.width, img.height) {
        fixes.push(format!(
            "cropped to the object ({}x{} of {}x{})",
            obj.width, obj.height, img.width, img.height
        ));
    }
    let target = tw as f32 / th as f32;
    let aspect = |o: &Rgba| o.width as f32 / o.height as f32;
    if allow_turn
        && fp.w != fp.h
        && mismatch(1.0 / aspect(&obj), target) * 1.25 < mismatch(aspect(&obj), target)
    {
        obj = obj.rotated(1);
        fixes.push(format!(
            "turned 90° to match the {}x{} footprint",
            fp.w, fp.h
        ));
    }
    let m = mismatch(aspect(&obj), target);
    if m > 1.6 {
        flags.push(format!(
            "object aspect {:.2} is far from the {}x{} footprint ({target:.2}); letterboxed, check the footprint",
            aspect(&obj),
            fp.w,
            fp.h
        ));
    }
    let pad = MIN_PAD.max(tw.min(th) * 3 / 100);
    let (iw, ih) = (
        tw.saturating_sub(2 * pad).max(1),
        th.saturating_sub(2 * pad).max(1),
    );
    let scale = (iw as f32 / obj.width as f32).min(ih as f32 / obj.height as f32);
    if scale > 1.5 {
        flags.push(format!(
            "upscaled ×{scale:.1}: the source object is only {}x{} px",
            obj.width, obj.height
        ));
    }
    let padded = pad_by(&obj, FILTER_PAD);
    let pw = ((padded.width as f32 * scale).round() as u32).max(1);
    let ph = ((padded.height as f32 * scale).round() as u32).max(1);
    let scaled = resize(&padded, pw, ph, false);
    fixes.push(format!(
        "fitted to {tw}x{th} (scale {scale:.3}, Mitchell filter)"
    ));
    Fitted {
        img: place_centred(&scaled, tw, th),
        fixes,
        flags,
    }
}

/// Crops a floor tile (bridge decking, dock planks, paving) to its silhouette,
/// turns it a quarter if that matches the footprint better, and stretches it
/// to cover the whole footprint edge to edge, so neighbouring tiles butt
/// together with no gaps.
#[must_use]
pub fn fit_fill(img: &Rgba, fp: Footprint, ppsq: u32) -> Fitted {
    let (tw, th) = (fp.w * ppsq, fp.h * ppsq);
    let mut fixes = Vec::new();
    let mut flags = Vec::new();
    let Some(b) = bbox(img) else {
        flags.push("nothing left after background removal".into());
        return Fitted {
            img: Rgba::new(tw, th),
            fixes,
            flags,
        };
    };
    let mut obj = crop(img, b);
    let target = tw as f32 / th as f32;
    let aspect = |o: &Rgba| o.width as f32 / o.height as f32;
    if fp.w != fp.h && mismatch(1.0 / aspect(&obj), target) < mismatch(aspect(&obj), target) {
        obj = obj.rotated(1);
        fixes.push(format!(
            "turned 90° to match the {}x{} footprint",
            fp.w, fp.h
        ));
    }
    let m = mismatch(aspect(&obj), target);
    if m > 1.4 {
        flags.push(format!(
            "floor tile aspect {:.2} stretched to the {}x{} footprint ({target:.2}); check",
            aspect(&obj),
            fp.w,
            fp.h
        ));
    }
    let filled = resize(&obj, tw, th, false);
    fixes.push(format!(
        "cropped to the tile ({}x{}) and filled the {tw}x{th} footprint edge to edge",
        obj.width, obj.height
    ));
    Fitted {
        img: filled,
        fixes,
        flags,
    }
}

/// Scales a square-framed piece (wall kits are drawn on a one-square
/// canvas with the vertex or edge midpoint at its centre) to `ppsq`,
/// centre-cropping a non-square canvas first.
#[must_use]
pub fn fit_canvas(img: &Rgba, ppsq: u32) -> Fitted {
    let mut fixes = Vec::new();
    let side = img.width.min(img.height);
    let sq = if img.width == img.height {
        img.clone()
    } else {
        let (x0, y0) = ((img.width - side) / 2, (img.height - side) / 2);
        fixes.push(format!(
            "centre-cropped the {}x{} canvas to {side}x{side}",
            img.width, img.height
        ));
        crop(img, (x0, y0, x0 + side, y0 + side))
    };
    if side != ppsq {
        fixes.push(format!(
            "scaled the canvas {side} → {ppsq} px (Mitchell filter)"
        ));
    }
    Fitted {
        img: resize(&sq, ppsq, ppsq, false),
        fixes,
        flags: Vec::new(),
    }
}

fn pad_by(img: &Rgba, p: u32) -> Rgba {
    let mut out = Rgba::new(img.width + 2 * p, img.height + 2 * p);
    for y in 0..img.height {
        for x in 0..img.width {
            out.set(x + p, y + p, img.get(x, y));
        }
    }
    out
}

/// Centres `img` on a transparent `w × h` canvas, clipping if larger.
#[must_use]
pub fn place_centred(img: &Rgba, w: u32, h: u32) -> Rgba {
    let mut out = Rgba::new(w, h);
    let ox = (i64::from(w) - i64::from(img.width)) / 2;
    let oy = (i64::from(h) - i64::from(img.height)) / 2;
    for y in 0..img.height {
        for x in 0..img.width {
            let (dx, dy) = (i64::from(x) + ox, i64::from(y) + oy);
            if dx >= 0 && dy >= 0 && dx < i64::from(w) && dy < i64::from(h) {
                out.set(dx as u32, dy as u32, img.get(x, y));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tall_object_turns_into_a_wide_footprint() {
        let mut img = Rgba::new(100, 300);
        for y in 20..280 {
            for x in 20..80 {
                img.set(x, y, [90, 60, 30, 255]);
            }
        }
        let f = fit_cutout(&img, Footprint { w: 2, h: 1 }, 32, true);
        assert_eq!((f.img.width, f.img.height), (64, 32));
        assert!(f.fixes.iter().any(|s| s.contains("turned")));
        assert_eq!(f.img.get(32, 16)[3], 255);
        assert_eq!(f.img.get(0, 0)[3], 0);
    }
}
