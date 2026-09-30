//! World blocks and windows over HTTP (logic/16 §api-tactical): the JSON
//! body (layout, rules, origin, meta and scene from one layout, goal 48),
//! the PNG and the WebP tile pyramid.
//!
//! Images are rendered from the window grown by an apron of [`APRON`]
//! squares taken from the neighbouring blocks, then cropped (logic/11
//! §seam-art 2), with the world's one render seed and the layout's world
//! `origin` (§seam-art 1, 3), so a block's edge pixels equal those of a
//! wider window and neighbours join invisibly (logic/16 Invariant 5).

use super::block::{Block, BlockRequest};
use super::dto::{TacticalBlockDto, LAYOUT_SCHEMA, TACTICAL_FORMAT};
use super::key::render_seed;
use super::{cached, tiles_dto_sized, Anchor, Encoded, Hit, RenderKey, Tactical};
use crate::error::{lock, ServerError, ServerResult};
use std::sync::Arc;

/// Squares borrowed from the neighbours on every side of a render. The
/// compositor's soft blends, shadows and canopies reach across an edge: 2
/// squares (logic/11 §seam-art's figure) left 11,402 differing bytes on the
/// MICRO river cell, 4 joined exactly; 6 keeps a margin for taller trees.
/// See the seam test in `tests/world_api/cells.rs`.
pub const APRON: u32 = 6;
/// Largest `/window` side, in cells.
pub const MAX_WINDOW_CELLS: u32 = 3;

/// Cache identity of a JSON body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BodyKey {
    /// The window and overlays.
    pub req: BlockRequest,
    /// The ppsq the `tiles` geometry describes.
    pub ppsq: u32,
}

fn json<T: serde::Serialize, U: serde::de::DeserializeOwned>(v: &T) -> ServerResult<U> {
    serde_json::to_value(v)
        .and_then(serde_json::from_value)
        .map_err(|e| ServerError::Internal(format!("mirror: {e}")))
}

impl Tactical {
    /// The composed block for `req` from the block source.
    ///
    /// # Errors
    /// The source's [`super::block::BlockError`].
    pub fn world_block(&self, req: &BlockRequest) -> ServerResult<Block> {
        Ok(self.blocks.block(req, &self.library)?)
    }

    /// The compositor seed of every world-anchored render of this world.
    #[must_use]
    pub fn world_render_seed(&self) -> u64 {
        render_seed(self.world_seed, &Anchor::Origin(0, 0))
    }

    /// The apron block and render key of `req`'s image. `world` is the
    /// world's extent in squares.
    ///
    /// # Errors
    /// Block source failures.
    pub fn world_render(
        &self,
        req: &BlockRequest,
        world: (i64, i64),
        ppsq: u32,
        grid: bool,
    ) -> ServerResult<(Block, RenderKey)> {
        let apron = req.with_apron(APRON, world);
        let block = self.world_block(&apron)?;
        let off = |a: i64, b: i64| u32::try_from(a - b).unwrap_or(0);
        let crop = [
            off(req.gsx0, apron.gsx0),
            off(req.gsy0, apron.gsy0),
            req.w,
            req.h,
        ];
        let anchor = Anchor::Origin(req.gsx0, req.gsy0);
        let key = self.key_cropped(&block.layout, anchor, ppsq, grid, Some(crop))?;
        Ok((block, key))
    }

    /// The PNG of `req`.
    ///
    /// # Errors
    /// Block, limit, render or encoding failures.
    pub fn world_png(
        &self,
        req: &BlockRequest,
        world: (i64, i64),
        ppsq: u32,
        grid: bool,
    ) -> ServerResult<(Arc<Encoded>, Hit)> {
        let (block, key) = self.world_render(req, world, ppsq, grid)?;
        self.png(&block.layout, &key)
    }

    /// Tile `(z, x, y)` of `req`'s image.
    ///
    /// # Errors
    /// [`ServerError::NotFound`] outside the pyramid; block or render failures.
    pub fn world_tile(
        &self,
        req: &BlockRequest,
        world: (i64, i64),
        (ppsq, grid): (u32, bool),
        zxy: (u32, u32, u32),
    ) -> ServerResult<(Arc<Encoded>, Hit)> {
        let (block, key) = self.world_render(req, world, ppsq, grid)?;
        self.tile(&block.layout, &key, zxy)
    }

    /// The JSON body of `req`: layout, rules, origin, meta, scene and tile
    /// geometry at `ppsq`.
    ///
    /// # Errors
    /// Block or scene failures.
    pub fn world_body(&self, req: &BlockRequest, ppsq: u32) -> ServerResult<(Arc<Encoded>, Hit)> {
        let key = BodyKey { req: *req, ppsq };
        if let Some(hit) = cached(&self.bodies, &key)? {
            return Ok((hit, Hit::Hit));
        }
        let block = self.world_block(req)?;
        let seed = self.world_render_seed();
        let scene =
            arda_scene::build_scene(&block.layout, &self.library, seed, block.rules.as_ref())
                .map_err(|e| ServerError::InvalidLayout(e.to_string()))?;
        let mut scene = serde_json::to_value(&scene)
            .map_err(|e| ServerError::Internal(format!("scene json: {e}")))?;
        // logic/16 §api-conventions: u64 seeds travel as decimal strings.
        if let Some(obj) = scene.as_object_mut() {
            obj.insert("seed".into(), seed.to_string().into());
            obj.insert("origin_gs".into(), serde_json::json!(block.origin));
        }
        let dto = TacticalBlockDto {
            tactical_format: TACTICAL_FORMAT,
            layout_schema: LAYOUT_SCHEMA,
            origin: block.origin,
            size: [block.layout.width, block.layout.height],
            render_seed: seed.to_string(),
            layout: json(&block.layout)?,
            rules: block.rules.as_ref().map(json).transpose()?,
            meta: block.meta.clone(),
            scene: Some(scene),
            tiles: tiles_dto_sized(block.layout.width, block.layout.height, ppsq),
        };
        let bytes =
            serde_json::to_vec(&dto).map_err(|e| ServerError::Internal(format!("json: {e}")))?;
        let encoded = Encoded::new(bytes);
        let n = encoded.bytes.len();
        let value = lock(&self.bodies)?.insert(key, Arc::new(encoded), n);
        Ok((value, Hit::Miss))
    }
}
