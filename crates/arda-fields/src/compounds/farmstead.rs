//! Farmsteads (`mockup-artifact.md` "Land use around settlements"): a
//! farmhouse, a barn and a yard behind a wall, gated towards the nearest road.
//!
//! The canonical plan below has its gate on the local south side; the frame
//! turns it to face the road.

use super::{axis_towards, cell_centre, facing, Compound, CompoundKind, Local};
use crate::geom::{h2, s11, u01, Sq};
use crate::input::{FieldInputs, Region};
use crate::linear::RoadNet;
use arda_tactical::catalog::{AssetClass, WallRole};
use arda_tactical::layout::EdgeAxis::{Horizontal as H, Vertical as V};

/// Yard width along the gate side, in squares.
pub const W: i64 = 26;
/// Yard depth, in squares.
pub const D: i64 = 20;
const SALT: u64 = 0xFA12;

/// Wall kits and floors chosen by wealth, region and culture.
struct Style {
    house: &'static str,
    yard: &'static str,
    floor: &'static str,
    stable: bool,
}

fn style(inputs: &FieldInputs<'_>) -> Style {
    let upland = inputs.region == Region::Upland;
    let stone = upland || inputs.wealth >= 150 || crate::boundary::stone_culture(inputs.culture);
    Style {
        house: if stone { "stone" } else { "timber" },
        yard: if upland || inputs.wealth >= 170 {
            "drystone"
        } else {
            "wattle"
        },
        floor: if inputs.wealth >= 120 {
            "planks"
        } else {
            "packed_earth"
        },
        stable: inputs.wealth >= 100,
    }
}

/// Whether a candidate yard avoids water and roads.
fn fits(inputs: &FieldInputs<'_>, net: &RoadNet, local: &Local) -> bool {
    for y in (0..D).step_by(3) {
        for x in (0..W).step_by(3) {
            let s = local.global(x, y);
            let m = s.centre_m();
            if inputs.terrain.sample(m[0], m[1]).water_depth_m > 0.0 {
                return false;
            }
            if net.near(s.centre(), 2.0) {
                return false;
            }
        }
    }
    true
}

/// Builds the farmstead of a cell, or `None` if no yard fits.
#[must_use]
pub fn build(
    inputs: &FieldInputs<'_>,
    net: &RoadNet,
    seed: u64,
    cell: (i64, i64),
) -> Option<Compound> {
    let st = style(inputs);
    for attempt in 0..16_i64 {
        let h = h2(seed, SALT + attempt.unsigned_abs(), cell.0, cell.1);
        #[allow(clippy::cast_possible_truncation)] // jitter of a few squares
        let (jx, jy) = (
            (s11(h) * 18.0).round() as i64,
            (s11(h >> 7) * 18.0).round() as i64,
        );
        let c = cell_centre(cell).offset(jx, jy);
        let down = match net.nearest(c.centre()) {
            Some((q, _)) => axis_towards([q[0] - c.centre()[0], q[1] - c.centre()[1]]),
            None => [[0, 1], [1, 0], [0, -1], [-1, 0]][crate::geom::pick4(h >> 20)],
        };
        let mirror = u01(h >> 3) < 0.5;
        // Local vertex (W/2, D/2) lands on the centre square's corner.
        let f0 = facing([0, 0], down, mirror);
        let mid = f0.pt([(W / 2) as f64, (D / 2) as f64]);
        #[allow(clippy::cast_possible_truncation)] // integral by construction
        let o = [c.x - mid[0].round() as i64, c.y - mid[1].round() as i64];
        let mut l = Local::new(facing(o, down, mirror));
        if !fits(inputs, net, &l) {
            continue;
        }
        plan(&mut l, &st, h);
        return Some(l.finish(
            CompoundKind::Farmstead,
            cell,
            true,
            Some((W / 2 - 1, D)),
            [0, 1],
        ));
    }
    None
}

/// The canonical plan: gate on the south, house and barn along the north.
fn plan(l: &mut Local, st: &Style, h: u64) {
    l.ground(0, 0, W, D, "packed_earth");
    l.rect_walls(0, 0, W, D, st.yard);
    l.opening(H, W / 2 - 1, D, WallRole::Gate, st.yard);
    l.opening(H, W / 2, D, WallRole::Gate, st.yard);
    farmhouse(l, st);
    barn(l, st);
    if st.stable {
        l.ground(2, 11, 8, 16, "packed_earth");
        l.rect_walls(2, 11, 8, 16, st.house);
        l.opening(V, 8, 13, WallRole::Door, st.house);
        l.id("prop.trough", 3.0, 12.5, 0);
        l.id("prop.hay_bale", 6.5, 14.5, 0);
        l.id("prop.hay_bale", 3.5, 14.5, 90);
    } else {
        l.ground(2, 12, 7, 17, "mud");
        l.rect_walls(2, 12, 7, 17, "wattle");
        l.opening(V, 7, 14, WallRole::Gate, "wattle");
        l.id("prop.trough", 4.0, 13.5, 0);
    }
    // Kitchen garden behind a hurdle fence.
    l.ground(19, 13, 24, 18, "farmland");
    l.rect_walls(19, 13, 24, 18, "wattle");
    l.opening(H, 21, 13, WallRole::Gate, "wattle");
    // Yard dressing.
    l.id("prop.well", 13.5, 12.5, 0);
    l.id("prop.trough", 16.0, 12.5, 0);
    l.id("prop.bucket", 14.5, 13.5, 0);
    l.id("prop.woodpile", 4.5, 8.5, 0);
    l.id("prop.woodpile", 5.5, 8.5, 90);
    l.id("prop.cart", 11.5, 15.0, if h & 1 == 0 { 0 } else { 180 });
    l.id("prop.wheelbarrow", 9.5, 11.5, 90);
    l.id("prop.hay_bale", 22.5, 10.5, 0);
    l.id("prop.sacks", 13.5, 9.5, 0);
    l.query(AssetClass::Prop, &["livestock:poultry"], 16.5, 16.5, 0);
    l.query(AssetClass::Prop, &["livestock:poultry"], 17.5, 15.5, 90);
}

fn farmhouse(l: &mut Local, st: &Style) {
    let (x0, y0, x1, y1) = (2, 2, 11, 8);
    l.ground(x0, y0, x1, y1, st.floor);
    l.rect_walls(x0, y0, x1, y1, st.house);
    l.opening(H, 6, y1, WallRole::Door, st.house);
    l.opening(H, 4, y0, WallRole::Window, st.house);
    l.opening(H, 8, y0, WallRole::Window, st.house);
    l.opening(H, 9, y1, WallRole::Window, st.house);
    l.opening(V, x0, 5, WallRole::Window, st.house);
    l.id("prop.hearth", 2.5, 4.5, 90);
    l.light(3.0, 4.5, 15, [255, 170, 90]);
    l.id("prop.table", 6.0, 4.5, 0);
    l.id("prop.bench", 6.0, 3.5, 0);
    l.id("prop.bench", 6.0, 5.5, 180);
    l.id("prop.bed", 10.5, 3.0, 0);
    l.id("prop.chest", 10.5, 5.5, 270);
    l.id("prop.cupboard", 3.5, 2.5, 0);
    l.id("prop.barrel", 9.5, 7.5, 0);
    l.id("prop.stool", 3.5, 6.5, 0);
}

fn barn(l: &mut Local, st: &Style) {
    let (x0, y0, x1, y1) = (14, 2, 24, 9);
    l.ground(x0, y0, x1, y1, "packed_earth");
    l.rect_walls(x0, y0, x1, y1, st.house);
    l.opening(H, 18, y1, WallRole::Gate, st.house);
    l.opening(H, 19, y1, WallRole::Gate, st.house);
    for (x, y, r) in [
        (15.5, 2.5, 0),
        (16.5, 2.5, 90),
        (15.5, 3.5, 0),
        (22.5, 2.5, 90),
        (23.5, 2.5, 0),
        (23.5, 3.5, 90),
    ] {
        l.id("prop.hay_bale", x, y, r);
    }
    l.id("prop.haycart", 19.0, 4.5, 0);
    l.id("prop.sacks", 22.5, 7.5, 0);
    l.id("prop.sacks", 23.5, 7.5, 90);
    l.id("prop.ladder", 14.5, 6.5, 0);
    l.id("prop.crate", 15.5, 7.5, 0);
}

/// The square just outside the gate, for tests and lanes.
#[must_use]
pub fn gate_outside(c: &Compound) -> Option<Sq> {
    c.lane_start
}
