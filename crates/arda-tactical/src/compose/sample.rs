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
/// Spread of the variants' mean luma (0–255) above which a structured
/// key's variants are different finishes (pale and dark boards, light and
/// dark furrows) rather than one surface: a room or field then takes one
/// finish instead of a patchwork of cells (see [`KeyTextures::finishes`]).
pub const FINISH_SPREAD: f32 = 16.0;
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
    /// Whether the variants are distinct finishes of a structured surface
    /// (their mean lumas spread over [`FINISH_SPREAD`]): one is chosen per
    /// enclosed room, else per key, never per cell.
    pub finishes: bool,
    pow2: bool,
}

impl KeyTextures {
    fn new(variants: Vec<Rgba>, structured: bool) -> Self {
        let mut sum = [0.0f64; 3];
        let mut n = 0u64;
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for img in &variants {
            let mut own = [0.0f64; 3];
            let mut k = 0u64;
            for p in img.data.as_chunks::<4>().0 {
                for ((s, o), v) in sum.iter_mut().zip(&mut own).zip(p) {
                    *s += f64::from(*v);
                    *o += f64::from(*v);
                }
                k += 1;
            }
            n += k;
            #[allow(clippy::cast_precision_loss)] // pixel counts
            let luma = (0.3 * own[0] + 0.59 * own[1] + 0.11 * own[2]) / k.max(1) as f64;
            lo = lo.min(luma);
            hi = hi.max(luma);
        }
        let finishes = structured && hi - lo > f64::from(FINISH_SPREAD);
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
            finishes,
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
            .map(|(k, (v, s))| (k, KeyTextures::new(v, s)))
            .collect();
        Self { by_key }
    }

    /// The textures of a key, if any.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&KeyTextures> {
        self.by_key.get(key).filter(|k| !k.variants.is_empty())
    }
}

/// One cell's choice: variant index and texel offset.
fn cell_pick(tex: &KeyTextures, salt: u64, ci: i64, cj: i64) -> (usize, i64, i64) {
    let h = hash2(salt, ci, cj);
    let n = tex.variants.len() as u64;
    let v = usize::try_from(h % n.max(1)).unwrap_or(0);
    if tex.structured {
        return (v, 0, 0);
    }
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
    sample_finish(tex, cells, frame, (px, py), warp, None)
}

/// [`sample`] with every cell's variant replaced by `finish` when given
/// (texel offsets are kept), so a surface shows one finish throughout.
#[must_use]
// Cell indices are floors of bounded world positions.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn sample_finish(
    tex: &KeyTextures,
    cells: &CellTable,
    frame: &Frame,
    (px, py): (u32, u32),
    warp: (f32, f32),
    finish: Option<usize>,
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
            let v = finish.filter(|&f| f < tex.variants.len()).unwrap_or(v);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(v: u8) -> Rgba {
        Rgba::filled(8, 8, [v, v, v, 255])
    }

    #[test]
    fn far_apart_structured_variants_are_finishes_and_a_finish_is_exact() {
        let near = KeyTextures::new(vec![solid(100), solid(110)], true);
        assert!(!near.finishes, "a 10-luma spread is one surface");
        let loose = KeyTextures::new(vec![solid(60), solid(160)], false);
        assert!(!loose.finishes, "natural textures keep their cells");
        let boards = KeyTextures::new(vec![solid(60), solid(160)], true);
        assert!(boards.finishes);
        let frame = Frame {
            ppsq: 4,
            origin: (0, 0),
        };
        let cells = CellTable::new(&boards, 7, &frame, 8, 8);
        for py in 0..32 {
            for px in 0..32 {
                for v in 0..2 {
                    let c = sample_finish(&boards, &cells, &frame, (px, py), (0.0, 0.0), Some(v));
                    let want = if v == 0 { 60.0 } else { 160.0 };
                    assert!((c[0] - want).abs() < 1e-3, "{px},{py}: {c:?}");
                }
            }
        }
    }
}
