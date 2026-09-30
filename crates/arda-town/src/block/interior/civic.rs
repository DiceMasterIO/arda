//! Faith, learning and military buildings: temple (nave, chancel, vestry),
//! shrine, library, school, keep (great hall, armoury, chamber), barracks
//! and guardhouse.

use super::{against, light_on, put, Ctx, CANDLE, FIRE};
use crate::block::frame::Canon;
use crate::function::BuildingFunction as F;
use crate::plan::grid::Side;

/// Furnishes a civic building.
pub fn furnish(c: &mut Canon, ctx: &mut Ctx<'_>, front_x: i64) {
    match ctx.b.function {
        F::Temple => temple(c, ctx, front_x, true),
        F::Shrine => temple(c, ctx, front_x, false),
        F::Library => library(c),
        F::School => school(c),
        F::Keep => keep(c, ctx),
        F::Barracks => barracks(c),
        _ => guardhouse(c, ctx),
    }
}

fn temple(c: &mut Canon, ctx: &mut Ctx<'_>, front_x: i64, vestry: bool) {
    let (w, d) = (c.w, c.d);
    let aisle = front_x.clamp(1, w - 2);
    // Nave from the door to the chancel; the aisle stays clear.
    c.reserve_rect([aisle, 0, aisle + 1, d - 3]);
    let chancel_y = d - 4;
    c.room("nave", [0, 0, w, chancel_y]);
    c.room("chancel", [0, chancel_y, w, d]);
    let mut y = 2;
    while y < chancel_y - 1 {
        let mut x = 1;
        while x + 2 <= aisle {
            let _ = put(c, "prop.pew", x, y, 0);
            x += 2;
        }
        let mut x = aisle + 1;
        while x + 2 < w {
            let _ = put(c, "prop.pew", x, y, 0);
            x += 2;
        }
        y += 2;
    }
    let alt =
        put(c, "prop.altar", aisle, d - 3, 0).or_else(|| put(c, "prop.altar", aisle - 1, d - 3, 0));
    let _ = alt;
    let _ = put(c, "prop.statue", aisle, d - 1, 0);
    for x in [aisle - 2, aisle + 2] {
        let i = put(c, "prop.candle_stand", x, d - 3, 0);
        light_on(c, i, 10, CANDLE);
    }
    if vestry && w >= 10 && d >= 12 {
        let vx = w - 4;
        c.vwall(vx, chancel_y, d, Some(chancel_y + 1), ctx.pkit);
        let v = [vx, chancel_y, w, d];
        c.room("vestry", v);
        let _ = against(c, v, Side::East, "prop.bookshelf");
        let _ = against(c, v, Side::South, "prop.chest");
    }
    let _ = against(c, [0, 0, w, chancel_y], Side::West, "prop.candle_stand");
}

fn library(c: &mut Canon) {
    let r = [0, 0, c.w, c.d];
    c.room("reading_room", r);
    for side in [Side::West, Side::East, Side::South] {
        for _ in 0..4 {
            let _ = against(c, r, side, "prop.bookshelf");
        }
    }
    let mut y = 3;
    while y + 2 < c.d - 1 {
        let _ = put(c, "prop.table", c.w / 2 - 1, y, 0);
        let _ = put(c, "prop.chair", c.w / 2 - 1, y - 1, 0);
        let _ = put(c, "prop.chair", c.w / 2, y + 1, 180);
        let i = put(c, "prop.candle_stand", c.w / 2 + 1, y, 0);
        light_on(c, i, 10, CANDLE);
        y += 4;
    }
}

fn school(c: &mut Canon) {
    let r = [0, 0, c.w, c.d];
    c.room("schoolroom", r);
    let _ = against(c, r, Side::South, "prop.table");
    let _ = put(c, "prop.chair", c.w / 2, c.d - 3, 0);
    let _ = against(c, r, Side::East, "prop.bookshelf");
    let mut y = 2;
    while y < c.d - 4 {
        let _ = put(c, "prop.bench", 1, y, 0);
        let _ = put(c, "prop.bench", c.w - 3, y, 0);
        y += 2;
    }
}

fn keep(c: &mut Canon, ctx: &mut Ctx<'_>) {
    let (w, d) = (c.w, c.d);
    let hall_d = (d * 3 / 5).max(5);
    let hall = [0, 0, w, hall_d];
    c.room("great_hall", hall);
    c.hwall(hall_d, 0, w, Some(w / 4), ctx.pkit);
    c.h(
        w * 3 / 4,
        hall_d,
        arda_tactical::catalog::WallRole::Door,
        ctx.pkit,
    );
    c.reserve(w * 3 / 4, hall_d);
    c.reserve(w * 3 / 4, hall_d - 1);
    c.vwall(w / 2, hall_d, d, None, ctx.pkit);
    let armoury = [0, hall_d, w / 2, d];
    let chamber = [w / 2, hall_d, w, d];
    c.room("armoury", armoury);
    c.room("chamber", chamber);
    c.floor_rect([w / 2 - 1, hall_d - 3, w / 2 + 2, hall_d], "rug");
    let _ = put(c, "prop.throne", w / 2, hall_d - 2, 180);
    let _ = against(c, hall, Side::South, "prop.banner");
    let _ = against(c, hall, Side::South, "prop.banner");
    for x in [2, w - 4] {
        let mut y = 1;
        while y + 1 < hall_d - 3 {
            let _ = put(c, "prop.table", x, y, 90);
            let _ = put(c, "prop.bench", x - 1, y, 90);
            let _ = put(c, "prop.bench", x + 1, y, 90);
            y += 3;
        }
    }
    let i = against(c, hall, Side::West, "prop.hearth");
    light_on(c, i, 25, FIRE);
    for _ in 0..3 {
        let _ = against(c, armoury, Side::West, "prop.weapon_rack");
        let _ = against(c, armoury, Side::South, "prop.armour_stand");
    }
    let _ = against(c, chamber, Side::South, "prop.bed");
    let _ = against(c, chamber, Side::East, "prop.chest");
    let _ = against(c, chamber, Side::East, "prop.stairs");
    let _ = put(c, "prop.rug_small", w / 2 + 1, hall_d + 1, 0);
}

fn barracks(c: &mut Canon) {
    let (w, d) = (c.w, c.d);
    let r = [0, 0, w, d];
    c.room("dormitory", r);
    let long_x = w >= d;
    if long_x {
        let mut x = 1;
        while x + 1 < w - 1 {
            let _ = put(c, "prop.bed", x, d - 2, 0);
            let _ = put(c, "prop.chest", x, d - 3, 0);
            x += 2;
        }
    } else {
        let mut y = 1;
        while y + 1 < d - 1 {
            let _ = put(c, "prop.bed", w - 2, y, 90);
            let _ = put(c, "prop.chest", w - 3, y, 0);
            y += 2;
        }
    }
    let _ = against(c, r, Side::West, "prop.weapon_rack");
    let _ = against(c, r, Side::West, "prop.armour_stand");
    let _ = against(c, r, Side::North, "prop.table");
    let i = against(c, r, Side::West, "prop.hearth");
    light_on(c, i, 20, FIRE);
}

fn guardhouse(c: &mut Canon, ctx: &mut Ctx<'_>) {
    let (w, d) = (c.w, c.d);
    let guard_d = if d >= 6 { d - 2 } else { d };
    let r = [0, 0, w, guard_d];
    c.room("guardroom", r);
    let _ = put(c, "prop.table", w / 2 - 1, guard_d / 2, 0);
    let _ = put(c, "prop.stool", w / 2 - 1, guard_d / 2 - 1, 0);
    let _ = put(c, "prop.stool", w / 2, guard_d / 2 + 1, 0);
    let _ = against(c, r, Side::West, "prop.weapon_rack");
    let _ = against(c, r, Side::East, "prop.chest");
    let i = against(c, r, Side::East, "prop.brazier");
    light_on(c, i, 15, FIRE);
    if guard_d < d {
        c.hwall(guard_d, 0, w, Some(1), ctx.pkit);
        let cell = [0, guard_d, w, d];
        c.room("cell", cell);
        let _ = against(c, cell, Side::South, "prop.bed");
    }
}
