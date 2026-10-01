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
mod shadows;
pub mod snow;
pub mod stamp;
pub mod terrain;
pub mod walls;
pub mod water;
mod weights;
pub mod world_tint;

use crate::catalog::{Asset, Layer};
use crate::error::TacticalError;
use crate::layout::{AssetRef, TacticalLayout};
use crate::library::Library;
use crate::noise::{hash2, hash_str};
use crate::raster::Rgba;
use lighting::{Buffers, Lighting, Pool};
use rayon::prelude::*;
use stamp::{Op, Stamp};
use std::collections::BTreeMap;
use std::sync::Arc;
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
    cache: BTreeMap<(String, u8, bool), Arc<Rgba>>,
}

impl Sprites<'_> {
    fn get(&mut self, a: &Asset, turns: u8, mirror: bool) -> Option<Arc<Rgba>> {
        let key = (a.id.clone(), turns % 4, mirror);
        if let Some(hit) = self.cache.get(&key) {
            return Some(Arc::clone(hit));
        }
        let src = self.lib.image(&a.id)?;
        let mut img = src.resized(a.footprint.w * self.ppsq, a.footprint.h * self.ppsq);
        if mirror {
            img = img.mirrored();
        }
        let img = Arc::new(img.rotated(turns));
        self.cache.insert(key, Arc::clone(&img));
        Some(img)
    }

    /// Queues `a` at top-left pixel `at`.
    fn push(&mut self, ops: &mut Vec<Op>, a: &Asset, turns: u8, mirror: bool, at: (i64, i64)) {
        if let Some(sprite) = self.get(a, turns, mirror) {
            ops.push(Op {
                sprite,
                ox: at.0,
                oy: at.1,
                stamp: stamp_of(a, self.ppsq),
            });
        }
    }
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
    render_clip(layout, lib, seed, opts, style, None, None)
}

/// Renders only the pixels `[x, y, w, h]` of a layout's image: equal to
/// cropping [`render`]'s output, but the per-pixel passes (ground, lighting
/// and grade) skip the pixels outside, so rendering a block within an
/// apron costs less (goal 50). Everything that reaches across pixels
/// (heights, sprites, shadows and blurs) is still computed over the whole
/// layout.
///
/// # Errors
/// Layout/library inconsistencies, out-of-range options or a region
/// outside the image.
pub fn render_region(
    layout: &TacticalLayout,
    lib: &Library,
    seed: u64,
    opts: &RenderOptions,
    [x, y, w, h]: [u32; 4],
) -> Result<Rgba, TacticalError> {
    let clip = [x, y, x.saturating_add(w), y.saturating_add(h)];
    render_clip(layout, lib, seed, opts, &Style::default(), Some(clip), None)
}

/// [`render_region`] with the optional world grade (goal 49): the ground
/// and water layers are pulled toward the world map's colours at their
/// world position ([`world_tint`]). The layout must carry its world
/// `origin`.
///
/// # Errors
/// As [`render_region`], or a lattice that does not cover the layout.
pub fn render_region_tinted(
    layout: &TacticalLayout,
    lib: &Library,
    seed: u64,
    opts: &RenderOptions,
    [x, y, w, h]: [u32; 4],
    tint: &world_tint::WorldTint,
) -> Result<Rgba, TacticalError> {
    let clip = [x, y, x.saturating_add(w), y.saturating_add(h)];
    render_clip(
        layout,
        lib,
        seed,
        opts,
        &Style::default(),
        Some(clip),
        Some(tint),
    )
}

/// Copies pixels `[x0, y0, x1, y1)` out of `img`.
fn cut(img: &Rgba, [x0, y0, x1, y1]: [u32; 4]) -> Rgba {
    let mut out = Rgba::new(x1 - x0, y1 - y0);
    let (src_w, row) = (img.width as usize * 4, (x1 - x0) as usize * 4);
    for (r, dst) in out.data.chunks_mut(row.max(1)).enumerate() {
        let from = (y0 as usize + r) * src_w + x0 as usize * 4;
        if let Some(src) = img.data.get(from..from + row) {
            dst.copy_from_slice(src);
        }
    }
    out
}

fn render_clip(
    layout: &TacticalLayout,
    lib: &Library,
    seed: u64,
    opts: &RenderOptions,
    style: &Style,
    clip: Option<[u32; 4]>,
    tint: Option<&world_tint::WorldTint>,
) -> Result<Rgba, TacticalError> {
    if !(8..=1024).contains(&opts.ppsq) {
        return Err(TacticalError::Options(format!(
            "ppsq {} is outside 8–1024",
            opts.ppsq
        )));
    }
    let ppsq = opts.ppsq;
    let (w, h) = canvas_size(layout, ppsq)?;
    if clip.is_some_and(|[x0, y0, x1, y1]| x0 >= x1 || y0 >= y1 || x1 > w || y1 > h) {
        return Err(TacticalError::Options(format!(
            "region {clip:?} lies outside the {w}x{h} image"
        )));
    }
    let area = clip.unwrap_or([0, 0, w, h]);
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
    let textures = lib.texture_set(ppsq);
    let Some(g) = ground::Ground::new(layout, &textures, frame, seed, &terrain, &cv.buf.height_map)
    else {
        return Err(TacticalError::Layout {
            layout: layout.name.clone(),
            message: "a ground type has no texture".into(),
        });
    };
    g.paint_clipped(&mut cv.img, area);
    cv.buf.water = ground::paint_water(&mut cv.img, layout, &textures, seed, ppsq);
    if let Some(t) = tint {
        world_tint::apply(
            &mut cv.img,
            layout,
            (&textures, &cv.buf.water),
            ppsq,
            t,
            area,
        )?;
    }

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
    let mut ops = Vec::new();
    let mut walls_done = false;
    for (layer, _, i, a) in &placed {
        if *layer > Layer::Wall && !walls_done {
            draw_walls(&mut ops, &mut sprites, lib, seed, layout);
            walls_done = true;
        }
        let p = &layout.placements[*i];
        let turns = u8::try_from(p.rotation / 90).unwrap_or(0);
        let at = anchor_origin(a, p.x, p.y, turns, p.mirror, ppsq);
        sprites.push(&mut ops, a, turns, p.mirror, at);
        if let Some(light) = a.light {
            pools.push(pool(p.x, p.y, light.radius_ft, light.colour, ppsq));
        }
    }
    if !walls_done {
        draw_walls(&mut ops, &mut sprites, lib, seed, layout);
    }
    stamp::stamp_all(&mut cv.img, &mut cv.buf, &mut cv.casts, cv.terrain, &ops);
    pools.extend(
        layout
            .lights
            .iter()
            .map(|l| pool(l.x, l.y, l.radius_ft, l.colour, ppsq)),
    );
    let mut img = cv.img;
    if opts.lighting {
        lighting::apply_clipped(
            &mut img,
            (&cv.buf, &cv.casts),
            ppsq,
            &Lighting::default(),
            &pools,
            area,
        );
        if style.grade {
            grade::warm_clipped(&mut img, area);
        }
    }
    if opts.grid {
        draw_grid(&mut img, ppsq);
    }
    Ok(match clip {
        Some(c) => cut(&img, c),
        None => img,
    })
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

/// Queues edge pieces, then joints over their ends.
fn draw_walls(
    ops: &mut Vec<Op>,
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
        sprites.push(ops, a, e.turns, false, (cx - p / 2, cy - p / 2));
    }
    for j in &joints {
        let pieces = lib.wall_pieces(j.kit, j.role);
        let Some(a) = pick(&pieces, seed, j.vertex.0, j.vertex.1) else {
            continue;
        };
        let (cx, cy) = (i64::from(j.vertex.0) * p, i64::from(j.vertex.1) * p);
        sprites.push(ops, a, j.turns, false, (cx - p / 2, cy - p / 2));
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
#[cfg(test)]
mod tests_snow;
