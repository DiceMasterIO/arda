//! The bake itself (1.0 task 04; see [`super::bake`]): checking a posted
//! cell, splicing it into its apron block, rendering it in the bake lane
//! and encoding its tile pyramid, all without the shared caches.

use super::bake::{BAKE_FORMAT, CELL_SQUARES};
use super::bake_dto::{BakeRequest, BakeRequestParsed};
use super::bake_store::BakeResult;
use super::block::BlockRequest;
use super::pyramid::{self, Pyramid, TACTICAL_TILE_PX};
use super::{encode_webp, tiles_dto_sized, Encoded, Tactical, PPSQ_OPTIONS};
use crate::error::{ServerError, ServerResult};
use arda_tactical::TacticalLayout;
use std::collections::BTreeMap;
use std::sync::Arc;

impl Tactical {
    /// Checks a posted bake against the world (`cells` wide and high), the
    /// limits and the library.
    ///
    /// # Errors
    /// 400 for a bad version, cell or ppsq; 422 `invalid_layout` for a
    /// layout that is not exactly the cell or does not resolve; 413 over
    /// the limits; 404 `no_block` for a cell without a block.
    pub fn check_bake(
        &self,
        body: &BakeRequest,
        cells: (u32, u32),
    ) -> ServerResult<BakeRequestParsed> {
        if let Some(v) = body.format_version.filter(|&v| v != BAKE_FORMAT) {
            return Err(ServerError::BadRequest(format!(
                "format_version must be {BAKE_FORMAT}, not {v}"
            )));
        }
        let (gx, gy) = (body.gx, body.gy);
        if gx >= cells.0 || gy >= cells.1 {
            return Err(ServerError::OutOfRange(format!(
                "cell {gx},{gy} is outside the {}x{}-cell world",
                cells.0, cells.1
            )));
        }
        let ppsq = body.ppsq.unwrap_or(super::DEFAULT_PPSQ);
        if !PPSQ_OPTIONS.contains(&ppsq) {
            return Err(ServerError::BadRequest(format!(
                "ppsq must be one of {PPSQ_OPTIONS:?}, not {ppsq}"
            )));
        }
        let layout: TacticalLayout = serde_json::to_value(&body.layout)
            .and_then(serde_json::from_value)
            .map_err(|e| ServerError::InvalidLayout(e.to_string()))?;
        let origin = [
            i64::from(gx) * i64::from(CELL_SQUARES),
            i64::from(gy) * i64::from(CELL_SQUARES),
        ];
        if (layout.width, layout.height) != (CELL_SQUARES, CELL_SQUARES)
            || layout.origin != Some(origin)
        {
            return Err(ServerError::InvalidLayout(format!(
                "a bake layout is the whole cell: {CELL_SQUARES}x{CELL_SQUARES} squares with origin \
                 {origin:?}, not {}x{} at {:?}",
                layout.width, layout.height, layout.origin
            )));
        }
        self.admit(&layout, ppsq)?;
        let world = (
            i64::from(cells.0) * i64::from(CELL_SQUARES),
            i64::from(cells.1) * i64::from(CELL_SQUARES),
        );
        self.world_block_cut(&BlockRequest::cell(gx, gy), world)?;
        Ok(BakeRequestParsed {
            gx,
            gy,
            ppsq,
            world_grade: body.world_grade.unwrap_or(false),
            layout,
        })
    }

    /// Renders `req` spliced into its apron block, as the cell route
    /// renders the unedited cell, and encodes every tile of its pyramid.
    /// Reads and writes no shared cache; the caller holds the bake lane.
    ///
    /// # Errors
    /// Block, splice, limit, render or encoding failures.
    pub(super) fn bake(
        &self,
        req: &BakeRequestParsed,
        world: (i64, i64),
    ) -> ServerResult<BakeResult> {
        let cell = BlockRequest::cell(req.gx, req.gy);
        let (block, key) =
            self.world_render_look(&cell, world, req.ppsq, (false, req.world_grade))?;
        let crop = key
            .crop
            .ok_or_else(|| ServerError::Internal("a cell render is cropped".into()))?;
        let layout = super::splice::splice(&block.layout, &req.layout, (crop[0], crop[1]))?;
        drop(block);
        let mut key = self.key_cropped(&layout, key.anchor, req.ppsq, false, Some(crop))?;
        key.world_grade = req.world_grade;
        let img = self.render_uncached(&key, &layout)?;
        drop(layout);
        let (w, h) = (img.width, img.height);
        let p = {
            let _slot = self.lanes.encode_slot_uncancelled();
            Pyramid::build(img)
        };
        let top = p.max_zoom();
        let mut encoded = BTreeMap::new();
        let mut bytes = 0_u64;
        for z in 0..=top {
            let (cols, rows) = pyramid::tile_grid(w, h, top, z);
            for y in 0..rows {
                for x in 0..cols {
                    let rgba = p.tile(z, x, y)?;
                    let tile = {
                        let _slot = self.lanes.encode_slot_uncancelled();
                        Encoded::new(encode_webp(&rgba, TACTICAL_TILE_PX)?)
                    };
                    bytes += tile.bytes.len() as u64;
                    encoded.insert((z, x, y), Arc::new(tile));
                }
            }
        }
        Ok(BakeResult {
            tiles: tiles_dto_sized(crop[2], crop[3], req.ppsq),
            encoded,
            bytes,
        })
    }

    /// Entries in the render, pyramid, PNG and tile caches (tests).
    #[doc(hidden)]
    #[must_use]
    pub fn cache_counts(&self) -> [usize; 4] {
        let n = |len: Option<usize>| len.unwrap_or(0);
        [
            n(self.renders.lock().ok().map(|c| c.len())),
            n(self.pyramids.lock().ok().map(|c| c.len())),
            n(self.pngs.lock().ok().map(|c| c.len())),
            n(self.tiles.lock().ok().map(|c| c.len())),
        ]
    }

    /// Full-resolution renders and tile pyramids built so far (tests:
    /// concurrent requests for one key build it once).
    #[doc(hidden)]
    #[must_use]
    pub fn build_counts(&self) -> [u64; 2] {
        self.lanes.built()
    }

    /// Tests: while `on`, no render enters the request lane, so requests
    /// line up behind it.
    #[doc(hidden)]
    pub fn hold_request_lane(&self, on: bool) {
        self.lanes.request_gate().hold(on);
    }

    /// Tests: requests parked at the request lane, the encode slot or
    /// another request's render, pyramid, tile or PNG.
    #[doc(hidden)]
    #[must_use]
    pub fn parked(&self) -> usize {
        self.lanes.parked()
    }
}
