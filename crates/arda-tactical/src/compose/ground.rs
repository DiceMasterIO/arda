//! The ground layer (goal 62).
//!
//! Each pixel's position is domain-warped by rotated value noise before the
//! per-square ground types are interpolated, so borders wander freely across
//! square boundaries instead of following the grid. The interpolated weights
//! are roughened by a per-key noise and sharpened, then height-blended with
//! the textures themselves: bright grass tufts and stones poke across a
//! border and vegetated ground creeps over bare ground, which reads as a
//! painted transition rather than a cross-fade. Bare ground next to
//! vegetation gets a faint contact shade. Textures are sampled with
//! stochastic tiling ([`super::sample`]), a low-frequency macro tint breaks
//! up map-scale repetition, and elevation shading ([`super::terrain`]) is
//! multiplied in. Floors never blend through walls.
// Pixel, grid and cell indices are bounded by the canvas size (at most
// 1024 ppsq × a few hundred squares), so these conversions cannot lose
// meaningful range.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]

use super::field::{
    byte, centred, corners_world, own_square, world_square, Field, Fields, Frame, WallGrid,
    WIDE_STEP,
};
pub use super::sample::TextureSet;
use super::sample::{sample, CellTable, KeyTextures};
use super::terrain::Terrain;
pub use super::water::paint_water;
use crate::layout::TacticalLayout;
use crate::noise::{hash_str, mix};
use crate::raster::Rgba;
use rayon::prelude::*;

/// Maximum warp in squares; typical displacements are about a third of it.
const WARP: f32 = 0.6;
/// Warp feature size in squares.
const WARP_SCALE: f32 = 1.6;
/// Height-blend depth and band (weights are in `[0, 1]`).
const HB_DEPTH: f32 = 0.42;
const HB_BAND: f32 = 0.16;

/// How much a ground key stands up over its neighbours at a border:
/// 2 for vegetation, 1 for loose ground, 0 for hard or laid surfaces.
#[must_use]
pub fn cover_class(key: &str) -> u8 {
    match key {
        "grass" | "meadow" | "pasture" | "heath" | "scrub" | "moss" | "forest_floor"
        | "leaf_litter" | "marsh" | "reed_bed" => 2,
        "dirt" | "mud" | "sand" | "gravel" | "packed_earth" | "farmland" | "scree" | "snow" => 1,
        // Laid surfaces hold their edges against loose ground.
        "cobbles" | "flagstone" | "stone_floor" | "planks" | "rug" => 1,
        _ => 0,
    }
}

/// One ground key in use.
struct Key<'a> {
    name: &'a str,
    tex: &'a KeyTextures,
    cells: CellTable,
    class: f32,
    rough: Field,
}

/// Everything the per-pixel ground function needs.
pub struct Ground<'a> {
    layout: &'a TacticalLayout,
    frame: Frame,
    keys: Vec<Key<'a>>,
    grid: Vec<usize>,
    uniform: Vec<bool>,
    walls: WallGrid,
    /// Border warp x, y, cell-border warp x, y, macro tint and hue.
    fields: Fields<6>,
    terrain: &'a Terrain<'a>,
    heights: &'a [i32],
}

/// Blend weights at a pixel: up to four `(key, weight)` pairs.
#[derive(Clone, Copy, Default)]
struct Weights {
    n: usize,
    k: [(usize, f32); 4],
}

impl Weights {
    fn one(key: usize) -> Self {
        Self {
            n: 1,
            k: [(key, 1.0), (0, 0.0), (0, 0.0), (0, 0.0)],
        }
    }

    fn add(&mut self, key: usize, w: f32) {
        for e in &mut self.k[..self.n] {
            if e.0 == key {
                e.1 += w;
                return;
            }
        }
        if self.n < 4 {
            self.k[self.n] = (key, w);
            self.n += 1;
        }
    }
}

impl<'a> Ground<'a> {
    /// Prepares the ground of a layout; `None` if a ground key has no texture.
    #[must_use]
    pub fn new(
        layout: &'a TacticalLayout,
        textures: &'a TextureSet,
        frame: Frame,
        seed: u64,
        terrain: &'a Terrain<'a>,
        heights: &'a [i32],
    ) -> Option<Self> {
        let (cw, ch) = (layout.width * frame.ppsq, layout.height * frame.ppsq);
        let mut keys: Vec<Key<'a>> = Vec::new();
        let mut grid = Vec::with_capacity(layout.squares.len());
        for sq in &layout.squares {
            let k = if let Some(k) = keys.iter().position(|k| k.name == sq.ground) {
                k
            } else {
                let tex = textures.get(&sq.ground)?;
                let salt = hash_str(seed, &sq.ground);
                keys.push(Key {
                    name: &sq.ground,
                    tex,
                    cells: CellTable::new(tex, salt, &frame, layout.width, layout.height),
                    class: f32::from(cover_class(&sq.ground)),
                    rough: Field::new(&frame, cw, ch, |u, v| {
                        centred(mix(salt), u * 2.4, v * 2.4, 2)
                    }),
                });
                keys.len() - 1
            };
            grid.push(k);
        }
        let walls = WallGrid::new(layout);
        let (w, h) = (i64::from(layout.width), i64::from(layout.height));
        let mut uniform = vec![false; grid.len()];
        for y in 0..h {
            for x in 0..w {
                let i = usize::try_from(y * w + x).unwrap_or(0);
                let at = |dx: i64, dy: i64| {
                    let (sx, sy) = ((x + dx).clamp(0, w - 1), (y + dy).clamp(0, h - 1));
                    grid[usize::try_from(sy * w + sx).unwrap_or(0)]
                };
                uniform[i] = (-1..=1).all(|dy| (-1..=1).all(|dx| at(dx, dy) == grid[i]));
            }
        }
        let c =
            |salt: u64, u: f32, v: f32, scale: f32| centred(seed ^ salt, u / scale, v / scale, 3);
        let fields = Fields::new(&frame, cw, ch, WIDE_STEP, |u, v| {
            [
                c(0x3A, u, v, WARP_SCALE),
                c(0x3B, u, v, WARP_SCALE),
                c(0xCE1, u, v, 1.2),
                c(0xCE2, u, v, 1.2),
                c(0x3AC0, u, v, 6.0),
                c(0x3AC1, u, v, 9.0),
            ]
        });
        Some(Self {
            layout,
            frame,
            keys,
            grid,
            uniform,
            walls,
            fields,
            terrain,
            heights,
        })
    }

    fn key_at(&self, x: i64, y: i64) -> usize {
        self.grid[usize::try_from(y * i64::from(self.layout.width) + x).unwrap_or(0)]
    }

    /// Raw (pre height-blend) weights at a pixel.
    #[allow(clippy::cast_precision_loss)]
    fn weights(&self, px: u32, py: u32, f: &[f32; 6]) -> Weights {
        let s = self.frame.ppsq as f32;
        let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
        let (u, v) = (fx / s, fy / s);
        let own = own_square(self.layout, u, v);
        let oi = usize::try_from(own.1 * i64::from(self.layout.width) + own.0).unwrap_or(0);
        if self.uniform[oi] {
            return Weights::one(self.grid[oi]);
        }
        let (dx, dy) = (f[0], f[1]);
        let walled = self.walls.near(own.0, own.1);
        // Warped positions are taken in world squares, so adjacent windows
        // agree bit for bit.
        let origin = self.frame.origin;
        let (uw, vw) = self.frame.world_sq(fx, fy);
        // Never warp through a wall: retry with less warp so borders near
        // walls stay noise-shaped instead of snapping to the own square.
        let (mut wx, mut wy) = (uw, vw);
        for amp in [WARP, WARP * 0.5, WARP * 0.25] {
            let (x, y) = (uw + amp * dx, vw + amp * dy);
            if !walled
                || !self
                    .walls
                    .separated(own, world_square(self.layout, origin, x, y))
            {
                (wx, wy) = (x, y);
                break;
            }
        }
        let mut acc = Weights::default();
        for (x, y, w) in corners_world(self.layout, origin, wx, wy) {
            if walled && self.walls.separated(own, (x, y)) {
                continue;
            }
            acc.add(self.key_at(x, y), w);
        }
        if acc.n <= 1 {
            return Weights::one(if acc.n == 1 {
                acc.k[0].0
            } else {
                self.grid[oi]
            });
        }
        let mut total = 0.0;
        for e in &mut acc.k[..acc.n] {
            let rough = self.keys[e.0].rough.at(fx, fy);
            let r = e.1 * (1.0 + 0.75 * rough);
            // Sharpen: w^4 keeps the transition soft but narrow.
            e.1 = r * r * r * r;
            total += e.1;
        }
        // Normalise and drop negligible keys (under 1 %): they could not
        // win the height blend and would only cost texture fetches.
        let mut kept = Weights::default();
        let mut sum = 0.0;
        for e in &acc.k[..acc.n] {
            let w = e.1 / total.max(1e-12);
            if w >= 0.01 {
                kept.k[kept.n] = (e.0, w);
                kept.n += 1;
                sum += w;
            }
        }
        if kept.n == 0 {
            return Weights::one(self.grid[oi]);
        }
        for e in &mut kept.k[..kept.n] {
            e.1 /= sum;
        }
        kept
    }

    /// Height-blends the samples of a multi-key pixel. Returns the colour
    /// and the dominant key.
    fn height_blend(&self, w: &Weights, samples: &[[f32; 3]; 4]) -> ([f32; 3], usize) {
        // The texture's own brightness plus its cover class decides who
        // wins inside the transition band.
        let mut m = f32::MIN;
        let mut hs = [0.0f32; 4];
        for (i, e) in w.k[..w.n].iter().enumerate() {
            let s = samples[i];
            let luma = (0.3 * s[0] + 0.59 * s[1] + 0.11 * s[2]) / 255.0;
            let key = &self.keys[e.0];
            hs[i] = e.1 + HB_DEPTH * (luma - key.tex.mean_luma() + 0.22 * key.class);
            m = m.max(hs[i]);
        }
        let mut ks = [0.0f32; 4];
        let mut total = 0.0;
        for i in 0..w.n {
            ks[i] = (hs[i] - m + HB_BAND).max(0.0);
            total += ks[i];
        }
        let mut c = [0.0f32; 3];
        let (mut top, mut best) = (-1.0, w.k[0].0);
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for (i, e) in w.k[..w.n].iter().enumerate() {
            ks[i] /= total.max(1e-9);
            for (ch, v) in c.iter_mut().enumerate() {
                *v += ks[i] * samples[i][ch];
            }
            if ks[i] > top {
                (top, best) = (ks[i], e.0);
            }
            let class = self.keys[e.0].class;
            lo = lo.min(class);
            hi = hi.max(class);
        }
        if hi > lo {
            let hi_w: f32 = (0..w.n)
                .filter(|&i| self.keys[w.k[i].0].class >= hi)
                .map(|i| ks[i])
                .sum();
            // A contact shade on the lower side of a raised border.
            if hi_w < 0.5 {
                let d = 1.0 - 0.2 * (hi_w * 2.0);
                c = c.map(|v| v * d);
            }
        }
        (c, best)
    }

    /// The raw weights governing pixel `(px, py)`. With an even ppsq they
    /// are computed once per 2 × 2 block, at its top-left pixel: square
    /// edges (and so walls) then fall on block edges, and the per-pixel
    /// texture detail of the height blend hides the 2-pixel quantisation.
    #[allow(clippy::cast_precision_loss)]
    fn block_weights(&self, px: u32, py: u32) -> Weights {
        let (bx, by) = if self.frame.ppsq.is_multiple_of(2) {
            (px & !1, py & !1)
        } else {
            (px, py)
        };
        let f = self.fields.at(bx as f32 + 0.5, by as f32 + 0.5);
        self.weights(bx, by, &f)
    }

    /// The blended ground colour and the dominant key at a pixel.
    #[allow(clippy::cast_precision_loss)]
    fn colour(&self, px: u32, py: u32, w: &Weights) -> ([f32; 3], usize) {
        let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
        let f = self.fields.at(fx, fy);
        let cw = (0.35 * f[2], 0.35 * f[3]);
        let w = *w;
        let mut samples = [[0.0f32; 3]; 4];
        for (i, e) in w.k[..w.n].iter().enumerate() {
            let key = &self.keys[e.0];
            samples[i] = sample(key.tex, &key.cells, &self.frame, px, py, cw);
        }
        let (c, best) = if w.n > 1 {
            self.height_blend(&w, &samples)
        } else {
            (samples[0], w.k[0].0)
        };
        let (m1, m2) = (f[4], f[5]);
        let shade = self
            .terrain
            .shade(self.heights, self.layout.width * self.frame.ppsq, px, py);
        let k = (1.0 + 0.07 * m1) * shade;
        (
            [
                c[0] * k * (1.0 + 0.025 * m2),
                c[1] * k,
                c[2] * k * (1.0 - 0.035 * m2),
            ],
            best,
        )
    }

    /// Paints the whole canvas, parallel over row pairs and deterministic.
    pub fn paint(&self, canvas: &mut Rgba) {
        let w = canvas.width as usize;
        canvas
            .data
            .par_chunks_mut(w * 8)
            .enumerate()
            .for_each(|(pair, rows)| {
                let y0 = u32::try_from(pair * 2).unwrap_or(0);
                let even = self.frame.ppsq.is_multiple_of(2);
                // One weight set per 2 × 2 block, shared by both rows.
                let blocks: Vec<Weights> = if even {
                    (0..w.div_ceil(2))
                        .map(|b| self.block_weights(u32::try_from(2 * b).unwrap_or(0), y0))
                        .collect()
                } else {
                    Vec::new()
                };
                for (i, out) in rows.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                    let px = u32::try_from(i % w).unwrap_or(0);
                    let py = y0 + u32::try_from(i / w).unwrap_or(0);
                    let wts = if even {
                        blocks[(i % w) / 2]
                    } else {
                        self.block_weights(px, py)
                    };
                    let (c, _) = self.colour(px, py, &wts);
                    out.copy_from_slice(&[byte(c[0]), byte(c[1]), byte(c[2]), 255]);
                }
            });
    }
}

/// Paints the ground layer over the whole canvas.
///
/// Returns `false` if a ground type has no texture.
#[must_use]
pub fn paint_ground(
    canvas: &mut Rgba,
    layout: &TacticalLayout,
    textures: &TextureSet,
    seed: u64,
    ppsq: u32,
) -> bool {
    let frame = Frame::of(layout, ppsq);
    let terrain = Terrain::new(layout, &frame, seed);
    let heights = terrain.heights(canvas.width, canvas.height);
    match Ground::new(layout, textures, frame, seed, &terrain, &heights) {
        Some(g) => {
            g.paint(canvas);
            true
        }
        None => false,
    }
}

/// The dominant ground type per pixel, as an index into the layout's
/// distinct ground names in order of first appearance. For tests of blend
/// geometry; `None` if a ground type has no texture.
#[must_use]
pub fn dominant_ground(
    layout: &TacticalLayout,
    textures: &TextureSet,
    seed: u64,
    ppsq: u32,
) -> Option<Vec<usize>> {
    let frame = Frame::of(layout, ppsq);
    let terrain = Terrain::new(layout, &frame, seed);
    let (w, h) = (layout.width * ppsq, layout.height * ppsq);
    let heights = terrain.heights(w, h);
    let g = Ground::new(layout, textures, frame, seed, &terrain, &heights)?;
    let mut out = Vec::with_capacity(w as usize * h as usize);
    for py in 0..h {
        for px in 0..w {
            // Keys are registered in order of first appearance.
            out.push(g.colour(px, py, &g.block_weights(px, py)).1);
        }
    }
    Some(out)
}
