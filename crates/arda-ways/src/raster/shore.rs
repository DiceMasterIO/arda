//! Ferry landing stages at the shores of open water too wide to rope
//! across ([`crate::plan::shore`]; review 3.5-9).

use super::Grid;
use crate::input::Terrain;
use crate::plan::crossing::LANDING_SQ;
use crate::plan::shore::Landing;
use crate::plan::Plan;
use crate::sidecar::Feature;

/// Paints every landing of the plan: [`LANDING_SQ`] squares of planked
/// stage from the shore into the water, level with the shore, a marker
/// post on the shore and a ferry boat moored at the stage's end.
#[allow(clippy::cast_precision_loss)] // small grid indices
pub fn paint(g: &mut Grid, plan: &Plan, terrain: &dyn Terrain) {
    for l in &plan.landings {
        let class = plan.ways.get(l.way).map(|w| w.class);
        let mid = (l.rows.0 + l.rows.1) / 2;
        let (sx, sy) = l.square(l.shore, mid);
        let shore_ft = g.get(sx, sy).map_or(0, super::Cell::elevation);
        let rot = if l.east_west { 90 } else { 0 };
        for k in 1..=LANDING_SQ {
            let a = l.shore + l.towards * k;
            for r in l.rows.0..=l.rows.1 {
                let (gx, gy) = l.square(a, r);
                let open = terrain.open_water(gx, gy) == Some(true);
                let Some(cell) = g.get_mut(gx, gy) else {
                    continue;
                };
                if !(open || cell.water) {
                    continue;
                }
                cell.feature = Feature::Landing;
                cell.class = class;
                cell.deck_ft = Some(shore_ft);
                let (x, y) = at(l, a as f64 + 0.5, r as f64 + 0.5);
                g.prop(x, y, "prop.dock_planks", rot);
            }
        }
        // A post where the stage leaves the shore, and the boat at its end.
        let edge = l.shore as f64 + 0.5 + 0.5 * l.towards as f64;
        let (x, y) = at(l, edge, l.rows.0 as f64 - 0.2);
        g.prop(x, y, "prop.marker_post", 0);
        let end = (l.shore + l.towards * (LANDING_SQ + 1)) as f64 + 0.5;
        let (x, y) = at(l, end, (l.rows.0 + l.rows.1 + 1) as f64 / 2.0);
        g.prop(x, y, "prop.ferry_boat", rot);
    }
}

/// Global anchor (in squares) of along coordinate `a` and across `r`.
const fn at(l: &Landing, a: f64, r: f64) -> (f64, f64) {
    if l.east_west {
        (a, r)
    } else {
        (r, a)
    }
}
