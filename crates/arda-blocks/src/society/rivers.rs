//! The refined rivers as `arda-ways` sees them: the same centreline pieces
//! arda-refine draws each block's channels from, so bridges and fords sit
//! on the water the map shows (not on settle's straight centre-to-centre
//! lines). Pieces are cached per cell; each is a pure function of the
//! world, so every window agrees.

use super::SQUARE_M;
use crate::overlays::OverlayCtx;
use crate::BlocksError;
use arda_people::terrain::Surroundings;
use arda_refine::pools::wet_cell;
use arda_refine::rivers::Piece;
use arda_refine::water::{cell_channels, RiverWater};
use arda_refine::{CellKey, Source};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Channel cells gathered beyond the window on each side: `arda-ways`
/// looks for water under its ways up to its dense margin (400 m) from the
/// window, and a cell's pieces reach under one cell beyond it.
pub const CHANNEL_MARGIN_CELLS: i64 = 6;
/// Cells whose pieces are kept (about 10 kB each at most).
const CACHE_CELLS: usize = 20_000;

/// Refined channel pieces by cell.
#[derive(Default)]
pub struct ChannelCache {
    cells: Mutex<BTreeMap<CellKey, Arc<Vec<Piece>>>>,
}

impl ChannelCache {
    fn cell(&self, src: &dyn Source, c: CellKey) -> Result<Arc<Vec<Piece>>, BlocksError> {
        let poisoned = || BlocksError::Overlay {
            layer: "ways",
            message: "channel cache poisoned".into(),
        };
        if let Some(p) = self.cells.lock().map_err(|_| poisoned())?.get(&c) {
            return Ok(Arc::clone(p));
        }
        let pieces = Arc::new(cell_channels(src, c)?);
        let mut guard = self.cells.lock().map_err(|_| poisoned())?;
        if guard.len() >= CACHE_CELLS {
            guard.clear();
        }
        Ok(Arc::clone(guard.entry(c).or_insert(pieces)))
    }

    /// The river water of every channel cell near the window.
    ///
    /// # Errors
    /// A world layer failed to read.
    pub fn around(
        &self,
        ctx: &OverlayCtx<'_>,
        src: &dyn Source,
    ) -> Result<RiverWater, BlocksError> {
        let m = CHANNEL_MARGIN_CELLS;
        let (cx0, cy0) = (ctx.gsx0.div_euclid(64) - m, ctx.gsy0.div_euclid(64) - m);
        let cx1 = (ctx.gsx0 + i64::from(ctx.width) - 1).div_euclid(64) + m;
        let cy1 = (ctx.gsy0 + i64::from(ctx.height) - 1).div_euclid(64) + m;
        let mut pieces = Vec::new();
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                pieces.extend(self.cell(src, CellKey::new(cx, cy))?.iter().cloned());
            }
        }
        Ok(RiverWater::new(pieces))
    }
}

/// `arda-ways` terrain over the world: heights from the cells around the
/// window, river water from the refined channels, wet ground from the
/// cells' marsh and floodplain rule (arda-refine's pools).
pub struct WaysTerrain<'a> {
    heights: &'a Surroundings,
    water: &'a RiverWater,
    src: &'a dyn Source,
    channels: Vec<arda_ways::RiverChannel>,
    wet: RefCell<BTreeMap<CellKey, bool>>,
}

impl<'a> WaysTerrain<'a> {
    /// The terrain; each refined piece becomes a guide channel in metres.
    #[must_use]
    pub fn new(heights: &'a Surroundings, water: &'a RiverWater, src: &'a dyn Source) -> Self {
        let channels = water
            .pieces()
            .enumerate()
            .map(|(k, p)| arda_ways::RiverChannel {
                id: u64::try_from(k + 1).unwrap_or(u64::MAX),
                centreline: p
                    .pts
                    .iter()
                    .map(|&(u, v)| [u * SQUARE_M, v * SQUARE_M])
                    .collect(),
                width_m: 2.0 * p.half.iter().copied().fold(0.0, f64::max) * SQUARE_M,
                depth_m: p.depth.iter().copied().fold(0.3, f64::max),
            })
            .collect();
        Self {
            heights,
            water,
            src,
            channels,
            wet: RefCell::new(BTreeMap::new()),
        }
    }
}

impl arda_ways::Terrain for WaysTerrain<'_> {
    fn height_m(&self, x_m: f64, y_m: f64) -> f64 {
        self.heights.height_m(x_m, y_m)
    }

    fn channels(&self) -> &[arda_ways::RiverChannel] {
        &self.channels
    }

    fn rivers_rasterised(&self) -> bool {
        true
    }

    fn river_water(&self, gx: i64, gy: i64) -> bool {
        self.water.is_water(gx, gy)
    }

    fn wet_ground(&self, gx: i64, gy: i64) -> bool {
        let k = CellKey::new(gx.div_euclid(64), gy.div_euclid(64));
        if let Some(&w) = self.wet.borrow().get(&k) {
            return w;
        }
        let (w, h) = self.src.cells_wide_high();
        let inside = k.x >= 0 && k.y >= 0 && k.x < w && k.y < h;
        let wet = inside && self.src.cell(k).is_ok_and(|c| wet_cell(&c));
        self.wet.borrow_mut().insert(k, wet);
        wet
    }
}
