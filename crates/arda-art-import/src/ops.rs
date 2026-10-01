//! Shared raster operations: luminance, distance transforms, nearest-colour
//! propagation, box blurs and a Mitchell–Netravali resampler.
//!
//! Everything is single-threaded IEEE `f32`/integer maths with only
//! `+ − × ÷` and `sqrt`, so results are byte-identical across runs.

// Pixel indices and filter taps are bounded by image dimensions (u32), so
// the integer casts here are exact.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use arda_tactical::Rgba;
use std::collections::VecDeque;

/// Rec. 709 luma of an RGB(A) pixel, 0–255.
#[must_use]
pub fn luma(p: [u8; 4]) -> f32 {
    0.2126 * f32::from(p[0]) + 0.7152 * f32::from(p[1]) + 0.0722 * f32::from(p[2])
}

/// Max-channel RGB distance.
#[must_use]
pub fn chan_dist(a: [u8; 4], b: [u8; 4]) -> u8 {
    (0..3).map(|c| a[c].abs_diff(b[c])).max().unwrap_or(0)
}

/// Saturating `f32` → `u8` with rounding.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn to_u8(v: f32) -> u8 {
    (v + 0.5).clamp(0.0, 255.0) as u8
}

/// Chebyshev distance from every pixel to the nearest `true` pixel of
/// `mask` (0 on the mask; `u32::MAX` when the mask is empty).
#[must_use]
pub fn distance_to(mask: &[bool], w: usize, h: usize) -> Vec<u32> {
    let mut d: Vec<u32> = mask.iter().map(|&m| if m { 0 } else { u32::MAX }).collect();
    let relax = |d: &mut [u32], i: usize, j: usize| {
        let v = d[j].saturating_add(1);
        if v < d[i] {
            d[i] = v;
        }
    };
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if x > 0 {
                relax(&mut d, i, i - 1);
            }
            if y > 0 {
                relax(&mut d, i, i - w);
                if x > 0 {
                    relax(&mut d, i, i - w - 1);
                }
                if x + 1 < w {
                    relax(&mut d, i, i - w + 1);
                }
            }
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            if x + 1 < w {
                relax(&mut d, i, i + 1);
            }
            if y + 1 < h {
                relax(&mut d, i, i + w);
                if x + 1 < w {
                    relax(&mut d, i, i + w + 1);
                }
                if x > 0 {
                    relax(&mut d, i, i + w - 1);
                }
            }
        }
    }
    d
}

/// For every pixel, the RGB of the nearest pixel where `known` holds, by
/// breadth-first propagation in scan order (pixels with no known pixel
/// reachable keep their own colour).
#[must_use]
pub fn nearest_colour(img: &Rgba, known: &[bool]) -> Vec<[u8; 3]> {
    let (w, h) = (img.width as usize, img.height as usize);
    let mut out: Vec<[u8; 3]> = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| [p[0], p[1], p[2]])
        .collect();
    let mut seen = known.to_vec();
    let mut queue: VecDeque<usize> = (0..w * h).filter(|&i| known[i]).collect();
    while let Some(i) = queue.pop_front() {
        let (x, y) = (i % w, i / w);
        for (dx, dy) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)] {
            let (nx, ny) = (x as i64 + dx, y as i64 + dy);
            if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
                continue;
            }
            let j = ny as usize * w + nx as usize;
            if !seen[j] {
                seen[j] = true;
                out[j] = out[i];
                queue.push_back(j);
            }
        }
    }
    out
}

/// In-place box blur of a plane, three passes, wrapping or clamping at the
/// borders. Three box passes approximate a Gaussian of sigma ≈ r.
pub fn box_blur(plane: &mut [f32], w: usize, h: usize, r: usize, wrap: bool) {
    let mut tmp = vec![0.0f32; plane.len()];
    for _ in 0..3 {
        blur_axis(plane, &mut tmp, (w, h), r, wrap, true);
        blur_axis(&tmp, plane, (w, h), r, wrap, false);
    }
}

fn blur_axis(
    src: &[f32],
    dst: &mut [f32],
    (w, h): (usize, usize),
    r: usize,
    wrap: bool,
    horizontal: bool,
) {
    let (len, lines) = if horizontal { (w, h) } else { (h, w) };
    let at = |line: usize, k: usize| {
        if horizontal {
            line * w + k
        } else {
            k * w + line
        }
    };
    let n = (2 * r + 1) as f32;
    for line in 0..lines {
        for k in 0..len {
            let mut s = 0.0;
            for o in 0..=2 * r {
                let p = k as i64 + o as i64 - r as i64;
                let p = if wrap {
                    p.rem_euclid(len as i64)
                } else {
                    p.clamp(0, len as i64 - 1)
                };
                s += src[at(line, p as usize)];
            }
            dst[at(line, k)] = s / n;
        }
    }
}

/// Mitchell–Netravali (B = C = 1/3) kernel.
fn mitchell(x: f32) -> f32 {
    let x = x.abs();
    let (b, c) = (1.0 / 3.0, 1.0 / 3.0);
    if x < 1.0 {
        ((12.0 - 9.0 * b - 6.0 * c) * x * x * x
            + (-18.0 + 12.0 * b + 6.0 * c) * x * x
            + (6.0 - 2.0 * b))
            / 6.0
    } else if x < 2.0 {
        ((-b - 6.0 * c) * x * x * x
            + (6.0 * b + 30.0 * c) * x * x
            + (-12.0 * b - 48.0 * c) * x
            + (8.0 * b + 24.0 * c))
            / 6.0
    } else {
        0.0
    }
}

/// Per-output-sample taps `(source index, weight)` along one axis.
fn taps(src: usize, dst: usize, wrap: bool) -> Vec<Vec<(usize, f32)>> {
    let scale = dst as f32 / src as f32;
    let shrink = scale.min(1.0);
    let support = 2.0 / shrink;
    (0..dst)
        .map(|i| {
            let centre = (i as f32 + 0.5) / scale - 0.5;
            let lo = (centre - support).floor() as i64;
            let hi = (centre + support).ceil() as i64;
            let mut t: Vec<(usize, f32)> = (lo..=hi)
                .map(|j| {
                    let wgt = mitchell((j as f32 - centre) * shrink);
                    let idx = if wrap {
                        j.rem_euclid(src as i64)
                    } else {
                        j.clamp(0, src as i64 - 1)
                    };
                    (idx as usize, wgt)
                })
                .filter(|(_, wgt)| *wgt != 0.0)
                .collect();
            let sum: f32 = t.iter().map(|(_, wgt)| wgt).sum();
            if sum != 0.0 {
                for (_, wgt) in &mut t {
                    *wgt /= sum;
                }
            }
            t
        })
        .collect()
}

/// Resamples with a separable Mitchell filter in premultiplied alpha, so
/// transparent pixels never bleed their colour into edges. `wrap` samples
/// across the borders toroidally (for tiling textures).
#[must_use]
pub fn resize(img: &Rgba, w: u32, h: u32, wrap: bool) -> Rgba {
    if (img.width, img.height) == (w, h) {
        return img.clone();
    }
    let (sw, sh) = (img.width as usize, img.height as usize);
    let (dw, dh) = (w as usize, h as usize);
    let pre: Vec<[f32; 4]> = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| {
            let a = f32::from(p[3]) / 255.0;
            [
                f32::from(p[0]) * a,
                f32::from(p[1]) * a,
                f32::from(p[2]) * a,
                f32::from(p[3]),
            ]
        })
        .collect();
    let tx = taps(sw, dw, wrap);
    let ty = taps(sh, dh, wrap);
    let mut mid = vec![[0.0f32; 4]; dw * sh];
    for y in 0..sh {
        for (x, t) in tx.iter().enumerate() {
            let mut acc = [0.0f32; 4];
            for &(sx, wgt) in t {
                let p = pre[y * sw + sx];
                for c in 0..4 {
                    acc[c] += p[c] * wgt;
                }
            }
            mid[y * dw + x] = acc;
        }
    }
    let mut out = Rgba::new(w, h);
    for (y, t) in ty.iter().enumerate() {
        for x in 0..dw {
            let mut acc = [0.0f32; 4];
            for &(sy, wgt) in t {
                let p = mid[sy * dw + x];
                for c in 0..4 {
                    acc[c] += p[c] * wgt;
                }
            }
            let a = acc[3].clamp(0.0, 255.0);
            let px = if a < 0.5 {
                [0, 0, 0, 0]
            } else {
                let k = 255.0 / a;
                [
                    to_u8(acc[0] * k),
                    to_u8(acc[1] * k),
                    to_u8(acc[2] * k),
                    to_u8(a),
                ]
            };
            out.set(x as u32, y as u32, px);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_is_chebyshev() {
        let mut m = vec![false; 25];
        m[12] = true;
        let d = distance_to(&m, 5, 5);
        assert_eq!(d[0], 2);
        assert_eq!(d[2], 2);
        assert_eq!(d[7], 1);
    }

    #[test]
    fn resize_keeps_flat_colour_and_premultiplies() {
        let img = Rgba::filled(10, 10, [40, 90, 200, 255]);
        let r = resize(&img, 7, 13, false);
        assert!(r
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [40, 90, 200, 255]));
        let mut half = Rgba::new(8, 8);
        for y in 0..8 {
            for x in 0..4 {
                half.set(x, y, [200, 10, 10, 255]);
            }
        }
        let small = resize(&half, 4, 4, false);
        // Edge pixels keep the opaque colour; only alpha falls off.
        let p = small.get(2, 2);
        assert!(p[0] > 190 && p[1] < 20, "{p:?}");
    }
}
