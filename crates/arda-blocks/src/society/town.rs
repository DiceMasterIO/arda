//! The town layer and what the other layers need to know of the towns: the
//! footprint fields run up to, and the streets that replace the world's
//! roads where those enter a town.

use super::{Bbox, SQUARE_M};
use crate::overlays::{compose_over, OverlayCtx, OverlayLayer, Owner};
use crate::BlocksError;
use arda_people::World;
use arda_scene::RulesSidecar;
use arda_town::plan::grid::Kind;
use arda_town::TownPlan;
use rayon::prelude::*;
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
        .filter(|x| reaches(x, &win, margin_m))
        .map(|x| x.id.get())
        .collect();
    near.sort_unstable();
    // Plans are independent and cached by the world: draw the missing ones
    // at once (goal 50), keeping id order.
    near.into_par_iter()
        .map(|id| {
            world
                .plan(id)
                .map(|p| (id, p))
                .map_err(|x| err(x.to_string()))
        })
        .collect()
}

/// Whether settlement `s`'s plan can come within `margin_m` of `win`: its
/// plan window grows with its people (`plan_radius_for`, OI-50), so a
/// large city's outer streets are not missed.
#[must_use]
pub fn reaches(s: &arda_settle::model::Settlement, win: &Bbox, margin_m: f64) -> bool {
    #[allow(clippy::cast_precision_loss)] // world metres
    let at = Bbox::of([[s.x_m as f64, s.y_m as f64]]);
    at.near(win, arda_people::town::plan_radius_for(s) + margin_m)
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

/// A plan's market and reach, squares, and the plan.
type Reach = ((f64, f64), f64, Arc<Option<TownPlan>>);

/// Where the towns near a window replace the world's roads with their own
/// streets. A plan turns every road that comes within its reach of the
/// market into a main street (`arda-town`'s arms, clipped to that reach),
/// but its streets may leave the road's line (onto a bank, across at the
/// narrows), so inside the reach a road square gives way only where the
/// plan lays public street ground on it; elsewhere the road stays road
/// (review 3.5-6: a road square given up off the street was neither).
#[derive(Debug, Clone, Default)]
pub struct Streets {
    /// Market and reach of each plan, squares, with the plan.
    discs: Vec<Reach>,
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
            .filter_map(|(_, p)| {
                let plan = p.as_ref().as_ref()?;
                let m = plan.focal.market;
                let reach = plan
                    .streets
                    .iter()
                    .filter(|s| s.class == arda_town::plan::StreetClass::Main)
                    .filter_map(|s| s.points.last())
                    .map(|p| (p.x - m.x).hypot(p.y - m.y))
                    .fold(0.0_f64, f64::max);
                (reach > 0.0).then(|| {
                    (
                        (m.x / SQUARE_M, m.y / SQUARE_M),
                        reach / SQUARE_M,
                        Arc::clone(p),
                    )
                })
            })
            .collect();
        Self { discs }
    }

    /// Whether global `(gx, gy)` lies inside a town's reach, where its
    /// streets take over the roads.
    #[must_use]
    pub fn within(&self, gx: i64, gy: i64) -> bool {
        self.reaching(gx, gy).next().is_some()
    }

    fn reaching(&self, gx: i64, gy: i64) -> impl Iterator<Item = &TownPlan> {
        #[allow(clippy::cast_precision_loss)] // squares are far below 2^52
        let p = (gx as f64 + 0.5, gy as f64 + 0.5);
        self.discs
            .iter()
            .filter(move |&&(c, r, _)| (p.0 - c.0).hypot(p.1 - c.1) < r - Self::JOIN)
            .filter_map(|(_, _, plan)| plan.as_ref().as_ref())
    }

    /// Whether a road square at global `(gx, gy)` gives way to a town's
    /// street: inside the town's reach, where the plan lays public street
    /// ground (a street, the square, a bridge, a gate or a green).
    #[must_use]
    pub fn replace(&self, gx: i64, gy: i64) -> bool {
        self.reaching(gx, gy)
            .any(|plan| arda_town::block::ground::kind(plan, gx, gy).public())
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
    let mut owners: Vec<Owner> = ctx.base.squares.iter().map(Owner::of_base).collect();
    let mut review = vec![false; n];
    let mut elevated = vec![false; n];
    let mut heights = arda_refine::heights::Heights::new();
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
        let elevation = super::levels::state(
            ctx,
            &world.src,
            &mut heights,
            plan,
            &blk,
            &mut part,
            &mut part_rules,
        )?;
        mark_relaxed(ctx, plan, &blk.relaxed, &mut review);
        let layer = OverlayLayer {
            layout: part,
            rules: part_rules,
            owned: blk.owned.clone(),
            soft,
            elevation,
            review: Vec::new(),
        };
        any |= compose_over(
            &mut layout,
            &mut rules,
            &mut owners,
            &layer,
            Owner::Town,
            Some(ctx.base),
        )? > 0;
        for (e, (&up, &o)) in elevated.iter_mut().zip(layer.elevation.iter().zip(&owners)) {
            *e |= up && o == Owner::Town;
        }
    }
    if !any {
        return Ok(None);
    }
    let owned = owners
        .iter()
        .map(|&o| matches!(o, Owner::Town | Owner::Croft))
        .collect();
    let soft = owners.iter().map(|&o| o == Owner::Croft).collect();
    let review = review
        .iter()
        .zip(&owners)
        .map(|(&r, &o)| r && matches!(o, Owner::Town | Owner::Croft))
        .collect();
    Ok(Some(OverlayLayer {
        layout,
        rules,
        owned,
        soft,
        elevation: elevated,
        review,
    }))
}

/// Marks the window squares of a plan's relaxed WFC problems: a relaxed
/// interior's footprint, a relaxed outdoor chunk's squares (goal 47).
fn mark_relaxed(
    ctx: &OverlayCtx<'_>,
    plan: &TownPlan,
    relaxed: &[arda_town::block::Relaxed],
    review: &mut [bool],
) {
    use arda_town::block::wfc::outdoor::CHUNK;
    use arda_town::block::Relaxed;
    let (w, h) = (i64::from(ctx.width), i64::from(ctx.height));
    for r in relaxed {
        let [x0, y0, x1, y1] = match *r {
            Relaxed::Interior { building } => {
                let Some(b) = plan.buildings.iter().find(|b| b.id.0 == building) else {
                    continue;
                };
                [b.rect.x0, b.rect.y0, b.rect.x1, b.rect.y1]
            }
            Relaxed::Outdoor { cx, cy } => {
                [cx * CHUNK, cy * CHUNK, (cx + 1) * CHUNK, (cy + 1) * CHUNK]
            }
        };
        for gy in y0.max(ctx.gsy0)..y1.min(ctx.gsy0 + h) {
            for gx in x0.max(ctx.gsx0)..x1.min(ctx.gsx0 + w) {
                let i = (gy - ctx.gsy0) * w + (gx - ctx.gsx0);
                if let Some(m) = usize::try_from(i).ok().and_then(|i| review.get_mut(i)) {
                    *m = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_settle::model::{Settlement, Tier};

    /// OI-50: a window just past a city's tier radius still sees a city
    /// of 120,000 people, whose plan window is wider.
    #[test]
    fn a_large_city_reaches_windows_past_its_tier_radius() {
        let r = arda_people::town::plan_radius_m(Tier::City);
        let x = r + 200.0;
        let win = Bbox {
            x0: x,
            y0: -10.0,
            x1: x + 100.0,
            y1: 10.0,
        };
        assert!(!reaches(
            &Settlement::bare(1, 0, 0, Tier::City, 8_000),
            &win,
            0.0
        ));
        assert!(reaches(
            &Settlement::bare(2, 0, 0, Tier::City, 120_000),
            &win,
            0.0
        ));
    }
}
