//! What the town layer knows of height and water (review 3.5-2, 3.5-4;
//! the rules contract, logic/12 §scene-sidecar): it states the elevation
//! of the ground it levels, and only that, against the refined terrain.
//!
//! - A walled building's floor is one level: the refined ground just
//!   outside its street door (logic/10 §town-footprints), so a house on a
//!   slope is not a flight of 5-ft steps and its threshold meets the
//!   street.
//! - The market square and the green are levelled to the ground at the
//!   market, a bailey to the ground at the keep's feature point.
//! - The curtain's wall-walk stands [`WALL_WALK_FT`] over the ground.
//! - A deck (a dock, a town bridge) states the deck and keeps the refined
//!   water and its ground beneath; a dock states its deck level, the
//!   ground at its landward end, as `deck_elevation_ft`.
//!
//! Every level is read from the refined terrain at a global square
//! ([`Heights`]), never from the window, so neighbouring windows level a
//! building that straddles them alike (goal 46).

use crate::overlays::OverlayCtx;
use crate::BlocksError;
use arda_refine::heights::Heights;
use arda_refine::Source;
use arda_scene::RulesSidecar;
use arda_tactical::TacticalLayout;
use arda_town::block::ground::{kind, WALL_WALK_FT};
use arda_town::block::TownBlock;
use arda_town::plan::grid::{Kind, Side};
use arda_town::plan::{Building, Door};
use arda_town::{BuildingFunction, TownPlan};

fn err(message: String) -> BlocksError {
    BlocksError::Overlay {
        layer: "town",
        message,
    }
}

/// The square just outside the middle of a rectangle's `side`.
const fn beyond(b: &Building, side: Side) -> (i64, i64) {
    let r = b.rect;
    let (cx, cy) = ((r.x0 + r.x1) / 2, (r.y0 + r.y1) / 2);
    match side {
        Side::North => (cx, r.y0 - 1),
        Side::South => (cx, r.y1),
        Side::West => (r.x0 - 1, cy),
        Side::East => (r.x1, cy),
    }
}

/// The square a building's level is read at: outside its street door, or
/// beyond the middle of its front (a dock's landward end).
fn anchor(b: &Building) -> (i64, i64) {
    b.doors
        .first()
        .map_or_else(|| beyond(b, b.front), Door::outside)
}

/// Sets the elevation of the squares of `part` the plan levels, keeps the
/// refined water and ground under its decks, and returns the row-major
/// mask of squares whose elevation the layer states.
///
/// # Errors
/// A world layer failed to read.
pub fn state(
    ctx: &OverlayCtx<'_>,
    src: &dyn Source,
    heights: &mut Heights,
    plan: &TownPlan,
    blk: &TownBlock,
    part: &mut TacticalLayout,
    rules: &mut RulesSidecar,
) -> Result<Vec<bool>, BlocksError> {
    let (w, h) = (i64::from(ctx.width), i64::from(ctx.height));
    let n = part.squares.len();
    let mut stated = vec![false; n];
    let mut at = |x: i64, y: i64| {
        heights
            .elevation_ft(src, x, y)
            .map_err(|e| err(e.to_string()))
    };
    let lin = |gx: i64, gy: i64| -> Option<usize> {
        let (x, y) = (gx - ctx.gsx0, gy - ctx.gsy0);
        ((0..w).contains(&x) && (0..h).contains(&y))
            .then(|| usize::try_from(y * w + x).ok())
            .flatten()
    };
    let square = arda_town::plan::square::square_of;
    let mut market: Option<i16> = None;
    let mut bailey: Option<i16> = None;
    for (i, st) in stated.iter_mut().enumerate() {
        if !blk.owned.get(i).copied().unwrap_or(false) {
            continue;
        }
        let (Some(sq), Some(base)) = (part.squares.get_mut(i), ctx.base.squares.get(i)) else {
            continue;
        };
        #[allow(clippy::cast_possible_wrap)] // window sides are small
        let (gx, gy) = (ctx.gsx0 + i as i64 % w, ctx.gsy0 + i as i64 / w);
        if blk.rules.get(i).is_some_and(|r| r.deck) {
            // The deck stands over the refined water: its ground and depth
            // are the base's.
            sq.ground.clone_from(&base.ground);
            sq.water_depth_ft = 0;
            continue;
        }
        let level = match kind(plan, gx, gy) {
            Kind::Wall => Some(base.elevation_ft.saturating_add(WALL_WALK_FT)),
            Kind::Square | Kind::Green => {
                if market.is_none() {
                    let (x, y) = square(plan.focal.market);
                    market = Some(at(x, y)?);
                }
                market
            }
            Kind::Bailey => {
                if bailey.is_none() {
                    let (x, y) = square(plan.focal.feature);
                    bailey = Some(at(x, y)?);
                }
                bailey
            }
            _ => None,
        };
        if let Some(l) = level {
            sq.elevation_ft = l;
            *st = true;
        }
    }
    for b in &plan.buildings {
        let r = b.rect;
        let inside =
            r.x1 > ctx.gsx0 && r.x0 < ctx.gsx0 + w && r.y1 > ctx.gsy0 && r.y0 < ctx.gsy0 + h;
        if !inside {
            continue;
        }
        let dock = b.function == BuildingFunction::Dock;
        if !b.function.walled() && !dock {
            continue;
        }
        let (ax, ay) = anchor(b);
        let level = at(ax, ay)?;
        for gy in r.y0.max(ctx.gsy0)..r.y1.min(ctx.gsy0 + h) {
            for gx in r.x0.max(ctx.gsx0)..r.x1.min(ctx.gsx0 + w) {
                let Some(i) = lin(gx, gy) else { continue };
                if !blk.owned.get(i).copied().unwrap_or(false) {
                    continue;
                }
                if dock {
                    if let Some(c) = rules.squares.get_mut(i) {
                        c.set_deck_level(level);
                    }
                } else if let Some(sq) = part.squares.get_mut(i) {
                    sq.elevation_ft = level;
                    stated[i] = true;
                }
            }
        }
    }
    Ok(stated)
}
