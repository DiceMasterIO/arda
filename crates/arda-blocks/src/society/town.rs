//! The town layer and what the other layers need to know of the towns: the
//! footprint fields run up to, and the streets that replace the world's
//! roads where those enter a town.

use super::{Bbox, SQUARE_M};
use crate::overlays::{compose, OverlayCtx, OverlayLayer, Owner};
use crate::BlocksError;
use arda_people::World;
use arda_scene::RulesSidecar;
use arda_town::plan::grid::Kind;
use arda_town::TownPlan;
use std::sync::Arc;

fn err(message: String) -> BlocksError {
    BlocksError::Overlay {
        layer: "town",
        message,
    }
}

/// Settlement ids with their plans (`None` for a settlement without one).
pub type Plans = Vec<(u64, Arc<Option<TownPlan>>)>;

/// The plans of every settlement whose plan can come within `margin_m` of
/// the window, in id order.
///
/// # Errors
/// Plan failures.
pub fn plans_near(
    ctx: &OverlayCtx<'_>,
    world: &World,
    margin_m: f64,
) -> Result<Plans, BlocksError> {
    let win = Bbox::of_window(ctx);
    let mut near: Vec<u64> = world
        .files
        .settlements
        .settlements
        .iter()
        .filter(|x| {
            #[allow(clippy::cast_precision_loss)]
            let at = Bbox::of([[x.x_m as f64, x.y_m as f64]]);
            at.near(&win, arda_people::town::plan_radius_m(x.tier) + margin_m)
        })
        .map(|x| x.id.get())
        .collect();
    near.sort_unstable();
    near.into_iter()
        .map(|id| {
            world
                .plan(id)
                .map(|p| (id, p))
                .map_err(|x| err(x.to_string()))
        })
        .collect()
}

/// Whether a plan occupies a global square (every kind but open country
/// and water; the town layer's `owned`).
#[must_use]
pub fn occupies(plan: &TownPlan, gx: i64, gy: i64) -> bool {
    !matches!(
        arda_town::block::ground::kind(plan, gx, gy),
        Kind::Open | Kind::Water
    )
}

/// Where the towns near a window replace the world's roads with their own
/// streets. A plan turns every road that comes within its reach of the
/// market into a main street (`arda-town`'s arms, clipped to that reach), so
/// inside the reach a road square is the same way twice and the street
/// wins; outside it the road continues from the street's end.
#[derive(Debug, Clone, Default)]
pub struct Streets {
    /// Market and reach of each plan, squares.
    discs: Vec<((f64, f64), f64)>,
}

impl Streets {
    /// Squares inside the reach where roads are kept, so a road always
    /// meets the street it continues.
    pub const JOIN: f64 = 2.0;

    /// The reaches of `plans`.
    #[must_use]
    pub fn of(plans: &Plans) -> Self {
        let discs = plans
            .iter()
            .filter_map(|(_, p)| p.as_ref().as_ref())
            .filter_map(|plan| {
                let m = plan.focal.market;
                let reach = plan
                    .streets
                    .iter()
                    .filter(|s| s.class == arda_town::plan::StreetClass::Main)
                    .filter_map(|s| s.points.last())
                    .map(|p| (p.x - m.x).hypot(p.y - m.y))
                    .fold(0.0_f64, f64::max);
                (reach > 0.0).then(|| ((m.x / SQUARE_M, m.y / SQUARE_M), reach / SQUARE_M))
            })
            .collect();
        Self { discs }
    }

    /// Whether a road square at global `(gx, gy)` lies where a town's
    /// streets replace the roads.
    #[must_use]
    pub fn replace(&self, gx: i64, gy: i64) -> bool {
        #[allow(clippy::cast_precision_loss)] // squares are far below 2^52
        let p = (gx as f64 + 0.5, gy as f64 + 0.5);
        self.discs
            .iter()
            .any(|&(c, r)| (p.0 - c.0).hypot(p.1 - c.1) < r - Self::JOIN)
    }
}

/// Streets, plots, buildings and crofts of every settlement whose plan can
/// reach the window, in id order; a square claimed by one plan is not taken
/// by the next. Croft squares are soft claims ([`Owner::Croft`]): they give
/// way to water and to the world's roads.
///
/// # Errors
/// Plan or block failures.
pub fn town(ctx: &OverlayCtx<'_>, world: &World) -> Result<Option<OverlayLayer>, BlocksError> {
    let (w, h) = (ctx.width, ctx.height);
    let n = w as usize * h as usize;
    let mut layout = ctx.base.clone();
    let mut rules = RulesSidecar::empty(w, h);
    let mut owners = vec![Owner::Natural; n];
    let mut any = false;
    for (id, plan) in plans_near(ctx, world, 0.0)? {
        let Some(plan) = plan.as_ref() else {
            continue;
        };
        let window = arda_town::block::Window {
            x: ctx.gsx0,
            y: ctx.gsy0,
            w: i64::from(w),
            h: i64::from(h),
        };
        let blk = arda_town::block::generate(plan, window).map_err(|x| err(x.to_string()))?;
        if !blk.owned.iter().any(|&o| o) {
            continue;
        }
        let res = arda_town::block::fallback::resolve(&blk, ctx.library, plan.seed);
        let mut part = res.layout;
        part.origin = ctx.base.origin;
        // Building ids are unique within a settlement: tag the settlement
        // too, so tokens and scenes can tell two plans' buildings apart.
        let mut part_rules = blk.to_rules();
        for cell in &mut part_rules.squares {
            if cell
                .ext
                .as_ref()
                .is_some_and(|x| x.contains_key("building"))
            {
                cell.set_ext("settlement", serde_json::Value::from(id.to_string()));
            }
        }
        let soft = (0..n)
            .map(|i| {
                let (x, y) = (i % w as usize, i / w as usize);
                #[allow(clippy::cast_possible_wrap)] // window sides are small
                let (gx, gy) = (ctx.gsx0 + x as i64, ctx.gsy0 + y as i64);
                arda_town::block::ground::kind(plan, gx, gy) == Kind::Croft
            })
            .collect();
        let layer = OverlayLayer {
            layout: part,
            rules: part_rules,
            owned: blk.owned.clone(),
            soft,
            elevation: false,
        };
        any |= compose(&mut layout, &mut rules, &mut owners, &layer, Owner::Town)? > 0;
    }
    if !any {
        return Ok(None);
    }
    let owned = owners
        .iter()
        .map(|&o| matches!(o, Owner::Town | Owner::Croft))
        .collect();
    let soft = owners.iter().map(|&o| o == Owner::Croft).collect();
    Ok(Some(OverlayLayer {
        layout,
        rules,
        owned,
        soft,
        elevation: false,
    }))
}
