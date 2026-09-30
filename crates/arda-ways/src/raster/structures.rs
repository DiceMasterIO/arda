//! Bridges, fords and ferries at 5-ft scale.

use super::Grid;
use crate::input::{CrossingKind, Terrain};
use crate::plan::crossing::{CrossingPlan, LANDING_SQ};
use crate::plan::Plan;
use crate::sidecar::{EdgeRole, Feature};
use arda_tactical::layout::EdgeAxis;
use arda_tactical::WallRole;

/// Paints every crossing of the plan.
pub fn paint(g: &mut Grid, plan: &Plan, terrain: &dyn Terrain) {
    for c in &plan.crossings {
        match c.kind {
            CrossingKind::Bridge => bridge(g, c),
            CrossingKind::Ford => ford(g, c, plan, terrain),
            CrossingKind::Ferry => ferry(g, c),
        }
    }
}

/// Global anchor (in squares) of along coordinate `a` and across `r`.
fn at(c: &CrossingPlan, a: f64, r: f64) -> (f64, f64) {
    if c.east_west {
        (a, r)
    } else {
        (r, a)
    }
}

/// Rotation of a linear piece whose rotation 0 runs east–west.
fn run_rot(c: &CrossingPlan) -> u16 {
    if c.east_west {
        0
    } else {
        90
    }
}

#[allow(clippy::cast_precision_loss)] // small grid indices
fn bridge(g: &mut Grid, c: &CrossingPlan) {
    let (floor, deck_id, kit) = if c.stone {
        ("flagstone", "prop.bridge_deck_stone", "stone")
    } else {
        ("planks", "prop.bridge_deck", "timber")
    };
    for a in c.span.0..=c.span.1 {
        let mut over_water = false;
        for r in c.rows.0..=c.rows.1 {
            let (gx, gy) = c.square(a, r);
            let Some(cell) = g.get_mut(gx, gy) else {
                continue;
            };
            cell.class = Some(c.class);
            cell.rank = (9, c.class.hierarchy());
            cell.difficult = false;
            if cell.water {
                over_water = true;
                cell.feature = Feature::Bridge;
                cell.deck_ft = Some(c.deck_ft);
            } else {
                cell.feature = Feature::Abutment;
                cell.ground = Some(floor);
                cell.water_ft = Some(0);
                cell.elev_ft = Some(c.deck_ft);
            }
        }
        // Timber decks are strips two squares across (an odd width overlaps
        // the last); stone decks are laid square by square like paving.
        if over_water && c.stone {
            for r in c.rows.0..=c.rows.1 {
                let (x, y) = at(c, a as f64 + 0.5, r as f64 + 0.5);
                g.prop(x, y, deck_id, run_rot(c));
            }
        } else if over_water {
            let width = c.rows.1 - c.rows.0 + 1;
            let mut centres: Vec<f64> = (0..width / 2)
                .map(|j| (c.rows.0 + 1 + 2 * j) as f64)
                .collect();
            if width % 2 == 1 {
                centres.push(c.rows.1 as f64);
            }
            for rc in centres {
                let (x, y) = at(c, a as f64 + 0.5, rc);
                g.prop(x, y, deck_id, run_rot(c));
            }
        }
        // Parapets on both long sides: block movement, not sight.
        for r in [c.rows.0, c.rows.1 + 1] {
            let (gx, gy) = c.square(a, r);
            let axis = if c.east_west {
                EdgeAxis::Horizontal
            } else {
                EdgeAxis::Vertical
            };
            g.wall(gx, gy, axis, WallRole::Run, kit, EdgeRole::Parapet);
        }
    }
}

#[allow(clippy::cast_precision_loss)] // small grid indices
fn ford(g: &mut Grid, c: &CrossingPlan, plan: &Plan, terrain: &dyn Terrain) {
    let ch = plan.channels.get(c.channel).filter(|ch| !ch.guide);
    let mid = (c.rows.0 + c.rows.1) as f64 / 2.0;
    for a in c.water.0 - 3..=c.water.1 + 3 {
        for r in c.rows.0 - 3..=c.rows.1 + 3 {
            let (gx, gy) = c.square(a, r);
            let near_bank = match ch {
                Some(ch) => ch
                    .dist(
                        crate::plan::square_centre(gx, gy),
                        ch.width_m / 2.0 - 2.5 * crate::input::SQUARE_M,
                    )
                    .is_none(),
                // Within two squares of dry ground on the terrain's raster.
                None => (-2..=2).any(|dy| {
                    (-2..=2)
                        .any(|dx| !crate::plan::is_water(&plan.channels, terrain, gx + dx, gy + dy))
                }),
            };
            let bar = g.hash(0x6BA2, gx, gy) < 0.7;
            let Some(cell) = g.get_mut(gx, gy) else {
                continue;
            };
            let in_road = (c.rows.0..=c.rows.1).contains(&r);
            if !cell.water {
                if in_road && matches!(cell.feature, Feature::Road | Feature::Ruts) {
                    cell.ground = Some("mud");
                }
                continue;
            }
            if in_road {
                let centre = (r as f64 - mid).abs() < 1.0;
                let depth = if centre { 2 } else { 1 };
                cell.water_ft = Some(cell.water_ft.unwrap_or(depth).min(depth).max(1));
                cell.ground = Some("gravel");
                cell.feature = Feature::Ford;
                cell.class = Some(c.class);
                cell.difficult = true;
            } else if (r - c.rows.0).abs().min((r - c.rows.1).abs()) <= 3 && near_bank && bar {
                cell.water_ft = Some(0);
                cell.ground = Some("gravel");
                cell.feature = Feature::GravelBar;
                cell.difficult = false;
            } else if (r == c.rows.0 - 1 || r == c.rows.1 + 1) && cell.water_ft.unwrap_or(0) > 3 {
                cell.water_ft = Some(3);
            }
        }
    }
    // Marker posts on both edges of the ford, every third square.
    for r in [c.rows.0 as f64 - 0.2, c.rows.1 as f64 + 1.2] {
        let mut posts: Vec<i64> = (c.water.0..=c.water.1).step_by(3).collect();
        if posts.last() != Some(&c.water.1) {
            posts.push(c.water.1);
        }
        for a in posts {
            let (x, y) = at(c, a as f64 + 0.5, r);
            g.prop(x, y, "prop.marker_post", 0);
        }
    }
}

#[allow(clippy::cast_precision_loss)] // small grid indices
fn ferry(g: &mut Grid, c: &CrossingPlan) {
    let len = c.water.1 - c.water.0 + 1;
    let land = LANDING_SQ.min((len - 2) / 2).max(1);
    let bank_ft = |g: &Grid, a: i64| {
        let (gx, gy) = c.square(a, c.rows.0);
        g.get(gx, gy).map_or(0, |x| x.elevation())
    };
    let (lo_ft, hi_ft) = (bank_ft(g, c.water.0 - 1), bank_ft(g, c.water.1 + 1));
    for (a0, a1, ft) in [
        (c.water.0, c.water.0 + land - 1, lo_ft),
        (c.water.1 - land + 1, c.water.1, hi_ft),
    ] {
        for a in a0..=a1 {
            for r in c.rows.0..=c.rows.1 {
                let (gx, gy) = c.square(a, r);
                let Some(cell) = g.get_mut(gx, gy) else {
                    continue;
                };
                cell.feature = Feature::Landing;
                cell.class = Some(c.class);
                cell.deck_ft = Some(ft);
                let (x, y) = at(c, a as f64 + 0.5, r as f64 + 0.5);
                g.prop(x, y, "prop.dock_planks", if c.east_west { 90 } else { 0 });
            }
        }
    }
    // Rope line one square beside the landings, posts on both banks.
    let rope_r = c.rows.1 as f64 + 1.5;
    for a in c.water.0..=c.water.1 {
        let (gx, gy) = c.square(a, c.rows.1 + 1);
        if let Some(cell) = g.get_mut(gx, gy) {
            if cell.water {
                cell.feature = Feature::FerryRope;
            }
        }
        let (x, y) = at(c, a as f64 + 0.5, rope_r);
        g.prop(x, y, "prop.ferry_rope", run_rot(c));
    }
    for a in [c.water.0 - 1, c.water.1 + 1] {
        let (x, y) = at(c, a as f64 + 0.5, rope_r);
        g.prop(x, y, "prop.marker_post", 0);
    }
    // The ferry itself, moored just off the first landing.
    let boat_a = (c.water.0 + land + 1).min(c.water.1) as f64;
    let (x, y) = at(c, boat_a + 0.5, (c.rows.0 + c.rows.1 + 1) as f64 / 2.0);
    g.prop(x, y, "prop.ferry_boat", if c.east_west { 90 } else { 0 });
}
