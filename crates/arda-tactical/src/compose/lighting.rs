//! The lighting pass (goal 63): top-left drop shadows from a height buffer,
//! ambient occlusion at wall bases and under props, darkened water banks and
//! warm light pools. All integer maths, parallel by rows or column strips
//! with every pixel a pure function of its inputs, so output stays
//! byte-identical.
//!
//! Shadows are graded, not binary: a pixel's darkness grows with how far
//! it lies below the sun line from its occluder, so each shadow fades out
//! towards its tip, and a three-pass box blur (close to a Gaussian) softens
//! its edges. Near an occluder the shadow is crisp and dark; far from it,
//! soft and faint.
//!
//! The art carries no shadows; everything here is derived from each asset's
//! `height_ft` hint and the layout's elevations.
// Pixel, grid and cell indices are bounded by the canvas size (at most
// 1024 ppsq × a few hundred squares), so these conversions cannot lose
// meaningful range.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]

use crate::raster::Rgba;
use rayon::prelude::*;

/// Per-pixel buffers gathered while compositing.
pub struct Buffers {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Surface height in 1/256 ft (ground elevation plus object heights).
    pub height_map: Vec<i32>,
    /// Occluder coverage for ambient occlusion (props and walls), 0–255.
    pub occluders: Vec<u8>,
    /// Water coverage, 0–255.
    pub water: Vec<u8>,
}

impl Buffers {
    /// Empty buffers.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        let n = width as usize * height as usize;
        Self {
            width,
            height,
            height_map: vec![0; n],
            occluders: vec![0; n],
            water: vec![0; n],
        }
    }
}

/// Lighting strengths, in 1/256.
#[derive(Debug, Clone, Copy)]
pub struct Lighting {
    /// Shadow length in 1/1024 square per foot of height.
    pub shadow_len: u32,
    /// Shadow darkening at full coverage.
    pub shadow: u32,
    /// Ambient-occlusion darkening at full coverage.
    pub occlusion: u32,
    /// Bank darkening on water next to land.
    pub bank: u32,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            shadow_len: 56,
            shadow: 110,
            occlusion: 96,
            bank: 90,
        }
    }
}

/// A light pool to add: centre and radius in pixels, colour.
pub struct Pool {
    /// Centre x in pixels.
    pub x: i64,
    /// Centre y in pixels.
    pub y: i64,
    /// Radius in pixels.
    pub r: i64,
    /// RGB colour.
    pub colour: [u8; 3],
}

/// Height below the sun line (1/256 ft) at which a shadow is fully dark.
pub(super) const FADE: i32 = 3 * 256;

/// Offset silhouettes cast by tree canopies, and canopy coverage, both
/// 0–255 per pixel. Canopies do not sweep a shadow along their height: a
/// top-down map shows a short, soft offset of the crown instead.
pub struct Casts {
    /// The canopy silhouettes, already offset towards the bottom-right.
    pub silhouettes: Vec<u8>,
    /// Canopy coverage; canopies are not darkened by these shadows.
    pub tops: Vec<u8>,
}

impl Casts {
    /// Empty buffers for a `width × height` canvas.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        let n = width as usize * height as usize;
        Self {
            silhouettes: vec![0; n],
            tops: vec![0; n],
        }
    }
}

/// Applies shadows, occlusion, bank darkening and light pools in place.
pub fn apply(canvas: &mut Rgba, buf: &Buffers, ppsq: u32, l: &Lighting, pools: &[Pool]) {
    apply_with(
        canvas,
        buf,
        &Casts::new(buf.width, buf.height),
        ppsq,
        l,
        pools,
    );
}

/// A half-resolution copy of a mask: `reduce` combines each 2 × 2 block.
fn half<T: Copy + Send + Sync, U: Copy + Send + Default>(
    src: &[T],
    w: usize,
    h: usize,
    reduce: impl Fn([T; 4]) -> U + Sync,
) -> Vec<U> {
    let (hw, hh) = (w.div_ceil(2), h.div_ceil(2));
    let mut out = vec![U::default(); hw * hh];
    out.par_chunks_mut(hw).enumerate().for_each(|(j, row)| {
        let (y0, y1) = (2 * j, (2 * j + 1).min(h - 1));
        for (i, o) in row.iter_mut().enumerate() {
            let (x0, x1) = (2 * i, (2 * i + 1).min(w - 1));
            *o = reduce([
                src[y0 * w + x0],
                src[y0 * w + x1],
                src[y1 * w + x0],
                src[y1 * w + x1],
            ]);
        }
    });
    out
}

/// Upsampling taps along one axis: for each full-resolution coordinate,
/// the two half-resolution neighbours and their weights in quarters.
fn taps(n_full: usize, n_half: usize) -> Vec<(usize, usize, u32, u32)> {
    (0..n_full)
        .map(|v| {
            let q = (2 * v).cast_signed() - 1;
            let i = q.div_euclid(4);
            let f = u32::try_from(q.rem_euclid(4)).unwrap_or(0);
            let a = usize::try_from(i.clamp(0, n_half.cast_signed() - 1)).unwrap_or(0);
            let b = usize::try_from((i + 1).clamp(0, n_half.cast_signed() - 1)).unwrap_or(0);
            (a, b, 4 - f, f)
        })
        .collect()
}

/// Bilinear sample of a half-resolution mask through precomputed taps.
fn up(src: &[u8], hw: usize, tx: (usize, usize, u32, u32), ty: (usize, usize, u32, u32)) -> u32 {
    let (x0, x1, wx0, wx1) = tx;
    let (r0, r1) = (ty.0 * hw, ty.1 * hw);
    let top = u32::from(src[r0 + x0]) * wx0 + u32::from(src[r0 + x1]) * wx1;
    let bottom = u32::from(src[r1 + x0]) * wx0 + u32::from(src[r1 + x1]) * wx1;
    (top * ty.2 + bottom * ty.3 + 8) / 16
}

/// Like [`apply`], with canopy offset shadows. The shadow, occlusion and
/// bank masks are computed at half resolution (they are all soft) and
/// upsampled bilinearly, which quarters the cost of the blurs.
pub fn apply_with(
    canvas: &mut Rgba,
    buf: &Buffers,
    casts: &Casts,
    ppsq: u32,
    l: &Lighting,
    pools: &[Pool],
) {
    let all = [0, 0, canvas.width, canvas.height];
    apply_clipped(canvas, (buf, casts), ppsq, l, pools, all);
}

/// [`apply_with`] on pixels `[x0, y0, x1, y1)` only. The masks still cover
/// the whole canvas (they reach across pixels); only the per-pixel shading
/// and the light pools are clipped, so the pixels inside are unchanged.
pub fn apply_clipped(
    canvas: &mut Rgba,
    (buf, casts): (&Buffers, &Casts),
    ppsq: u32,
    l: &Lighting,
    pools: &[Pool],
    [x0, y0, x1, y1]: [u32; 4],
) {
    let (w, h) = (buf.width as usize, buf.height as usize);
    let (hw, hh) = (w.div_ceil(2), h.div_ceil(2));
    let hp = (ppsq / 2).max(1);
    let heights = half(&buf.height_map, w, h, |v| {
        v[0].max(v[1]).max(v[2]).max(v[3])
    });
    let half_buf = Buffers {
        width: u32::try_from(hw).unwrap_or(0),
        height: u32::try_from(hh).unwrap_or(0),
        height_map: heights,
        occluders: Vec::new(),
        water: Vec::new(),
    };
    let max4 = |v: [u8; 4]| v[0].max(v[1]).max(v[2]).max(v[3]);
    let mean4 =
        |v: [u8; 4]| crate::raster::to_u8((v.iter().map(|&x| u32::from(x)).sum::<u32>() + 2) / 4);
    let sweep = soften(
        &super::shadows::drop_shadows(&half_buf, hp, l),
        hw,
        hh,
        (hp / 26).max(1),
        3,
    );
    let crown = soften(
        &half(&casts.silhouettes, w, h, mean4),
        hw,
        hh,
        (hp / 9).max(1),
        3,
    );
    let ao = soften(
        &half(&buf.occluders, w, h, max4),
        hw,
        hh,
        (hp / 10).max(1),
        2,
    );
    let land: Vec<u8> = half(&buf.water, w, h, mean4)
        .par_iter()
        .map(|&v| 255 - v)
        .collect();
    let bank = soften(&land, hw, hh, (hp / 7).max(1), 2);
    let (xt, yt) = (taps(w, hw), taps(h, hh));
    canvas
        .data
        .par_chunks_mut(w * 4)
        .enumerate()
        .for_each(|(y, row)| {
            if y < y0 as usize || y >= y1 as usize {
                return;
            }
            let ty = yt[y];
            let span = x0 as usize..(x1 as usize).min(w);
            for (x, p) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                if !span.contains(&x) {
                    continue;
                }
                let i = y * w + x;
                let tx = xt[x];
                // Canopy shadows: slightly darker than a sweep, never on
                // the canopies themselves.
                let c = up(&crown, hw, tx, ty) * (255 - u32::from(casts.tops[i])) / 255;
                let shade = up(&sweep, hw, tx, ty).max(c * 230 / 255);
                let mut dark = shade * l.shadow;
                // Occlusion only darkens the surfaces around occluders, not
                // their tops.
                let open = 255 - u32::from(buf.occluders[i]);
                dark += up(&ao, hw, tx, ty) * l.occlusion * open / 255;
                let wet = u32::from(buf.water[i]);
                if wet > 0 {
                    dark += up(&bank, hw, tx, ty) * l.bank * wet / 255;
                }
                if dark == 0 {
                    continue;
                }
                let keep = 65_025u32.saturating_sub(dark.min(65_025 * 3 / 4));
                for c in &mut p[..3] {
                    *c = crate::raster::to_u8(u32::from(*c) * keep / 65_025);
                }
            }
        });
    add_pools(canvas, pools, [x0, y0, x1, y1]);
}

/// `passes` separable box blurs of radius `r`: a cheap soft edge.
fn soften(mask: &[u8], w: usize, h: usize, r: u32, passes: u32) -> Vec<u8> {
    let mut out = box_blur(mask, w, h, r as usize);
    for _ in 1..passes {
        out = box_blur(&out, w, h, r as usize);
    }
    out
}

/// Column strip width for the vertical blur.
const STRIP: usize = 64;

fn box_blur(src: &[u8], w: usize, h: usize, r: usize) -> Vec<u8> {
    let win = u32::try_from(2 * r + 1).unwrap_or(u32::MAX);
    let mut tmp = vec![0u8; w * h];
    tmp.par_chunks_mut(w)
        .zip(src.par_chunks(w))
        .for_each(|(o, s)| blur_line(|i| s[i], w, r, win, |i, v| o[i] = v));
    // Vertical: independent column strips with running sums, then scatter.
    let strips: Vec<Vec<u8>> = (0..w.div_ceil(STRIP))
        .into_par_iter()
        .map(|si| {
            let (c0, c1) = (si * STRIP, (si * STRIP + STRIP).min(w));
            let sw = c1 - c0;
            let mut out = vec![0u8; sw * h];
            let at = |y: isize, c: usize| {
                let yy = usize::try_from(y.clamp(0, h.cast_signed() - 1)).unwrap_or(0);
                u32::from(tmp[yy * w + c0 + c])
            };
            let ri = r.cast_signed();
            let mut sums: Vec<u32> = (0..sw)
                .map(|c| (-ri..=ri).map(|y| at(y, c)).sum())
                .collect();
            for y in 0..h {
                let yi = y.cast_signed();
                for (c, sum) in sums.iter_mut().enumerate() {
                    out[y * sw + c] = crate::raster::to_u8((*sum + win / 2) / win);
                    *sum = *sum + at(yi + ri + 1, c) - at(yi - ri, c);
                }
            }
            out
        })
        .collect();
    let mut out = vec![0u8; w * h];
    out.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
        for (si, st) in strips.iter().enumerate() {
            let c0 = si * STRIP;
            let sw = st.len() / h.max(1);
            row[c0..c0 + sw].copy_from_slice(&st[y * sw..(y + 1) * sw]);
        }
    });
    out
}

/// Sliding-window mean along one line, clamping at the ends.
fn blur_line(
    get: impl Fn(usize) -> u8,
    n: usize,
    r: usize,
    win: u32,
    mut put: impl FnMut(usize, u8),
) {
    let at = |i: isize| {
        u32::from(get(
            usize::try_from(i.clamp(0, n.cast_signed() - 1)).unwrap_or(0)
        ))
    };
    let ri = r.cast_signed();
    let mut sum: u32 = (-ri..=ri).map(at).sum();
    for i in 0..n {
        put(i, crate::raster::to_u8((sum + win / 2) / win));
        let ii = i.cast_signed();
        sum = sum + at(ii + ri + 1) - at(ii - ri);
    }
}

/// Adds every light pool to pixels `[x0, y0, x1, y1)`, row-parallel.
fn add_pools(canvas: &mut Rgba, pools: &[Pool], [x0, y0, x1, y1]: [u32; 4]) {
    if pools.is_empty() {
        return;
    }
    let w = canvas.width as usize;
    let span = (i64::from(x0), i64::from(x1));
    canvas
        .data
        .par_chunks_mut(w * 4)
        .enumerate()
        .for_each(|(y, row)| {
            if y < y0 as usize || y >= y1 as usize {
                return;
            }
            let y = i64::try_from(y).unwrap_or(0);
            for p in pools {
                if y < p.y - p.r || y >= p.y + p.r {
                    continue;
                }
                add_pool_row(row, y, p, span);
            }
        });
}

fn add_pool_row(row: &mut [u8], y: i64, p: &Pool, (sx0, sx1): (i64, i64)) {
    let r2 = p.r * p.r;
    let w = i64::try_from(row.len() / 4).unwrap_or(0).min(sx1);
    for x in (p.x - p.r).max(0).max(sx0)..(p.x + p.r).min(w) {
        let d2 = (x - p.x) * (x - p.x) + (y - p.y) * (y - p.y);
        if d2 >= r2 {
            continue;
        }
        // Quadratic fall-off, peak 30 % towards the light colour ("screen").
        let f = u64::try_from((r2 - d2) * 77 / r2).unwrap_or(0);
        let f = f * f / 77;
        let i = usize::try_from(x).unwrap_or(0) * 4;
        for c in 0..3 {
            let v = u64::from(row[i + c]);
            let screen = 255 - (255 - v) * (255 - u64::from(p.colour[c])) / 255;
            row[i + c] = u8::try_from(v + (screen - v) * f / 255).unwrap_or(255);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blur_preserves_flat_fields() {
        let src = vec![200u8; 30 * 20];
        assert!(soften(&src, 30, 20, 3, 3).iter().all(|&v| v == 200));
    }
}
