//! Homes: town house (hall, bedchamber, store), cottage (one room),
//! farmhouse (long house with a byre) and manor (hall, solar, kitchen).

use super::{against, light_on, put, Ctx, FIRE};
use crate::block::frame::Canon;
use crate::function::BuildingFunction as F;
use crate::plan::grid::Side;
use crate::plan::WealthLevel;
use arda_tactical::catalog::WallRole;

/// Furnishes a home.
pub fn furnish(c: &mut Canon, ctx: &mut Ctx<'_>, front_x: i64) {
    match ctx.b.function {
        F::Cottage => cottage(c, ctx),
        F::Farmhouse => farmhouse(c, ctx),
        F::Manor => manor(c, ctx),
        _ => house(c, ctx, front_x),
    }
}

/// A hearth against a side wall, with its fire.
fn hearth(c: &mut Canon, room: [i64; 4], prefer_west: bool) {
    let walls = if prefer_west {
        [Side::West, Side::East]
    } else {
        [Side::East, Side::West]
    };
    for w in walls {
        let i = against(c, room, w, "prop.hearth");
        if i.is_some() {
            light_on(c, i, 20, FIRE);
            return;
        }
    }
    let i = against(c, room, Side::South, "prop.hearth");
    light_on(c, i, 20, FIRE);
}

/// A table in the middle of a room with seats on its long sides.
fn dining(c: &mut Canon, room: [i64; 4], seat: &'static str, long: bool) {
    let (cx, cy) = ((room[0] + room[2]) / 2 - 1, (room[1] + room[3]) / 2);
    if put(c, "prop.table", cx, cy, 0).is_none() {
        return;
    }
    if long {
        let _ = put(c, "prop.table", cx + 2, cy, 0);
    }
    if long {
        for k in 0..4 {
            let _ = put(c, seat, cx + k, cy - 1, 0);
            let _ = put(c, seat, cx + k, cy + 1, 180);
        }
    } else {
        let _ = put(c, seat, cx, cy - 1, 0);
        let _ = put(c, seat, cx + 1, cy + 1, 180);
    }
}

fn stairs(c: &mut Canon, room: [i64; 4]) {
    for w in [Side::East, Side::West, Side::South] {
        if against(c, room, w, "prop.stairs").is_some() {
            return;
        }
    }
}

fn house(c: &mut Canon, ctx: &mut Ctx<'_>, front_x: i64) {
    let (w, d) = (c.w, c.d);
    let split = d >= 7;
    let hall_d = if split { (d * 5 / 9).max(3) } else { d };
    let hall = [0, 0, w, hall_d];
    c.room("hall", hall);
    if split {
        let door = ctx.rng.range(1, i32::try_from(w - 2).unwrap_or(1));
        c.hwall(hall_d, 0, w, Some(i64::from(door)), ctx.pkit);
        if w >= 7 {
            let mid = w / 2;
            c.vwall(mid, hall_d, d, Some(hall_d + 1), ctx.pkit);
            // Both back rooms open from the hall.
            c.h(mid / 2, hall_d, WallRole::Door, ctx.pkit);
            c.reserve(mid / 2, hall_d);
            c.reserve(mid / 2, hall_d - 1);
            bedroom(c, [0, hall_d, mid, d]);
            store(c, [mid, hall_d, w, d]);
        } else {
            bedroom(c, [0, hall_d, w, d]);
        }
    }
    hearth(c, hall, front_x > w / 2);
    let seat = if ctx.b.wealth_level == WealthLevel::Poor {
        "prop.stool"
    } else {
        "prop.chair"
    };
    dining(c, hall, seat, false);
    let _ = against(
        c,
        hall,
        if front_x > w / 2 {
            Side::East
        } else {
            Side::West
        },
        "prop.cupboard",
    );
    if ctx.b.storeys > 1 {
        stairs(c, hall);
    }
    if ctx.b.wealth_level == WealthLevel::Wealthy {
        c.floor_rect([1, 1, w - 1, hall_d - 1], "rug");
    }
    if !split {
        let _ = against(c, hall, Side::South, "prop.bed");
        let _ = against(c, hall, Side::South, "prop.chest");
    }
}

fn bedroom(c: &mut Canon, r: [i64; 4]) {
    c.room("bedchamber", r);
    let _ = against(c, r, Side::South, "prop.bed");
    if r[2] - r[0] >= 4 {
        let _ = against(c, r, Side::South, "prop.bed");
    }
    let _ = against(c, r, Side::South, "prop.chest");
    let _ = against(c, r, Side::West, "prop.chest");
}

fn store(c: &mut Canon, r: [i64; 4]) {
    c.room("store", r);
    let _ = against(c, r, Side::South, "prop.shelf");
    for id in ["prop.barrel", "prop.sacks", "prop.barrel", "prop.crate"] {
        let _ = against(c, r, Side::East, id);
    }
}

fn cottage(c: &mut Canon, ctx: &mut Ctx<'_>) {
    let r = [0, 0, c.w, c.d];
    c.room("hall", r);
    hearth(c, r, ctx.rng.chance(0.5));
    let _ = against(c, r, Side::South, "prop.bed");
    let _ = against(c, r, Side::South, "prop.chest");
    let _ = put(c, "prop.table", c.w / 2 - 1, c.d / 2, 0);
    let _ = put(c, "prop.stool", c.w / 2 - 1, c.d / 2 - 1, 0);
    let _ = against(c, r, Side::North, "prop.shelf");
}

fn farmhouse(c: &mut Canon, ctx: &mut Ctx<'_>) {
    let (w, d) = (c.w, c.d);
    let split = (w * 11 / 20).max(4);
    let living = [0, 0, split, d];
    let byre = [split, 0, w, d];
    c.vwall(split, 0, d, Some(d / 2), ctx.pkit);
    c.room("hall", living);
    c.room("byre", byre);
    c.floor_rect(byre, "packed_earth");
    // The byre has its own wide door to the yard side of the street.
    let gx = split + (w - split) / 2;
    c.h(gx, 0, WallRole::Gate, ctx.kit);
    c.reserve(gx, 0);
    hearth(c, living, true);
    let _ = put(c, "prop.table", 1, d / 2, 90);
    let _ = put(c, "prop.bench", 2, d / 2, 90);
    let _ = against(c, living, Side::South, "prop.bed");
    let _ = against(c, living, Side::South, "prop.bed");
    let _ = against(c, living, Side::North, "prop.chest");
    for _ in 0..3 {
        let _ = against(c, byre, Side::South, "prop.hay_bale");
    }
    let _ = against(c, byre, Side::East, "prop.trough");
    let _ = against(c, byre, Side::North, "prop.bucket");
}

fn manor(c: &mut Canon, ctx: &mut Ctx<'_>) {
    let (w, d) = (c.w, c.d);
    let hall_d = (d / 2).max(4);
    let hall = [0, 0, w, hall_d];
    c.room("great_hall", hall);
    c.hwall(hall_d, 0, w, Some(w / 4), ctx.pkit);
    c.h(w * 3 / 4, hall_d, WallRole::Door, ctx.pkit);
    c.reserve(w * 3 / 4, hall_d);
    c.reserve(w * 3 / 4, hall_d - 1);
    let mid = w / 2;
    c.vwall(mid, hall_d, d, None, ctx.pkit);
    let solar = [0, hall_d, mid, d];
    let kitchen = [mid, hall_d, w, d];
    c.room("solar", solar);
    c.room("kitchen", kitchen);
    c.floor_rect([2, 1, w - 2, hall_d - 1], "rug");
    hearth(c, hall, true);
    dining(c, hall, "prop.chair", true);
    let _ = against(c, hall, Side::North, "prop.banner");
    let _ = against(c, hall, Side::East, "prop.banner");
    stairs(c, hall);
    let _ = against(c, solar, Side::South, "prop.bed");
    let _ = against(c, solar, Side::South, "prop.chest");
    let _ = against(c, solar, Side::West, "prop.bookshelf");
    let _ = put(c, "prop.rug_small", 1, hall_d + 1, 0);
    let i = against(c, kitchen, Side::South, "prop.oven");
    light_on(c, i, 15, FIRE);
    let _ = put(c, "prop.table", mid + 1, hall_d + 1, 0);
    for id in ["prop.barrel", "prop.barrel", "prop.sacks", "prop.shelf"] {
        let _ = against(c, kitchen, Side::East, id);
    }
    let _ = ctx;
}
