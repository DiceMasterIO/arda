//! Tileable placeholder ground and water textures.
//!
//! Each texture is painted on a wrapping [`Tile`]: a multi-scale base from
//! periodic rotated noise, then layers of wrapped brush strokes, dabs and
//! pebbles. Every noise term is periodic over the tile and every stroke
//! wraps, so the textures pass the seam rule. Structured surfaces (cobbles,
//! slabs, boards, rugs, furrows) take their layout from `structure`, which
//! all variants of a kind share, so variants cross-fade without ghosting.
// Art generation casts bounded pixel coordinates and cell indices.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

mod arid;
mod built;
mod earthy;
mod grassy;
mod kit;
mod rocky;
mod trail;
mod water;

use super::brush::Tile;
use super::material::Rng;
use crate::noise::{fbm, hash2, mix as mix64, unit};
use crate::raster::Rgba;

/// Ground types, their variant counts and whether they are water.
pub const TYPES: [(&str, u32, bool); 33] = [
    ("grass", 3, false),
    ("dirt", 3, false),
    ("cobbles", 3, false),
    ("mud", 2, false),
    ("sand", 2, false),
    ("gravel", 2, false),
    ("stone_floor", 2, false),
    ("planks", 2, false),
    ("water_shallow", 2, true),
    ("water_deep", 2, true),
    ("meadow", 3, false),
    ("forest_floor", 3, false),
    ("leaf_litter", 2, false),
    ("heath", 2, false),
    ("scrub", 2, false),
    ("moss", 2, false),
    ("scree", 3, false),
    ("rock", 2, false),
    ("cliff", 2, false),
    ("snow", 2, false),
    ("ice", 2, false),
    ("marsh", 2, false),
    ("reed_bed", 2, false),
    ("farmland", 2, false),
    ("pasture", 3, false),
    ("packed_earth", 2, false),
    ("flagstone", 2, false),
    ("rug", 2, false),
    ("trail", 2, false),
    ("salt_crust", 2, false),
    ("mudflat", 2, false),
    ("cave_floor", 2, false),
    ("bedrock", 2, false),
];

/// Ground kinds whose variants share a layout that must stay aligned: the
/// compositor never offsets these, only cross-fades their variants. The
/// catalogue marks them with the free tag `structured`.
pub const STRUCTURED: [&str; 7] = [
    "cobbles",
    "stone_floor",
    "planks",
    "flagstone",
    "rug",
    "farmland",
    "cliff",
];

/// Texture side in squares.
pub const SQUARES: u32 = 2;

/// Texture side in squares for one kind: structured surfaces the
/// compositor cannot offset get a larger tile so their layout repeats
/// less often.
#[must_use]
pub fn squares(kind: &str) -> u32 {
    match kind {
        "cobbles" | "flagstone" => 4,
        _ => SQUARES,
    }
}

/// Painting context for one texture.
pub struct Tex {
    /// Variant seed.
    pub seed: u64,
    /// Layout seed shared by every variant of a kind.
    pub structure: u64,
    /// Side in pixels.
    pub size: f32,
    /// Pixel scale relative to the 128 px-per-square reference.
    pub k: f32,
    /// Area relative to the 256-pixel reference tile (scales counts).
    pub area: f32,
    /// Side in squares.
    pub span: u32,
}

impl Tex {
    /// Periodic noise with `cells` features across the texture.
    #[must_use]
    pub fn noise(&self, salt: u64, x: f32, y: f32, cells: i64, oct: u32) -> f32 {
        let k = cells as f32 / self.size;
        fbm(mix64(self.seed ^ salt), x * k, y * k, oct, Some(cells))
    }

    /// Periodic noise from the shared layout seed.
    #[must_use]
    pub fn layout_noise(&self, salt: u64, x: f32, y: f32, cells: i64, oct: u32) -> f32 {
        let k = cells as f32 / self.size;
        fbm(mix64(self.structure ^ salt), x * k, y * k, oct, Some(cells))
    }

    /// A per-pixel hash in `[0, 1)`.
    #[must_use]
    pub fn speckle(&self, salt: u64, x: f32, y: f32) -> f32 {
        unit(hash2(self.seed ^ salt, x as i64, y as i64))
    }

    /// A variant RNG.
    #[must_use]
    pub fn rng(&self, salt: u64) -> Rng {
        Rng::new(self.seed ^ salt)
    }

    /// A layout RNG, identical for every variant.
    #[must_use]
    pub fn layout_rng(&self, salt: u64) -> Rng {
        Rng::new(self.structure ^ salt)
    }

    /// A count scaled by the tile area.
    #[must_use]
    pub fn n(&self, count: u32) -> u32 {
        ((count as f32) * self.area).max(1.0) as u32
    }

    /// Periodic jittered-grid Voronoi on the layout seed: `(F1, F2, cell
    /// hash, nearest point)` in pixels, `n` cells across.
    #[must_use]
    pub fn voronoi(&self, x: f32, y: f32, n: i64, jitter: f32) -> (f32, f32, u64, (f32, f32)) {
        let cell = self.size / n as f32;
        let (cx, cy) = ((x / cell).floor() as i64, (y / cell).floor() as i64);
        let (mut f1, mut f2, mut id, mut at) = (f32::MAX, f32::MAX, 0, (0.0, 0.0));
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (i, j) = (cx + dx, cy + dy);
                let h = hash2(self.structure ^ 0xC0B, i.rem_euclid(n), j.rem_euclid(n));
                let o = (1.0 - jitter) / 2.0;
                let px = (i as f32 + o + jitter * unit(h)) * cell;
                let py = (j as f32 + o + jitter * unit(mix64(h))) * cell;
                let d = ((x - px) * (x - px) + (y - py) * (y - py)).sqrt();
                if d < f1 {
                    (f2, f1, id, at) = (f1, d, h, (px, py));
                } else if d < f2 {
                    f2 = d;
                }
            }
        }
        (f1, f2, id, at)
    }
}

/// Paints one texture variant, [`squares`]`(kind) × ppsq` pixels square.
#[must_use]
pub fn texture(kind: &str, seed: u64, structure: u64, ppsq: u32) -> Rgba {
    let span = squares(kind);
    let side = span * ppsq;
    let t = Tex {
        seed,
        structure,
        size: side as f32,
        k: ppsq as f32 / 128.0,
        area: (side as f32 / 256.0) * (side as f32 / 256.0),
        span,
    };
    let tile: Tile = match kind {
        "grass" => grassy::grass(&t),
        "meadow" => grassy::meadow(&t),
        "pasture" => grassy::pasture(&t),
        "scrub" => grassy::scrub(&t),
        "heath" => grassy::heath(&t),
        "moss" => grassy::moss(&t),
        "marsh" => grassy::marsh(&t),
        "reed_bed" => grassy::reed_bed(&t),
        "dirt" => earthy::dirt(&t),
        "mud" => earthy::mud(&t),
        "sand" => earthy::sand(&t),
        "gravel" => earthy::gravel(&t),
        "packed_earth" => earthy::packed_earth(&t),
        "farmland" => earthy::farmland(&t),
        "forest_floor" => earthy::forest_floor(&t),
        "leaf_litter" => earthy::leaf_litter(&t),
        "snow" => earthy::snow(&t),
        "ice" => earthy::ice(&t),
        "scree" => rocky::scree(&t),
        "rock" => rocky::rock(&t),
        "cliff" => rocky::cliff(&t),
        "cave_floor" => rocky::cave_floor(&t),
        "bedrock" => rocky::bedrock(&t),
        "cobbles" => built::cobbles(&t),
        "stone_floor" => built::stone_floor(&t),
        "flagstone" => built::flagstone(&t),
        "planks" => built::planks(&t),
        "rug" => built::rug(&t),
        "trail" => trail::trail(&t),
        "salt_crust" => arid::salt_crust(&t),
        "mudflat" => arid::mudflat(&t),
        "water_shallow" => water::shallow(&t),
        _ => water::deep(&t),
    };
    tile.into_rgba()
}

#[cfg(test)]
mod tests;
