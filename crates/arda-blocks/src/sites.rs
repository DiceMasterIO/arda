//! Dungeon and cave entrances on the surface map (logic/18
//! §site-entrance): the entrance prop north of the entrance square, and
//! clear dry ground a party can stand on around it.

use crate::overlays::{OverlayCtx, OverlayLayer, Overlays};
use crate::BlocksError;
use arda_scene::RulesSidecar;
use arda_tactical::layout::{AssetRef, Placement};
use std::sync::Arc;

/// Squares cleared around the entrance square on every side.
pub const CLEAR: i64 = 2;

/// One site's entrance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entrance {
    /// Entrance square column, world squares.
    pub gsx: i64,
    /// Entrance square row.
    pub gsy: i64,
    /// A cave mouth rather than a dungeon's stair head.
    pub cave: bool,
}

/// The entrances' layer for a window, `None` when none touches it.
#[must_use]
pub fn layer(ctx: &OverlayCtx<'_>, entrances: &[Entrance]) -> Option<OverlayLayer> {
    let (w, h) = (i64::from(ctx.width), i64::from(ctx.height));
    let mut layout = ctx.base.clone();
    layout.walls.clear();
    layout.placements.clear();
    layout.lights.clear();
    let mut owned = vec![false; ctx.base.squares.len()];
    for e in entrances {
        let (ex, ey) = (e.gsx - ctx.gsx0, e.gsy - ctx.gsy0);
        for y in (ey - CLEAR).max(0)..=(ey + CLEAR).min(h - 1) {
            for x in (ex - CLEAR).max(0)..=(ex + CLEAR).min(w - 1) {
                let Ok(i) = usize::try_from(y * w + x) else {
                    continue;
                };
                owned[i] = true;
                let sq = &mut layout.squares[i];
                sq.ground = if e.cave { "gravel" } else { "flagstone" }.to_owned();
                sq.water_depth_ft = 0;
            }
        }
        // Both props end on the entrance square's north edge: the 1 × 2
        // stair head covers the two squares north of it, the 2 × 2 mouth
        // those and their west neighbours. A placement belongs to the
        // square its position falls in, so it is laid only where that
        // square is in the window and every window agrees.
        let (ax, ay) = (ex, ey - 1);
        if (0..w).contains(&ax) && (0..h).contains(&ay) {
            #[allow(clippy::cast_precision_loss)]
            let (x, y) = (
                if e.cave { ex as f32 } else { ex as f32 + 0.5 },
                ey as f32 - 1.0,
            );
            let asset = if e.cave {
                "prop.cave_mouth"
            } else {
                "prop.site_entrance"
            };
            layout.placements.push(Placement {
                asset: AssetRef::Id(asset.to_owned()),
                x,
                y,
                rotation: 0,
                mirror: false,
            });
        }
    }
    owned.contains(&true).then(|| OverlayLayer {
        layout,
        rules: RulesSidecar::empty(ctx.width, ctx.height),
        owned,
        soft: Vec::new(),
        elevation: Vec::new(),
        review: Vec::new(),
    })
}

/// The world's overlays with its sites' entrances on top.
#[derive(Debug)]
pub struct SiteOverlays {
    inner: Option<Arc<dyn Overlays>>,
    entrances: Vec<Entrance>,
}

impl SiteOverlays {
    /// `inner` supplies ways, fields and town; `entrances` the sites.
    #[must_use]
    pub fn new(inner: Option<Arc<dyn Overlays>>, entrances: Vec<Entrance>) -> Self {
        Self { inner, entrances }
    }
}

impl Overlays for SiteOverlays {
    fn ways(&self, ctx: &OverlayCtx<'_>) -> Result<Option<OverlayLayer>, BlocksError> {
        self.inner.as_ref().map_or(Ok(None), |o| o.ways(ctx))
    }

    fn fields(&self, ctx: &OverlayCtx<'_>) -> Result<Option<OverlayLayer>, BlocksError> {
        self.inner.as_ref().map_or(Ok(None), |o| o.fields(ctx))
    }

    fn town(&self, ctx: &OverlayCtx<'_>) -> Result<Option<OverlayLayer>, BlocksError> {
        self.inner.as_ref().map_or(Ok(None), |o| o.town(ctx))
    }

    fn sites(&self, ctx: &OverlayCtx<'_>) -> Result<Option<OverlayLayer>, BlocksError> {
        Ok(layer(ctx, &self.entrances))
    }
}
