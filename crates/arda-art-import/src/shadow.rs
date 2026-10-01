//! Baked drop-shadow detection (README: "No baked shadows").
//!
//! A cast shadow on a plain backdrop is the backdrop colour darkened: the
//! same chromaticity, lower luminance, smooth, and getting darker towards
//! the object. On a cut-out with alpha it is a dark, grey, soft-alpha
//! region that reaches well beyond the solid silhouette, off-centre (the
//! usual AI shadow falls to the bottom-right). Confident detections are
//! stripped; doubtful ones and shadows painted solid are flagged.

// Pixel indices are bounded by image dimensions (u32), so casts are exact.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use crate::manifest::ShadowMode;
use crate::matte::{neighbours, Background};
use crate::ops::{distance_to, luma};
use arda_tactical::Rgba;
use std::collections::VecDeque;

/// A detected shadow region.
#[derive(Debug, Clone, Default)]
pub struct Found {
    /// Shadow pixels.
    pub mask: Vec<bool>,
    /// Number of shadow pixels.
    pub px: usize,
    /// Shadow centroid minus object centroid, in object radii.
    pub offset: (f32, f32),
    /// Shadow pixels per object pixel.
    pub share: f32,
    /// Large and off-centre enough to strip without asking.
    pub confident: bool,
}

/// Grows a shadow from the flooded background into darker pixels of the
/// backdrop's own chromaticity, stepping only through smooth, non-brightening
/// pixels.
#[must_use]
pub fn grow_on_background(img: &Rgba, removed: &[bool], bg: Background) -> Found {
    let (w, h) = (img.width as usize, img.height as usize);
    let px = |i: usize| img.get((i % w) as u32, (i / w) as u32);
    let lb = luma(bg.colour).max(1.0);
    let chroma = |p: [u8; 4]| {
        let s = (u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])).max(1) as f32;
        [
            f32::from(p[0]) / s,
            f32::from(p[1]) / s,
            f32::from(p[2]) / s,
        ]
    };
    let cb = chroma(bg.colour);
    let shadowy = |p: [u8; 4]| {
        let r = luma(p) / lb;
        let c = chroma(p);
        (0.2..0.97).contains(&r) && (0..3).all(|k| (c[k] - cb[k]).abs() < 0.03)
    };
    let lum: Vec<f32> = (0..w * h).map(|i| luma(px(i))).collect();
    let cand: Vec<bool> = (0..w * h).map(|i| !removed[i] && shadowy(px(i))).collect();
    // Roughness among candidate neighbours only: a flat shadow is smooth
    // inside even when its edge is hard; a grey object has texture.
    let rough = |i: usize| {
        neighbours(i, w, h)
            .filter(|&n| cand[n])
            .map(|n| (lum[n] - lum[i]).abs())
            .fold(0.0f32, f32::max)
    };
    let mut mask = vec![false; w * h];
    let mut queue: VecDeque<usize> = (0..w * h).filter(|&i| removed[i]).collect();
    while let Some(i) = queue.pop_front() {
        let lq = lum[i];
        for j in neighbours(i, w, h) {
            if !cand[j] || mask[j] || rough(j) > 0.035 * lb {
                continue;
            }
            let lp = lum[j];
            // Entering from the backdrop may cross a hard shadow edge;
            // inside, steps stay small and never brighten much.
            if removed[i] || ((lp - lq).abs() <= 0.06 * lb && lp <= lq + 0.03 * lb) {
                mask[j] = true;
                queue.push_back(j);
            }
        }
    }
    drop_small_regions(&mut mask, w, h, MIN_REGION_PX.max(w * h / 500));
    let object: Vec<bool> = (0..w * h).map(|i| !removed[i] && !mask[i]).collect();
    summarise(mask, &object, w, 0.03)
}

/// A shadow is one coherent smooth region; scattered smooth pixels on a
/// textured grey object are not.
const MIN_REGION_PX: usize = 64;

/// Clears 4-connected regions of `mask` smaller than `min` pixels.
fn drop_small_regions(mask: &mut [bool], w: usize, h: usize, min: usize) {
    let mut seen = vec![false; w * h];
    for start in 0..w * h {
        if !mask[start] || seen[start] {
            continue;
        }
        let mut region = vec![start];
        seen[start] = true;
        let mut k = 0;
        while k < region.len() {
            let i = region[k];
            k += 1;
            for j in neighbours(i, w, h) {
                if mask[j] && !seen[j] {
                    seen[j] = true;
                    region.push(j);
                }
            }
        }
        if region.len() < min {
            for i in region {
                mask[i] = false;
            }
        }
    }
}

/// Finds a soft, dark, grey region on a cut-out that already has alpha and
/// reaches at least 3 px beyond the solid silhouette.
#[must_use]
pub fn grow_in_alpha(img: &Rgba) -> Found {
    let (w, h) = (img.width as usize, img.height as usize);
    let px = |i: usize| img.get((i % w) as u32, (i / w) as u32);
    let solid: Vec<bool> = (0..w * h).map(|i| px(i)[3] >= 250).collect();
    let dist = distance_to(&solid, w, h);
    let candidate = |p: [u8; 4]| {
        let sat = p[0].max(p[1]).max(p[2]) - p[0].min(p[1]).min(p[2]);
        p[3] > 0 && p[3] < 250 && luma(p) < 110.0 && sat < 40
    };
    let mut mask = vec![false; w * h];
    let mut queue: VecDeque<usize> = (0..w * h).filter(|&i| px(i)[3] == 0).collect();
    while let Some(i) = queue.pop_front() {
        for j in neighbours(i, w, h) {
            if !mask[j] && candidate(px(j)) {
                mask[j] = true;
                queue.push_back(j);
            }
        }
    }
    // Keep the 1-px anti-aliased rim; only what reaches past it is shadow.
    for (m, d) in mask.iter_mut().zip(&dist) {
        *m = *m && *d >= 2;
    }
    let far = mask
        .iter()
        .zip(&dist)
        .filter(|(m, d)| **m && **d >= 3)
        .count();
    let mut found = summarise(mask, &solid, w, 0.02);
    let solid_px = solid.iter().filter(|&&s| s).count().max(1);
    found.confident = found.confident && far * 100 >= solid_px;
    found
}

fn summarise(mask: Vec<bool>, object: &[bool], w: usize, min_share: f32) -> Found {
    let centroid = |m: &[bool]| {
        let (mut sx, mut sy, mut n) = (0.0f64, 0.0f64, 0usize);
        for (i, _) in m.iter().enumerate().filter(|(_, &v)| v) {
            sx += (i % w) as f64;
            sy += (i / w) as f64;
            n += 1;
        }
        let n1 = n.max(1) as f64;
        (sx / n1, sy / n1, n)
    };
    let (ox, oy, on) = centroid(object);
    let (sx, sy, sn) = centroid(&mask);
    let radius = (on.max(1) as f64).sqrt() / 2.0;
    let offset = (((sx - ox) / radius) as f32, ((sy - oy) / radius) as f32);
    let share = sn as f32 / on.max(1) as f32;
    let off = (offset.0 * offset.0 + offset.1 * offset.1).sqrt();
    Found {
        px: sn,
        confident: sn > 0 && share >= min_share && off >= 0.1,
        share,
        mask,
        offset,
    }
}

/// Compass direction of an offset (y grows south).
#[must_use]
pub fn direction(offset: (f32, f32)) -> &'static str {
    let (x, y) = offset;
    let ew = if x > 0.35 * y.abs() {
        "E"
    } else if x < -0.35 * y.abs() {
        "W"
    } else {
        ""
    };
    let ns = if y > 0.35 * x.abs() {
        "S"
    } else if y < -0.35 * x.abs() {
        "N"
    } else {
        ""
    };
    match (ns, ew) {
        ("", "") => "centre",
        ("S", "E") => "SE",
        ("S", "W") => "SW",
        ("N", "E") => "NE",
        ("N", "W") => "NW",
        ("S", _) => "S",
        ("N", _) => "N",
        (_, "E") => "E",
        _ => "W",
    }
}

/// Records a detection: strips it into `removed` when confident and
/// allowed, else flags it.
pub fn apply(
    found: &Found,
    mode: ShadowMode,
    removed: &mut [bool],
    fixes: &mut Vec<String>,
    flags: &mut Vec<String>,
) {
    if found.px == 0 {
        return;
    }
    let dir = direction(found.offset);
    match (found.confident, mode) {
        (true, ShadowMode::Strip) => {
            for (r, m) in removed.iter_mut().zip(&found.mask) {
                *r |= *m;
            }
            fixes.push(format!(
                "stripped a baked drop shadow ({} px, falling {dir})",
                found.px
            ));
        }
        (true, ShadowMode::Keep) => flags.push(format!(
            "baked drop shadow ({} px, falling {dir}) kept because the manifest says shadow = \"keep\"",
            found.px
        )),
        (false, _) if found.px >= 64 && found.share >= 0.015 => flags.push(format!(
            "possible baked shadow ({} px, {dir}) too small or centred to strip with confidence; check",
            found.px
        )),
        _ => {}
    }
}

/// Flags a shadow that survived as solid pixels: the south-east rim of the
/// silhouette much darker and greyer than the north-west rim.
#[must_use]
pub fn residual_flag(img: &Rgba) -> Option<String> {
    let (w, h) = (img.width as usize, img.height as usize);
    let solid: Vec<bool> = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| p[3] >= 240)
        .collect();
    let open: Vec<bool> = solid.iter().map(|s| !s).collect();
    let dist = distance_to(&open, w, h);
    let mut lumas: Vec<f32> = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[3] >= 240)
        .map(|p| luma(*p))
        .collect();
    if lumas.len() < 64 {
        return None;
    }
    lumas.sort_by(f32::total_cmp);
    let median = lumas[lumas.len() / 2];
    let sat_of = |p: [u8; 4]| p[0].max(p[1]).max(p[2]) - p[0].min(p[1]).min(p[2]);
    let mut sats: Vec<u8> = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[3] >= 240)
        .map(|p| sat_of(*p))
        .collect();
    sats.sort_unstable();
    let median_sat = f32::from(sats[sats.len() / 2]);
    let (mut cx, mut cy, mut n) = (0.0f64, 0.0f64, 0.0f64);
    for i in (0..w * h).filter(|&i| solid[i]) {
        cx += (i % w) as f64;
        cy += (i / w) as f64;
        n += 1.0;
    }
    let (cx, cy) = (cx / n, cy / n);
    let (mut se, mut se_dark, mut nw, mut nw_dark) = (0u32, 0u32, 0u32, 0u32);
    for i in (0..w * h).filter(|&i| solid[i] && dist[i] <= 4) {
        let p = img.get((i % w) as u32, (i / w) as u32);
        let sat = sat_of(p);
        // Shadow-like: dark and grey, or grey on a colourful object.
        let grey = sat < 30 && f32::from(sat) < 0.35 * median_sat;
        let dark = u32::from((luma(p) < 0.45 * median && sat < 30) || grey);
        if (i % w) as f64 - cx + (i / w) as f64 - cy > 0.0 {
            (se, se_dark) = (se + 1, se_dark + dark);
        } else {
            (nw, nw_dark) = (nw + 1, nw_dark + dark);
        }
    }
    let f_se = se_dark as f32 / se.max(1) as f32;
    let f_nw = nw_dark as f32 / nw.max(1) as f32;
    (f_se > 0.3 && f_se > 2.0 * f_nw + 0.1).then(|| {
        format!(
            "the south-east rim is grey or dark ({:.0}% vs {:.0}% north-west): possible baked shadow; check",
            f_se * 100.0,
            f_nw * 100.0
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directions() {
        assert_eq!(direction((1.0, 1.0)), "SE");
        assert_eq!(direction((0.0, -1.0)), "N");
        assert_eq!(direction((0.0, 0.0)), "centre");
    }
}
