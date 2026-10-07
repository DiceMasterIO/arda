//! Stochastic tiling of ground textures (goal 62: no visible repetition).
//!
//! The world is cut into cells of [`CELL`] squares whose borders are
//! domain-warped, so they never follow the grid. Each cell picks a texture
//! variant and, for natural textures, a random offset into it. Inside a cell
//! one texel is fetched; within a narrow band around its border the
//! neighbouring cells' samples are blended with a variance-preserving
//! blend (Heitz and Neyret 2018), which keeps contrast instead of washing
//! out to a blurry average. Structured textures (cobbles, boards, slabs,
//! furrows) keep their alignment and only cross-fade between variants,
//! whose layouts match.
//!
//! Natural variants are flattened when the set is built ([`flatten`]):
//! most of their low-frequency tone is removed so the cells never show as
//! a checker of lighter and darker squares, and one variant covers a
//! region of 3 × 3 cells.
// Pixel, grid and cell indices are bounded by the canvas size (at most
// 1024 ppsq × a few hundred squares), so these conversions cannot lose
// meaningful range.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]

use super::field::Frame;
use crate::library::Library;
use crate::noise::{hash2, mix};
use crate::raster::Rgba;
use std::collections::BTreeMap;

/// Cell size in squares.
pub const CELL: f32 = 1.75;
/// Blend band at each cell border, as a fraction of the cell.
const BAND: f32 = 0.22;

/// Every variant of one ground key, scaled to the output resolution.
pub struct KeyTextures {
    /// Variant images, all the same size.
    pub variants: Vec<Rgba>,
    /// Whether the variants must stay aligned (free tag `structured`).
    pub structured: bool,
    /// Mean colour over all variants, for the variance-preserving blend.
    pub mean: [f32; 3],
    pow2: bool,
}

/// Share of each natural ground variant's low-frequency tone removed by
/// [`flatten`]: variants keep their grain but share one tone.
const FLATTEN: f32 = 0.85;
/// The same for water, whose darker pools are part of the look.
const FLATTEN_WATER: f32 = 0.5;

/// Wrap-around box blur of one channel plane, radius `r` pixels, applied
/// along x then y.
fn blur_wrap(plane: &[f32], w: usize, h: usize, r: usize) -> Vec<f32> {
    let k = (2 * r + 1) as f32;
    let mut tmp = vec![0.0f32; plane.len()];
    for y in 0..h {
        let row = &plane[y * w..(y + 1) * w];
        let mut acc: f32 = (0..=2 * r).map(|i| row[(i + w - r % w) % w]).sum();
        for x in 0..w {
            tmp[y * w + x] = acc / k;
            acc += row[(x + r + 1) % w] - row[(x + w - r % w) % w];
        }
    }
    let mut out = vec![0.0f32; plane.len()];
    for x in 0..w {
        let at = |y: usize| tmp[(y % h) * w + x];
        let mut acc: f32 = (0..=2 * r).map(|i| at(i + h - r % h)).sum();
        for y in 0..h {
            out[y * w + x] = acc / k;
            acc += at(y + r + 1) - at(y + h - r % h);
        }
    }
    out
}

/// Removes most of a natural texture variant's low-frequency tone (its
/// wrap-around blur at a quarter of its width) and re-centres it on the
/// key's mean over all variants. AI and photo textures carry lighting
/// gradients and per-variant tone shifts; stochastic tiling cuts them into
/// 1.75-square cells, which read as a blocky checker. Flattened variants
/// differ only in grain, so cell borders vanish; the compositor's own
/// world-space macro tint supplies the large-scale variation instead.
fn flatten(img: &Rgba, mean: [f32; 3], strength: f32) -> Rgba {
    let (w, h) = (img.width as usize, img.height as usize);
    let r = (w.min(h) / 8).max(1);
    if w < 4 || h < 4 {
        return img.clone();
    }
    let mut out = img.clone();
    for ch in 0..3 {
        let plane: Vec<f32> = img
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| f32::from(p[ch]))
            .collect();
        let low = blur_wrap(&plane, w, h, r);
        for (i, px) in out.data.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let v = plane[i] - strength * (low[i] - mean[ch]);
            px[ch] = v.round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

fn mean_of(variants: &[Rgba]) -> [f32; 3] {
    let mut sum = [0.0f64; 3];
    let mut n = 0u64;
    for img in variants {
        for p in img.data.as_chunks::<4>().0 {
            for (s, v) in sum.iter_mut().zip(p) {
                *s += f64::from(*v);
            }
            n += 1;
        }
    }
    sum.map(|s| (s / n.max(1) as f64) as f32)
}

impl KeyTextures {
    fn new(key: &str, variants: Vec<Rgba>, structured: bool) -> Self {
        let variants = if structured {
            variants
        } else {
            let mean = mean_of(&variants);
            let strength = if key.starts_with("water") {
                FLATTEN_WATER
            } else {
                FLATTEN
            };
            variants
                .iter()
                .map(|v| flatten(v, mean, strength))
                .collect()
        };
        let mut sum = [0.0f64; 3];
        let mut n = 0u64;
        for img in &variants {
            for p in img.data.as_chunks::<4>().0 {
                for (s, v) in sum.iter_mut().zip(p) {
                    *s += f64::from(*v);
                }
                n += 1;
            }
        }
        // Means of bounded bytes: the narrowing is exact enough.
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        let mean = sum.map(|s| (s / n.max(1) as f64) as f32);
        let pow2 = variants
            .first()
            .is_some_and(|f| f.width.is_power_of_two() && f.height.is_power_of_two());
        Self {
            variants,
            structured,
            mean,
            pow2,
        }
    }

    /// Luma of the mean colour, in `[0, 1]`.
    #[must_use]
    pub fn mean_luma(&self) -> f32 {
        (0.3 * self.mean[0] + 0.59 * self.mean[1] + 0.11 * self.mean[2]) / 255.0
    }
}

/// Ground and water texture variants by key, scaled to the output ppsq.
#[derive(Default)]
pub struct TextureSet {
    by_key: BTreeMap<String, KeyTextures>,
}

impl TextureSet {
    /// Scales every ground and water texture in `lib` to `ppsq`.
    #[must_use]
    pub fn new(lib: &Library, ppsq: u32) -> Self {
        let mut raw: BTreeMap<String, (Vec<Rgba>, bool)> = BTreeMap::new();
        for a in lib.catalog.assets.iter().filter(|a| a.class.is_texture()) {
            if let (Some(key), Some(img)) = (&a.ground, lib.image(&a.id)) {
                let scaled = img.resized(a.footprint.w * ppsq, a.footprint.h * ppsq);
                let e = raw.entry(key.clone()).or_default();
                // Variants of one key must share a size to share cells.
                if e.0
                    .first()
                    .is_none_or(|f| f.width == scaled.width && f.height == scaled.height)
                {
                    e.0.push(scaled);
                }
                e.1 |= a.tags.free.iter().any(|t| t == "structured");
            }
        }
        let by_key = raw
            .into_iter()
            .map(|(k, (v, s))| {
                let t = KeyTextures::new(&k, v, s);
                (k, t)
            })
            .collect();
        Self { by_key }
    }

    /// The textures of a key, if any.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&KeyTextures> {
        self.by_key.get(key).filter(|k| !k.variants.is_empty())
    }
}

/// Cells per side of a natural texture's variant region.
const REGION: i64 = 3;

/// One cell's choice: variant index and texel offset. Natural textures
/// share one variant over a region of [`REGION`] × [`REGION`] cells (each
/// cell still takes its own offset), so the grain changes character in
/// patches several squares across rather than cell by cell.
fn cell_pick(tex: &KeyTextures, salt: u64, ci: i64, cj: i64) -> (usize, i64, i64) {
    let h = hash2(salt, ci, cj);
    let n = tex.variants.len() as u64;
    if tex.structured {
        return (usize::try_from(h % n.max(1)).unwrap_or(0), 0, 0);
    }
    let region = hash2(salt ^ 0x7E61, ci.div_euclid(REGION), cj.div_euclid(REGION));
    let v = usize::try_from(region % n.max(1)).unwrap_or(0);
    let img = &tex.variants[v];
    let hx = mix(h ^ 0x51);
    let (w, hh) = (u64::from(img.width), u64::from(img.height));
    (
        v,
        i64::try_from(hx % w.max(1)).unwrap_or(0),
        i64::try_from(mix(hx) % hh.max(1)).unwrap_or(0),
    )
}

/// Precomputed cell choices of one key over a canvas (plus a margin for
/// the border warp), so sampling does no hashing.
pub struct CellTable {
    ci0: i64,
    cj0: i64,
    nw: i64,
    nh: i64,
    picks: Vec<(usize, i64, i64)>,
    salt: u64,
}

impl CellTable {
    /// The table of `tex` under `salt` for a canvas of `width × height`
    /// squares placed by `frame`.
    #[must_use]
    // Cell ranges are floors and ceilings of bounded world positions.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    pub fn new(tex: &KeyTextures, salt: u64, frame: &Frame, width: u32, height: u32) -> Self {
        let (ox, oy) = (frame.origin.0 as f32, frame.origin.1 as f32);
        let ci0 = ((ox - 2.0) / CELL).floor() as i64;
        let cj0 = ((oy - 2.0) / CELL).floor() as i64;
        let ci1 = ((ox + width as f32 + 2.0) / CELL).ceil() as i64;
        let cj1 = ((oy + height as f32 + 2.0) / CELL).ceil() as i64;
        let (nw, nh) = (ci1 - ci0 + 1, cj1 - cj0 + 1);
        let mut picks = Vec::with_capacity(usize::try_from(nw * nh).unwrap_or(0));
        for j in 0..nh {
            for i in 0..nw {
                picks.push(cell_pick(tex, salt, ci0 + i, cj0 + j));
            }
        }
        Self {
            ci0,
            cj0,
            nw,
            nh,
            picks,
            salt,
        }
    }

    fn get(&self, tex: &KeyTextures, ci: i64, cj: i64) -> (usize, i64, i64) {
        let (i, j) = (ci - self.ci0, cj - self.cj0);
        if (0..self.nw).contains(&i) && (0..self.nh).contains(&j) {
            self.picks[usize::try_from(j * self.nw + i).unwrap_or(0)]
        } else {
            cell_pick(tex, self.salt, ci, cj)
        }
    }
}

fn fetch(tex: &KeyTextures, v: usize, x: i64, y: i64) -> [f32; 3] {
    let img = &tex.variants[v];
    let (w, h) = (i64::from(img.width), i64::from(img.height));
    // Power-of-two textures (the usual case) wrap with a mask, which is
    // also exact for negative world coordinates in two's complement.
    let (xm, ym) = if tex.pow2 {
        (x & (w - 1), y & (h - 1))
    } else {
        (x.rem_euclid(w), y.rem_euclid(h))
    };
    let i = usize::try_from(ym * w + xm).unwrap_or(0) * 4;
    let d = &img.data[i..i + 3];
    [f32::from(d[0]), f32::from(d[1]), f32::from(d[2])]
}

/// Weights of the own cell and its neighbour along one axis.
fn axis_weights(f: f32) -> [(i64, f32); 2] {
    let g = f.min(1.0 - f);
    let side = if f < 0.5 { -1 } else { 1 };
    if g >= BAND / 2.0 {
        [(0, 1.0), (side, 0.0)]
    } else {
        let t = super::field::smooth(0.5 - g / BAND);
        [(0, 1.0 - t), (side, t)]
    }
}

/// Samples key `tex` at a canvas pixel. `warp` is the cell-border warp in
/// squares (from a coarse field) and `cells` the key's cell choices.
#[must_use]
// Cell indices are floors of bounded world positions.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn sample(
    tex: &KeyTextures,
    cells: &CellTable,
    frame: &Frame,
    px: u32,
    py: u32,
    warp: (f32, f32),
) -> [f32; 3] {
    let (wx, wy) = frame.world_px(px, py);
    let (u, v) = frame.world_sq(px as f32 + 0.5, py as f32 + 0.5);
    let cx = (u + warp.0) / CELL;
    let cy = (v + warp.1) / CELL;
    let (fx, fy) = (cx.floor(), cy.floor());
    let (ci, cj) = (fx as i64, fy as i64);
    let (ax, ay) = (axis_weights(cx - fx), axis_weights(cy - fy));
    let mut acc = [0.0f32; 3];
    let mut w2 = 0.0f32;
    let mut wsum = 0.0f32;
    for (dx, wxw) in ax {
        for (dy, wyw) in ay {
            let w = wxw * wyw;
            if w <= 0.0 {
                continue;
            }
            let (v, ox, oy) = cells.get(tex, ci + dx, cj + dy);
            let c = fetch(tex, v, wx + ox, wy + oy);
            for k in 0..3 {
                acc[k] += w * (c[k] - tex.mean[k]);
            }
            w2 += w * w;
            wsum += w;
        }
    }
    if tex.structured || w2 >= 0.999 {
        // Aligned variants cross-fade linearly; a single sample is exact.
        let k = 1.0 / wsum.max(1e-6);
        return [
            tex.mean[0] + acc[0] * k,
            tex.mean[1] + acc[1] * k,
            tex.mean[2] + acc[2] * k,
        ];
    }
    let k = 1.0 / w2.sqrt();
    [
        (tex.mean[0] + acc[0] * k).clamp(0.0, 255.0),
        (tex.mean[1] + acc[1] * k).clamp(0.0, 255.0),
        (tex.mean[2] + acc[2] * k).clamp(0.0, 255.0),
    ]
}
