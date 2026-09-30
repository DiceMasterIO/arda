//! Trade and craft buildings: inn (common room, kitchen, stairs), tavern,
//! smithy (forge area), bakery, brewery, mill, workshops by craft, tannery,
//! apothecary, warehouse, market hall, stall, dock, boathouse, stable, barn.

use super::{against, light_on, put, put_query, Ctx, CANDLE, FIRE};
use crate::block::frame::Canon;
use crate::function::BuildingFunction as F;
use crate::plan::grid::Side;
use arda_tactical::catalog::WallRole;

/// Furnishes a trade or craft building.
pub fn furnish(c: &mut Canon, ctx: &mut Ctx<'_>, front_x: i64) {
    match ctx.b.function {
        F::Inn => inn(c, ctx, true),
        F::Tavern => inn(c, ctx, false),
        F::Smithy => smithy(c, ctx),
        F::Bakery => bakery(c, ctx),
        F::Brewery => brewery(c),
        F::Mill => mill(c),
        F::Workshop => workshop(c, ctx),
        F::Tannery => tannery(c),
        F::Apothecary => apothecary(c, ctx),
        F::Warehouse => warehouse(c, ctx),
        F::MarketHall => market_hall(c, ctx),
        F::Stall => {
            let _ = put(c, "prop.market_stall", 0, 0, 0);
        }
        F::Dock => dock(c),
        F::Boathouse => boathouse(c),
        F::Stable => stable(c, ctx),
        F::Barn => barn(c),
        _ => {
            let _ = front_x;
            let r = [0, 0, c.w, c.d];
            let _ = against(c, r, Side::South, "prop.crate");
        }
    }
}

fn inn(c: &mut Canon, ctx: &mut Ctx<'_>, rooms: bool) {
    let (w, d) = (c.w, c.d);
    let common_d = (d * 3 / 5).max(4);
    let common = [0, 0, w, common_d];
    c.room("common_room", common);
    let kitchen_w = (w / 2).max(3);
    c.hwall(common_d, 0, w, Some(kitchen_w / 2), ctx.pkit);
    let kitchen = [0, common_d, kitchen_w, d];
    let back = [kitchen_w, common_d, w, d];
    c.vwall(kitchen_w, common_d, d, Some(common_d + 1), ctx.pkit);
    c.h(
        kitchen_w + (w - kitchen_w) / 2,
        common_d,
        WallRole::Door,
        ctx.pkit,
    );
    c.reserve(kitchen_w + (w - kitchen_w) / 2, common_d - 1);
    c.reserve(kitchen_w + (w - kitchen_w) / 2, common_d);
    c.room("kitchen", kitchen);
    c.room(if rooms { "stair_hall" } else { "store" }, back);
    // Bar along the east of the common room, casks behind it.
    let bx = w - 2;
    let _ = put(c, "prop.cask_rack", w - 1, common_d - 3, 90);
    let _ = put(c, "prop.bar_counter", bx, common_d - 3, 90);
    let i = against(c, common, Side::West, "prop.hearth");
    light_on(c, i, 25, FIRE);
    tables(c, [0, 1, bx, common_d], ctx.rng.chance(0.5));
    let i = against(c, kitchen, Side::South, "prop.oven");
    light_on(c, i, 15, FIRE);
    let _ = put(c, "prop.table", 1, common_d + 1, 90);
    for id in ["prop.barrel", "prop.sacks", "prop.shelf", "prop.barrel"] {
        let _ = against(c, kitchen, Side::West, id);
    }
    if rooms {
        let _ = against(c, back, Side::East, "prop.stairs");
        let _ = against(c, back, Side::South, "prop.chest");
        let _ = against(c, back, Side::South, "prop.bench");
    } else {
        for id in ["prop.barrel", "prop.barrel", "prop.crate", "prop.cask_rack"] {
            let _ = against(c, back, Side::South, id);
        }
    }
    c.light(
        f64::from(u8::try_from(w / 2).unwrap_or(1)),
        0.6,
        15,
        CANDLE,
        None,
    );
}

/// Fills a room with tables and seats in groups, horizontal where they
/// fit and vertical in narrow rooms, one square apart.
pub fn tables(c: &mut Canon, room: [i64; 4], benches: bool) {
    let mut y = room[1];
    while y + 2 < room[3] {
        let mut x = room[0] + 1;
        while x + 1 < room[2] {
            let across = [x, y, x + 2, y + 3];
            let along = [x, y, x + 3, y + 2];
            if c.free(across) && x + 2 <= room[2] {
                let _ = put(c, "prop.table", x, y + 1, 0);
                if benches {
                    let _ = put(c, "prop.bench", x, y, 0);
                    let _ = put(c, "prop.bench", x, y + 2, 180);
                } else {
                    let _ = put(c, "prop.stool", x, y, 0);
                    let _ = put(c, "prop.stool", x + 1, y + 2, 0);
                }
                x += 3;
            } else if c.free(along) && x + 3 <= room[2] {
                let _ = put(c, "prop.table", x + 1, y, 90);
                if benches {
                    let _ = put(c, "prop.bench", x, y, 90);
                    let _ = put(c, "prop.bench", x + 2, y, 270);
                } else {
                    let _ = put(c, "prop.stool", x, y, 0);
                    let _ = put(c, "prop.stool", x + 2, y + 1, 0);
                }
                x += 4;
            } else {
                x += 1;
            }
        }
        y += 3;
    }
}

fn smithy(c: &mut Canon, ctx: &mut Ctx<'_>) {
    let (w, d) = (c.w, c.d);
    let shop_d = if d >= 7 { d * 2 / 3 } else { d };
    let forge_area = [0, 0, w, shop_d];
    c.room("forge", forge_area);
    if shop_d < d {
        c.hwall(shop_d, 0, w, Some(w - 2), ctx.pkit);
        let back = [0, shop_d, w, d];
        c.room("living", back);
        let _ = against(c, back, Side::South, "prop.bed");
        let _ = against(c, back, Side::South, "prop.chest");
    }
    let forge = put(c, "prop.forge", 0, shop_d - 2, 90)
        .or_else(|| against(c, forge_area, Side::West, "prop.forge"));
    light_on(c, forge, 20, [255, 140, 60]);
    let _ = put(c, "prop.anvil", 2, shop_d - 2, 0);
    let _ = put(c, "prop.trough", 2, shop_d - 1, 0);
    let _ = against(c, forge_area, Side::East, "prop.weapon_rack");
    let _ = against(c, forge_area, Side::East, "prop.grindstone");
    let _ = against(c, forge_area, Side::West, "prop.woodpile");
    let _ = against(c, forge_area, Side::North, "prop.bucket");
}

fn bakery(c: &mut Canon, ctx: &mut Ctx<'_>) {
    let (w, d) = (c.w, c.d);
    let shop_d = (d * 2 / 5).max(2);
    c.hwall(shop_d, 0, w, Some(w / 2), ctx.pkit);
    let shop = [0, 0, w, shop_d];
    let bake = [0, shop_d, w, d];
    c.room("shop", shop);
    c.room("bakehouse", bake);
    let _ = against(c, shop, Side::West, "prop.shelf");
    let _ = against(c, shop, Side::East, "prop.shelf");
    for _ in 0..2 {
        let i = against(c, bake, Side::South, "prop.oven");
        light_on(c, i, 15, FIRE);
    }
    let _ = put(c, "prop.table", w / 2 - 1, shop_d + 2, 0);
    for id in ["prop.sacks", "prop.sacks", "prop.woodpile", "prop.barrel"] {
        let _ = against(c, bake, Side::West, id);
    }
}

fn brewery(c: &mut Canon) {
    let r = [0, 0, c.w, c.d];
    c.room("brewhouse", r);
    let i = against(c, r, Side::South, "prop.hearth");
    light_on(c, i, 15, FIRE);
    for _ in 0..3 {
        let _ = against(c, r, Side::West, "prop.cask_rack");
        let _ = against(c, r, Side::East, "prop.cask_rack");
    }
    for id in [
        "prop.barrel",
        "prop.barrel",
        "prop.sacks",
        "prop.trough",
        "prop.bucket",
    ] {
        let _ = against(c, r, Side::South, id);
    }
}

fn mill(c: &mut Canon) {
    let r = [0, 0, c.w, c.d];
    c.room("mill_floor", r);
    let _ = put(c, "prop.millstone", c.w / 2 - 1, c.d / 2 - 1, 0);
    for id in [
        "prop.sacks",
        "prop.sacks",
        "prop.sacks",
        "prop.barrel",
        "prop.ladder",
        "prop.crate",
    ] {
        let _ = against(c, r, Side::South, id).or_else(|| against(c, r, Side::West, id));
    }
}

fn workshop(c: &mut Canon, ctx: &mut Ctx<'_>) {
    let (w, d) = (c.w, c.d);
    let craft = ctx.b.tags.first().map_or("craft:carpentry", String::as_str);
    let shop_d = if d >= 7 { d * 3 / 5 } else { d };
    let shop = [0, 0, w, shop_d];
    c.room("workshop", shop);
    let (main, extra): (&'static str, &'static str) = match craft {
        "craft:weaving" => ("prop.loom", "prop.shelf"),
        "craft:pottery" => ("prop.workbench", "prop.oven"),
        "craft:masonry" => ("prop.workbench", "prop.grindstone"),
        "craft:jewellery" | "craft:tinkering" => ("prop.workbench", "prop.cupboard"),
        _ => ("prop.workbench", "prop.shelf"),
    };
    let _ = against(c, shop, Side::West, main);
    let _ = against(c, shop, Side::East, main);
    let _ = against(c, shop, Side::South, extra);
    let _ = put(c, "prop.stool", 1, 1, 0);
    let _ = against(c, shop, Side::East, "prop.crate");
    if shop_d < d {
        c.hwall(shop_d, 0, w, Some(1), ctx.pkit);
        let back = [0, shop_d, w, d];
        c.room("living", back);
        let _ = against(c, back, Side::South, "prop.bed");
        let i = against(c, back, Side::East, "prop.hearth");
        light_on(c, i, 15, FIRE);
    }
}

fn tannery(c: &mut Canon) {
    let r = [0, 0, c.w, c.d];
    c.room("tanning_floor", r);
    let mut y = 2;
    while y + 1 < c.d {
        let _ = put(c, "prop.trough", 1, y, 0);
        let _ = put(c, "prop.trough", c.w - 3, y, 0);
        y += 2;
    }
    for id in [
        "prop.barrel",
        "prop.bucket",
        "prop.workbench",
        "prop.barrel",
    ] {
        let _ = against(c, r, Side::South, id);
    }
}

fn apothecary(c: &mut Canon, ctx: &mut Ctx<'_>) {
    let (w, d) = (c.w, c.d);
    let shop_d = if d >= 6 { d / 2 } else { d };
    let shop = [0, 0, w, shop_d];
    c.room("shop", shop);
    let _ = against(c, shop, Side::West, "prop.shelf");
    let _ = against(c, shop, Side::East, "prop.shelf");
    let _ = put(c, "prop.table", w / 2 - 1, shop_d - 2, 0);
    let i = against(c, shop, Side::South, "prop.candle_stand");
    light_on(c, i, 10, CANDLE);
    if shop_d < d {
        c.hwall(shop_d, 0, w, Some(1), ctx.pkit);
        let back = [0, shop_d, w, d];
        c.room("stillroom", back);
        let _ = against(c, back, Side::South, "prop.bookshelf");
        let _ = against(c, back, Side::East, "prop.cupboard");
        let _ = against(c, back, Side::West, "prop.bed");
        let _ = put(c, "prop.table", w / 2 - 1, shop_d + 1, 0);
    }
}

fn warehouse(c: &mut Canon, ctx: &mut Ctx<'_>) {
    let (w, d) = (c.w, c.d);
    let hall = [0, 0, w, d];
    c.room("hall", hall);
    // A clerk's office in the back corner.
    let (ow, od) = (4.min(w - 2), 4.min(d - 2));
    c.vwall(w - ow, d - od, d, Some(d - od + 1), ctx.pkit);
    c.hwall(d - od, w - ow, w, None, ctx.pkit);
    let office = [w - ow, d - od, w, d];
    c.room("office", office);
    let _ = against(c, office, Side::South, "prop.table");
    let _ = against(c, office, Side::East, "prop.chest");
    let _ = put(c, "prop.chair", w - ow + 1, d - 3, 0);
    // Goods in two-by-two stacks with aisles between them.
    let goods = ["function:warehouse", "container"];
    let mut y = 1;
    while y + 1 < d - 1 {
        let mut x = 1;
        while x + 1 < w - 1 {
            if !(x + 1 >= w - ow - 1 && y + 1 >= d - od - 1) {
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let _ = put_query(c, &goods, x + dx, y + dy);
                }
            }
            x += 3;
        }
        y += 3;
    }
}

fn market_hall(c: &mut Canon, ctx: &mut Ctx<'_>) {
    let (w, d) = (c.w, c.d);
    // An open arcade: every other bay of the long walls is an opening.
    for x in (1..w - 1).step_by(2) {
        c.h(x, 0, WallRole::Gate, ctx.kit);
        c.h(x, d, WallRole::Gate, ctx.kit);
        c.reserve(x, 0);
        c.reserve(x, d - 1);
    }
    let r = [0, 0, w, d];
    c.room("arcade", r);
    let mut x = 2;
    while x + 2 <= w - 2 {
        let _ = put(c, "prop.table", x, d / 2 - 1, 0);
        let _ = put(c, "prop.crate", x, d / 2, 0);
        let _ = put(c, "prop.sacks", x + 1, d / 2, 0);
        x += 4;
    }
}

fn dock(c: &mut Canon) {
    for y in 0..c.d {
        for x in 0..c.w {
            let _ = put(
                c,
                "prop.dock_planks",
                x,
                y,
                if (x + y) % 2 == 0 { 0 } else { 90 },
            );
        }
    }
}

fn boathouse(c: &mut Canon) {
    let r = [0, 0, c.w, c.d];
    c.room("boat_shed", r);
    let _ = put(c, "prop.rowboat", c.w / 2, c.d / 2 - 1, 0);
    for id in ["prop.crate", "prop.sacks", "prop.ladder", "prop.barrel"] {
        let _ = against(c, r, Side::West, id).or_else(|| against(c, r, Side::East, id));
    }
}

fn stable(c: &mut Canon, ctx: &mut Ctx<'_>) {
    let (w, d) = (c.w, c.d);
    c.room("stalls", [0, 0, w, d]);
    // Stalls two squares wide along the back wall.
    let depth = 3.min(d - 2);
    let mut x = 0;
    while x + 2 <= w {
        if x > 0 {
            c.vwall(x, d - depth, d, None, ctx.pkit);
        }
        let _ = put(c, "prop.hay_bale", x, d - 1, 0);
        let _ = put(c, "prop.trough", x, d - depth, 0);
        x += 2;
    }
    let _ = put(c, "prop.bucket", 0, 0, 0);
}

fn barn(c: &mut Canon) {
    let r = [0, 0, c.w, c.d];
    c.room("barn_floor", r);
    for _ in 0..6 {
        let _ = against(c, r, Side::South, "prop.hay_bale")
            .or_else(|| against(c, r, Side::West, "prop.hay_bale"));
    }
    let _ = put(c, "prop.haycart", c.w / 2, 1, 0);
    let _ = against(c, r, Side::East, "prop.sacks");
    let _ = against(c, r, Side::East, "prop.ladder");
}
