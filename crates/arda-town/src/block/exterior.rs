//! Exterior dressing (goal 64): orchards and hedges in gardens, graves in
//! churchyards, reeds on banks, trees in the fields; barrels, benches,
//! carts and woodpiles by doors and in yards; the well, braziers and goods
//! around the market; boats and cargo at docks; braziers at gates.
//!
//! Each feature (a square, a building, a stall) decides its dressing from
//! its own hash and uses only cells it owns, so the result never depends on
//! the window being cut and blocks join without seams.

use super::frame::Want;
use super::interior::{size, GLight, GProp};
use crate::function::BuildingFunction as F;
use crate::plan::grid::{Kind, Side};
use crate::plan::{Building, TownPlan, WealthLevel};
use crate::rng::{hash_i, unit, Rng};

pub(super) fn kind(plan: &TownPlan, x: i64, y: i64) -> Kind {
    super::ground::kind(plan, x, y)
}

fn free_building(plan: &TownPlan, x: i64, y: i64) -> bool {
    plan.grid
        .gidx(x, y)
        .is_some_and(|k| plan.grid.building[k] == 0)
}

pub(super) fn prop(id: &'static str, x0: i64, y0: i64, rot: u16) -> GProp {
    let (w, h) = size(id);
    let (w, h) = if rot % 180 == 90 { (h, w) } else { (w, h) };
    #[allow(clippy::cast_precision_loss)]
    let e = [x0 as f64, y0 as f64, (x0 + w) as f64, (y0 + h) as f64];
    GProp {
        want: Want::Id(id),
        x: (e[0] + e[2]) * 0.5,
        y: (e[1] + e[3]) * 0.5,
        rot,
        extent: e,
    }
}

pub(super) fn all(plan: &TownPlan, x0: i64, y0: i64, w: i64, h: i64, ok: &[Kind]) -> bool {
    (y0..y0 + h)
        .all(|y| (x0..x0 + w).all(|x| ok.contains(&kind(plan, x, y)) && free_building(plan, x, y)))
}

/// Per-square dressing anchored at square `(x, y)` (its top-left cell).
fn cell(plan: &TownPlan, x: i64, y: i64, out: &mut Vec<GProp>) {
    let s = plan.seed;
    let h = hash_i(s ^ 0xD2E5, x, y);
    let pick = h % 1000;
    match kind(plan, x, y) {
        Kind::Garden => {
            if pick < 30 && all(plan, x, y, 2, 2, &[Kind::Garden]) && (x + y) % 3 == 0 {
                out.push(prop("veg.tree_fruit", x, y, 0));
            } else if (30..45).contains(&pick) && all(plan, x, y, 1, 1, &[Kind::Garden]) {
                out.push(prop("veg.bush_flowering", x, y, 0));
            }
        }
        Kind::Churchyard => {
            if x.rem_euclid(2) == 0 && y.rem_euclid(3) == 0 && pick < 600 {
                out.push(prop("prop.grave", x, y, 0));
            } else if pick > 985 && all(plan, x, y, 3, 3, &[Kind::Churchyard]) {
                out.push(prop("veg.tree_elm", x, y, 0));
            }
        }
        Kind::Bridge => {
            // Plank decking: long boards two squares long, paired by row.
            let below = kind(plan, x, y + 1) == Kind::Bridge;
            let above = kind(plan, x, y - 1) == Kind::Bridge;
            if y.rem_euclid(2) == 0 && below {
                out.push(prop("prop.bridge_deck", x, y, 0));
            } else if !(y.rem_euclid(2) == 1 && above) {
                out.push(prop("prop.dock_planks", x, y, 0));
            }
        }
        Kind::Water => {
            let bank = plan
                .grid
                .gidx(x, y)
                .is_some_and(|k| plan.grid.depth[k] == 1);
            if bank && pick < 220 {
                out.push(prop("veg.reeds", x, y, 0));
            }
        }
        Kind::Open => {
            let ok = [Kind::Open];
            if pick < 6 && all(plan, x, y, 3, 3, &ok) {
                let id = if pick < 3 {
                    "veg.tree_oak"
                } else {
                    "veg.tree_elm"
                };
                out.push(prop(id, x, y, 0));
            } else if (6..9).contains(&pick) && all(plan, x, y, 2, 2, &ok) {
                out.push(prop("veg.tree_birch", x, y, 0));
            } else if (9..16).contains(&pick) {
                out.push(prop("veg.bush", x, y, 0));
            } else if pick == 16 {
                out.push(prop("veg.stones", x, y, 0));
            }
        }
        Kind::Croft => super::yard::croft(plan, x, y, pick, out),
        Kind::Green => {
            if pick < 4 && all(plan, x, y, 3, 3, &[Kind::Green]) {
                out.push(prop("veg.tree_oak", x, y, 0));
            }
        }
        Kind::Gate => {
            // Braziers at the mouth corners of each gate passage: beside a
            // tower and facing out of the band.
            let around =
                [(1, 0), (-1, 0), (0, 1), (0, -1)].map(|(dx, dy)| kind(plan, x + dx, y + dy));
            let tower = around.contains(&Kind::Wall);
            let mouth = around
                .iter()
                .any(|&k| !matches!(k, Kind::Wall | Kind::Gate | Kind::WaterGate));
            if tower && mouth {
                out.push(prop("prop.brazier", x, y, 0));
            }
        }
        _ => {}
    }
}

/// A free outside square next to a building side, walking along it.
fn beside(plan: &TownPlan, b: &Building, side: Side, k: i64, ok: &[Kind]) -> Option<(i64, i64)> {
    beside_at(plan, b, side, k, 1, ok)
}

/// A free outside square `depth` squares out from a building side, `k`
/// squares along it, away from its doors.
pub(super) fn beside_at(
    plan: &TownPlan,
    b: &Building,
    side: Side,
    k: i64,
    depth: i64,
    ok: &[Kind],
) -> Option<(i64, i64)> {
    let r = b.rect;
    let (x, y) = match side {
        Side::North => (r.x0 + k, r.y0 - depth),
        Side::South => (r.x0 + k, r.y1 + depth - 1),
        Side::West => (r.x0 - depth, r.y0 + k),
        Side::East => (r.x1 + depth - 1, r.y0 + k),
    };
    let inside = match side {
        Side::North | Side::South => k < r.w(),
        _ => k < r.h(),
    };
    let door = b.doors.iter().any(|d| {
        let o = d.outside();
        (o.0 - x).abs() <= 1 && (o.1 - y).abs() <= 1
    });
    (inside && !door && ok.contains(&kind(plan, x, y)) && free_building(plan, x, y))
        .then_some((x, y))
}

/// Dressing owned by one building: by its front door and in its yard.
fn building(plan: &TownPlan, b: &Building, out: &mut Vec<GProp>, lights: &mut Vec<GLight>) {
    let mut rng = Rng::keyed(plan.seed, 0xE7E5, b.id.0);
    let front_ok = [Kind::Front, Kind::Street, Kind::Square];
    let yard_ok = [Kind::Yard];
    let span = match b.front {
        Side::North | Side::South => b.rect.w(),
        _ => b.rect.h(),
    };
    let drop =
        |ids: &[&'static str], side: Side, ok: &[Kind], rng: &mut Rng, out: &mut Vec<GProp>| {
            let start = i64::from(rng.range(0, i32::try_from(span.max(1) - 1).unwrap_or(0)));
            let mut placed = 0;
            for k in 0..span {
                let kk = (start + k) % span.max(1);
                if placed >= ids.len() {
                    break;
                }
                if let Some((x, y)) = beside(plan, b, side, kk, ok) {
                    out.push(prop(ids[placed], x, y, 0));
                    placed += 1;
                }
            }
        };
    let back = b.front.opposite();
    match b.function {
        F::Inn | F::Tavern => {
            drop(
                &["prop.barrel", "prop.barrel", "prop.bench"],
                b.front,
                &front_ok,
                &mut rng,
                out,
            );
            drop(
                &["prop.barrel", "prop.crate", "prop.woodpile"],
                back,
                &yard_ok,
                &mut rng,
                out,
            );
            if let Some(d) = b.doors.first() {
                let (ox, oy) = d.outside();
                #[allow(clippy::cast_precision_loss)]
                lights.push(GLight {
                    x: ox as f64 + 0.5,
                    y: oy as f64 + 0.5,
                    radius_ft: 15,
                    colour: [255, 200, 120],
                    owner: None,
                });
            }
        }
        F::Warehouse => drop(
            &["prop.crate", "prop.crate", "prop.sacks", "prop.barrel"],
            b.front,
            &front_ok,
            &mut rng,
            out,
        ),
        F::Smithy => {
            drop(
                &["prop.woodpile", "prop.barrel"],
                b.front,
                &front_ok,
                &mut rng,
                out,
            );
            drop(
                &["prop.woodpile", "prop.woodpile"],
                back,
                &yard_ok,
                &mut rng,
                out,
            );
        }
        F::Barn | F::Stable => drop(
            &["prop.hay_bale", "prop.hay_bale", "prop.wheelbarrow"],
            b.front,
            &yard_ok,
            &mut rng,
            out,
        ),
        f if super::yard::dressed(f) => super::yard::home(plan, b, &mut rng, out),
        F::Stall => stall(plan, b, &mut rng, out),
        F::Dock => dock(plan, b, &mut rng, out),
        _ => {}
    }
}

fn stall(plan: &TownPlan, b: &Building, rng: &mut Rng, out: &mut Vec<GProp>) {
    let goods = ["prop.crate", "prop.sacks", "prop.barrel"];
    let mut n = 0;
    for side in [b.front, b.front.opposite()] {
        for k in 0..2 {
            if n < 2 && rng.chance(0.7) {
                if let Some((x, y)) = beside(plan, b, side, k, &[Kind::Square, Kind::Green]) {
                    // Never block the aisle between stall rows (x or y step of 2).
                    if (x + y).rem_euclid(2) == 0 {
                        out.push(prop(goods[rng.index(3)], x, y, 0));
                        n += 1;
                    }
                }
            }
        }
    }
}

fn dock(plan: &TownPlan, b: &Building, rng: &mut Rng, out: &mut Vec<GProp>) {
    let r = b.rect;
    // Cargo on the deck and a boat moored at the far end.
    for _ in 0..3 {
        let x = r.x0 + i64::from(rng.range(0, i32::try_from(r.w() - 1).unwrap_or(0)));
        let y = r.y0 + i64::from(rng.range(0, i32::try_from(r.h() - 1).unwrap_or(0)));
        out.push(prop(
            ["prop.crate", "prop.barrel", "prop.sacks"][rng.index(3)],
            x,
            y,
            0,
        ));
    }
    let (bx, by, rot) = if r.h() >= r.w() {
        (r.x1, r.y0 + r.h() / 2 - 1, 0)
    } else {
        (r.x0 + r.w() / 2 - 1, r.y1, 90)
    };
    if kind(plan, bx, by) == Kind::Water {
        out.push(prop("prop.rowboat", bx, by, rot));
    }
    if b.wealth_level != WealthLevel::Poor && r.w() >= 3 && r.h() >= 3 {
        out.push(prop("prop.crane", r.x0, r.y0 + r.h() / 2, 0));
    }
}

/// The square's well and braziers, placed once per plan.
fn square(plan: &TownPlan, out: &mut Vec<GProp>) {
    let Some(sq) = &plan.square else { return };
    let c = crate::geom::centroid(&sq.polygon);
    let (cx, cy) = crate::plan::square::square_of(c);
    'find: for r in 0..12i64 {
        for dy in -r..=r {
            for dx in -r..=r {
                let (x, y) = (cx + dx, cy + dy);
                if all(plan, x - 1, y - 1, 3, 3, &[Kind::Square, Kind::Green]) {
                    out.push(prop("prop.well", x, y, 0));
                    break 'find;
                }
            }
        }
    }
    for b in plan
        .buildings
        .iter()
        .filter(|b| b.function == F::MarketHall)
    {
        let r = b.rect;
        for (x, y) in [
            (r.x0 - 1, r.y0 - 1),
            (r.x1, r.y0 - 1),
            (r.x0 - 1, r.y1),
            (r.x1, r.y1),
        ] {
            if all(plan, x, y, 1, 1, &[Kind::Square]) {
                out.push(prop("prop.brazier", x, y, 0));
            }
        }
    }
}

/// All exterior dressing touching the window `x0..x1 × y0..y1`.
#[must_use]
pub fn dress(plan: &TownPlan, x0: i64, y0: i64, x1: i64, y1: i64) -> (Vec<GProp>, Vec<GLight>) {
    let mut props = Vec::new();
    let mut lights = Vec::new();
    let m = 3;
    for y in y0 - m..y1 + m {
        for x in x0 - m..x1 + m {
            cell(plan, x, y, &mut props);
        }
    }
    let near = crate::plan::grid::SquareRect {
        x0: x0 - 3,
        y0: y0 - 3,
        x1: x1 + 3,
        y1: y1 + 3,
    };
    for b in plan
        .buildings
        .iter()
        .filter(|b| b.rect.grown(2).overlaps(&near))
    {
        building(plan, b, &mut props, &mut lights);
    }
    square(plan, &mut props);
    let _ = unit(0);
    let inside = |p: &GProp| {
        #[allow(clippy::cast_precision_loss)]
        let (a, b, c, d) = (x0 as f64, y0 as f64, x1 as f64, y1 as f64);
        p.extent[0] < c && p.extent[2] > a && p.extent[1] < d && p.extent[3] > b
    };
    props.retain(inside);
    (props, lights)
}

/// The plan-fixed exterior dressing the town WFC keeps (goal 44 seam with
/// `block::wfc::outdoor`): reeds on banks, bridge decking and trees in
/// open country, cargo and boats at docks, the market well and braziers,
/// and the lanterns at inn doors. Streets, squares, yards, gardens,
/// churchyards, greens, crofts and walls are left to the WFC.
#[must_use]
pub fn fixed(plan: &TownPlan, x0: i64, y0: i64, x1: i64, y1: i64) -> (Vec<GProp>, Vec<GLight>) {
    let mut props = Vec::new();
    let mut lights = Vec::new();
    let m = 3;
    for y in y0 - m..y1 + m {
        for x in x0 - m..x1 + m {
            if matches!(kind(plan, x, y), Kind::Water | Kind::Bridge | Kind::Open) {
                cell(plan, x, y, &mut props);
            }
        }
    }
    let near = crate::plan::grid::SquareRect {
        x0: x0 - 3,
        y0: y0 - 3,
        x1: x1 + 3,
        y1: y1 + 3,
    };
    for b in plan
        .buildings
        .iter()
        .filter(|b| b.rect.grown(2).overlaps(&near))
    {
        match b.function {
            F::Dock => dock(
                plan,
                b,
                &mut Rng::keyed(plan.seed, 0xE7E5, b.id.0),
                &mut props,
            ),
            F::Inn | F::Tavern => {
                let mut all = Vec::new();
                building(plan, b, &mut all, &mut lights);
            }
            _ => {}
        }
        super::yard::working(plan, b, &mut props);
    }
    square(plan, &mut props);
    let inside = |p: &GProp| {
        #[allow(clippy::cast_precision_loss)]
        let (a, b, c, d) = (x0 as f64, y0 as f64, x1 as f64, y1 as f64);
        p.extent[0] < c && p.extent[2] > a && p.extent[1] < d && p.extent[3] > b
    };
    props.retain(inside);
    (props, lights)
}

/// Squares the market well and braziers take, global.
#[must_use]
pub fn square_features(plan: &TownPlan) -> Vec<(i64, i64)> {
    let mut props = Vec::new();
    square(plan, &mut props);
    props
        .iter()
        .map(|p| {
            (
                crate::num::floor_i(p.extent[0]),
                crate::num::floor_i(p.extent[1]),
            )
        })
        .collect()
}
