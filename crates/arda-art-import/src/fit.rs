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
    let scale = fill_scale(&obj, tw, th);
    if scale > 1.5 {
        flags.push(format!(
            "upscaled ×{scale:.1}: the source object is only {}x{} px",
            obj.width, obj.height
        ));
    }
    fixes.push(format!(
        "fitted to {tw}x{th} (scale {scale:.3}, Mitchell filter)"
    ));
    Fitted {
        img: scale_centred(&obj, scale, tw, th),
        fixes,
        flags,
    }
}

/// The scale at which `obj` fills a `tw × th` footprint less the margin.
fn fill_scale(obj: &Rgba, tw: u32, th: u32) -> f32 {
    let pad = MIN_PAD.max(tw.min(th) * 3 / 100);
    let (iw, ih) = (
        tw.saturating_sub(2 * pad).max(1),
        th.saturating_sub(2 * pad).max(1),
    );
    (iw as f32 / obj.width as f32).min(ih as f32 / obj.height as f32)
}

/// Scales `obj` by `scale` (Mitchell, with a filter margin) and centres it
/// on a transparent `tw × th` canvas.
fn scale_centred(obj: &Rgba, scale: f32, tw: u32, th: u32) -> Rgba {
    let padded = pad_by(obj, FILTER_PAD);
    let pw = ((padded.width as f32 * scale).round() as u32).max(1);
    let ph = ((padded.height as f32 * scale).round() as u32).max(1);
    let scaled = resize(&padded, pw, ph, false);
    place_centred(&scaled, tw, th)
}

/// Below this share of the footprint (its larger side) a vegetation cut-out
/// is flagged as tiny.
const TINY_SHARE: f32 = 0.4;

/// Fits a vegetation cut-out keeping the size it was drawn at: the whole
/// source frame is scaled onto the footprint (aspect kept, the frame's
/// shorter fit wins), so a sapling drawn small in its frame stays smaller
/// than an old oak drawn filling its frame. The object is cropped to its
/// silhouette and centred, and it is shrunk if it would not fit inside the
/// footprint less the margin. Non-square footprints (fallen logs) have no
/// clear frame-to-footprint mapping and are fitted like any cut-out.
#[must_use]
pub fn fit_vegetation(img: &Rgba, fp: Footprint, ppsq: u32) -> Fitted {
    if fp.w != fp.h {
        return fit_cutout(img, fp, ppsq, true);
    }
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
    let obj = crop(img, b);
    let frame = (tw as f32 / img.width as f32).min(th as f32 / img.height as f32);
    let fill = fill_scale(&obj, tw, th);
    let scale = frame.min(fill);
    if frame > fill {
        fixes.push(format!(
            "the object ({}x{} of a {}x{} frame) is drawn larger than the footprint allows; shrunk to fit",
            obj.width, obj.height, img.width, img.height
        ));
    }
    let share = (obj.width as f32 * scale / tw as f32).max(obj.height as f32 * scale / th as f32);
    if share < TINY_SHARE {
        flags.push(format!(
            "tiny: the object spans only {:.0}% of the {}x{} footprint ({}x{} of a {}x{} frame); check the generation",
            share * 100.0,
            fp.w,
            fp.h,
            obj.width,
            obj.height,
            img.width,
            img.height
        ));
    }
    if scale > 1.5 {
        flags.push(format!(
            "upscaled ×{scale:.1}: the source frame is only {}x{} px",
            img.width, img.height
        ));
    }
    fixes.push(format!(
        "kept the drawn size: frame scaled to {tw}x{th} (scale {scale:.3}, object {:.0}% of the footprint, Mitchell filter)",
        share * 100.0
    ));
    Fitted {
        img: scale_centred(&obj, scale, tw, th),
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

    /// A `side`-px frame with an opaque `w × h` block at its centre.
    fn framed(side: u32, w: u32, h: u32) -> Rgba {
        let mut img = Rgba::new(side, side);
        let (x0, y0) = ((side - w) / 2, (side - h) / 2);
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                img.set(x, y, [60, 110, 40, 255]);
            }
        }
        img
    }

    /// Width and height of the opaque (alpha ≥ 128) part of `img`.
    fn extent(img: &Rgba) -> (u32, u32) {
        let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
        for y in 0..img.height {
            for x in 0..img.width {
                if img.get(x, y)[3] >= 128 {
                    (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
                }
            }
        }
        (x1.saturating_sub(x0), y1.saturating_sub(y0))
    }

    const SQ2: Footprint = Footprint { w: 2, h: 2 };

    #[test]
    fn vegetation_keeps_the_drawn_size() {
        // A sapling drawn at half the frame and an oak drawn at 90 % of it.
        let small = fit_vegetation(&framed(400, 200, 200), SQ2, 64);
        let big = fit_vegetation(&framed(400, 360, 360), SQ2, 64);
        assert_eq!((small.img.width, small.img.height), (128, 128));
        let (sw, sh) = extent(&small.img);
        let (bw, _) = extent(&big.img);
        assert!(
            (63..=65).contains(&sw) && (63..=65).contains(&sh),
            "{sw}x{sh}"
        );
        assert!((114..=117).contains(&bw), "{bw}");
        assert!(small.flags.is_empty() && big.flags.is_empty());
        // The plain cut-out fit makes both the same size.
        let a = extent(&fit_cutout(&framed(400, 200, 200), SQ2, 64, true).img);
        let b = extent(&fit_cutout(&framed(400, 360, 360), SQ2, 64, true).img);
        assert_eq!(a, b);
    }

    #[test]
    fn vegetation_never_exceeds_the_footprint() {
        let f = fit_vegetation(&framed(400, 400, 400), SQ2, 64);
        let (w, h) = extent(&f.img);
        assert!(w <= 122 && h <= 122, "{w}x{h}");
        for x in 0..128 {
            assert_eq!(f.img.get(x, 0)[3], 0);
            assert_eq!(f.img.get(0, x)[3], 0);
        }
        assert!(f.fixes.iter().any(|s| s.contains("shrunk to fit")));
    }

    #[test]
    fn tiny_vegetation_is_flagged() {
        let f = fit_vegetation(&framed(400, 120, 120), SQ2, 64);
        assert!(
            f.flags.iter().any(|s| s.starts_with("tiny")),
            "{:?}",
            f.flags
        );
        let (w, _) = extent(&f.img);
        assert!((37..=40).contains(&w), "{w}");
    }

    #[test]
    fn vegetation_on_a_non_square_footprint_fits_like_a_cut_out() {
        let img = framed(400, 300, 100);
        let fp = Footprint { w: 2, h: 1 };
        assert_eq!(
            fit_vegetation(&img, fp, 32).img,
            fit_cutout(&img, fp, 32, true).img
        );
    }

    #[test]
    fn empty_vegetation_is_flagged() {
        let f = fit_vegetation(&Rgba::new(50, 50), SQ2, 32);
        assert_eq!((f.img.width, f.img.height), (64, 64));
        assert!(!f.flags.is_empty());
    }

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
