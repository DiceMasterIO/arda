//! Background removal for raw images without alpha: flood from the border
//! by colour distance, optionally swallow a baked drop shadow, then pull a
//! soft matte at the edge by unmixing each edge pixel from its local
//! background colour (which also defringes the colour spill).

// Pixel indices are bounded by image dimensions (u32), so casts are exact.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use crate::manifest::ShadowMode;
use crate::ops::{chan_dist, distance_to, nearest_colour, to_u8};
use crate::shadow;
use arda_tactical::Rgba;
use std::collections::VecDeque;

/// Rim width (px) that is always unmixed against the background.
const RIM_PX: u32 = 2;
/// Width (px) of the band in which pale, background-like pixels (halos,
/// glows) are unmixed too.
const HALO_PX: u32 = 8;

/// The result of cutting out one image.
#[derive(Debug, Clone)]
pub struct Cutout {
    /// The image with alpha.
    pub img: Rgba,
    /// What was done, for the report.
    pub fixes: Vec<String>,
    /// What needs a human look.
    pub flags: Vec<String>,
}

/// The background colour and tolerance estimated from the border ring.
#[derive(Debug, Clone, Copy)]
pub struct Background {
    /// Median border colour.
    pub colour: [u8; 4],
    /// Max-channel distance still counted as background.
    pub tol: u8,
}

/// Estimates the background from a 2-px border ring.
#[must_use]
pub fn estimate_background(img: &Rgba) -> Background {
    let (w, h) = (img.width, img.height);
    let mut ring = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if x < 2 || y < 2 || x + 2 >= w || y + 2 >= h {
                ring.push(img.get(x, y));
            }
        }
    }
    let median = |c: usize| {
        let mut v: Vec<u8> = ring.iter().map(|p| p[c]).collect();
        v.sort_unstable();
        v.get(v.len() / 2).copied().unwrap_or(255)
    };
    let colour = [median(0), median(1), median(2), 255];
    let mut dev: Vec<u8> = ring.iter().map(|p| chan_dist(*p, colour)).collect();
    dev.sort_unstable();
    let mad = dev.get(dev.len() / 2).copied().unwrap_or(0);
    let tol = (u32::from(mad) * 3 + 12).clamp(12, 48) as u8;
    Background { colour, tol }
}

/// Floods the background from every border pixel within tolerance. A step
/// may also follow a gentle gradient (vignetted or graded backdrops): within
/// half the tolerance of the pixel it came from and three times of the
/// background colour.
#[must_use]
pub fn flood_background(img: &Rgba, bg: Background) -> Vec<bool> {
    let (w, h) = (img.width as usize, img.height as usize);
    let mut removed = vec![false; w * h];
    let mut queue = VecDeque::new();
    let px = |i: usize| img.get((i % w) as u32, (i / w) as u32);
    for (i, r) in removed.iter_mut().enumerate() {
        let (x, y) = (i % w, i / w);
        let border = x == 0 || y == 0 || x + 1 == w || y + 1 == h;
        if border && chan_dist(px(i), bg.colour) <= bg.tol {
            *r = true;
            queue.push_back(i);
        }
    }
    let far = bg.tol.saturating_mul(3);
    while let Some(i) = queue.pop_front() {
        let here = px(i);
        for j in neighbours(i, w, h) {
            if removed[j] {
                continue;
            }
            let p = px(j);
            let d = chan_dist(p, bg.colour);
            if d <= bg.tol || (d <= far && chan_dist(p, here) <= bg.tol / 2) {
                removed[j] = true;
                queue.push_back(j);
            }
        }
    }
    removed
}

/// 4-neighbours of pixel `i`, in a fixed order.
pub fn neighbours(i: usize, w: usize, h: usize) -> impl Iterator<Item = usize> {
    let (x, y) = (i % w, i / w);
    [
        (x > 0).then(|| i - 1),
        (x + 1 < w).then(|| i + 1),
        (y > 0).then(|| i - w),
        (y + 1 < h).then(|| i + w),
    ]
    .into_iter()
    .flatten()
}

/// Cuts an opaque raw image out of its background.
#[must_use]
pub fn cut_out(img: &Rgba, mode: ShadowMode) -> Cutout {
    let (w, h) = (img.width as usize, img.height as usize);
    let bg = estimate_background(img);
    let mut removed = flood_background(img, bg);
    let mut fixes = Vec::new();
    let mut flags = Vec::new();
    let bg_px = removed.iter().filter(|&&r| r).count();
    if bg_px * 50 < w * h {
        flags.push(format!(
            "no uniform background found around the object (border colour #{:02x}{:02x}{:02x}); not cut out",
            bg.colour[0], bg.colour[1], bg.colour[2]
        ));
        return Cutout {
            img: img.clone(),
            fixes,
            flags,
        };
    }
    fixes.push(format!(
        "removed a #{:02x}{:02x}{:02x} background (tolerance {})",
        bg.colour[0], bg.colour[1], bg.colour[2], bg.tol
    ));
    let found = shadow::grow_on_background(img, &removed, bg);
    shadow::apply(&found, mode, &mut removed, &mut fixes, &mut flags);
    let out = soft_matte(img, &removed, bg);
    Cutout {
        img: out,
        fixes,
        flags,
    }
}

/// Alpha from unmixing each edge pixel `C = a·F + (1 − a)·B` with `F` the
/// nearest solid object colour and `B` the nearest removed colour; edge
/// colours are replaced by the unmixed foreground (defringe).
fn soft_matte(img: &Rgba, removed: &[bool], bg: Background) -> Rgba {
    let (w, h) = (img.width as usize, img.height as usize);
    let dist = distance_to(removed, w, h);
    let pale_tol = bg.tol.saturating_mul(4).max(64);
    let back = nearest_colour(img, removed);
    let band: Vec<bool> = (0..w * h)
        .map(|i| {
            let d = dist[i];
            let p = img.get((i % w) as u32, (i / w) as u32);
            let b = back[i];
            d > 0
                && (d <= RIM_PX
                    || (d <= HALO_PX && chan_dist(p, [b[0], b[1], b[2], 255]) < pale_tol))
        })
        .collect();
    let core: Vec<bool> = (0..w * h).map(|i| !removed[i] && !band[i]).collect();
    let fore = nearest_colour(img, &core);
    let mut out = img.clone();
    for i in 0..w * h {
        let (x, y) = ((i % w) as u32, (i / w) as u32);
        if removed[i] {
            out.set(x, y, [0, 0, 0, 0]);
        } else if band[i] {
            out.set(x, y, unmix(img.get(x, y), fore[i], back[i]));
        }
    }
    out
}

fn unmix(c: [u8; 4], f: [u8; 3], b: [u8; 3]) -> [u8; 4] {
    let fb: [f32; 3] = std::array::from_fn(|k| f32::from(f[k]) - f32::from(b[k]));
    let cb: [f32; 3] = std::array::from_fn(|k| f32::from(c[k]) - f32::from(b[k]));
    let den = fb[0] * fb[0] + fb[1] * fb[1] + fb[2] * fb[2];
    if den < 300.0 {
        // Object and backdrop are too alike to unmix: keep the pixel solid.
        return [c[0], c[1], c[2], 255];
    }
    let a = ((cb[0] * fb[0] + cb[1] * fb[1] + cb[2] * fb[2]) / den).clamp(0.0, 1.0);
    if a < 0.5 {
        return [f[0], f[1], f[2], to_u8(a * 255.0)];
    }
    let rgb: [u8; 3] =
        std::array::from_fn(|k| to_u8((f32::from(c[k]) - (1.0 - a) * f32::from(b[k])) / a));
    [rgb[0], rgb[1], rgb[2], to_u8(a * 255.0)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unmix_recovers_alpha_and_colour() {
        // 40 % red over white.
        let c = [255, 153, 153, 255];
        let p = unmix(c, [255, 0, 0], [255, 255, 255]);
        assert!((100..=104).contains(&p[3]), "{p:?}");
        assert_eq!(&p[..3], &[255, 0, 0]);
    }

    fn disc_scene(shadow: bool) -> Rgba {
        let mut img = Rgba::filled(80, 80, [245, 245, 241, 255]);
        for y in 0..80u32 {
            for x in 0..80u32 {
                let d = |cx: i32, cy: i32| (x as i32 - cx).pow(2) + (y as i32 - cy).pow(2);
                if d(36, 36) < 400 {
                    // A grainy grey stone: neutral like the backdrop, but textured.
                    let g = 120 + (arda_tactical::noise::hash2(5, x.into(), y.into()) % 25) as u8;
                    img.set(x, y, [g, g, g - 6, 255]);
                } else if shadow && d(44, 44) < 400 {
                    // A hard-edged, flat baked shadow falling SE.
                    img.set(x, y, [150, 150, 147, 255]);
                }
            }
        }
        img
    }

    #[test]
    fn a_hard_flat_shadow_is_stripped_but_a_grey_stone_is_kept() {
        let cut = cut_out(&disc_scene(true), ShadowMode::Strip);
        assert!(
            cut.fixes.iter().any(|f| f.contains("stripped")),
            "{:?}",
            cut.fixes
        );
        assert_eq!(cut.img.get(58, 58)[3], 0, "shadow pixel left");
        assert_eq!(cut.img.get(36, 36)[3], 255, "stone eaten");
        let clean = cut_out(&disc_scene(false), ShadowMode::Strip);
        assert!(
            !clean.fixes.iter().any(|f| f.contains("stripped")),
            "{:?}",
            clean.fixes
        );
        assert_eq!(clean.img.get(36, 36)[3], 255);
    }

    #[test]
    fn a_disc_on_white_is_cut_out() {
        let mut img = Rgba::filled(40, 40, [250, 250, 248, 255]);
        for y in 0..40 {
            for x in 0..40 {
                if (x as i32 - 20).pow(2) + (y as i32 - 20).pow(2) < 100 {
                    img.set(x, y, [150, 60, 30, 255]);
                }
            }
        }
        let cut = cut_out(&img, ShadowMode::Strip);
        assert_eq!(cut.img.get(0, 0)[3], 0);
        assert_eq!(cut.img.get(20, 20), [150, 60, 30, 255]);
    }
}
