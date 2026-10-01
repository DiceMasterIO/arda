//! Homes (goal 64): town house, cottage, farmhouse and manor, each varied by
//! the building's wealth, size, storeys and own stream. A town house picks
//! one of several partitions (open hall, back chambers, side chamber, front
//! workroom, cross passage) and a household trade that flavours its
//! clutter, then places hearth, table, beds and storage on randomly chosen
//! walls and spots, so houses along a street read as different households.

use super::clutter::{along, anywhere_along, area, beside, corner, scatter, table, walls};
use super::stock::{household, trade, Trade, BYRE, STORE, TRADES};
use super::{light_on, put, Ctx, CANDLE, FIRE};
use crate::block::frame::Canon;
use crate::function::BuildingFunction as F;
use crate::plan::grid::Side;
use crate::plan::WealthLevel as W;
use crate::rng::Rng;
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

/// A uniform integer in `lo..=hi` (`lo` when the range is empty).
fn range(rng: &mut Rng, lo: i64, hi: i64) -> i64 {
    if hi <= lo {
        return lo;
    }
    let n = u64::try_from(hi - lo + 1).unwrap_or(1);
    lo + i64::try_from(rng.next_u64() % n).unwrap_or(0)
}

/// A uniform count in `lo..=hi`.
fn count(rng: &mut Rng, lo: usize, hi: usize) -> usize {
    lo + rng.index(hi.saturating_sub(lo) + 1)
}

/// A fire: a hearth against a random wall, or (sometimes, in poor homes)
/// an open fire pit in the room; wood or a bucket beside it.
fn fire(c: &mut Canon, rng: &mut Rng, room: [i64; 4], pit: bool) {
    let mut i = None;
    if pit && area(room) >= 12 {
        let x = range(rng, room[0] + 1, room[2] - 2);
        let y = range(rng, room[1] + 1, room[3] - 2);
        i = put(c, "prop.brazier", x, y, 0);
        light_on(c, i, 15, FIRE);
    }
    if i.is_none() {
        for wall in walls(rng) {
            i = along(c, rng, room, wall, "prop.hearth");
            if i.is_some() {
                break;
            }
        }
        light_on(c, i, 20, FIRE);
    }
    if i.is_none() {
        // No wall left for a hearth: an open fire where there is room.
        'find: for y in room[1] + 1..room[3] - 1 {
            for x in room[0] + 1..room[2] - 1 {
                i = put(c, "prop.brazier", x, y, 0);
                if i.is_some() {
                    light_on(c, i, 15, FIRE);
                    break 'find;
                }
            }
        }
    }
    let by = if rng.chance(0.7) {
        "prop.woodpile"
    } else {
        "prop.bucket"
    };
    let _ = beside(c, rng, i, by);
    if rng.chance(0.3) {
        let _ = beside(c, rng, i, "prop.stool");
    }
}

fn seat(rng: &mut Rng, w: W) -> &'static str {
    let u = rng.f64();
    match w {
        W::Poor if u < 0.7 => "prop.stool",
        W::Poor => "prop.bench",
        W::Modest if u < 0.4 => "prop.bench",
        W::Modest if u < 0.75 => "prop.chair",
        W::Modest => "prop.stool",
        W::Wealthy => "prop.chair",
    }
}

/// Beds (with chests at their sides) in a sleeping room.
fn sleep(c: &mut Canon, rng: &mut Rng, room: [i64; 4], beds: usize, w: W) {
    for _ in 0..beds {
        let i = if rng.chance(0.6) {
            corner(c, rng, room, "prop.bed")
        } else {
            anywhere_along(c, rng, room, "prop.bed")
        };
        let i = i.or_else(|| anywhere_along(c, rng, room, "prop.bed"));
        if rng.chance(0.6) {
            let _ = beside(c, rng, i, "prop.chest");
        }
    }
    if w != W::Poor && rng.chance(0.5) {
        let _ = anywhere_along(c, rng, room, "prop.cupboard");
    }
    let n = count(
        rng,
        1,
        usize::try_from(area(room) / 8).unwrap_or(1).clamp(1, 4),
    );
    scatter(c, rng, room, household(w), n);
    if w == W::Wealthy {
        if rng.chance(0.3) {
            let _ = anywhere_along(c, rng, room, "prop.rug_small");
        }
        if rng.chance(0.4) {
            let i = corner(c, rng, room, "prop.candle_stand");
            light_on(c, i, 10, CANDLE);
        }
    }
}

/// The hall's fire, placed before anything else claims its walls.
fn hall_fire(c: &mut Canon, ctx: &mut Ctx<'_>, hall: [i64; 4]) {
    let pit = ctx.b.wealth_level == W::Poor && ctx.rng.chance(0.35);
    fire(c, &mut ctx.rng, hall, pit);
}

/// The main living room (after its fire): table, storage, stairs and
/// clutter.
fn living(c: &mut Canon, ctx: &mut Ctx<'_>, hall: [i64; 4], clutter: usize) {
    let w = ctx.b.wealth_level;
    let s = seat(&mut ctx.rng, w);
    let most = usize::try_from(area(hall) / 10).unwrap_or(1).clamp(1, 4);
    let seats = count(&mut ctx.rng, 1, most);
    if ctx.rng.chance(0.9) {
        let _ = table(c, &mut ctx.rng, hall, s, seats);
    }
    let rng = &mut ctx.rng;
    if ctx.b.storeys > 1 {
        let id = if w == W::Wealthy || rng.chance(0.6) {
            "prop.stairs"
        } else {
            "prop.ladder"
        };
        let _ = anywhere_along(c, rng, hall, id);
    }
    if w != W::Poor && rng.chance(0.7) {
        let id = if rng.chance(0.5) {
            "prop.cupboard"
        } else {
            "prop.shelf"
        };
        let _ = anywhere_along(c, rng, hall, id);
    }
    if w == W::Wealthy {
        if rng.chance(0.45) {
            let _ = anywhere_along(c, rng, hall, "prop.rug_small");
        }
        if rng.chance(0.3) {
            let _ = anywhere_along(c, rng, hall, "prop.banner");
        }
    }
    scatter(c, rng, hall, household(w), clutter);
}

/// Partitions of a town house.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Partition {
    Open,
    Back,
    Side,
    Front,
    Cross,
}

pub(crate) fn partition(rng: &mut Rng, w: i64, d: i64, wealth: W) -> Partition {
    let fits = |p: Partition| match p {
        Partition::Open => true,
        Partition::Back => d >= 6,
        Partition::Side => w >= 6 && d >= 4,
        Partition::Front => d >= 7 && w >= 4,
        Partition::Cross => w >= 8 && d >= 4,
    };
    let weights: [u32; 5] = match wealth {
        W::Poor => [4, 2, 2, 1, 1],
        W::Modest => [1, 3, 3, 2, 2],
        W::Wealthy => [1, 3, 2, 2, 3],
    };
    let all = [
        Partition::Open,
        Partition::Back,
        Partition::Side,
        Partition::Front,
        Partition::Cross,
    ];
    let ok: Vec<(Partition, u32)> = all.into_iter().zip(weights).filter(|p| fits(p.0)).collect();
    let total: u32 = ok.iter().map(|p| p.1).sum();
    let mut t = u32::try_from(rng.next_u64() % u64::from(total.max(1))).unwrap_or(0);
    for (p, wt) in ok {
        if t < wt {
            return p;
        }
        t -= wt;
    }
    Partition::Open
}

/// A door in horizontal partition `y` at a random column of `x0..x1`.
fn hdoor(c: &mut Canon, rng: &mut Rng, y: i64, x0: i64, x1: i64, kit: &'static str) {
    let x = range(rng, x0, x1 - 1);
    c.h(x, y, WallRole::Door, kit);
    c.reserve(x, y);
    c.reserve(x, y - 1);
}

fn house(c: &mut Canon, ctx: &mut Ctx<'_>, front_x: i64) {
    let (w, d) = (c.w, c.d);
    let wealth = ctx.b.wealth_level;
    let most = usize::try_from(w * d / 16).unwrap_or(1).clamp(1, 3);
    let beds = count(&mut ctx.rng, 1, most);
    let job = trade(&mut ctx.rng, wealth);
    let most = usize::try_from(w * d / 6).unwrap_or(2).clamp(2, 8);
    let clutter = count(&mut ctx.rng, most / 2, most);
    let pk = ctx.pkit;
    match partition(&mut ctx.rng, w, d, wealth) {
        Partition::Open => {
            let r = [0, 0, w, d];
            c.room("hall", r);
            hall_fire(c, ctx, r);
            sleep(c, &mut ctx.rng, r, beds.min(2), wealth);
            living(c, ctx, r, clutter);
            work(c, &mut ctx.rng, r, job);
        }
        Partition::Back => {
            let hall_d = range(&mut ctx.rng, (d * 2 / 5).max(3), (d * 3 / 5).min(d - 3));
            let hall = [0, 0, w, hall_d];
            c.room("hall", hall);
            c.hwall(hall_d, 0, w, None, pk);
            let rooms = if w >= 7 && ctx.rng.chance(0.7) {
                let mid = range(&mut ctx.rng, 3, w - 3);
                c.vwall(mid, hall_d, d, None, pk);
                vec![[0, hall_d, mid, d], [mid, hall_d, w, d]]
            } else {
                vec![[0, hall_d, w, d]]
            };
            for r in &rooms {
                hdoor(c, &mut ctx.rng, hall_d, r[0] + 1, r[2] - 1, pk);
            }
            hall_fire(c, ctx, hall);
            back_rooms(c, ctx, &rooms, beds, job);
            living(c, ctx, hall, clutter);
        }
        Partition::Side => {
            let split = range(&mut ctx.rng, (w * 2 / 5).max(3), (w * 3 / 5).min(w - 3));
            // The hall keeps the street door.
            let (hall, side) = if front_x < split {
                ([0, 0, split, d], [split, 0, w, d])
            } else {
                ([split, 0, w, d], [0, 0, split, d])
            };
            let dy = range(&mut ctx.rng, 1, d - 2);
            c.vwall(split, 0, d, Some(dy), pk);
            c.room("hall", hall);
            c.room("bedchamber", side);
            hall_fire(c, ctx, hall);
            sleep(c, &mut ctx.rng, side, beds, wealth);
            living(c, ctx, hall, clutter);
            work(c, &mut ctx.rng, hall, job);
        }
        Partition::Front => {
            let front_d = range(&mut ctx.rng, 3, (d / 2).max(3));
            let front = [0, 0, w, front_d];
            let hall = [0, front_d, w, d];
            c.hwall(front_d, 0, w, None, pk);
            hdoor(c, &mut ctx.rng, front_d, 1, w - 1, pk);
            c.room("workroom", front);
            c.room("hall", hall);
            hall_fire(c, ctx, hall);
            let job = job.unwrap_or(&TRADES[ctx.rng.index(TRADES.len())]);
            work(c, &mut ctx.rng, front, Some(job));
            sleep(c, &mut ctx.rng, hall, beds.min(2), wealth);
            living(c, ctx, hall, clutter);
        }
        Partition::Cross => {
            let a = range(&mut ctx.rng, 2, (front_x - 1).max(2)).min(w - 6);
            let b = range(&mut ctx.rng, (front_x + 2).max(a + 3), w - 2).max(a + 3);
            let (da, db) = (range(&mut ctx.rng, 1, d - 2), range(&mut ctx.rng, 1, d - 2));
            c.vwall(a, 0, d, Some(da), pk);
            c.vwall(b, 0, d, Some(db), pk);
            let (left, hall, right) = ([0, 0, a, d], [a, 0, b, d], [b, 0, w, d]);
            let (bed, store) = if ctx.rng.chance(0.5) {
                (left, right)
            } else {
                (right, left)
            };
            c.room("hall", hall);
            c.room("bedchamber", bed);
            c.room("store", store);
            hall_fire(c, ctx, hall);
            sleep(c, &mut ctx.rng, bed, beds, wealth);
            stock(c, &mut ctx.rng, store);
            living(c, ctx, hall, clutter);
            work(c, &mut ctx.rng, hall, job);
        }
    }
}

/// Back rooms: a bedchamber, then a workroom, a second bedchamber or a
/// store.
fn back_rooms(
    c: &mut Canon,
    ctx: &mut Ctx<'_>,
    rooms: &[[i64; 4]],
    beds: usize,
    job: Option<&Trade>,
) {
    let wealth = ctx.b.wealth_level;
    let first = ctx.rng.index(rooms.len());
    for (k, r) in rooms.iter().enumerate() {
        if k == first {
            c.room("bedchamber", *r);
            sleep(c, &mut ctx.rng, *r, beds, wealth);
        } else if job.is_some() && ctx.rng.chance(0.5) {
            c.room("workroom", *r);
            work(c, &mut ctx.rng, *r, job);
        } else if beds > 1 && ctx.rng.chance(0.4) {
            c.room("bedchamber", *r);
            sleep(c, &mut ctx.rng, *r, beds - 1, wealth);
        } else {
            c.room("store", *r);
            stock(c, &mut ctx.rng, *r);
        }
    }
}

/// A trade's main piece and some of its clutter.
fn work(c: &mut Canon, rng: &mut Rng, room: [i64; 4], job: Option<&Trade>) {
    let Some(job) = job else { return };
    let i = anywhere_along(c, rng, room, job.main);
    if job.main == "prop.table" {
        let _ = beside(c, rng, i, "prop.stool");
    }
    let n = count(rng, 1, 3);
    scatter(c, rng, room, job.pool, n);
}

/// A store: shelves, then goods packed along the walls.
fn stock(c: &mut Canon, rng: &mut Rng, room: [i64; 4]) {
    if rng.chance(0.6) {
        let _ = anywhere_along(c, rng, room, "prop.shelf");
    }
    let most = usize::try_from(area(room) / 3).unwrap_or(2).clamp(2, 8);
    let n = count(rng, most / 2, most);
    scatter(c, rng, room, STORE, n);
}

fn cottage(c: &mut Canon, ctx: &mut Ctx<'_>) {
    let r = [0, 0, c.w, c.d];
    let wealth = ctx.b.wealth_level;
    let rng = &mut ctx.rng;
    c.room("hall", r);
    let pit = rng.chance(0.3);
    fire(c, rng, r, pit);
    let beds = count(rng, 1, 2);
    sleep(c, rng, r, beds, wealth);
    if rng.chance(0.75) {
        let s = seat(rng, W::Poor);
        let n = count(rng, 1, 2);
        let _ = table(c, rng, r, s, n);
    }
    if rng.chance(0.25) {
        let _ = anywhere_along(c, rng, r, "prop.loom");
    }
    if rng.chance(0.6) {
        let _ = anywhere_along(c, rng, r, "prop.shelf");
    }
    let n = count(rng, 1, 3);
    scatter(c, rng, r, household(W::Poor), n);
}

fn farmhouse(c: &mut Canon, ctx: &mut Ctx<'_>) {
    let (w, d) = (c.w, c.d);
    let wealth = ctx.b.wealth_level;
    let (kit, pkit) = (ctx.kit, ctx.pkit);
    let rng = &mut ctx.rng;
    let split = range(rng, (w * 9 / 20).max(3), (w * 13 / 20).max(4)).min(w - 2);
    let (living_r, byre, wall_x) = if rng.chance(0.6) {
        ([0, 0, split, d], [split, 0, w, d], split)
    } else {
        ([w - split, 0, w, d], [0, 0, w - split, d], w - split)
    };
    let dy = range(rng, 1, d - 2);
    c.vwall(wall_x, 0, d, Some(dy), pkit);
    c.room("hall", living_r);
    c.room("byre", byre);
    c.floor_rect(byre, "packed_earth");
    // The byre has its own wide door to the yard side of the street.
    let gx = range(rng, byre[0], byre[2] - 1);
    c.h(gx, 0, WallRole::Gate, kit);
    c.reserve(gx, 0);
    c.reserve(gx, 1);
    fire(c, rng, living_r, false);
    let s = if rng.chance(0.6) {
        "prop.bench"
    } else {
        "prop.stool"
    };
    let n = count(rng, 1, 3);
    let _ = table(c, rng, living_r, s, n);
    let beds = count(rng, 1, 3);
    sleep(c, rng, living_r, beds, wealth);
    let n = count(rng, 1, 3);
    scatter(c, rng, living_r, household(wealth), n);
    if rng.chance(0.8) {
        let _ = anywhere_along(c, rng, byre, "prop.trough");
    }
    let n = count(rng, 2, 6);
    scatter(c, rng, byre, BYRE, n);
    if rng.chance(0.2) {
        let _ = corner(c, rng, byre, "prop.haycart");
    }
}

fn manor(c: &mut Canon, ctx: &mut Ctx<'_>) {
    let (w, d) = (c.w, c.d);
    let pkit = ctx.pkit;
    let rng = &mut ctx.rng;
    let hall_d = range(rng, (d * 2 / 5).max(4), (d * 3 / 5).max(4)).min(d - 3);
    let hall = [0, 0, w, hall_d];
    c.room("great_hall", hall);
    c.hwall(hall_d, 0, w, None, pkit);
    let mid = range(rng, (w * 2 / 5).max(3), (w * 3 / 5).max(3)).min(w - 3);
    c.vwall(mid, hall_d, d, None, pkit);
    let (a, b) = ([0, hall_d, mid, d], [mid, hall_d, w, d]);
    let (solar, kitchen) = if rng.chance(0.5) { (a, b) } else { (b, a) };
    for r in [solar, kitchen] {
        hdoor(c, rng, hall_d, r[0] + 1, r[2] - 1, pkit);
    }
    c.room("solar", solar);
    c.room("kitchen", kitchen);
    fire(c, rng, hall, false);
    let n = count(rng, 3, 6);
    if let Some(t) = table(c, rng, hall, "prop.chair", n) {
        let _ = beside(c, rng, Some(t), "prop.candle_stand");
    }
    if rng.chance(0.3) {
        let _ = along(c, rng, hall, Side::South, "prop.throne");
    }
    for _ in 0..count(rng, 1, 3) {
        let _ = anywhere_along(c, rng, hall, "prop.banner");
    }
    for _ in 0..count(rng, 0, 2) {
        let i = corner(c, rng, hall, "prop.candle_stand");
        light_on(c, i, 10, CANDLE);
    }
    let _ = anywhere_along(c, rng, hall, "prop.stairs");
    for _ in 0..count(rng, 1, 3) {
        let _ = anywhere_along(c, rng, hall, "prop.rug_small");
    }
    let beds = count(rng, 1, 2);
    sleep(c, rng, solar, beds, W::Wealthy);
    let _ = anywhere_along(c, rng, solar, "prop.bookshelf");
    let i = anywhere_along(c, rng, kitchen, "prop.oven");
    light_on(c, i, 15, FIRE);
    let _ = anywhere_along(c, rng, kitchen, "prop.table");
    let n = count(rng, 3, 6);
    scatter(c, rng, kitchen, STORE, n);
}
