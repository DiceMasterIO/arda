//! Demo overlays from the crates' own synthetic samples, so the overlay
//! seam, the composition order (ways, fields, town) and the seams between
//! independently composed windows are exercised before settlement data is
//! integrated (it arrives with integrate/product).
//!
//! The samples live in their own metre frames. The demo frame is
//! `arda-town`'s Thornby sample (a farming village where two roads cross):
//! its market square is placed at the centre of the world cell `anchor`.
//! Every world square `gs` maps to the demo square
//! `gs - anchor_centre + market`, a pure translation, so neighbouring
//! windows composed with the same anchor agree square for square.
//!
//! - ways: Thornby's entering roads, laid by `arda-ways` on the refined
//!   window (no channels: where a road meets refined water the composition
//!   refuses the claim rather than drying the river);
//! - fields: `arda-fields`' `village_strips` scenario, translated so its
//!   village sits on Thornby's market; its built core is left to the town;
//! - town: the Thornby plan, cut with `arda-town`'s block generator.
//!
//! Demo layers keep the refined elevation (`OverlayLayer::elevation` is
//! false): the samples' synthetic terrain is not the world's.

mod layers;

use crate::overlays::{OverlayCtx, OverlayLayer, Overlays};
use crate::pipeline::SQUARES_PER_CELL;
use crate::BlocksError;
use arda_town::TownPlan;
use std::sync::{Arc, Mutex};

/// Overlays built from the synthetic samples, anchored at a world cell.
#[derive(Debug)]
pub struct DemoOverlays {
    anchor: [i64; 2],
    town: Mutex<Option<(u64, Arc<TownPlan>)>>,
}

impl DemoOverlays {
    /// Demo overlays whose village centres on world cell `(gx, gy)`.
    #[must_use]
    pub const fn at_cell(gx: i64, gy: i64) -> Self {
        Self {
            anchor: [gx, gy],
            town: Mutex::new(None),
        }
    }

    /// The anchor cell.
    #[must_use]
    pub const fn anchor(&self) -> [i64; 2] {
        self.anchor
    }

    /// The Thornby plan for `seed`, generated once per seed.
    fn plan(&self, seed: u64) -> Result<Arc<TownPlan>, BlocksError> {
        let mut guard = self.town.lock().map_err(|_| BlocksError::Poisoned)?;
        if let Some((s, p)) = guard.as_ref() {
            if *s == seed {
                return Ok(Arc::clone(p));
            }
        }
        let plan = Arc::new(layers::thornby_plan(seed)?);
        *guard = Some((seed, Arc::clone(&plan)));
        Ok(plan)
    }

    /// Offset from world squares to the demo (Thornby) frame, in squares.
    fn offset(&self, plan: &TownPlan) -> (i64, i64) {
        let (mx, my) = layers::market_square(plan);
        let half = SQUARES_PER_CELL / 2;
        (
            mx - (self.anchor[0] * SQUARES_PER_CELL + half),
            my - (self.anchor[1] * SQUARES_PER_CELL + half),
        )
    }
}

impl Overlays for DemoOverlays {
    fn ways(&self, ctx: &OverlayCtx<'_>) -> Result<Option<OverlayLayer>, BlocksError> {
        let plan = self.plan(ctx.seed)?;
        let off = self.offset(&plan);
        layers::ways(ctx, off).map(Some)
    }

    fn fields(&self, ctx: &OverlayCtx<'_>) -> Result<Option<OverlayLayer>, BlocksError> {
        let plan = self.plan(ctx.seed)?;
        let off = self.offset(&plan);
        let (mx, my) = layers::market_square(&plan);
        layers::fields(ctx, off, (mx, my)).map(Some)
    }

    fn town(&self, ctx: &OverlayCtx<'_>) -> Result<Option<OverlayLayer>, BlocksError> {
        let plan = self.plan(ctx.seed)?;
        let off = self.offset(&plan);
        layers::town(ctx, &plan, off).map(Some)
    }
}
