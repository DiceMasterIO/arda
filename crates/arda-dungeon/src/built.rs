//! Built dungeons: BSP rooms, MST corridors plus loops, doors, purposes and
//! dressing.

use crate::dress::{item, Dresser, Item, Rect, Side, SIDES};
use crate::grid::{self, edge_between, EdgeKey, Grid, Opening, CORRIDOR, FIRST_ROOM, ROCK};
use crate::rng::Rng;
use crate::{
    Door, Dungeon, DungeonError, Exit, ExitKind, Params, Purpose, Room, DUNGEON_KIT, ROCK_GROUND,
};
use arda_tactical::layout::{Square, TAG_LOCKED, TAG_SECRET};
use arda_tactical::{TacticalLayout, WallRole};
use std::collections::{BTreeMap, BTreeSet};

/// Smallest BSP leaf side.
const MIN_LEAF: u32 = 7;
/// Largest room side.
const MAX_ROOM: u32 = 12;

const ROOM_FLOOR: &str = "flagstone";
const CORRIDOR_FLOOR: &str = "stone_floor";

const STAIRS_UP: Item = item("prop.stairs", 1, 2, false);
const STAIRS_DOWN: Item = item("prop.stairs_down", 1, 2, false);
const SCONCE: Item = item("prop.torch_sconce", 1, 1, false);
const BRAZIER: Item = item("prop.brazier", 1, 1, true);
const TOMB: Item = item("prop.tomb", 1, 2, true);
const COFFIN: Item = item("prop.coffin", 1, 2, false);
const BONES: Item = item("prop.bone_pile", 1, 1, false);
const SKELETON: Item = item("prop.skeleton", 1, 2, false);
const CANDLES: Item = item("prop.candle_stand", 1, 1, true);
const BED: Item = item("prop.bed", 1, 2, false);
const CHEST: Item = item("prop.chest", 1, 1, false);
const TABLE: Item = item("prop.table", 2, 1, true);
const BENCH: Item = item("prop.bench", 2, 1, false);
const WEAPON_RACK: Item = item("prop.weapon_rack", 2, 1, true);
const ARMOUR: Item = item("prop.armour_stand", 1, 1, true);
const BARREL: Item = item("prop.barrel", 1, 1, true);
const CRATE: Item = item("prop.crate", 1, 1, true);
const SACKS: Item = item("prop.sacks", 1, 1, false);
const CASK_RACK: Item = item("prop.cask_rack", 2, 1, true);
const ALTAR: Item = item("prop.altar", 2, 1, true);
const STATUE: Item = item("prop.statue", 1, 1, true);
const CAGE: Item = item("prop.cage", 1, 1, true);
const COT: Item = item("prop.gaol_cot", 1, 2, false);
const BUCKET: Item = item("prop.bucket", 1, 1, false);
const TREASURE: Item = item("prop.chest_treasure", 1, 1, false);
const RUBBLE: Item = item("prop.rubble_pile", 1, 1, false);
const BANNER: Item = item("prop.banner", 1, 1, true);

/// An opening found along the paths: the rooms it enters, whether only
/// loop paths use it, and the squares on either side.
type Entry = (BTreeSet<usize>, bool, (u32, u32), (u32, u32));

#[derive(Debug, Clone, Copy)]
struct Leaf(Rect);

fn split(rng: &mut Rng, r: Rect, out: &mut Vec<Leaf>) {
    let can_v = r.w >= 2 * MIN_LEAF;
    let can_h = r.h >= 2 * MIN_LEAF;
    // Stop early now and then once leaves are modest, for varied sizes.
    if (!can_v && !can_h) || (r.w <= MAX_ROOM + 4 && r.h <= MAX_ROOM + 4 && rng.chance(0.2)) {
        out.push(Leaf(r));
        return;
    }
    let vertical = match (can_v, can_h) {
        (true, false) => true,
        (false, true) => false,
        _ if r.w * 4 > r.h * 5 => true,
        _ if r.h * 4 > r.w * 5 => false,
        _ => rng.chance(0.5),
    };
    if vertical {
        let at = rng.range(MIN_LEAF, r.w - MIN_LEAF);
        split(rng, Rect { w: at, ..r }, out);
        split(
            rng,
            Rect {
                x: r.x + at,
                w: r.w - at,
                ..r
            },
            out,
        );
    } else {
        let at = rng.range(MIN_LEAF, r.h - MIN_LEAF);
        split(rng, Rect { h: at, ..r }, out);
        split(
            rng,
            Rect {
                y: r.y + at,
                h: r.h - at,
                ..r
            },
            out,
        );
    }
}

/// A room inside a leaf, one square of rock kept on every side.
fn room_in(rng: &mut Rng, leaf: Rect) -> Option<Rect> {
    if leaf.w < 5 || leaf.h < 5 {
        return None;
    }
    let (mw, mh) = ((leaf.w - 2).min(MAX_ROOM), (leaf.h - 2).min(MAX_ROOM));
    let w = rng.range(3.max(mw * 3 / 5), mw);
    let h = rng.range(3.max(mh * 3 / 5), mh);
    Some(Rect {
        x: leaf.x + 1 + rng.range(0, leaf.w - 2 - w),
        y: leaf.y + 1 + rng.range(0, leaf.h - 2 - h),
        w,
        h,
    })
}

fn dist(a: (u32, u32), b: (u32, u32)) -> u32 {
    a.0.abs_diff(b.0) + a.1.abs_diff(b.1)
}

/// Prim's minimum spanning tree over room centres: `(a, b)` pairs.
fn mst(centres: &[(u32, u32)]) -> Vec<(usize, usize)> {
    let n = centres.len();
    let mut inside = vec![false; n];
    let mut out = Vec::new();
    if n == 0 {
        return out;
    }
    inside[0] = true;
    for _ in 1..n {
        let mut best: Option<(u32, usize, usize)> = None;
        for a in (0..n).filter(|&a| inside[a]) {
            for b in (0..n).filter(|&b| !inside[b]) {
                let d = dist(centres[a], centres[b]);
                if best.is_none_or(|(bd, _, _)| d < bd) {
                    best = Some((d, a, b));
                }
            }
        }
        if let Some((_, a, b)) = best {
            inside[b] = true;
            out.push((a, b));
        }
    }
    out
}

/// An L-shaped path of squares from `a` to `b`.
fn l_path(rng: &mut Rng, a: (u32, u32), b: (u32, u32)) -> Vec<(u32, u32)> {
    let mut out = vec![a];
    let (mut x, mut y) = a;
    let horizontal_first = rng.chance(0.5);
    let step_x = |x: &mut u32, out: &mut Vec<(u32, u32)>, y: u32| {
        while *x != b.0 {
            *x = if *x < b.0 { *x + 1 } else { *x - 1 };
            out.push((*x, y));
        }
    };
    let step_y = |y: &mut u32, out: &mut Vec<(u32, u32)>, x: u32| {
        while *y != b.1 {
            *y = if *y < b.1 { *y + 1 } else { *y - 1 };
            out.push((x, *y));
        }
    };
    if horizontal_first {
        step_x(&mut x, &mut out, y);
        step_y(&mut y, &mut out, x);
    } else {
        step_y(&mut y, &mut out, x);
        step_x(&mut x, &mut out, y);
    }
    out
}

/// Graph hops from `from` over the room adjacency.
fn hops(n: usize, links: &[(usize, usize)], from: usize) -> Vec<u32> {
    let mut d = vec![u32::MAX; n];
    let mut queue = std::collections::VecDeque::from([from]);
    if from < n {
        d[from] = 0;
    }
    while let Some(a) = queue.pop_front() {
        for &(p, q) in links {
            let b = if p == a {
                q
            } else if q == a {
                p
            } else {
                continue;
            };
            if d[b] == u32::MAX {
                d[b] = d[a] + 1;
                queue.push_back(b);
            }
        }
    }
    d
}

/// Prison cells along one wall: returns the cell zones' rectangles and
/// gate edges, and the room rectangle left for dressing.
struct Cells {
    cells: Vec<(Rect, EdgeKey, (u32, u32))>,
    rest: Rect,
}

fn cells(room: Rect, touched: &BTreeSet<(u32, u32)>) -> Option<Cells> {
    if room.w < 4 || room.h < 5 {
        return None;
    }
    for north in [true, false] {
        let rows = if north {
            room.y..room.y + 2
        } else {
            room.y + room.h - 2..room.y + room.h
        };
        if touched
            .iter()
            .any(|&(x, y)| room.contains(x, y) && rows.contains(&y))
        {
            continue;
        }
        let n = room.w / 2;
        let mut out = Vec::new();
        for c in 0..n {
            let cx = room.x + 2 * c;
            let cw = if c + 1 == n { room.w - 2 * c } else { 2 };
            let r = Rect {
                x: cx,
                y: rows.start,
                w: cw,
                h: 2,
            };
            let (inner, outer) = if north {
                ((cx, room.y + 1), (cx, room.y + 2))
            } else {
                ((cx, room.y + room.h - 2), (cx, room.y + room.h - 3))
            };
            out.push((r, edge_between(inner, outer), outer));
        }
        let rest = if north {
            Rect {
                y: room.y + 2,
                h: room.h - 2,
                ..room
            }
        } else {
            Rect {
                h: room.h - 2,
                ..room
            }
        };
        return Some(Cells { cells: out, rest });
    }
    None
}

#[allow(clippy::too_many_lines)]
pub(crate) fn generate(p: &Params) -> Result<Dungeon, DungeonError> {
    let mut rng = Rng::new(p.seed ^ 0xB0117);
    let (w, h) = (p.width, p.height);
    let mut leaves = Vec::new();
    split(
        &mut rng,
        Rect {
            x: 1,
            y: 1,
            w: w - 2,
            h: h - 2,
        },
        &mut leaves,
    );
    let mut rects: Vec<Rect> = leaves
        .iter()
        .filter_map(|l| room_in(&mut rng, l.0))
        .collect();
    if rects.len() < 2 {
        // A tiny map: two rooms side by side.
        let rw = ((w - 4) / 2).max(3);
        rects = vec![
            Rect {
                x: 1,
                y: 1,
                w: rw - 1,
                h: h - 2,
            },
            Rect {
                x: rw + 2,
                y: 1,
                w: w - rw - 3,
                h: h - 2,
            },
        ];
    }
    let n = rects.len();
    let mut g = Grid::new(w, h);
    for (i, r) in rects.iter().enumerate() {
        let z = FIRST_ROOM + u32::try_from(i).unwrap_or(0);
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                g.set(x, y, z);
            }
        }
    }
    let centres: Vec<(u32, u32)> = rects.iter().map(Rect::centre).collect();
    let tree = mst(&centres);
    let mut links: Vec<(usize, usize, bool)> = tree.iter().map(|&(a, b)| (a, b, false)).collect();
    let longest = tree
        .iter()
        .map(|&(a, b)| dist(centres[a], centres[b]))
        .max()
        .unwrap_or(0);
    let mut extra: Vec<(u32, usize, usize)> = (0..n)
        .flat_map(|a| (a + 1..n).map(move |b| (a, b)))
        .filter(|&(a, b)| !tree.contains(&(a, b)) && !tree.contains(&(b, a)))
        .map(|(a, b)| (dist(centres[a], centres[b]), a, b))
        .filter(|&(d, _, _)| d <= longest + longest / 3)
        .collect();
    extra.sort_unstable();
    let loops = (n / 4).max(1);
    for (_, a, b) in extra.into_iter().take(loops * 2) {
        if links.iter().filter(|l| l.2).count() < loops && rng.chance(0.6) {
            links.push((a, b, true));
        }
    }
    // Carve every path first, then read the zone changes along each.
    let paths: Vec<(Vec<(u32, u32)>, bool)> = links
        .iter()
        .map(|&(a, b, lp)| (l_path(&mut rng, centres[a], centres[b]), lp))
        .collect();
    for (path, _) in &paths {
        for &(x, y) in path {
            if g.get(i64::from(x), i64::from(y)) == ROCK {
                g.set(x, y, CORRIDOR);
            }
        }
    }
    // Opening edge -> (rooms it enters, whether only loop paths use it).
    let mut entries: BTreeMap<EdgeKey, Entry> = BTreeMap::new();
    for (path, lp) in &paths {
        for pair in path.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let (za, zb) = (
                g.get(i64::from(a.0), i64::from(a.1)),
                g.get(i64::from(b.0), i64::from(b.1)),
            );
            if za == zb {
                continue;
            }
            let e = edge_between(a, b);
            let entry = entries
                .entry(e)
                .or_insert_with(|| (BTreeSet::new(), true, a, b));
            for z in [za, zb] {
                if z >= FIRST_ROOM {
                    entry.0.insert((z - FIRST_ROOM) as usize);
                }
            }
            entry.1 &= *lp;
        }
    }
    let plain: Vec<(usize, usize)> = links.iter().map(|&(a, b, _)| (a, b)).collect();
    // Purposes: entrance near a random corner, stairs down farthest away,
    // treasure in a dead end.
    let corner = [(0, 0), (w, 0), (0, h), (w, h)][rng.index(4)];
    let entrance = (0..n)
        .min_by_key(|&i| (dist(centres[i], corner), i))
        .unwrap_or(0);
    let far = hops(n, &plain, entrance);
    let exit = (0..n)
        .filter(|&i| i != entrance)
        .max_by_key(|&i| (far[i], dist(centres[i], centres[entrance]), usize::MAX - i))
        .unwrap_or(0);
    let degree = |i: usize| plain.iter().filter(|&&(a, b)| a == i || b == i).count();
    let treasure = (0..n)
        .filter(|&i| i != entrance && i != exit && degree(i) == 1)
        .min_by_key(|&i| (rects[i].area(), i))
        .or_else(|| {
            (n >= 4).then(|| {
                (0..n)
                    .filter(|&i| i != entrance && i != exit)
                    .min_by_key(|&i| (rects[i].area(), i))
            })?
        });
    let mut cycle = vec![
        Purpose::Crypt,
        Purpose::Barracks,
        Purpose::Storage,
        Purpose::Shrine,
        Purpose::Prison,
        Purpose::Hall,
    ];
    rng.shuffle(&mut cycle);
    let mut purposes = Vec::with_capacity(n);
    let mut k = 0;
    for i in 0..n {
        let purpose = if i == entrance {
            Purpose::Entrance
        } else if Some(i) == treasure {
            Purpose::Treasure
        } else {
            k += 1;
            cycle[(k - 1) % cycle.len()]
        };
        purposes.push(purpose);
    }
    // Openings: treasure doors locked, loop doors sometimes secret, some
    // arches, some locked doors.
    let mut openings: BTreeMap<EdgeKey, Opening> = BTreeMap::new();
    let mut touched: BTreeSet<(u32, u32)> = BTreeSet::new();
    for (e, (rooms, loop_only, a, b)) in &entries {
        touched.insert(*a);
        touched.insert(*b);
        let into_treasure = rooms.iter().any(|&r| Some(r) == treasure);
        let opening = if into_treasure {
            Opening::Door(WallRole::Door, vec![TAG_LOCKED.into()])
        } else if *loop_only && rng.chance(0.4) {
            Opening::Door(WallRole::Door, vec![TAG_SECRET.into()])
        } else if rng.chance(0.2) {
            Opening::Arch
        } else if rng.chance(0.15) {
            Opening::Door(WallRole::Door, vec![TAG_LOCKED.into()])
        } else {
            Opening::Door(WallRole::Door, Vec::new())
        };
        openings.insert(*e, opening);
    }
    // An arch beside another opening would leave the neighbour's door with
    // a free end; such arches get a plain door instead.
    let crowded: Vec<EdgeKey> = openings
        .iter()
        .filter(|(_, o)| **o == Opening::Arch)
        .map(|(e, _)| *e)
        .filter(|&(axis, x, y)| {
            let (before, after) = match axis {
                arda_tactical::layout::EdgeAxis::Horizontal => {
                    ((axis, x.wrapping_sub(1), y), (axis, x + 1, y))
                }
                arda_tactical::layout::EdgeAxis::Vertical => {
                    ((axis, x, y.wrapping_sub(1)), (axis, x, y + 1))
                }
            };
            openings.contains_key(&before) || openings.contains_key(&after)
        })
        .collect();
    for e in crowded {
        openings.insert(e, Opening::Door(WallRole::Door, Vec::new()));
    }
    // Prison cells become zones of their own, each behind a locked gate.
    let mut next_zone = FIRST_ROOM + u32::try_from(n).unwrap_or(0);
    let mut dress_rect: Vec<Rect> = rects.clone();
    let mut cell_list: Vec<(Rect, u32, (u32, u32))> = Vec::new();
    for i in 0..n {
        if purposes[i] != Purpose::Prison {
            continue;
        }
        match cells(rects[i], &touched) {
            Some(c) => {
                for (r, gate, outer) in c.cells {
                    for y in r.y..r.y + r.h {
                        for x in r.x..r.x + r.w {
                            g.set(x, y, next_zone);
                        }
                    }
                    openings.insert(gate, Opening::Door(WallRole::Gate, vec![TAG_LOCKED.into()]));
                    cell_list.push((r, next_zone, outer));
                    next_zone += 1;
                }
                dress_rect[i] = c.rest;
            }
            None => purposes[i] = Purpose::Storage,
        }
    }
    // Squares.
    let mut layout = TacticalLayout::new(&format!("dungeon_{}", p.seed), w, h, ROCK_GROUND);
    for (x, y) in g.squares() {
        let z = g.get(i64::from(x), i64::from(y));
        let ground = match z {
            ROCK => continue,
            CORRIDOR => CORRIDOR_FLOOR,
            z if z >= FIRST_ROOM + u32::try_from(n).unwrap_or(0) => CORRIDOR_FLOOR,
            _ => ROOM_FLOOR,
        };
        if let Some(sq) = layout.square_mut(x, y) {
            *sq = Square {
                ground: ground.into(),
                elevation_ft: 0,
                water_depth_ft: 0,
                dryness: 0,
            };
        }
    }
    layout.walls = grid::walls(&g, &openings, &BTreeSet::new(), DUNGEON_KIT);
    // Dressing.
    let mut d = Dresser::new(w, h);
    for &(a, b) in openings
        .keys()
        .map(|e| grid::sides(*e))
        .collect::<Vec<_>>()
        .iter()
    {
        for s in [a, b] {
            if g.floor(s.0, s.1) {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                d.keep(s.0 as u32, s.1 as u32);
            }
        }
    }
    let mut exits = Vec::new();
    for (i, it, kind) in [
        (entrance, STAIRS_UP, ExitKind::StairsUp),
        (exit, STAIRS_DOWN, ExitKind::StairsDown),
    ] {
        let room = dress_rect[i];
        let z = g.get(i64::from(room.x), i64::from(room.y));
        let region = |x: u32, y: u32| g.get(i64::from(x), i64::from(y)) == z;
        let mut order = SIDES;
        rng.shuffle(&mut order);
        let mut done = false;
        'sides: for side in order {
            for _ in 0..12 {
                let Some((x0, y0, foot)) = stair_slot(&mut rng, room, side) else {
                    break;
                };
                if !region(foot.0, foot.1) || d.is_used(foot.0, foot.1) {
                    continue;
                }
                if d.place(&it, x0, y0, side.rotation(), &region, (0.0, 0.0)) {
                    d.keep(foot.0, foot.1);
                    exits.push(Exit {
                        kind,
                        x: foot.0,
                        y: foot.1,
                    });
                    done = true;
                    break 'sides;
                }
            }
        }
        if !done {
            return Err(DungeonError::Generation(format!(
                "no wall in room {i} takes the {:?}",
                kind
            )));
        }
    }
    for i in 0..n {
        let room = dress_rect[i];
        let z = FIRST_ROOM + u32::try_from(i).unwrap_or(0);
        let region = |x: u32, y: u32| g.get(i64::from(x), i64::from(y)) == z;
        dress_room(&mut rng, &mut d, purposes[i], room, &region);
        // One or two torches on the walls of every room.
        let torches = 1 + u32::from(room.area() > 30);
        for _ in 0..torches {
            sconce(&mut rng, &mut d, room, &region);
        }
    }
    for (r, z, outer) in &cell_list {
        d.keep(outer.0, outer.1);
        let region = |x: u32, y: u32| g.get(i64::from(x), i64::from(y)) == *z;
        let _ = d.on_wall(&mut rng, &COT, *r, &region, 8);
        let _ = d.free(&mut rng, &BUCKET, *r, 0, &region, 6);
    }
    // Corridors: an occasional torch, rubble or bones.
    let corridor: Vec<(u32, u32)> = g
        .squares()
        .filter(|&(x, y)| g.get(i64::from(x), i64::from(y)) == CORRIDOR)
        .collect();
    let in_corridor = |x: u32, y: u32| g.get(i64::from(x), i64::from(y)) == CORRIDOR;
    for &(x, y) in &corridor {
        let roll = rng.f();
        if roll < 0.035 {
            corridor_sconce(&mut d, &g, x, y, &in_corridor);
        } else if roll < 0.06 {
            let _ = d.place(&RUBBLE, x, y, 0, &in_corridor, (0.0, 0.0));
        } else if roll < 0.07 {
            let _ = d.place(
                &BONES,
                x,
                y,
                [0, 90, 180, 270][rng.index(4)],
                &in_corridor,
                (0.0, 0.0),
            );
        }
    }
    layout.placements = d.placements;
    // Doors for the description.
    let doors = layout
        .walls
        .iter()
        .filter(|s| matches!(s.kind, WallRole::Door | WallRole::Gate))
        .map(|s| Door {
            x: s.x,
            y: s.y,
            axis: s.axis,
            locked: s.has_tag(TAG_LOCKED),
            secret: s.has_tag(TAG_SECRET),
        })
        .collect();
    let rooms = rects
        .iter()
        .enumerate()
        .map(|(id, r)| Room {
            id,
            purpose: purposes[id],
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
        })
        .collect();
    let difficult = vec![false; w as usize * h as usize];
    Ok(Dungeon {
        params: *p,
        rules: grid::rules(&g, &difficult),
        layout,
        rooms,
        doors,
        exits,
    })
}

/// A random 1 × 2 stair slot against wall `side` of `room`: the
/// footprint's top-left corner and the square at its foot (one beyond the
/// end away from the wall), or `None` when the room is too small.
fn stair_slot(rng: &mut Rng, room: Rect, side: Side) -> Option<(u32, u32, (u32, u32))> {
    match side {
        Side::N | Side::S if room.h >= 3 => {
            let x = room.x + rng.range(0, room.w - 1);
            Some(if side == Side::N {
                (x, room.y, (x, room.y + 2))
            } else {
                (x, room.y + room.h - 2, (x, room.y + room.h - 3))
            })
        }
        Side::E | Side::W if room.w >= 3 => {
            let y = room.y + rng.range(0, room.h - 1);
            Some(if side == Side::W {
                (room.x, y, (room.x + 2, y))
            } else {
                (room.x + room.w - 2, y, (room.x + room.w - 3, y))
            })
        }
        _ => None,
    }
}

fn sconce(rng: &mut Rng, d: &mut Dresser, room: Rect, region: &dyn Fn(u32, u32) -> bool) {
    for _ in 0..10 {
        let side = SIDES[rng.index(4)];
        let (x, y) = match side {
            Side::N => (room.x + rng.range(0, room.w - 1), room.y),
            Side::S => (room.x + rng.range(0, room.w - 1), room.y + room.h - 1),
            Side::E => (room.x + room.w - 1, room.y + rng.range(0, room.h - 1)),
            Side::W => (room.x, room.y + rng.range(0, room.h - 1)),
        };
        if d.place(&SCONCE, x, y, side.rotation(), region, (0.0, 0.0)) {
            return;
        }
    }
}

/// A torch on a corridor square, turned to a neighbouring rock face.
fn corridor_sconce(d: &mut Dresser, g: &Grid, x: u32, y: u32, region: &dyn Fn(u32, u32) -> bool) {
    let (xi, yi) = (i64::from(x), i64::from(y));
    for (side, (dx, dy)) in [
        (Side::N, (0, -1)),
        (Side::E, (1, 0)),
        (Side::S, (0, 1)),
        (Side::W, (-1, 0)),
    ] {
        if g.get(xi + dx, yi + dy) == ROCK {
            let _ = d.place(&SCONCE, x, y, side.rotation(), region, (0.0, 0.0));
            return;
        }
    }
}

fn some(rng: &mut Rng, lo: u32, hi: u32) -> u32 {
    rng.range(lo, hi.max(lo))
}

fn dress_room(
    rng: &mut Rng,
    d: &mut Dresser,
    purpose: Purpose,
    room: Rect,
    region: &dyn Fn(u32, u32) -> bool,
) {
    let a = room.area();
    let wall = |rng: &mut Rng, d: &mut Dresser, it: &Item, n: u32| {
        for _ in 0..n {
            let _ = d.on_wall(rng, it, room, region, 12);
        }
    };
    match purpose {
        Purpose::Entrance => {
            let _ = d.free(rng, &BRAZIER, room, 1, region, 12);
            let k = some(rng, 0, 2);
            wall(rng, d, &CRATE, k);
            let k = some(rng, 0, 2);
            wall(rng, d, &BARREL, k);
        }
        Purpose::Crypt => {
            // Tombs in a row down the middle, coffins along the walls.
            let mut placed = 0;
            let rows = if room.h >= room.w { room.w } else { room.h };
            let want = (a / 14).clamp(1, 4);
            for _ in 0..30 {
                if placed >= want {
                    break;
                }
                let rot = if rows == room.w { 0 } else { 90 };
                let (fw, fh) = crate::dress::turned(&TOMB, rot);
                if room.w < fw + 2 || room.h < fh + 2 {
                    break;
                }
                let x0 = room.x + 1 + rng.range(0, room.w - fw - 2);
                let y0 = room.y + 1 + rng.range(0, room.h - fh - 2);
                if d.place(&TOMB, x0, y0, rot, region, (0.0, 0.0)) {
                    placed += 1;
                }
            }
            let k = some(rng, 1, (a / 10).min(4));
            wall(rng, d, &COFFIN, k);
            for _ in 0..some(rng, 1, 3) {
                let _ = d.free(rng, &BONES, room, 0, region, 8);
            }
            if rng.chance(0.6) {
                let _ = d.free(rng, &SKELETON, room, 0, region, 8);
            }
            let k = some(rng, 0, 2);
            wall(rng, d, &CANDLES, k);
        }
        Purpose::Barracks => {
            wall(rng, d, &BED, (a / 8).clamp(2, 6));
            let k = some(rng, 1, 3);
            wall(rng, d, &CHEST, k);
            wall(rng, d, &WEAPON_RACK, 1);
            let k = some(rng, 0, 2);
            wall(rng, d, &ARMOUR, k);
            if d.free(rng, &TABLE, room, 1, region, 12) {
                let _ = d.free(rng, &BENCH, room, 1, region, 12);
            }
        }
        Purpose::Storage => {
            for _ in 0..(a / 5).clamp(3, 10) {
                let it = [BARREL, CRATE, SACKS, CRATE][rng.index(4)];
                let _ = d.on_wall(rng, &it, room, region, 12);
            }
            let k = some(rng, 0, 2);
            wall(rng, d, &CASK_RACK, k);
            if rng.chance(0.5) {
                let _ = d.free(rng, &RUBBLE, room, 0, region, 6);
            }
        }
        Purpose::Shrine => {
            let side = SIDES[rng.index(4)];
            for _ in 0..12 {
                if d.at_wall(rng, &ALTAR, room, side, region) {
                    break;
                }
            }
            wall(rng, d, &CANDLES, 2);
            let k = some(rng, 1, 2);
            wall(rng, d, &STATUE, k);
            for _ in 0..some(rng, 1, 2) {
                let _ = d.free(rng, &BRAZIER, room, 1, region, 12);
            }
        }
        Purpose::Prison => {
            for _ in 0..some(rng, 1, 2) {
                let _ = d.free(rng, &CAGE, room, 0, region, 10);
            }
            wall(rng, d, &BUCKET, 1);
            wall(rng, d, &CHEST, 1);
        }
        Purpose::Treasure => {
            let k = some(rng, 2, 3);
            wall(rng, d, &TREASURE, k);
            let k = some(rng, 1, 2);
            wall(rng, d, &CHEST, k);
            let k = some(rng, 0, 1);
            wall(rng, d, &STATUE, k);
            if rng.chance(0.5) {
                let _ = d.free(rng, &SKELETON, room, 0, region, 8);
            }
        }
        Purpose::Hall => {
            if d.free(rng, &TABLE, room, 1, region, 12) {
                for _ in 0..2 {
                    let _ = d.free(rng, &BENCH, room, 1, region, 12);
                }
            }
            let _ = d.free(rng, &BRAZIER, room, 1, region, 12);
            let k = some(rng, 1, 2);
            wall(rng, d, &BANNER, k);
        }
        Purpose::Cavern => {}
    }
}
