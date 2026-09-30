//! Real overlays from the world's society data (adapter A9; logic/09
//! §reservations): roads and crossings from `society/roads.json` through
//! `arda-ways`, land use from `society/landuse.bin` through `arda-fields`
//! (A17), and town plans through `arda-town` (a `TownSite` from each
//! settlement record, terrain from the world; `arda-people`).
//!
//! Every layer is a pure function of global coordinates, the world and its
//! society files, so neighbouring windows agree (goal 46). Inputs are
//! pre-filtered to the window plus a margin by bounding box; the margins
//! exceed the reach of any curve, switchback, crossing channel or plan.

mod layers;
pub mod rivers;
pub mod town;

use crate::overlays::{OverlayCtx, OverlayLayer, Overlays};
use crate::BlocksError;
use arda_people::World;
use std::sync::Arc;

pub use layers::{fields_class, LandUseRaster};

/// Metres per square (convention I2: 100/64 m).
pub const SQUARE_M: f64 = 100.0 / 64.0;
/// Roads and crossings further than this from a window are not planned
/// into it, metres.
pub const WAYS_MARGIN_M: f64 = 600.0;
/// Settlements and roads further than this from a window do not shape its
/// fields, metres.
pub const FIELDS_MARGIN_M: f64 = 3_000.0;

/// An axis-aligned box in world metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bbox {
    /// West edge.
    pub x0: f64,
    /// North edge.
    pub y0: f64,
    /// East edge.
    pub x1: f64,
    /// South edge.
    pub y1: f64,
}

impl Bbox {
    /// The box around `points`.
    #[must_use]
    pub fn of(points: impl IntoIterator<Item = [f64; 2]>) -> Self {
        let mut b = Self {
            x0: f64::INFINITY,
            y0: f64::INFINITY,
            x1: f64::NEG_INFINITY,
            y1: f64::NEG_INFINITY,
        };
        for [x, y] in points {
            (b.x0, b.y0, b.x1, b.y1) = (b.x0.min(x), b.y0.min(y), b.x1.max(x), b.y1.max(y));
        }
        b
    }

    /// The window's box in world metres.
    #[must_use]
    pub fn of_window(ctx: &OverlayCtx<'_>) -> Self {
        #[allow(clippy::cast_precision_loss)] // squares are far below 2^52
        let m = |s: i64| s as f64 * SQUARE_M;
        Self {
            x0: m(ctx.gsx0),
            y0: m(ctx.gsy0),
            x1: m(ctx.gsx0 + i64::from(ctx.width)),
            y1: m(ctx.gsy0 + i64::from(ctx.height)),
        }
    }

    /// Whether the boxes come within `margin` of each other.
    #[must_use]
    pub fn near(&self, other: &Self, margin: f64) -> bool {
        self.x0 - margin <= other.x1
            && other.x0 <= self.x1 + margin
            && self.y0 - margin <= other.y1
            && other.y0 <= self.y1 + margin
    }
}

/// The world's society as the three overlay layers.
pub struct SocietyOverlays {
    world: Arc<World>,
    ways_roads: Vec<(Bbox, arda_ways::Road)>,
    crossings: Vec<arda_ways::Crossing>,
    field_roads: Vec<(Bbox, arda_fields::Road)>,
    landuse: LandUseRaster,
    channels: rivers::ChannelCache,
}

impl std::fmt::Debug for SocietyOverlays {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SocietyOverlays")
            .field("roads", &self.ways_roads.len())
            .field("crossings", &self.crossings.len())
            .finish_non_exhaustive()
    }
}

impl SocietyOverlays {
    /// Overlays over `world`: converts its roads and crossings once and
    /// reads `landuse.bin`.
    ///
    /// # Errors
    /// [`BlocksError::Overlay`] for unreadable or malformed society files.
    pub fn open(world: Arc<World>) -> Result<Self, BlocksError> {
        let (ways_roads, crossings, field_roads) = layers::roads(&world)?;
        let rural = |id: u64| {
            world.files.settlement(id).is_ok_and(|(s, _)| {
                matches!(
                    s.tier,
                    arda_settle::model::Tier::Hamlet | arda_settle::model::Tier::Village
                )
            })
        };
        let landuse = LandUseRaster::read(&world.files.dir.join("landuse.bin"), rural)?;
        Ok(Self {
            world,
            ways_roads,
            crossings,
            field_roads,
            landuse,
            channels: rivers::ChannelCache::default(),
        })
    }

    /// The world the overlays read.
    #[must_use]
    pub fn world(&self) -> &Arc<World> {
        &self.world
    }
}

impl Overlays for SocietyOverlays {
    fn ways(&self, ctx: &OverlayCtx<'_>) -> Result<Option<OverlayLayer>, BlocksError> {
        let win = Bbox::of_window(ctx);
        let roads: Vec<arda_ways::Road> = self
            .ways_roads
            .iter()
            .filter(|(b, _)| b.near(&win, WAYS_MARGIN_M))
            .map(|(_, r)| r.clone())
            .collect();
        if roads.is_empty() {
            return Ok(None);
        }
        let crossings: Vec<arda_ways::Crossing> = self
            .crossings
            .iter()
            .filter(|c| {
                #[allow(clippy::cast_precision_loss)]
                let at = Bbox::of([[c.x_m as f64, c.y_m as f64]]);
                at.near(&win, WAYS_MARGIN_M + f64::from(c.width_m))
            })
            .cloned()
            .collect();
        let water = self.channels.around(ctx, &self.world.src)?;
        layers::ways(ctx, &self.world, &roads, &crossings, &water).map(Some)
    }

    fn fields(&self, ctx: &OverlayCtx<'_>) -> Result<Option<OverlayLayer>, BlocksError> {
        let win = Bbox::of_window(ctx);
        let roads: Vec<arda_fields::Road> = self
            .field_roads
            .iter()
            .filter(|(b, _)| b.near(&win, FIELDS_MARGIN_M))
            .map(|(_, r)| r.clone())
            .collect();
        layers::fields(ctx, &self.world, &self.landuse, &roads).map(Some)
    }

    fn town(&self, ctx: &OverlayCtx<'_>) -> Result<Option<OverlayLayer>, BlocksError> {
        town::town(ctx, &self.world)
    }
}
