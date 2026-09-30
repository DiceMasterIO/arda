//! The deterministic compositor (goal 62) and its lighting pass (goal 63).
//!
//! Layers are drawn in [`Layer`] order: ground, water, floor, props, walls,
//! canopies. Then come the lighting pass, the optional warm grade and the
//! optional grid. Every pixel is a pure function of the inputs computed
//! with integer or plain IEEE `f32` arithmetic (`+ − × ÷`, `sqrt`,
//! `floor`); the per-pixel passes run in parallel over rows, which cannot
//! change a result, so the same inputs give byte-identical output.

pub mod field;
pub mod grade;
pub mod ground;
pub mod lighting;
pub mod sample;
pub mod terrain;
pub mod walls;
pub mod water;

use crate::catalog::{Asset, Layer};
use crate::error::TacticalError;
use crate::layout::{AssetRef, TacticalLayout};
use crate::library::Library;
use crate::noise::{hash2, hash_str};
use crate::raster::Rgba;
use lighting::{Buffers, Lighting, Pool};
use rayon::prelude::*;
use std::collections::BTreeMap;
use terrain::Terrain;

/// Ground key of shallow water textures.
pub const WATER_SHALLOW: &str = "water_shallow";
/// Ground key of deep water textures.
pub const WATER_DEEP: &str = "water_deep";

/// Render options.
#[derive(Debug, Clone, Copy)]
pub struct RenderOptions {
    /// Output pixels per square; art is rescaled from the library's ppsq.
    pub ppsq: u32,
    /// Draw a square grid on top.
    pub grid: bool,
    /// Run the lighting pass.
    pub lighting: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            ppsq: 128,
            grid: false,
            lighting: true,
        }
    }
}

/// Look options beyond [`RenderOptions`]; [`render`] uses the default.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    /// Apply the warm colour grade (with lighting only).
    pub grade: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self { grade: true }
    }
}

/// Every asset an [`AssetRef`] could resolve to.
#[must_use]
pub fn candidates<'a>(lib: &'a Library, r: &AssetRef) -> Vec<&'a Asset> {
    match r {
        AssetRef::Id(id) => lib.asset(id).into_iter().collect(),
        AssetRef::Query { class, tags } => lib.query(*class, tags),
    }
}

/// Resolves an [`AssetRef`]; queries pick by hashing the seed and index.
#[must_use]
pub fn resolve<'a>(lib: &'a Library, r: &AssetRef, seed: u64, index: usize) -> Option<&'a Asset> {
    let all = candidates(lib, r);
    let n = all.len() as u64;
    if n == 0 {
        return None;
    }
    let pick = hash2(seed ^ 0x9E50, i64::try_from(index).unwrap_or(0), 0) % n;
    all.get(usize::try_from(pick).ok()?).copied()
}

/// Scaled, mirrored and rotated sprites, cached by `(id, turns, mirror)`.
struct Sprites<'a> {
    lib: &'a Library,
    ppsq: u32,
    cache: BTreeMap<(String, u8, bool), Rgba>,
}

impl Sprites<'_> {
    fn get(&mut self, a: &Asset, turns: u8, mirror: bool) -> Option<&Rgba> {
        let key = (a.id.clone(), turns % 4, mirror);
        if !self.cache.contains_key(&key) {
            let src = self.lib.image(&a.id)?;
            let mut img = src.resized(a.footprint.w * self.ppsq, a.footprint.h * self.ppsq);
            if mirror {
                img = img.mirrored();
            }
            self.cache.insert(key.clone(), img.rotated(turns));
        }
        self.cache.get(&key)
    }
}

/// What a stamped sprite contributes to the lighting buffers.
#[derive(Clone, Copy)]
struct Stamp {
    /// Height above the ground in 1/256 ft, if it sweeps a shadow.
    height: Option<i32>,
    /// Whether it darkens its surroundings (ambient occlusion).
    occludes: bool,
    /// Offset of a canopy's cast silhouette in pixels, if it is a canopy.
    crown: Option<i64>,
}

/// Swept shadows are capped at this height (ft): about two-thirds of a
/// square, so tall walls and cranes do not smear across the map.
const SWEEP_CAP_FT: u16 = 12;

struct Canvas<'a> {
    img: Rgba,
    buf: Buffers,
    casts: lighting::Casts,
    terrain: &'a Terrain<'a>,
}

impl Canvas<'_> {
    fn stamp(&mut self, sprite: &Rgba, ox: i64, oy: i64, s: Stamp) {
        let (cw, ch) = (i64::from(self.img.width), i64::from(self.img.height));
        for sy in 0..sprite.height {
            let y = oy + i64::from(sy);
            if y < 0 || y >= ch {
                continue;
            }
            for sx in 0..sprite.width {
                let x = ox + i64::from(sx);
                if x < 0 || x >= cw {
                    continue;
                }
                let px = sprite.get(sx, sy);
                if px[3] == 0 {
                    continue;
                }
                let (ux, uy) = (u32::try_from(x).unwrap_or(0), u32::try_from(y).unwrap_or(0));
                self.img.blend(ux, uy, px);
                let i = usize::try_from(y * cw + x).unwrap_or(0);
                let a = u32::from(px[3]);
                self.buf.water[i] =
                    crate::raster::to_u8(u32::from(self.buf.water[i]) * (255 - a) / 255);
                if s.occludes {
                    self.buf.occluders[i] = self.buf.occluders[i].max(px[3]);
                }
                if let Some(off) = s.crown {
                    self.casts.tops[i] = self.casts.tops[i].max(px[3]);
                    let (sx2, sy2) = (x + off, y + off);
                    if sx2 < cw && sy2 < ch {
                        let j = usize::try_from(sy2 * cw + sx2).unwrap_or(0);
                        self.casts.silhouettes[j] = self.casts.silhouettes[j].max(px[3]);
                    }
                }
                if let Some(hgt) = s.height {
                    if px[3] >= 128 {
                        #[allow(clippy::cast_precision_loss)]
                        let base =
                            terrain::to_units(self.terrain.height(x as f32 + 0.5, y as f32 + 0.5));
                        self.buf.height_map[i] = self.buf.height_map[i].max(base + hgt);
                    }
                }
            }
        }
    }
}

/// Top-left pixel of a sprite whose anchor lands at `(x, y)` squares.
#[allow(clippy::cast_possible_truncation)] // rounded map positions
fn anchor_origin(a: &Asset, x: f32, y: f32, turns: u8, mirror: bool, ppsq: u32) -> (i64, i64) {
    let anchor = a.anchor_or_centre();
    let (mut ax, mut ay) = (anchor.x, anchor.y);
    let (mut w, mut h) = (a.footprint.w as f32, a.footprint.h as f32);
    if mirror {
        ax = w - ax;
    }
    for _ in 0..turns % 4 {
        // A clockwise turn maps (x, y) in a w × h image to (h - y, x).
        (ax, ay) = (h - ay, ax);
        (w, h) = (h, w);
    }
    let s = ppsq as f32;
    (((x - ax) * s).round() as i64, ((y - ay) * s).round() as i64)
}

fn stamp_of(a: &Asset, ppsq: u32) -> Stamp {
    let canopy = a.layer == Layer::Canopy;
    let height =
        (a.casts_shadow && !canopy).then(|| i32::from(a.height_ft.min(SWEEP_CAP_FT)) * 256);
    // A crown's shadow is offset by about 0.45 of its radius along the
    // sun's diagonal (0.32 of the radius on each axis).
    let radius = i64::from(a.footprint.w.min(a.footprint.h) * ppsq) / 2;
    Stamp {
        height,
        occludes: a.casts_shadow && matches!(a.layer, Layer::Prop | Layer::Wall),
        crown: (a.casts_shadow && canopy).then_some(radius * 32 / 100),
    }
}

/// Renders a layout to an RGBA image.
///
/// # Errors
/// Layout/library inconsistencies and out-of-range options.
pub fn render(
    layout: &TacticalLayout,
    lib: &Library,
    seed: u64,
    opts: &RenderOptions,
) -> Result<Rgba, TacticalError> {
    render_with(layout, lib, seed, opts, &Style::default())
}

/// Renders a layout with explicit [`Style`] options.
///
/// # Errors
/// Layout/library inconsistencies and out-of-range options.
pub fn render_with(
    layout: &TacticalLayout,
    lib: &Library,
    seed: u64,
    opts: &RenderOptions,
    style: &Style,
) -> Result<Rgba, TacticalError> {
    if !(8..=1024).contains(&opts.ppsq) {
        return Err(TacticalError::Options(format!(
            "ppsq {} is outside 8–1024",
            opts.ppsq
        )));
    }
    let ppsq = opts.ppsq;
    let (w, h) = canvas_size(layout, ppsq)?;
    layout.check(lib)?;
    let seed = seed ^ hash_str(0, &lib.catalog.library_version);
    let frame = field::Frame::of(layout, ppsq);
    let terrain = Terrain::new(layout, &frame, seed);
    let mut cv = Canvas {
        img: Rgba::new(w, h),
        buf: Buffers::new(w, h),
        casts: lighting::Casts::new(w, h),
        terrain: &terrain,
    };
    cv.buf.height_map = terrain.heights(w, h);
    let textures = ground::TextureSet::new(lib, ppsq);
    let Some(g) = ground::Ground::new(layout, &textures, frame, seed, &terrain, &cv.buf.height_map)
    else {
        return Err(TacticalError::Layout {
            layout: layout.name.clone(),
            message: "a ground type has no texture".into(),
        });
    };
    g.paint(&mut cv.img);
    cv.buf.water = ground::paint_water(&mut cv.img, layout, &textures, seed, ppsq);

    let mut sprites = Sprites {
        lib,
        ppsq,
        cache: BTreeMap::new(),
    };
    let mut placed: Vec<(Layer, i16, usize, &Asset)> = Vec::new();
    for (i, p) in layout.placements.iter().enumerate() {
        let a = resolve(lib, &p.asset, seed, i).ok_or_else(|| TacticalError::Layout {
            layout: layout.name.clone(),
            message: format!("placement {i} matches no asset"),
        })?;
        placed.push((a.layer, a.z, i, a));
    }
    placed.sort_by_key(|(layer, z, i, _)| (*layer, *z, *i));
    let mut pools = Vec::new();
    let mut walls_done = false;
    for (layer, _, i, a) in &placed {
        if *layer > Layer::Wall && !walls_done {
            draw_walls(&mut cv, &mut sprites, lib, seed, layout);
            walls_done = true;
        }
        let p = &layout.placements[*i];
        let turns = u8::try_from(p.rotation / 90).unwrap_or(0);
        let (ox, oy) = anchor_origin(a, p.x, p.y, turns, p.mirror, ppsq);
        let st = stamp_of(a, ppsq);
        if let Some(sprite) = sprites.get(a, turns, p.mirror) {
            cv.stamp(sprite, ox, oy, st);
        }
        if let Some(light) = a.light {
            pools.push(pool(p.x, p.y, light.radius_ft, light.colour, ppsq));
        }
    }
    if !walls_done {
        draw_walls(&mut cv, &mut sprites, lib, seed, layout);
    }
    pools.extend(
        layout
            .lights
            .iter()
            .map(|l| pool(l.x, l.y, l.radius_ft, l.colour, ppsq)),
    );
    let mut img = cv.img;
    if opts.lighting {
        lighting::apply_with(
            &mut img,
            &cv.buf,
            &cv.casts,
            ppsq,
            &Lighting::default(),
            &pools,
        );
        if style.grade {
            grade::warm(&mut img);
        }
    }
    if opts.grid {
        draw_grid(&mut img, ppsq);
    }
    Ok(img)
}

/// Goal-prompt §8 memory ceiling for one render.
pub const MEMORY_CEILING_BYTES: u64 = 16 << 30;

/// Peak bytes held per canvas pixel: the RGBA image, the height, occluder,
/// water and shadow-cast buffers, the water mask and the lighting pass's
/// scratch planes.
pub const PEAK_BYTES_PER_PIXEL: u64 = 24;

/// Canvas size in pixels, refused before anything is allocated when it
/// overflows `u32` or its peak memory exceeds [`MEMORY_CEILING_BYTES`].
///
/// # Errors
/// [`TacticalError::ResourceLimit`].
pub fn canvas_size(layout: &TacticalLayout, ppsq: u32) -> Result<(u32, u32), TacticalError> {
    let too_big = || {
        TacticalError::ResourceLimit(format!(
            "{}x{} squares at {ppsq} px/square exceed the {MEMORY_CEILING_BYTES}-byte render ceiling",
            layout.width, layout.height
        ))
    };
    let w = layout.width.checked_mul(ppsq).ok_or_else(too_big)?;
    let h = layout.height.checked_mul(ppsq).ok_or_else(too_big)?;
    let peak = u64::from(w) * u64::from(h) * PEAK_BYTES_PER_PIXEL;
    if peak > MEMORY_CEILING_BYTES {
        return Err(too_big());
    }
    Ok((w, h))
}

#[allow(clippy::cast_possible_truncation)] // rounded map positions
fn pool(x: f32, y: f32, radius_ft: u16, colour: [u8; 3], ppsq: u32) -> Pool {
    let s = ppsq as f32;
    Pool {
        x: (x * s).round() as i64,
        y: (y * s).round() as i64,
        r: i64::from(radius_ft) * i64::from(ppsq) / 5,
        colour,
    }
}

fn pick<'a>(pieces: &[&'a Asset], seed: u64, a: u32, b: u32) -> Option<&'a Asset> {
    let n = pieces.len() as u64;
    let i = usize::try_from(hash2(seed ^ 0xA11, i64::from(a), i64::from(b)) % n.max(1)).ok()?;
    pieces.get(i).copied()
}

/// Draws edge pieces, then joints over their ends.
fn draw_walls(
    cv: &mut Canvas<'_>,
    sprites: &mut Sprites<'_>,
    lib: &Library,
    seed: u64,
    layout: &TacticalLayout,
) {
    let (edges, joints) = walls::assemble(layout);
    let p = i64::from(sprites.ppsq);
    for e in &edges {
        let pieces = lib.wall_pieces(&e.segment.kit, e.segment.kind);
        let Some(a) = pick(&pieces, seed, e.centre2.0, e.centre2.1) else {
            continue;
        };
        let (cx, cy) = (
            i64::from(e.centre2.0) * p / 2,
            i64::from(e.centre2.1) * p / 2,
        );
        let st = stamp_of(a, sprites.ppsq);
        if let Some(sprite) = sprites.get(a, e.turns, false) {
            cv.stamp(sprite, cx - p / 2, cy - p / 2, st);
        }
    }
    for j in &joints {
        let pieces = lib.wall_pieces(j.kit, j.role);
        let Some(a) = pick(&pieces, seed, j.vertex.0, j.vertex.1) else {
            continue;
        };
        let (cx, cy) = (i64::from(j.vertex.0) * p, i64::from(j.vertex.1) * p);
        let st = stamp_of(a, sprites.ppsq);
        if let Some(sprite) = sprites.get(a, j.turns, false) {
            cv.stamp(sprite, cx - p / 2, cy - p / 2, st);
        }
    }
}

/// A thin translucent square grid.
fn draw_grid(img: &mut Rgba, ppsq: u32) {
    let t = (ppsq / 96).max(1);
    let w = img.width as usize;
    img.data
        .par_chunks_mut(w * 4)
        .enumerate()
        .for_each(|(y, row)| {
            let on_row = u32::try_from(y).unwrap_or(0) % ppsq < t;
            for (x, p) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                if on_row || u32::try_from(x).unwrap_or(0) % ppsq < t {
                    // Source-over of [20, 18, 14] at alpha 70 on an opaque pixel.
                    for (c, g) in p.iter_mut().zip([20u32, 18, 14]) {
                        *c = crate::raster::to_u8((u32::from(*c) * 185 + g * 70 + 127) / 255);
                    }
                }
            }
        });
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_look;
