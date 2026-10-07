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
use super::sample::{sample_finish, CellTable, KeyTextures};
use super::snow::{Snow, SNOW};
use super::terrain::Terrain;
pub use super::water::paint_water;
use super::weights::Weights;
use crate::layout::{EdgeAxis, TacticalLayout};
use crate::noise::{hash2, hash_str, mix};
use crate::raster::Rgba;
use rayon::prelude::*;

/// Maximum warp in squares; typical displacements are about a third of it.
const WARP: f32 = 0.6;
/// Warp feature size in squares.
const WARP_SCALE: f32 = 1.6;
/// Largest side, squares, of a walled area that takes its own finish (a
/// room, a fenced yard); larger or open areas share the key's finish.
const ROOM_SIDE: i64 = 16;
/// Salts of the per-room and per-key finish choices.
const ROOM_SALT: u64 = 0xF1_7105;
const KEY_SALT: u64 = 0xF1_7106;

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
        "dirt" | "mud" | "sand" | "gravel" | "packed_earth" | "farmland" | "scree" | "snow"
        | "mudflat" | "cave_floor" => 1,
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
    salt: u64,
}

/// Everything the per-pixel ground function needs.
pub struct Ground<'a> {
    layout: &'a TacticalLayout,
    frame: Frame,
    keys: Vec<Key<'a>>,
    grid: Vec<usize>,
    uniform: Vec<bool>,
    /// Per square: the world square naming its walled room, when it lies
    /// in one ([`rooms`]).
    rooms: Vec<Option<(i64, i64)>>,
    walls: WallGrid,
    /// Border warp x, y, cell-border warp x, y, macro tint and hue.
    fields: Fields<6>,
    terrain: &'a Terrain<'a>,
    heights: &'a [i32],
    /// Soft snow cover, when the layout has snow.
    snow: Option<Snow>,
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
                    salt,
                });
                keys.len() - 1
            };
            grid.push(k);
        }
        let walls = WallGrid::new(layout);
        let rooms = if keys.iter().any(|k| k.tex.finishes) {
            rooms(layout, &walls, frame.origin)
        } else {
            vec![None; grid.len()]
        };
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
        let snow_key = keys.iter().position(|k| k.name == SNOW);
        let snow = Snow::new(layout, &grid, snow_key, &frame, seed);
        Some(Self {
            layout,
            frame,
            keys,
            grid,
            uniform,
            rooms,
            walls,
            fields,
            terrain,
            heights,
            snow,
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

    /// The finish a key of distinct finishes shows at a pixel: its room's
    /// when the pixel's square lies in a walled room, else the key's own,
    /// so a floor or field never patches two finishes together.
    #[allow(clippy::cast_precision_loss)]
    fn finish(&self, key: &Key<'_>, px: u32, py: u32) -> Option<usize> {
        if !key.tex.finishes {
            return None;
        }
        let n = key.tex.variants.len().max(1) as u64;
        let s = self.frame.ppsq as f32;
        let own = own_square(self.layout, (px as f32 + 0.5) / s, (py as f32 + 0.5) / s);
        let oi = usize::try_from(own.1 * i64::from(self.layout.width) + own.0).unwrap_or(0);
        let h = match self.rooms.get(oi).copied().flatten() {
            Some((rx, ry)) => hash2(key.salt ^ ROOM_SALT, rx, ry),
            None => mix(key.salt ^ KEY_SALT),
        };
        usize::try_from(h % n).ok()
    }

    /// Samples and height-blends the keys of `w` at a pixel (unshaded).
    fn blend(&self, px: u32, py: u32, w: &Weights, cw: (f32, f32)) -> ([f32; 3], usize) {
        let mut samples = [[0.0f32; 3]; 4];
        for (i, e) in w.k[..w.n].iter().enumerate() {
            let key = &self.keys[e.0];
            let finish = self.finish(key, px, py);
            samples[i] = sample_finish(key.tex, &key.cells, &self.frame, (px, py), cw, finish);
        }
        if w.n > 1 {
            self.height_blend(w, &samples)
        } else {
            (samples[0], w.k[0].0)
        }
    }

    /// The soft snow cover governing a pixel, if its square is near a snow
    /// border away from walls: the cover and the ground under it.
    #[allow(clippy::cast_precision_loss)]
    fn soft_snow(&self, px: u32, py: u32) -> Option<(&Snow, usize)> {
        let snow = self.snow.as_ref()?;
        let s = self.frame.ppsq as f32;
        let own = own_square(self.layout, (px as f32 + 0.5) / s, (py as f32 + 0.5) / s);
        let oi = usize::try_from(own.1 * i64::from(self.layout.width) + own.0).unwrap_or(0);
        let under = snow.soft_at(oi)?;
        (!self.walls.near(own.0, own.1)).then_some((snow, under))
    }

    /// Snow laid over the ground beneath it (`super::snow`): the pixel's
    /// other keys, or the commonest other key nearby, blended as usual,
    /// then the snow texture at the cover's opacity.
    fn snow_blend(
        &self,
        (px, py): (u32, u32),
        w: &Weights,
        cw: (f32, f32),
        (snow, under): (&Snow, usize),
    ) -> ([f32; 3], usize) {
        let mut rest = Weights::default();
        let mut sum = 0.0;
        for e in &w.k[..w.n] {
            if e.0 != snow.key {
                rest.add(e.0, e.1);
                sum += e.1;
            }
        }
        if rest.n == 0 {
            rest = Weights::one(under);
        } else {
            for e in &mut rest.k[..rest.n] {
                e.1 /= sum.max(1e-9);
            }
        }
        let (ground, best) = self.blend(px, py, &rest, cw);
        let a = snow.alpha(self.layout, &self.grid, &self.frame, px, py);
        if a <= 0.0 {
            return (ground, best);
        }
        let key = &self.keys[snow.key];
        let white = Snow::tint(
            sample_finish(key.tex, &key.cells, &self.frame, (px, py), cw, None),
            a,
        );
        let c = std::array::from_fn(|k| ground[k] + (white[k] - ground[k]) * a);
        (c, if a >= 0.5 { snow.key } else { best })
    }

    /// The blended ground colour and the dominant key at a pixel.
    #[allow(clippy::cast_precision_loss)]
    fn colour(&self, px: u32, py: u32, w: &Weights) -> ([f32; 3], usize) {
        let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
        let f = self.fields.at(fx, fy);
        let cw = (0.35 * f[2], 0.35 * f[3]);
        let (c, best) = match self.soft_snow(px, py) {
            Some(soft) => self.snow_blend((px, py), w, cw, soft),
            None => self.blend(px, py, w, cw),
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
        self.paint_clipped(canvas, [0, 0, canvas.width, canvas.height]);
    }

    /// Paints pixels `[x0, y0, x1, y1)` only; every pixel is a function of
    /// its own position, so they equal those of [`Self::paint`].
    pub fn paint_clipped(&self, canvas: &mut Rgba, [x0, y0, x1, y1]: [u32; 4]) {
        let w = canvas.width as usize;
        let (cx0, cx1) = (x0 as usize, (x1 as usize).min(w));
        canvas
            .data
            .par_chunks_mut(w * 8)
            .enumerate()
            .for_each(|(pair, rows)| {
                let ry = u32::try_from(pair * 2).unwrap_or(0);
                if ry + 1 < y0 || ry >= y1 {
                    return;
                }
                let even = self.frame.ppsq.is_multiple_of(2);
                // One weight set per 2 × 2 block, shared by both rows.
                let b0 = cx0 / 2;
                let blocks: Vec<Weights> = if even {
                    (b0..cx1.div_ceil(2))
                        .map(|b| self.block_weights(u32::try_from(2 * b).unwrap_or(0), ry))
                        .collect()
                } else {
                    Vec::new()
                };
                for (i, out) in rows.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                    let px = u32::try_from(i % w).unwrap_or(0);
                    let py = ry + u32::try_from(i / w).unwrap_or(0);
                    if (i % w) < cx0 || (i % w) >= cx1 || py < y0 || py >= y1 {
                        continue;
                    }
                    let wts = if even {
                        blocks[(i % w) / 2 - b0]
                    } else {
                        self.block_weights(px, py)
                    };
                    let (c, _) = self.colour(px, py, &wts);
                    out.copy_from_slice(&[byte(c[0]), byte(c[1]), byte(c[2]), 255]);
                }
            });
    }
}

/// The walled rooms of a layout: each square of a wall-bounded area that
/// stays clear of the layout's border and spans at most [`ROOM_SIDE`]
/// squares a side gets the world square of the area's first square (row
/// by row), so every window holding the whole room names it alike.
fn rooms(layout: &TacticalLayout, walls: &WallGrid, origin: (i64, i64)) -> Vec<Option<(i64, i64)>> {
    let (w, h) = (i64::from(layout.width), i64::from(layout.height));
    let n = usize::try_from(w * h).unwrap_or(0);
    let idx = |x: i64, y: i64| usize::try_from(y * w + x).unwrap_or(0);
    let mut out = vec![None; n];
    let mut seen = vec![false; n];
    let mut members = Vec::new();
    for y0 in 0..h {
        for x0 in 0..w {
            if seen[idx(x0, y0)] {
                continue;
            }
            seen[idx(x0, y0)] = true;
            members.clear();
            let mut stack = vec![(x0, y0)];
            let mut closed = true;
            let (mut lx, mut ly, mut hx, mut hy) = (x0, y0, x0, y0);
            while let Some((x, y)) = stack.pop() {
                members.push((x, y));
                (lx, ly, hx, hy) = (lx.min(x), ly.min(y), hx.max(x), hy.max(y));
                // (neighbour, the edge between them).
                let steps = [
                    (x + 1, y, EdgeAxis::Vertical, x + 1, y),
                    (x - 1, y, EdgeAxis::Vertical, x, y),
                    (x, y + 1, EdgeAxis::Horizontal, x, y + 1),
                    (x, y - 1, EdgeAxis::Horizontal, x, y),
                ];
                for (nx, ny, axis, ex, ey) in steps {
                    if walls.wall(axis, ex, ey) {
                        continue;
                    }
                    if !(0..w).contains(&nx) || !(0..h).contains(&ny) {
                        closed = false;
                        continue;
                    }
                    if !seen[idx(nx, ny)] {
                        seen[idx(nx, ny)] = true;
                        stack.push((nx, ny));
                    }
                }
            }
            if closed && hx - lx < ROOM_SIDE && hy - ly < ROOM_SIDE {
                let name = (origin.0 + x0, origin.1 + y0);
                for &(x, y) in &members {
                    out[idx(x, y)] = Some(name);
                }
            }
        }
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::WallRole;
    use crate::layout::WallSegment;

    fn wall(x: u32, y: u32, axis: EdgeAxis) -> WallSegment {
        WallSegment {
            x,
            y,
            axis,
            kind: WallRole::Run,
            kit: "timber".into(),
        }
    }

    #[test]
    fn a_walled_room_is_named_by_its_first_world_square() {
        // A 2 x 2 room at (2, 2)..(4, 4) of a 6 x 6 layout, one door edge
        // walled too (doors are wall segments).
        let mut l = TacticalLayout::new("t", 6, 6, "planks");
        for k in 2..4 {
            l.walls.push(wall(k, 2, EdgeAxis::Horizontal));
            l.walls.push(wall(k, 4, EdgeAxis::Horizontal));
            l.walls.push(wall(2, k, EdgeAxis::Vertical));
            l.walls.push(wall(4, k, EdgeAxis::Vertical));
        }
        let walls = WallGrid::new(&l);
        let r = rooms(&l, &walls, (100, 200));
        for y in 0..6 {
            for x in 0..6 {
                let inside = (2..4).contains(&x) && (2..4).contains(&y);
                let want = inside.then_some((102, 202));
                assert_eq!(r[y * 6 + x], want, "square {x},{y}");
            }
        }
        // Opened on one side the area runs to the border: no room.
        l.walls
            .retain(|w| (w.x, w.y, w.axis) != (4, 3, EdgeAxis::Vertical));
        let r = rooms(&l, &WallGrid::new(&l), (100, 200));
        assert!(r.iter().all(Option::is_none));
    }
}
