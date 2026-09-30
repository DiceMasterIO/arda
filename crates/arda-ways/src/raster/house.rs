//! Toll houses and waystations: floor, walls with a door facing the road,
//! windows, and furniture (desk, bench, bed, chest, hearth, stores).

use super::Grid;
use crate::plan::house::{House, Side};
use crate::plan::Plan;
use crate::sidecar::{EdgeRole, Feature};
use arda_tactical::layout::EdgeAxis;
use arda_tactical::WallRole;

/// A local frame: `u` runs along the door wall, `v` away from it.
struct Frame {
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
    side: Side,
}

impl Frame {
    /// Length of the door wall and depth, in squares.
    fn size(&self) -> (i64, i64) {
        let (w, h) = (self.x1 - self.x0 + 1, self.y1 - self.y0 + 1);
        match self.side {
            Side::North | Side::South => (w, h),
            Side::East | Side::West => (h, w),
        }
    }

    /// Global anchor of local `(u, v)` in squares.
    #[allow(clippy::cast_precision_loss)] // small grid indices
    fn point(&self, u: f64, v: f64) -> (f64, f64) {
        let (x0, y0, x1, y1) = (
            self.x0 as f64,
            self.y0 as f64,
            self.x1 as f64,
            self.y1 as f64,
        );
        match self.side {
            Side::North => (x0 + u, y0 + v),
            Side::South => (x0 + u, y1 + 1.0 - v),
            Side::West => (x0 + v, y0 + u),
            Side::East => (x1 + 1.0 - v, y0 + u),
        }
    }

    /// Rotation for a piece whose rotation 0 runs along global x, laid along `u`.
    fn along_u(&self) -> u16 {
        match self.side {
            Side::North | Side::South => 0,
            Side::East | Side::West => 90,
        }
    }

    /// Rotation for a piece whose rotation 0 runs along global y, laid along `v`.
    fn along_v(&self) -> u16 {
        self.along_u()
    }

    /// The global edge of wall square `u` on the door wall (`far` = opposite).
    fn wall_edge(&self, u: i64, far: bool) -> (i64, i64, EdgeAxis) {
        match (self.side, far) {
            (Side::North, false) | (Side::South, true) => {
                (self.x0 + u, self.y0, EdgeAxis::Horizontal)
            }
            (Side::South, false) | (Side::North, true) => {
                (self.x0 + u, self.y1 + 1, EdgeAxis::Horizontal)
            }
            (Side::West, false) | (Side::East, true) => (self.x0, self.y0 + u, EdgeAxis::Vertical),
            (Side::East, false) | (Side::West, true) => {
                (self.x1 + 1, self.y0 + u, EdgeAxis::Vertical)
            }
        }
    }
}

/// Paints every planned house.
pub fn paint(g: &mut Grid, plan: &Plan) {
    for h in &plan.houses {
        one(g, h);
    }
}

#[allow(clippy::cast_precision_loss)] // small grid indices
fn one(g: &mut Grid, h: &House) {
    let (x0, y0, x1, y1) = h.rect;
    let f = Frame {
        x0,
        y0,
        x1,
        y1,
        side: h.door_side,
    };
    let floor_ft = g
        .get((x0 + x1) / 2, (y0 + y1) / 2)
        .map_or(0, super::Cell::elevation);
    for gy in y0..=y1 {
        for gx in x0..=x1 {
            if let Some(c) = g.get_mut(gx, gy) {
                c.ground = Some(h.floor);
                c.feature = Feature::Building;
                c.elev_ft = Some(floor_ft);
                c.water_ft = Some(0);
                c.difficult = false;
                c.rank = (10, 0);
                c.class = None;
            }
        }
    }
    // Perimeter runs.
    for gx in x0..=x1 {
        g.wall(
            gx,
            y0,
            EdgeAxis::Horizontal,
            WallRole::Run,
            h.kit,
            EdgeRole::Building,
        );
        g.wall(
            gx,
            y1 + 1,
            EdgeAxis::Horizontal,
            WallRole::Run,
            h.kit,
            EdgeRole::Building,
        );
    }
    for gy in y0..=y1 {
        g.wall(
            x0,
            gy,
            EdgeAxis::Vertical,
            WallRole::Run,
            h.kit,
            EdgeRole::Building,
        );
        g.wall(
            x1 + 1,
            gy,
            EdgeAxis::Vertical,
            WallRole::Run,
            h.kit,
            EdgeRole::Building,
        );
    }
    let (u_len, v_len) = f.size();
    let door_u = u_len / 2;
    // Later pushes win when the window emits (keyed by edge).
    let (dx, dy, da) = f.wall_edge(door_u, false);
    g.wall(dx, dy, da, WallRole::Door, h.kit, EdgeRole::Building);
    for u in [1, u_len - 2] {
        let (wx, wy, wa) = f.wall_edge(u, true);
        g.wall(wx, wy, wa, WallRole::Window, h.kit, EdgeRole::Building);
    }
    let (wx, wy, wa) = f.wall_edge(0, false);
    if door_u > 1 {
        g.wall(wx, wy, wa, WallRole::Window, h.kit, EdgeRole::Building);
    }
    let (u, v) = (u_len as f64, v_len as f64);
    let du = door_u as f64 + 0.5;
    let items: [(f64, f64, &'static str, u16); 9] = [
        (du, 2.0, "prop.table", f.along_u()),
        (du, 3.0, "prop.bench", f.along_u()),
        (0.5, v - 1.0, "prop.bed", f.along_v()),
        (1.5, v - 0.5, "prop.chest", f.along_u()),
        (u - 0.5, v - 0.5, "prop.hearth", 0),
        (u - 0.5, 0.5, "prop.barrel", 0),
        (u - 1.5, 0.5, "prop.crate", 0),
        (0.5, 0.5, "prop.sacks", 0),
        (u - 0.5, v - 1.5, "prop.cupboard", f.along_v()),
    ];
    for (iu, iv, id, rot) in items {
        let (x, y) = f.point(iu, iv);
        g.prop(x, y, id, rot);
    }
    // A lantern by the door outside, lit.
    let (lx, ly) = f.point(du + 1.0, -0.5);
    g.prop(lx, ly, "prop.lantern", 0);
    g.lights.push((lx, ly, 15));
}
