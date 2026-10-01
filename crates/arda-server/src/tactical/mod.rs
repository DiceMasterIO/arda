//! Tactical battle maps over HTTP (goals 48, 65–68): the loaded asset
//! library, the built-in layouts, bounded render caches and the seam for
//! world-derived blocks.
//!
//! Every image is cached by [`RenderKey`] — world seed, anchor (world origin
//! or layout name), the BLAKE3 hash of the layout's JSON, ppsq, grid flag
//! and catalogue `library_version` — so identical requests give identical
//! bytes, whether cold or cached. The compositor seed derives from the world
//! seed and the anchor ([`key::render_seed`]).

pub mod block;
pub mod cells;
pub mod dto;
pub mod encode;
pub mod images;
pub mod key;
pub mod library_dto;
pub mod prefetch;
pub mod pyramid;
mod raw;
pub mod refine_blocks;
pub mod routes;
pub mod rules_dto;
pub mod scene;
pub mod tokens;
pub mod world_grade;
pub mod world_routes;

use crate::cache::ByteLru;
use crate::error::{lock, ServerError, ServerResult};
use arda_tactical::{layouts, Library, Rgba, TacticalLayout};
use axum::body::Bytes;
use block::{BlockSource, PendingBlocks};
use dto::{TacticalLayoutSummary, TacticalLayouts};
pub use key::{Anchor, RenderKey};
use pyramid::{Pyramid, TACTICAL_TILE_PX};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Output pixels per square clients may request.
pub const PPSQ_OPTIONS: [u32; 3] = [64, 96, 128];
/// Default output pixels per square.
pub const DEFAULT_PPSQ: u32 = 128;
/// Default `--library` directory, relative to the working directory.
pub const DEFAULT_LIBRARY: &str = "assets/tactical/placeholder";

/// Tactical request limits and cache budgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TacticalLimits {
    /// Largest `POST /render` body, bytes.
    pub max_body_bytes: usize,
    /// Largest layout, in squares.
    pub max_squares: u64,
    /// Largest render, in output pixels.
    pub max_render_px: u64,
    /// Budget for decoded full-resolution renders.
    pub render_cache_bytes: usize,
    /// Budget for decoded tile pyramids.
    pub pyramid_cache_bytes: usize,
    /// Budget for encoded PNGs and for encoded tiles, each.
    pub encoded_cache_bytes: usize,
}

impl Default for TacticalLimits {
    fn default() -> Self {
        Self {
            max_body_bytes: 4 << 20,
            // A 3 x 3-cell window plus its apron (logic/16 §api-tactical;
            // `cells::APRON` = 6): 204 x 204 squares.
            max_squares: 204 * 204,
            // A cell plus its apron at 128 px per square ((64 + 2 * 6) x
            // 128 px a side, about 95 Mpx); larger windows need a smaller
            // ppsq.
            max_render_px: 96 << 20,
            render_cache_bytes: 768 << 20,
            pyramid_cache_bytes: 1 << 30,
            encoded_cache_bytes: 256 << 20,
        }
    }
}

impl TacticalLimits {
    /// Worst-case resident bytes: full caches plus one transient render
    /// (RGBA, compositor buffers of about 6 bytes a pixel, and its pyramid).
    #[must_use]
    pub fn admitted_bytes(&self) -> u64 {
        let caches = self.render_cache_bytes + self.pyramid_cache_bytes;
        // PNGs, tiles and JSON bodies, plus the composed-block cache.
        let encoded = 3 * self.encoded_cache_bytes + refine_blocks::COMPOSED_CACHE_BYTES;
        (caches + encoded) as u64 + self.transient_render_bytes() + self.max_body_bytes as u64
    }

    /// Bytes of one render in flight: RGBA, compositor buffers of about 6
    /// bytes a pixel, and its pyramid.
    #[must_use]
    pub const fn transient_render_bytes(&self) -> u64 {
        self.max_render_px * (4 + 6 + 6)
    }
}

/// Longest layout name `POST /render` accepts, bytes. The name keys the
/// cache anchor and is echoed into logs and error messages.
pub const MAX_NAME_BYTES: usize = 128;
/// Squares a placement or light may sit outside the map (sprite overhang).
pub const POSITION_MARGIN_SQ: f32 = 16.0;
/// Placements allowed per square (plus [`PLACEMENT_SLACK`]).
pub const PLACEMENTS_PER_SQUARE: u64 = 4;
/// Placements allowed on top of the per-square allowance.
pub const PLACEMENT_SLACK: u64 = 64;
/// Lit pixels allowed per output pixel, summed over every free light.
pub const LIGHT_WORK_PER_PX: u64 = 16;

/// Refuses layouts whose size limits pass but whose work does not: long
/// names (cache-key and log bytes), unbounded placement counts, lights whose
/// summed pools cover the image many times over, and positions far off the
/// map (which overflow the compositor's pixel arithmetic). Refused, never
/// trimmed (goal-prompt §8).
fn admit_work(layout: &TacticalLayout, ppsq: u32, px: u64) -> ServerResult<()> {
    if layout.name.len() > MAX_NAME_BYTES || layout.name.chars().any(char::is_control) {
        return Err(ServerError::InvalidLayout(format!(
            "layout names are at most {MAX_NAME_BYTES} bytes without control characters"
        )));
    }
    let squares = u64::from(layout.width) * u64::from(layout.height);
    let max_placements = squares * PLACEMENTS_PER_SQUARE + PLACEMENT_SLACK;
    if layout.placements.len() as u64 > max_placements {
        return Err(ServerError::PayloadTooLarge(format!(
            "{} placements; the limit for this map is {max_placements}",
            layout.placements.len()
        )));
    }
    let (w, h) = (layout.width as f32, layout.height as f32);
    let on_map = |x: f32, y: f32| {
        x.is_finite()
            && y.is_finite()
            && (-POSITION_MARGIN_SQ..=w + POSITION_MARGIN_SQ).contains(&x)
            && (-POSITION_MARGIN_SQ..=h + POSITION_MARGIN_SQ).contains(&y)
    };
    if let Some(i) = layout.placements.iter().position(|p| !on_map(p.x, p.y)) {
        return Err(ServerError::InvalidLayout(format!(
            "placement {i} lies more than {POSITION_MARGIN_SQ} squares off the map"
        )));
    }
    if let Some(i) = layout.lights.iter().position(|l| !on_map(l.x, l.y)) {
        return Err(ServerError::InvalidLayout(format!(
            "light {i} lies more than {POSITION_MARGIN_SQ} squares off the map"
        )));
    }
    // Each light visits its pool's box clipped to the image (compose/lighting).
    let (wpx, hpx) = (
        u64::from(layout.width) * u64::from(ppsq),
        u64::from(layout.height) * u64::from(ppsq),
    );
    let work: u64 = layout
        .lights
        .iter()
        .map(|l| {
            let side = 2 * u64::from(l.radius_ft) * u64::from(ppsq) / 5;
            side.min(wpx) * side.min(hpx)
        })
        .fold(0u64, u64::saturating_add);
    let budget = px.saturating_mul(LIGHT_WORK_PER_PX);
    if work > budget {
        return Err(ServerError::PayloadTooLarge(format!(
            "the lights would light {work} px; the limit is {budget}"
        )));
    }
    Ok(())
}

/// An encoded image and its strong ETag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encoded {
    /// PNG or WebP bytes.
    pub bytes: Bytes,
    /// Quoted BLAKE3 prefix of `bytes`.
    pub etag: String,
}

impl Encoded {
    fn new(bytes: Vec<u8>) -> Self {
        let hash = blake3::hash(&bytes).to_hex();
        let etag = format!("\"{}\"", &hash.as_str()[..32]);
        Self {
            bytes: Bytes::from(bytes),
            etag,
        }
    }
}

/// Whether a response came from cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    /// Served from cache.
    Hit,
    /// Rendered or encoded for this request.
    Miss,
}

type TileKey = (RenderKey, u32, u32, u32);

/// The loaded library, built-in layouts, render caches and block source.
pub struct Tactical {
    library: Library,
    world_seed: u64,
    limits: TacticalLimits,
    named: BTreeMap<String, TacticalLayout>,
    blocks: Box<dyn BlockSource>,
    renders: Mutex<ByteLru<RenderKey, Rgba>>,
    pyramids: Mutex<ByteLru<RenderKey, Pyramid>>,
    pngs: Mutex<ByteLru<RenderKey, Encoded>>,
    tiles: Mutex<ByteLru<TileKey, Encoded>>,
    bodies: Mutex<ByteLru<cells::BodyKey, Encoded>>,
    lanes: raw::Lanes,
    /// Where the opt-in world grade reads world-map colours (goal 49).
    tint_source: Option<Arc<arda_midzoom::ReliefWorld>>,
}

impl std::fmt::Debug for Tactical {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tactical")
            .field("library_version", &self.library_version())
            .field("limits", &self.limits)
            .field("blocks", &self.blocks)
            .finish_non_exhaustive()
    }
}

fn cached<K: Ord + Clone, V>(m: &Mutex<ByteLru<K, V>>, key: &K) -> ServerResult<Option<Arc<V>>> {
    Ok(lock(m)?.get(key))
}

impl Tactical {
    /// Loads and validates the library at `dir` for the world of `world_seed`.
    ///
    /// # Errors
    /// [`ServerError::Tactical`] when loading or validation fails.
    pub fn open(dir: &Path, world_seed: u64, limits: TacticalLimits) -> ServerResult<Self> {
        let library = Library::load(dir)?;
        // Scale the ground textures for every served ppsq once, up front,
        // instead of in the first render at each (goal 50).
        {
            use rayon::prelude::*;
            PPSQ_OPTIONS
                .iter()
                .chain(&world_routes::WINDOW_PPSQ)
                .collect::<Vec<_>>()
                .par_iter()
                .for_each(|&&p| {
                    let _ = library.texture_set(p);
                });
        }
        let named = layouts::all()
            .into_iter()
            .map(|l| (l.name.clone(), l))
            .collect();
        Ok(Self {
            library,
            world_seed,
            limits,
            named,
            blocks: Box::new(PendingBlocks),
            renders: Mutex::new(ByteLru::new(limits.render_cache_bytes)),
            pyramids: Mutex::new(ByteLru::new(limits.pyramid_cache_bytes)),
            pngs: Mutex::new(ByteLru::new(limits.encoded_cache_bytes)),
            tiles: Mutex::new(ByteLru::new(limits.encoded_cache_bytes)),
            bodies: Mutex::new(ByteLru::new(limits.encoded_cache_bytes)),
            lanes: raw::Lanes::default(),
            tint_source: None,
        })
    }

    /// Replaces the block source.
    pub fn set_block_source(&mut self, source: Box<dyn BlockSource>) {
        self.blocks = source;
    }

    /// The block source.
    #[must_use]
    pub fn blocks(&self) -> &dyn BlockSource {
        self.blocks.as_ref()
    }

    /// Configured limits.
    #[must_use]
    pub const fn limits(&self) -> &TacticalLimits {
        &self.limits
    }

    /// Seed of the served world.
    #[must_use]
    pub const fn world_seed(&self) -> u64 {
        self.world_seed
    }

    /// The loaded library.
    #[must_use]
    pub const fn library(&self) -> &Library {
        &self.library
    }

    /// Catalogue `library_version`.
    #[must_use]
    pub fn library_version(&self) -> &str {
        &self.library.catalog.library_version
    }

    /// A built-in layout by name.
    ///
    /// # Errors
    /// [`ServerError::NotFound`] for unknown names.
    pub fn named(&self, name: &str) -> ServerResult<&TacticalLayout> {
        self.named.get(name).ok_or_else(|| {
            ServerError::NotFound(format!(
                "no tactical layout {name:?}; see /v1/tactical/layouts"
            ))
        })
    }

    /// The `/layouts` listing.
    #[must_use]
    pub fn listing(&self) -> TacticalLayouts {
        let layouts = self
            .named
            .values()
            .map(|l| TacticalLayoutSummary {
                name: l.name.clone(),
                width: l.width,
                height: l.height,
                tiles: tiles_dto(l, DEFAULT_PPSQ),
            })
            .collect();
        TacticalLayouts {
            world_seed: self.world_seed.to_string(),
            library_version: self.library_version().to_owned(),
            ppsq_options: PPSQ_OPTIONS.to_vec(),
            default_ppsq: DEFAULT_PPSQ,
            layouts,
        }
    }

    /// Checks size limits, then the layout against the library.
    ///
    /// # Errors
    /// [`ServerError::PayloadTooLarge`] or [`ServerError::InvalidLayout`].
    pub fn admit(&self, layout: &TacticalLayout, ppsq: u32) -> ServerResult<()> {
        let squares = u64::from(layout.width) * u64::from(layout.height);
        if squares > self.limits.max_squares {
            return Err(ServerError::PayloadTooLarge(format!(
                "{}x{} is {squares} squares; the limit is {}",
                layout.width, layout.height, self.limits.max_squares
            )));
        }
        let px = squares * u64::from(ppsq) * u64::from(ppsq);
        if px > self.limits.max_render_px {
            return Err(ServerError::PayloadTooLarge(format!(
                "the render would be {px} px at ppsq {ppsq}; the limit is {}",
                self.limits.max_render_px
            )));
        }
        admit_work(layout, ppsq, px)?;
        layout
            .check(&self.library)
            .map_err(|e| ServerError::InvalidLayout(e.to_string()))
    }

    /// The cache key of `layout` anchored at `anchor`, at `ppsq`, with or
    /// without grid.
    ///
    /// # Errors
    /// [`ServerError::Internal`] if the layout cannot be serialised.
    pub fn key(
        &self,
        layout: &TacticalLayout,
        anchor: Anchor,
        ppsq: u32,
        grid: bool,
    ) -> ServerResult<RenderKey> {
        self.key_cropped(layout, anchor, ppsq, grid, None)
    }

    /// As [`Self::key`], keeping only `crop` (`[x, y, w, h]` squares) of
    /// the render.
    ///
    /// # Errors
    /// [`ServerError::Internal`] if the layout cannot be serialised.
    pub fn key_cropped(
        &self,
        layout: &TacticalLayout,
        anchor: Anchor,
        ppsq: u32,
        grid: bool,
        crop: Option<[u32; 4]>,
    ) -> ServerResult<RenderKey> {
        let json = serde_json::to_vec(layout)
            .map_err(|e| ServerError::Internal(format!("layout json: {e}")))?;
        Ok(RenderKey {
            world_seed: self.world_seed,
            anchor,
            layout: *blake3::hash(&json).as_bytes(),
            ppsq,
            grid,
            library_version: self.library_version().to_owned(),
            crop,
            world_grade: false,
        })
    }

    /// The PNG of `layout`.
    ///
    /// # Errors
    /// Limits, layout validation, render or encoding failures.
    pub fn png(
        &self,
        layout: &TacticalLayout,
        key: &RenderKey,
    ) -> ServerResult<(Arc<Encoded>, Hit)> {
        if let Some(hit) = cached(&self.pngs, key)? {
            return Ok((hit, Hit::Hit));
        }
        let img = self.raw(key, layout)?;
        let start = Instant::now();
        let encoded = Encoded::new(encode::encode_png(img.width, img.height, &img.data)?);
        stage("png encode", &img, start);
        let bytes = encoded.bytes.len();
        let value = lock(&self.pngs)?.insert(key.clone(), Arc::new(encoded), bytes);
        Ok((value, Hit::Miss))
    }

    fn pyramid(&self, layout: &TacticalLayout, key: &RenderKey) -> ServerResult<Arc<Pyramid>> {
        self.pyramid_in(layout, key, raw::Lane::Request)
    }

    fn pyramid_in(
        &self,
        layout: &TacticalLayout,
        key: &RenderKey,
        lane: raw::Lane,
    ) -> ServerResult<Arc<Pyramid>> {
        if let Some(hit) = cached(&self.pyramids, key)? {
            return Ok(hit);
        }
        let img = self.raw_in(key, layout, lane)?;
        let start = Instant::now();
        let p = Pyramid::build(Rgba::clone(&img));
        stage("pyramid", &img, start);
        let bytes = p.bytes();
        Ok(lock(&self.pyramids)?.insert(key.clone(), Arc::new(p), bytes))
    }

    /// Tile `(z, x, y)` of `layout` as lossless WebP.
    ///
    /// # Errors
    /// [`ServerError::NotFound`] outside the pyramid, or render failures.
    pub fn tile(
        &self,
        layout: &TacticalLayout,
        key: &RenderKey,
        (z, x, y): (u32, u32, u32),
    ) -> ServerResult<(Arc<Encoded>, Hit)> {
        let tkey = (key.clone(), z, x, y);
        if let Some(hit) = cached(&self.tiles, &tkey)? {
            return Ok((hit, Hit::Hit));
        }
        let (sw, sh) = key
            .crop
            .map_or((layout.width, layout.height), |c| (c[2], c[3]));
        let (w, h) = (sw.saturating_mul(key.ppsq), sh.saturating_mul(key.ppsq));
        let top = pyramid::max_zoom(w, h);
        let (cols, rows) = pyramid::tile_grid(w, h, top, z.min(top));
        if z > top || x >= cols || y >= rows {
            return Err(ServerError::NotFound(format!(
                "tile {z}/{x}/{y} is outside the pyramid (zoom 0..={top})"
            )));
        }
        let rgba = self.pyramid(layout, key)?.tile(z, x, y)?;
        let encoded = Encoded::new(encode_webp(&rgba, TACTICAL_TILE_PX)?);
        let bytes = encoded.bytes.len();
        let value = lock(&self.tiles)?.insert(tkey, Arc::new(encoded), bytes);
        Ok((value, Hit::Miss))
    }
}

use images::stage;
pub use images::{crop, encode_webp, tiles_dto, tiles_dto_sized};
