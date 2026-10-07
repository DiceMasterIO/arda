//! Natural caves: cellular-automaton caverns, joined by tunnels, with a
//! mouth, water, loose ground and cave dressing.

use crate::dress::{item, Dresser, Item, Side};
use crate::grid::{self, flood, EdgeKey, Grid, Opening, FIRST_ROOM, N4, ROCK};
use crate::rng::Rng;
use crate::{Dungeon, DungeonError, Exit, ExitKind, Params, Purpose, Room, CAVE_KIT, ROCK_GROUND};
use arda_tactical::layout::{EdgeAxis, LightSource, Square};
use arda_tactical::noise::fbm;
use arda_tactical::TacticalLayout;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// The one cavern zone.
const CAVERN: u32 = FIRST_ROOM;

const STALAGMITE: Item = item("veg.stalagmite", 1, 1, true);
const BOULDER: Item = item("veg.boulder", 1, 1, true);
const ROCK_SMALL: Item = item("veg.rock_small", 1, 1, true);
const ROCK_LARGE: Item = item("veg.rock_large", 2, 2, true);
const STONES: Item = item("veg.stones", 1, 1, true);
const RUBBLE: Item = item("prop.rubble_pile", 1, 1, false);
const MUSHROOMS: Item = item("veg.mushroom_ring", 1, 1, false);
const BONES: Item = item("prop.bone_pile", 1, 1, false);
const SKELETON: Item = item("prop.skeleton", 1, 2, false);
const TREASURE: Item = item("prop.chest_treasure", 1, 1, false);
const STAIRS_DOWN: Item = item("prop.stairs_down", 1, 2, false);

/// Fraction of the interior seeded as rock.
const FILL: f32 = 0.46;

/// One cellular-automaton cavern; `None` when the main cavern is too
/// small to use.
fn caverns(rng: &mut Rng, w: u32, h: u32) -> Option<Grid> {
    let mut g = Grid::new(w, h);
    for (x, y) in g.squares().collect::<Vec<_>>() {
        let border = x == 0 || y == 0 || x + 1 == w || y + 1 == h;
        if !border && !rng.chance(FILL) {
            g.set(x, y, CAVERN);
        }
    }
    for _ in 0..5 {
        let mut next = g.clone();
        for (x, y) in g.squares() {
            let border = x == 0 || y == 0 || x + 1 == w || y + 1 == h;
            let mut rock = 0;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if !g.floor(i64::from(x) + dx, i64::from(y) + dy) {
                        rock += 1;
                    }
                }
            }
            // The 4-5 rule: rock where most of the 3 x 3 block is rock.
            next.set(x, y, if border || rock >= 5 { ROCK } else { CAVERN });
        }
        g = next;
    }
    // Keep the largest region; tunnel the others of any size to it.
    let mut label = vec![0usize; w as usize * h as usize];
    let mut regions: Vec<Vec<(u32, u32)>> = Vec::new();
    for (x, y) in g.squares().collect::<Vec<_>>() {
        let i = g.idx(x, y);
        if g.zone[i] == ROCK || label[i] != 0 {
            continue;
        }
        let seen = flood(w, h, (x, y), |a, b| g.zone[g.idx(a, b)] != ROCK);
        let squares: Vec<(u32, u32)> = g.squares().filter(|&(a, b)| seen[g.idx(a, b)]).collect();
        for &(a, b) in &squares {
            label[g.idx(a, b)] = regions.len() + 1;
        }
        regions.push(squares);
    }
    let main = (0..regions.len()).max_by_key(|&i| (regions[i].len(), usize::MAX - i))?;
    let interior = (w as usize - 2) * (h as usize - 2);
    if regions[main].len() * 4 < interior {
        return None;
    }
    let mut joined: Vec<(u32, u32)> = regions[main].clone();
    for (i, region) in regions.iter().enumerate() {
        if i == main {
            continue;
        }
        if region.len() < 6 {
            for &(x, y) in region {
                g.set(x, y, ROCK);
            }
            continue;
        }
        let from = region[rng.index(region.len())];
        let to = joined
            .iter()
            .copied()
            .min_by_key(|&(x, y)| (x.abs_diff(from.0) + y.abs_diff(from.1), x, y))?;
        tunnel(rng, &mut g, from, to);
        joined.extend(region.iter().copied());
    }
    Some(g)
}

/// Carves a two-wide winding tunnel from `a` to `b`, never touching the
/// map border.
fn tunnel(rng: &mut Rng, g: &mut Grid, a: (u32, u32), b: (u32, u32)) {
    let (mut x, mut y) = a;
    let mut guard = 0;
    while (x, y) != b && guard < 4 * (g.w + g.h) {
        guard += 1;
        let go_x = x != b.0 && (y == b.1 || rng.chance(0.5));
        if go_x {
            x = if x < b.0 { x + 1 } else { x - 1 };
        } else {
            y = if y < b.1 { y + 1 } else { y - 1 };
        }
        for (dx, dy) in [(0, 0), (1, 0), (0, 1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx >= 1 && ny >= 1 && nx + 1 < g.w && ny + 1 < g.h {
                g.set(nx, ny, CAVERN);
            }
        }
    }
}

/// Breadth-first distances over floor from `starts`.
fn distances(g: &Grid, starts: &[(u32, u32)]) -> Vec<u32> {
    let mut d = vec![u32::MAX; g.zone.len()];
    let mut q = VecDeque::new();
    for &s in starts {
        let i = g.idx(s.0, s.1);
        if g.zone[i] != ROCK {
            d[i] = 0;
            q.push_back(s);
        }
    }
    while let Some((x, y)) = q.pop_front() {
        let here = d[g.idx(x, y)];
        for (dx, dy) in N4 {
            let (nx, ny) = (i64::from(x) + dx, i64::from(y) + dy);
            if !g.floor(nx, ny) {
                continue;
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let (nx, ny) = (nx as u32, ny as u32);
            let i = g.idx(nx, ny);
            if d[i] == u32::MAX {
                d[i] = here + 1;
                q.push_back((nx, ny));
            }
        }
    }
    d
}

/// Distance from every square to the nearest rock (rock itself is 0).
fn rock_distance(g: &Grid) -> Vec<u32> {
    let mut d = vec![u32::MAX; g.zone.len()];
    let mut q = VecDeque::new();
    for (x, y) in g.squares() {
        let i = g.idx(x, y);
        if g.zone[i] == ROCK {
            d[i] = 0;
            q.push_back((x, y));
        }
    }
    while let Some((x, y)) = q.pop_front() {
        let here = d[g.idx(x, y)];
        for (dx, dy) in N4 {
            let (nx, ny) = (i64::from(x) + dx, i64::from(y) + dy);
            if nx < 0 || ny < 0 || nx >= i64::from(g.w) || ny >= i64::from(g.h) {
                continue;
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let i = g.idx(nx as u32, ny as u32);
            if d[i] == u32::MAX {
                d[i] = here + 1;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                q.push_back((nx as u32, ny as u32));
            }
        }
    }
    d
}

/// Opens a mouth from the cavern to a random map edge; returns the border
/// edges left open and the border square a token arrives on.
fn mouth(rng: &mut Rng, g: &mut Grid) -> (BTreeSet<EdgeKey>, (u32, u32)) {
    let (w, h) = (g.w, g.h);
    let side = [Side::N, Side::E, Side::S, Side::W][rng.index(4)];
    let edge_dist = |x: u32, y: u32| match side {
        Side::N => y,
        Side::S => h - 1 - y,
        Side::W => x,
        Side::E => w - 1 - x,
    };
    let floor: Vec<(u32, u32)> = g
        .squares()
        .filter(|&(x, y)| g.zone[g.idx(x, y)] != ROCK)
        .collect();
    // Among the floor squares nearest the edge, a random one away from the
    // corners.
    let best = floor
        .iter()
        .map(|&(x, y)| edge_dist(x, y))
        .min()
        .unwrap_or(0);
    let near: Vec<(u32, u32)> = floor
        .iter()
        .copied()
        .filter(|&(x, y)| edge_dist(x, y) <= best + 2)
        .filter(|&(x, y)| x >= 2 && y >= 2 && x + 3 <= w && y + 3 <= h)
        .collect();
    let start = if near.is_empty() {
        floor.first().copied().unwrap_or((w / 2, h / 2))
    } else {
        near[rng.index(near.len())]
    };
    let mut open = BTreeSet::new();
    let (mut x, mut y) = start;
    loop {
        // Two squares wide, across the direction of travel.
        let across = match side {
            Side::N | Side::S => (x + 1, y),
            Side::E | Side::W => (x, y + 1),
        };
        for (a, b) in [(x, y), across] {
            g.set(a, b, CAVERN);
        }
        let at_edge = edge_dist(x, y) == 0;
        if at_edge {
            for (a, b) in [(x, y), across] {
                open.insert(match side {
                    Side::N => (EdgeAxis::Horizontal, a, 0),
                    Side::S => (EdgeAxis::Horizontal, a, h),
                    Side::W => (EdgeAxis::Vertical, 0, b),
                    Side::E => (EdgeAxis::Vertical, w, b),
                });
            }
            return (open, (x, y));
        }
        match side {
            Side::N => y -= 1,
            Side::S => y += 1,
            Side::W => x -= 1,
            Side::E => x += 1,
        }
    }
}

/// The shortest floor path between two squares.
fn path(g: &Grid, a: (u32, u32), b: (u32, u32)) -> Vec<(u32, u32)> {
    let d = distances(g, &[b]);
    let mut out = vec![a];
    let mut at = a;
    while at != b {
        let here = d[g.idx(at.0, at.1)];
        if here == u32::MAX || here == 0 {
            break;
        }
        let mut next = None;
        for (dx, dy) in N4 {
            let (nx, ny) = (i64::from(at.0) + dx, i64::from(at.1) + dy);
            if !g.floor(nx, ny) {
                continue;
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let n = (nx as u32, ny as u32);
            if d[g.idx(n.0, n.1)] + 1 == here {
                next = Some(n);
                break;
            }
        }
        let Some(n) = next else { break };
        out.push(n);
        at = n;
    }
    out
}

#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
pub(crate) fn generate(p: &Params) -> Result<Dungeon, DungeonError> {
    let (w, h) = (p.width, p.height);
    let mut rng = Rng::new(p.seed ^ 0xCA7E);
    let mut g = None;
    for _ in 0..16 {
        if let Some(found) = caverns(&mut rng, w, h) {
            g = Some(found);
            break;
        }
    }
    let mut g =
        g.ok_or_else(|| DungeonError::Generation(format!("no usable cavern on a {w}x{h} map")))?;
    let (open, mouth_sq) = mouth(&mut rng, &mut g);
    let n = w as usize * h as usize;
    let floor_count = g.zone.iter().filter(|z| **z != ROCK).count();
    let near_rock = rock_distance(&g);
    let from_mouth = distances(&g, &[mouth_sq]);
    let salt = p.seed ^ 0x57A1;
    let noise = |k: u64, x: u32, y: u32, scale: f32| {
        fbm(salt ^ k, x as f32 / scale, y as f32 / scale, 3, None)
    };
    // Water: pools in open ground away from the mouth, and a stream.
    let mut depth = vec![0u8; n];
    for (x, y) in g.squares() {
        let i = g.idx(x, y);
        if g.zone[i] == ROCK || from_mouth[i] < 6 {
            continue;
        }
        let v = noise(1, x, y, 7.0);
        if near_rock[i] >= 2 && v > 0.67 {
            depth[i] = if v > 0.73 && near_rock[i] >= 3 { 6 } else { 2 };
        }
    }
    if rng.chance(0.8) && floor_count > 60 {
        let floor: Vec<(u32, u32)> = g
            .squares()
            .filter(|&(x, y)| g.zone[g.idx(x, y)] != ROCK)
            .collect();
        let seed_sq = floor[rng.index(floor.len())];
        let far = |d: &[u32]| {
            floor
                .iter()
                .copied()
                .filter(|&(x, y)| d[g.idx(x, y)] != u32::MAX)
                .max_by_key(|&(x, y)| (d[g.idx(x, y)], x, y))
        };
        if let Some(a) = far(&distances(&g, &[seed_sq])) {
            if let Some(b) = far(&distances(&g, &[a])) {
                for (x, y) in path(&g, a, b) {
                    let i = g.idx(x, y);
                    if from_mouth[i] >= 3 && depth[i] == 0 {
                        depth[i] = 1 + u8::from(rng.chance(0.3));
                    }
                }
            }
        }
    }
    let wet = |x: i64, y: i64| {
        x >= 0
            && y >= 0
            && x < i64::from(w)
            && y < i64::from(h)
            && depth[usize::try_from(y * i64::from(w) + x).unwrap_or(0)] > 0
    };
    // Ground.
    let mut layout = TacticalLayout::new(&format!("cave_{}", p.seed), w, h, ROCK_GROUND);
    let mut difficult = vec![false; n];
    for (x, y) in g.squares() {
        let i = g.idx(x, y);
        if g.zone[i] == ROCK {
            continue;
        }
        let (xi, yi) = (i64::from(x), i64::from(y));
        let by_water = N4.iter().any(|(dx, dy)| wet(xi + dx, yi + dy));
        let ground = if depth[i] > 0 {
            "gravel"
        } else if by_water {
            if noise(2, x, y, 3.0) > 0.5 {
                "mud"
            } else {
                "gravel"
            }
        } else if noise(3, x, y, 6.0) > 0.66 && near_rock[i] <= 2 {
            difficult[i] = true;
            "scree"
        } else if noise(4, x, y, 5.0) > 0.72 {
            "moss"
        } else {
            "cave_floor"
        };
        if let Some(sq) = layout.square_mut(x, y) {
            *sq = Square {
                ground: ground.into(),
                elevation_ft: 0,
                water_depth_ft: depth[i],
                dryness: 0,
            };
        }
    }
    layout.walls = grid::walls(&g, &BTreeMap::<EdgeKey, Opening>::new(), &open, CAVE_KIT);
    // Dressing.
    let mut d = Dresser::new(w, h);
    d.wet = depth.iter().map(|&v| v > 0).collect();
    for (x, y) in g.squares() {
        if from_mouth[g.idx(x, y)] <= 2 {
            d.keep(x, y);
        }
    }
    let region = |x: u32, y: u32| g.zone[g.idx(x, y)] != ROCK;
    // The descent: the farthest dry wall slot from the mouth.
    let mut by_far: Vec<(u32, u32)> = g
        .squares()
        .filter(|&(x, y)| {
            let i = g.idx(x, y);
            g.zone[i] != ROCK && from_mouth[i] != u32::MAX
        })
        .collect();
    by_far.sort_by_key(|&(x, y)| (std::cmp::Reverse(from_mouth[g.idx(x, y)]), y, x));
    let mut exits = vec![Exit {
        kind: ExitKind::Mouth,
        x: mouth_sq.0,
        y: mouth_sq.1,
    }];
    'descent: for &(x, y) in &by_far {
        for side in [Side::N, Side::E, Side::S, Side::W] {
            // The stair's back against rock, its foot on dry floor.
            let (x0, y0, back, foot) = match side {
                Side::N => (x, y, (x, y.wrapping_sub(1)), (x, y + 2)),
                Side::S => (x, y.wrapping_sub(1), (x, y + 1), (x, y.wrapping_sub(2))),
                Side::W => (x, y, (x.wrapping_sub(1), y), (x + 2, y)),
                Side::E => (x.wrapping_sub(1), y, (x + 1, y), (x.wrapping_sub(2), y)),
            };
            let in_map = |(a, b): (u32, u32)| a < w && b < h;
            if !in_map(back) || !in_map(foot) || g.zone[g.idx(back.0, back.1)] != ROCK {
                continue;
            }
            if !region(foot.0, foot.1) || d.is_used(foot.0, foot.1) {
                continue;
            }
            if d.place(&STAIRS_DOWN, x0, y0, side.rotation(), &region, (0.0, 0.0)) {
                d.keep(foot.0, foot.1);
                exits.push(Exit {
                    kind: ExitKind::StairsDown,
                    x: foot.0,
                    y: foot.1,
                });
                break 'descent;
            }
        }
    }
    if exits.len() < 2 {
        return Err(DungeonError::Generation("no slot for the descent".into()));
    }
    let spots: Vec<(u32, u32)> = g
        .squares()
        .filter(|&(x, y)| region(x, y) && depth[g.idx(x, y)] == 0)
        .collect();
    let scatter =
        |rng: &mut Rng, d: &mut Dresser, it: &Item, count: usize, ok: &dyn Fn(u32, u32) -> bool| {
            let cands: Vec<(u32, u32)> = spots.iter().copied().filter(|&(x, y)| ok(x, y)).collect();
            if cands.is_empty() {
                return Vec::new();
            }
            let mut placed = Vec::new();
            for _ in 0..count * 4 {
                if placed.len() >= count {
                    break;
                }
                let (x, y) = cands[rng.index(cands.len())];
                let rot = [0u16, 90, 180, 270][rng.index(4)];
                if d.place(it, x, y, rot, &region, (0.0, 0.0)) {
                    placed.push((x, y));
                }
            }
            placed
        };
    let open_floor = |x: u32, y: u32| near_rock[g.idx(x, y)] >= 2;
    let by_wall = |x: u32, y: u32| near_rock[g.idx(x, y)] == 1;
    let fc = floor_count;
    scatter(&mut rng, &mut d, &STALAGMITE, fc / 45 + 1, &open_floor);
    scatter(&mut rng, &mut d, &ROCK_LARGE, fc / 260, &open_floor);
    scatter(&mut rng, &mut d, &BOULDER, fc / 90, &by_wall);
    scatter(&mut rng, &mut d, &ROCK_SMALL, fc / 90, &by_wall);
    scatter(&mut rng, &mut d, &STONES, fc / 120, &by_wall);
    scatter(&mut rng, &mut d, &RUBBLE, fc / 70 + 1, &|_, _| true);
    let damp = |x: u32, y: u32| {
        let (xi, yi) = (i64::from(x), i64::from(y));
        layout.square(xi, yi).ground == "moss" || N4.iter().any(|(dx, dy)| wet(xi + dx, yi + dy))
    };
    let shrooms = scatter(&mut rng, &mut d, &MUSHROOMS, fc / 80 + 1, &damp);
    // Lair leftovers in a quiet corner.
    let bones = 1 + rng.index(3);
    scatter(&mut rng, &mut d, &BONES, bones, &by_wall);
    if rng.chance(0.5) {
        scatter(&mut rng, &mut d, &SKELETON, 1, &by_wall);
    }
    if rng.chance(0.4) {
        scatter(&mut rng, &mut d, &TREASURE, 1, &by_wall);
    }
    layout.placements = d.placements;
    // Faintly glowing fungus: the only light down here.
    for (k, &(x, y)) in shrooms.iter().enumerate() {
        if k % 2 == 0 {
            layout.lights.push(LightSource {
                x: x as f32 + 0.5,
                y: y as f32 + 0.5,
                radius_ft: 10,
                colour: [120, 220, 200],
            });
        }
    }
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for (x, y) in g.squares() {
        if region(x, y) {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    Ok(Dungeon {
        params: *p,
        rules: grid::rules(&g, &difficult),
        layout,
        rooms: vec![Room {
            id: 0,
            purpose: Purpose::Cavern,
            x: x0,
            y: y0,
            w: x1.saturating_sub(x0) + 1,
            h: y1.saturating_sub(y0) + 1,
        }],
        doors: Vec::new(),
        exits,
    })
}
