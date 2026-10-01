//! Alpha clean-up for cut-outs: drop specks, kill haze and halos beyond the
//! validator's 2-px fringe, and decontaminate edge colours.

// Pixel indices are bounded by image dimensions (u32), so casts are exact.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use crate::ops::{distance_to, nearest_colour, to_u8};
use arda_tactical::Rgba;
use std::collections::VecDeque;

/// Alpha at or above this is solid (the validator's `opaque_alpha`).
pub const SOLID: u8 = 240;
/// Semi-transparent pixels may sit this far from a solid one (validator).
pub const FRINGE_PX: u32 = 2;
/// Alpha below this is noise and becomes 0.
const ALPHA_FLOOR: u8 = 8;
/// Components smaller than this share of the largest are specks.
const SPECK_SHARE: usize = 50;

/// Removes solid components smaller than 2 % of the largest (8-connected,
/// alpha ≥ 128), with their soft surroundings. Returns the pixels removed.
pub fn remove_specks(img: &mut Rgba) -> usize {
    let (w, h) = (img.width as usize, img.height as usize);
    let solid: Vec<bool> = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| p[3] >= 128)
        .collect();
    let mut label = vec![usize::MAX; w * h];
    let mut sizes = Vec::new();
    for start in 0..w * h {
        if !solid[start] || label[start] != usize::MAX {
            continue;
        }
        let id = sizes.len();
        let mut n = 0;
        let mut queue = VecDeque::from([start]);
        label[start] = id;
        while let Some(i) = queue.pop_front() {
            n += 1;
            let (x, y) = ((i % w) as i64, (i / w) as i64);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
                        continue;
                    }
                    let j = ny as usize * w + nx as usize;
                    if solid[j] && label[j] == usize::MAX {
                        label[j] = id;
                        queue.push_back(j);
                    }
                }
            }
        }
        sizes.push(n);
    }
    let largest = sizes.iter().copied().max().unwrap_or(0);
    let keep: Vec<bool> = (0..w * h)
        .map(|i| label[i] != usize::MAX && sizes[label[i]] * SPECK_SHARE >= largest)
        .collect();
    let dist = distance_to(&keep, w, h);
    let mut removed = 0;
    for i in 0..w * h {
        let a = img.data[i * 4 + 3];
        if a > 0 && !keep[i] && (solid[i] || dist[i] > FRINGE_PX) {
            img.data[i * 4 + 3] = 0;
            removed += 1;
        }
    }
    removed
}

/// Sets alpha to 0 for noise (alpha < 8) and for semi-transparent pixels
/// further than [`FRINGE_PX`] from a solid one (haze, halos, glows).
/// Returns how many visible pixels were cleared.
pub fn kill_haze(img: &mut Rgba) -> usize {
    let (w, h) = (img.width as usize, img.height as usize);
    let solid: Vec<bool> = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| p[3] >= SOLID)
        .collect();
    let dist = distance_to(&solid, w, h);
    let mut cleared = 0;
    for (i, d) in dist.iter().enumerate() {
        let a = &mut img.data[i * 4 + 3];
        if *a > 0 && *a < SOLID && (*a < ALPHA_FLOOR || *d > FRINGE_PX) {
            *a = 0;
            cleared += 1;
        }
    }
    for p in img.data.as_chunks_mut::<4>().0.iter_mut() {
        if p[3] == 0 {
            *p = [0; 4];
        }
    }
    cleared
}

/// Pulls the colour of semi-transparent edge pixels toward the nearest solid
/// colour, removing backdrop spill (white or green rims). Returns how many
/// pixels changed.
pub fn defringe(img: &mut Rgba) -> usize {
    let w = img.width as usize;
    let solid: Vec<bool> = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| p[3] >= SOLID)
        .collect();
    if !solid.iter().any(|&s| s) {
        return 0;
    }
    let near = nearest_colour(img, &solid);
    let mut changed = 0;
    for (i, n) in near.iter().enumerate() {
        let p = img.get((i % w) as u32, (i / w) as u32);
        if p[3] == 0 || p[3] >= SOLID {
            continue;
        }
        let t = f32::from(p[3]) / 255.0;
        let k = t * t;
        let rgb: [u8; 3] =
            std::array::from_fn(|c| to_u8(f32::from(n[c]) * (1.0 - k) + f32::from(p[c]) * k));
        if rgb != [p[0], p[1], p[2]] {
            changed += 1;
        }
        img.set(
            (i % w) as u32,
            (i / w) as u32,
            [rgb[0], rgb[1], rgb[2], p[3]],
        );
    }
    changed
}

/// Tight bounding box `(x0, y0, x1, y1)` (exclusive end) of alpha > 0.
#[must_use]
pub fn bbox(img: &Rgba) -> Option<(u32, u32, u32, u32)> {
    let mut b: Option<(u32, u32, u32, u32)> = None;
    for y in 0..img.height {
        for x in 0..img.width {
            if img.get(x, y)[3] > 0 {
                b = Some(match b {
                    None => (x, y, x + 1, y + 1),
                    Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1)),
                });
            }
        }
    }
    b
}

/// Copies a sub-rectangle.
#[must_use]
pub fn crop(img: &Rgba, (x0, y0, x1, y1): (u32, u32, u32, u32)) -> Rgba {
    let mut out = Rgba::new(x1 - x0, y1 - y0);
    for y in y0..y1 {
        for x in x0..x1 {
            out.set(x - x0, y - y0, img.get(x, y));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn haze_beyond_two_pixels_is_cleared() {
        let mut img = Rgba::new(20, 20);
        for x in 0..20 {
            img.set(x, 10, [10, 10, 10, 255]);
            img.set(x, 12, [10, 10, 10, 100]);
            img.set(x, 15, [10, 10, 10, 100]);
        }
        assert_eq!(kill_haze(&mut img), 20);
        assert_eq!(img.get(3, 12)[3], 100);
        assert_eq!(img.get(3, 15)[3], 0);
    }

    #[test]
    fn specks_go_and_the_object_stays() {
        let mut img = Rgba::new(30, 30);
        for y in 5..25 {
            for x in 5..25 {
                img.set(x, y, [200, 0, 0, 255]);
            }
        }
        img.set(1, 1, [0, 0, 0, 255]);
        assert_eq!(remove_specks(&mut img), 1);
        assert_eq!(img.get(10, 10)[3], 255);
    }
}
