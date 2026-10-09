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
pub mod variants;
pub mod walls;
pub mod water;
mod weights;
pub mod world_tint;

use crate::catalog::{Asset, Layer, WallRole};
use crate::error::TacticalError;
use crate::layout::TacticalLayout;
use crate::library::Library;
use crate::noise::{hash2, hash_str};
use crate::raster::Rgba;
use lighting::{Buffers, Lighting, Pool};
use rayon::prelude::*;
use stamp::{Op, Stamp};
use std::collections::BTreeMap;
use std::sync::Arc;
use terrain::Terrain;
pub use variants::{
    candidates, resolve, resolve_all, Resolved, FIXED_POSE, ROT_FREE, SCALE_MAX_PCT, SCALE_MIN_PCT,
    SCALE_STEP_PCT,
};

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

/// How a sprite is drawn: quarter turns, mirror, scale in percent of the
/// footprint and diagonal transpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Pose {
    turns: u8,
    mirror: bool,
    scale_pct: u8,
    transpose: bool,
    thin: Thin,
}

/// How a wall piece is thinned for an interior partition that has no
/// dedicated `<kit>_partition` art.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Thin {
    /// Drawn as painted.
    No,
    /// An edge piece: the band squeezed towards its centre line.
    Band,
    /// A joint: shrunk about its vertex.
    Joint,
}

/// Thickness of a squeezed partition piece, percent of the kit's band.
pub const PARTITION_PCT: u32 = 50;

impl Pose {
    /// Turned and mirrored only.
    fn turned(turns: u8, mirror: bool) -> Self {
        Self {
            turns: turns % 4,
            mirror,
            scale_pct: 100,
            transpose: false,
            thin: Thin::No,
        }
    }
}

/// Scaled, transposed, mirrored and rotated sprites, cached by id and pose.
struct Sprites<'a> {
    lib: &'a Library,
    ppsq: u32,
    cache: BTreeMap<(String, Pose), Arc<Rgba>>,
}

impl Sprites<'_> {
    fn get(&mut self, a: &Asset, pose: Pose) -> Option<Arc<Rgba>> {
        let key = (a.id.clone(), pose);
        if let Some(hit) = self.cache.get(&key) {
            return Some(Arc::clone(hit));
        }
        let src = self.lib.image(&a.id)?;
        let px = |squares: u32| (squares * self.ppsq * u32::from(pose.scale_pct) + 50) / 100;
        let mut img = src.resized(px(a.footprint.w).max(1), px(a.footprint.h).max(1));
        let thin = |v: u32| (v * PARTITION_PCT).div_ceil(100);
        match pose.thin {
            Thin::No => {}
            Thin::Band => img = img.inset(img.width, thin(img.height)),
            Thin::Joint => img = img.inset(thin(img.width), thin(img.height)),
        }
        if pose.transpose {
            img = img.transposed();
        }
        if pose.mirror {
            img = img.mirrored();
        }
        let img = Arc::new(img.rotated(pose.turns));
        self.cache.insert(key, Arc::clone(&img));
        Some(img)
    }

    /// Queues `a` at top-left pixel `at`.
    fn push(&mut self, ops: &mut Vec<Op>, a: &Asset, pose: Pose, at: (i64, i64)) {
        if let Some(sprite) = self.get(a, pose) {
            ops.push(Op {
                stamp: stamp_of(a, self.ppsq, pose.scale_pct),
                sprite,
                ox: at.0,
                oy: at.1,
            });
        }
    }

    /// Queues placement sprite `a` drawn at `pose` with its anchor at
    /// `(x, y)` squares. A scaled sprite is posed only on a square
    /// footprint anchored at its centre, so it is centred on `(x, y)`.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn place(&mut self, ops: &mut Vec<Op>, a: &Asset, pose: Pose, (x, y): (f32, f32)) {
        if pose.scale_pct == 100 {
            let at = anchor_origin(a, x, y, pose.turns, pose.mirror, self.ppsq);
            self.push(ops, a, pose, at);
            return;
        }
        let Some(sprite) = self.get(a, pose) else {
            return;
        };
        let s = self.ppsq as f32;
        let at = (
            (x * s - sprite.width as f32 / 2.0).round() as i64,
            (y * s - sprite.height as f32 / 2.0).round() as i64,
        );
        self.push(ops, a, pose, at);
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

/// What `a` drawn at `scale_pct` percent adds to the lighting buffers: its
/// swept-shadow height and crown radius follow the scale.
fn stamp_of(a: &Asset, ppsq: u32, scale_pct: u8) -> Stamp {
    let canopy = a.layer == Layer::Canopy;
    let pct = i32::from(scale_pct);
    let height = (a.casts_shadow && !canopy)
        .then(|| i32::from(a.height_ft.min(SWEEP_CAP_FT)) * 256 * pct / 100);
    // A crown's shadow is offset by about 0.45 of its radius along the
    // sun's diagonal (0.32 of the radius on each axis).
    let radius = i64::from(a.footprint.w.min(a.footprint.h) * ppsq) * i64::from(pct) / 200;
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
    let mut placed: Vec<(Layer, i16, usize, Resolved<'_>)> = Vec::new();
    for (i, r) in resolve_all(lib, layout, seed).into_iter().enumerate() {
        let r = r.ok_or_else(|| TacticalError::Layout {
            layout: layout.name.clone(),
            message: format!("placement {i} matches no asset"),
        })?;
        placed.push((r.asset.layer, r.asset.z, i, r));
    }
    placed.sort_by_key(|(layer, z, i, _)| (*layer, *z, *i));
    let mut pools = Vec::new();
    let mut ops = Vec::new();
    let mut walls_done = false;
    for (layer, _, i, r) in &placed {
        if *layer > Layer::Wall && !walls_done {
            draw_walls(&mut ops, &mut sprites, lib, seed, layout);
            walls_done = true;
        }
        let (p, a) = (&layout.placements[*i], r.asset);
        let pose = Pose {
            scale_pct: r.scale_pct,
            transpose: r.transpose,
            ..Pose::turned(r.turns, r.mirror)
        };
        sprites.place(&mut ops, a, pose, (p.x, p.y));
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

/// One of a kit role's pieces (its `.alt<N>` takes included), by a hash
/// of world coordinates `(a, b)` so neighbouring windows agree.
fn pick<'a>(pieces: &[&'a Asset], seed: u64, a: i64, b: i64) -> Option<&'a Asset> {
    let n = pieces.len() as u64;
    let i = usize::try_from(hash2(seed ^ 0xA11, a, b) % n.max(1)).ok()?;
    pieces.get(i).copied()
}

/// The pieces of `kit` with `role`: for a partition, the dedicated
/// `<kit>_partition` pieces when the library has them, else the kit's own
/// pieces thinned by `fallback`.
fn kit_pieces<'a>(
    lib: &'a Library,
    kit: &str,
    role: WallRole,
    partition: bool,
    fallback: Thin,
) -> (Vec<&'a Asset>, Thin) {
    if partition {
        let own = lib.wall_pieces(&format!("{kit}_partition"), role);
        if !own.is_empty() {
            return (own, Thin::No);
        }
        return (lib.wall_pieces(kit, role), fallback);
    }
    (lib.wall_pieces(kit, role), Thin::No)
}

/// Queues edge pieces, then joints over their ends. Pieces are picked by
/// world edge midpoint (in half squares) and world vertex.
fn draw_walls(
    ops: &mut Vec<Op>,
    sprites: &mut Sprites<'_>,
    lib: &Library,
    seed: u64,
    layout: &TacticalLayout,
) {
    let (edges, joints) = walls::assemble(layout);
    let p = i64::from(sprites.ppsq);
    let (ox, oy) = layout.world_origin();
    for e in &edges {
        let (pieces, thin) = kit_pieces(
            lib,
            &e.segment.kit,
            e.segment.drawn_role(),
            e.segment.is_partition(),
            Thin::Band,
        );
        let (wx, wy) = (
            2 * ox + i64::from(e.centre2.0),
            2 * oy + i64::from(e.centre2.1),
        );
        let Some(a) = pick(&pieces, seed, wx, wy) else {
            continue;
        };
        let (cx, cy) = (
            i64::from(e.centre2.0) * p / 2,
            i64::from(e.centre2.1) * p / 2,
        );
        sprites.push(
            ops,
            a,
            Pose {
                thin,
                ..Pose::turned(e.turns, false)
            },
            (cx - p / 2, cy - p / 2),
        );
    }
    for j in &joints {
        let (pieces, thin) = kit_pieces(lib, j.kit, j.role, j.thin, Thin::Joint);
        let (wx, wy) = (ox + i64::from(j.vertex.0), oy + i64::from(j.vertex.1));
        let Some(a) = pick(&pieces, seed, wx, wy) else {
            continue;
        };
        let (cx, cy) = (i64::from(j.vertex.0) * p, i64::from(j.vertex.1) * p);
        sprites.push(
            ops,
            a,
            Pose {
                thin,
                ..Pose::turned(j.turns, false)
            },
            (cx - p / 2, cy - p / 2),
        );
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
mod tests_biome;
#[cfg(test)]
mod tests_look;
#[cfg(test)]
mod tests_pose;
#[cfg(test)]
mod tests_snow;
#[cfg(test)]
mod tests_structures;
#[cfg(test)]
mod tests_variants;
