//! Watermills (goal 39: "mills on rivers"): a stone mill house on the bank,
//! a leat taken off the river upstream and running past the mill's river
//! wall, a wheel in the leat and a tailrace back to the river.
//!
//! Local frame: the river lies to local south (+y) and upstream is local
//! east (+x). Local square `(0, 0)` is the bank square nearest the cell
//! centre.

use super::{cell_centre, facing, CSquare, Compound, CompoundKind, Local};
use crate::geom::{h2, u01, Sq, CELL_SQUARES};
use crate::input::FieldInputs;
use crate::linear::RoadNet;
use arda_tactical::catalog::WallRole;
use arda_tactical::layout::EdgeAxis::{Horizontal as H, Vertical as V};

const SALT: u64 = 0x3111;
/// Furthest a leat or tailrace is dug looking for the river, in squares.
const DIG: i64 = 14;
/// Leat depth in feet.
const LEAT_FT: u8 = 3;

fn wet(inputs: &FieldInputs<'_>, s: Sq) -> bool {
    let m = s.centre_m();
    inputs.terrain.sample(m[0], m[1]).water_depth_m > 0.0
}

/// Bank squares of the cell, nearest the centre first, each with the axis
/// towards the water (the mean offset of wet squares within 7 squares).
fn banks(inputs: &FieldInputs<'_>, net: &RoadNet, cell: (i64, i64)) -> Vec<(Sq, [i64; 2])> {
    let (x0, y0) = (cell.0 * CELL_SQUARES, cell.1 * CELL_SQUARES);
    let c = cell_centre(cell);
    let mut out = Vec::new();
    for y in (y0 + 4)..(y0 + CELL_SQUARES - 4) {
        for x in (x0 + 4)..(x0 + CELL_SQUARES - 4) {
            let s = Sq::new(x, y);
            if wet(inputs, s) || net.near(s.centre(), 3.0) {
                continue;
            }
            let edge = [(0, 1), (1, 0), (0, -1), (-1, 0)]
                .into_iter()
                .any(|(dx, dy)| wet(inputs, s.offset(dx, dy)));
            if !edge {
                continue;
            }
            let mut v = [0.0, 0.0];
            for dy in -7..=7_i64 {
                for dx in -7..=7_i64 {
                    if wet(inputs, s.offset(dx, dy)) {
                        #[allow(clippy::cast_precision_loss)] // small offsets
                        {
                            v[0] += dx as f64;
                            v[1] += dy as f64;
                        }
                    }
                }
            }
            let dist = (x - c.x).pow(2) + (y - c.y).pow(2);
            out.push((dist, s, super::axis_towards(v)));
        }
    }
    out.sort_by_key(|(d, s, _)| (*d, *s));
    out.into_iter().map(|(_, s, d)| (s, d)).collect()
}

/// Mean water-surface height a few squares out from the bank on one side.
#[allow(clippy::cast_precision_loss)] // small counts
fn surface(inputs: &FieldInputs<'_>, b: Sq, down: [i64; 2], along: [i64; 2]) -> Option<f64> {
    let mut sum = 0.0;
    let mut n = 0.0;
    for k in 6..=16 {
        for out in 1..=4 {
            let s = b.offset(along[0] * k + down[0] * out, along[1] * k + down[1] * out);
            let m = s.centre_m();
            let t = inputs.terrain.sample(m[0], m[1]);
            if t.water_depth_m > 0.0 {
                sum += t.height_m;
                n += 1.0;
                break;
            }
        }
    }
    (n > 0.0).then(|| sum / n)
}

/// Builds the watermill of a cell, or `None` without a river bank.
#[must_use]
pub fn build(
    inputs: &FieldInputs<'_>,
    net: &RoadNet,
    seed: u64,
    cell: (i64, i64),
) -> Option<Compound> {
    let h = h2(seed, SALT, cell.0, cell.1);
    let mut chosen = None;
    for (b, down) in banks(inputs, net, cell).into_iter().take(60) {
        let east = [down[1], -down[0]];
        let west = [-east[0], -east[1]];
        let upstream_west = match (
            surface(inputs, b, down, east),
            surface(inputs, b, down, west),
        ) {
            (Some(e), Some(w)) if (e - w).abs() > 1e-3 => w > e,
            _ => u01(h) < 0.5,
        };
        // Local (0, 0) is the bank square: its centre maps to b's centre.
        let f0 = facing([0, 0], down, upstream_west);
        let c0 = f0.pt([0.5, 0.5]);
        #[allow(clippy::cast_possible_truncation)] // integral by construction
        let o = [
            b.x - (c0[0] - 0.5).round() as i64,
            b.y - (c0[1] - 0.5).round() as i64,
        ];
        let l = Local::new(facing(o, down, upstream_west));
        let dry = (-14..-3).all(|y| {
            (-10..4).all(|x| {
                let g = l.global(x, y);
                !wet(inputs, g) && !net.near(g.centre(), 1.0)
            })
        });
        if dry {
            chosen = Some(l);
            break;
        }
    }
    let mut l = chosen?;
    let wet_local = |l: &Local, x: i64, y: i64| wet(inputs, l.global(x, y));
    plan(&mut l, inputs.wealth);
    // Leat along the river wall, intake upstream, tailrace downstream.
    let water = CSquare {
        ground: "mud",
        water_ft: LEAT_FT,
        elev_ft: 0,
    };
    for y in -3..-1 {
        for x in -8..18 {
            l.square(x, y, water);
        }
    }
    for x0 in [16, -8] {
        for y in -1..DIG {
            if (x0..x0 + 2).all(|x| wet_local(&l, x, y)) {
                break;
            }
            for x in x0..x0 + 2 {
                l.square(x, y, water);
            }
        }
    }
    l.id("prop.waterwheel", -2.5, -2.0, 0);
    Some(l.finish(CompoundKind::Mill, cell, false, Some((-2, -15)), [0, -1]))
}

/// The mill house and its yard, north of the leat.
fn plan(l: &mut Local, wealth: u8) {
    let kit = if wealth >= 40 { "stone" } else { "timber" };
    l.ground(-10, -14, 4, -3, "packed_earth");
    let (x0, y0, x1, y1) = (-6, -10, 2, -3);
    l.ground(x0, y0, x1, y1, "planks");
    l.rect_walls(x0, y0, x1, y1, kit);
    l.opening(H, -2, y0, WallRole::Door, kit);
    l.opening(V, x0, -6, WallRole::Window, kit);
    l.opening(V, x1, -7, WallRole::Window, kit);
    l.id("prop.millstone", -2.0, -6.0, 0);
    l.id("prop.sacks", -5.5, -9.5, 0);
    l.id("prop.sacks", -4.5, -9.5, 90);
    l.id("prop.sacks", 0.5, -9.5, 0);
    l.id("prop.barrel", 1.5, -4.5, 0);
    l.id("prop.crate", -5.5, -4.5, 0);
    l.id("prop.bench", -4.0, -7.5, 90);
    // Yard: a cart waiting for grain and a mounting of sacks by the door.
    l.id("prop.cart", -7.5, -12.0, 0);
    l.id("prop.sacks", 0.5, -11.5, 0);
    l.id("prop.grindstone", 3.0, -8.5, 0);
}
