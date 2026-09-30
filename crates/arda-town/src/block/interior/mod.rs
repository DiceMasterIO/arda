//! Furnished interiors per building function (goal 44: rooms, doors,
//! furniture; goal 64: dressing by function). Each programme partitions the
//! canonical frame into rooms and furnishes them with vocabulary props.

mod civic;
mod homes;
mod trade;

use super::frame::{Canon, Frame, Want};
use super::kits;
use crate::function::BuildingFunction as F;
use crate::plan::grid::{Side, SquareRect};
use crate::plan::{Building, Door, TownPlan};
use crate::rng::Rng;
use arda_tactical::catalog::WallRole;
use arda_tactical::layout::EdgeAxis;

/// A prop in global squares.
#[derive(Debug, Clone)]
pub struct GProp {
    /// Asset wanted.
    pub want: Want,
    /// Anchor (footprint centre), global squares.
    pub x: f64,
    /// Anchor, global squares.
    pub y: f64,
    /// Rotation, degrees.
    pub rot: u16,
    /// Footprint extents `[x0, y0, x1, y1]`, global squares.
    pub extent: [f64; 4],
}

/// A light in global squares.
#[derive(Debug, Clone, Copy)]
pub struct GLight {
    /// Position.
    pub x: f64,
    /// Position.
    pub y: f64,
    /// Radius, feet.
    pub radius_ft: u16,
    /// Colour.
    pub colour: [u8; 3],
    /// Index of the prop that emits it (within the same [`Interior`]).
    pub owner: Option<usize>,
}

/// A wall edge in global squares and its piece.
pub type Edge = ((i64, i64, EdgeAxis), (WallRole, &'static str));

/// A building's interior in global squares.
#[derive(Debug, Clone)]
pub struct Interior {
    /// Floor keys per global square.
    pub floors: Vec<((i64, i64), &'static str)>,
    /// Wall edges `(x, y, axis) → (role, kit)`.
    pub walls: Vec<Edge>,
    /// Props.
    pub props: Vec<GProp>,
    /// Lights.
    pub lights: Vec<GLight>,
    /// Rooms `(kind, rectangle)`.
    pub rooms: Vec<(&'static str, SquareRect)>,
}

/// Footprint `(w, h)` in squares at rotation 0 for a vocabulary prop.
#[must_use]
pub fn size(id: &str) -> (i64, i64) {
    match id {
        "prop.table" | "prop.bench" | "prop.shelf" | "prop.bookshelf" | "prop.hearth"
        | "prop.oven" | "prop.bar_counter" | "prop.cask_rack" | "prop.forge" | "prop.workbench"
        | "prop.loom" | "prop.weapon_rack" | "prop.altar" | "prop.pew" | "prop.trough"
        | "prop.rug_small" => (2, 1),
        "prop.bed" | "prop.cart" | "prop.rowboat" | "prop.bridge_deck" | "prop.stairs"
        | "prop.haycart" => (1, 2),
        "prop.market_stall" | "prop.tent" | "prop.crane" | "prop.millstone" => (2, 2),
        "veg.tree_oak" | "veg.tree_elm" => (3, 3),
        "veg.tree_birch" | "veg.tree_fruit" => (2, 2),
        _ => (1, 1),
    }
}

/// Furnishing context for one building.
pub struct Ctx<'a> {
    /// The building.
    pub b: &'a Building,
    /// Its stream.
    pub rng: Rng,
    /// Shell kit.
    pub kit: &'static str,
    /// Partition kit.
    pub pkit: &'static str,
    /// Main floor.
    pub floor: &'static str,
}

/// Places an id at cell `(x, y)` with rotation; the footprint follows it.
pub fn put(c: &mut Canon, id: &'static str, x: i64, y: i64, rot: u16) -> Option<usize> {
    let (w, h) = size(id);
    let (w, h) = if rot % 180 == 90 { (h, w) } else { (w, h) };
    c.put_id(id, [x, y, x + w, y + h], rot)
}

/// Places a prop against the given wall of a room, trying positions from
/// the middle outwards; the prop's long side runs along the wall.
pub fn against(c: &mut Canon, room: [i64; 4], wall: Side, id: &'static str) -> Option<usize> {
    let (w, h) = size(id);
    let long = w.max(h);
    let short = w.min(h);
    let along_x = matches!(wall, Side::North | Side::South);
    // Rotation so the long side is along the wall.
    let rot = if (w >= h) == along_x { 0 } else { 90 };
    let (span0, span1) = if along_x {
        (room[0], room[2])
    } else {
        (room[1], room[3])
    };
    let len = span1 - span0;
    if len < long {
        return None;
    }
    let mid = span0 + (len - long) / 2;
    for k in 0..=len - long {
        let off = if k % 2 == 0 { k / 2 } else { -(k / 2) - 1 };
        let s = mid + off;
        if s < span0 || s + long > span1 {
            continue;
        }
        let r = match wall {
            Side::North => [s, room[1], s + long, room[1] + short],
            Side::South => [s, room[3] - short, s + long, room[3]],
            Side::West => [room[0], s, room[0] + short, s + long],
            Side::East => [room[2] - short, s, room[2], s + long],
        };
        if let Some(i) = c.put_id(id, r, rot) {
            return Some(i);
        }
    }
    None
}

/// Places a query placement at one cell.
pub fn put_query(c: &mut Canon, tags: &[&str], x: i64, y: i64) -> Option<usize> {
    c.put(
        Want::Query(tags.iter().map(|s| (*s).to_string()).collect()),
        [x, y, x + 1, y + 1],
        0,
    )
}

/// Warm hearth light.
pub const FIRE: [u8; 3] = [255, 168, 82];
/// Candle light.
pub const CANDLE: [u8; 3] = [255, 214, 150];

/// Adds a light at the centre of prop `i`.
pub fn light_on(c: &mut Canon, i: Option<usize>, radius_ft: u16, colour: [u8; 3]) {
    if let Some(i) = i {
        let p = &c.props[i];
        #[allow(clippy::cast_precision_loss)]
        let (x, y) = ((p.x0 + p.x1) as f64 * 0.5, (p.y0 + p.y1) as f64 * 0.5);
        c.light(x, y, radius_ft, colour, Some(i));
    }
}

fn side_to_canonical(side: Side, turns: u8) -> Side {
    let t = (side.turns() + 4 - turns % 4) % 4;
    match t {
        0 => Side::North,
        1 => Side::East,
        2 => Side::South,
        _ => Side::West,
    }
}

/// Canonical edge of a door: `(x, y, horizontal)`.
fn door_edge(f: &Frame, d: &Door) -> Option<(i64, i64, bool)> {
    let (u, v) = f.inverse_cell(d.x, d.y)?;
    Some(match side_to_canonical(d.side, f.turns) {
        Side::North => (u, v, true),
        Side::South => (u, v + 1, true),
        Side::West => (u, v, false),
        Side::East => (u + 1, v, false),
    })
}

fn wide_entrance(f: F) -> bool {
    matches!(
        f,
        F::Barn | F::Stable | F::Warehouse | F::Smithy | F::Boathouse | F::MarketHall
    )
}

/// Builds the interior of one building.
#[must_use]
pub fn build(plan: &TownPlan, b: &Building) -> Interior {
    let frame = Frame::new(b.rect, b.front.turns());
    let t = kits::tradition(&plan.culture);
    let mut ctx = Ctx {
        b,
        rng: Rng::keyed(plan.seed, 0xB01D, b.id.0),
        kit: kits::shell(b, t),
        pkit: kits::partition(b),
        floor: kits::floor(b),
    };
    let mut c = Canon::new(frame.w, frame.d, ctx.floor);
    if b.function.walled() {
        c.shell(ctx.kit);
        c.windows(if b.storeys > 1 { 3 } else { 4 }, ctx.kit);
    }
    let mut doors = Vec::new();
    for (k, d) in b.doors.iter().enumerate() {
        if let Some(e) = door_edge(&frame, d) {
            let role = if k == 0 && wide_entrance(b.function) {
                WallRole::Gate
            } else {
                WallRole::Door
            };
            c.walls.insert(e, (role, ctx.kit));
            let (x, y, hz) = e;
            if hz {
                c.reserve(x, y);
                c.reserve(x, y - 1);
            } else {
                c.reserve(x, y);
                c.reserve(x - 1, y);
            }
            doors.push(e);
        }
    }
    let front_x = doors.first().map_or(frame.w / 2, |e| e.0);
    match b.function {
        F::House | F::Cottage | F::Farmhouse | F::Manor => {
            homes::furnish(&mut c, &mut ctx, front_x);
        }
        F::Temple | F::Shrine | F::Library | F::School | F::Keep | F::Barracks | F::Guardhouse => {
            civic::furnish(&mut c, &mut ctx, front_x);
        }
        _ => trade::furnish(&mut c, &mut ctx, front_x),
    }
    to_global(&c, &frame)
}

fn to_global(c: &Canon, f: &Frame) -> Interior {
    let mut floors = Vec::new();
    for y in 0..c.d {
        for x in 0..c.w {
            let k = usize::try_from(y * c.w + x).unwrap_or(0);
            floors.push((f.cell(x, y), c.floor[k]));
        }
    }
    let walls = c
        .walls
        .iter()
        .map(|(&(x, y, hz), &v)| {
            let (gx, gy, axis) = f.edge(x, y, hz);
            ((gx, gy, axis), v)
        })
        .collect();
    let props = c
        .props
        .iter()
        .map(|p| {
            #[allow(clippy::cast_precision_loss)]
            let (a, b) = (
                f.point(p.x0 as f64, p.y0 as f64),
                f.point(p.x1 as f64, p.y1 as f64),
            );
            let extent = [a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1)];
            GProp {
                want: p.want.clone(),
                x: (extent[0] + extent[2]) * 0.5,
                y: (extent[1] + extent[3]) * 0.5,
                rot: f.rotation(p.rot),
                extent,
            }
        })
        .collect();
    let lights = c
        .lights
        .iter()
        .map(|l| {
            let (x, y) = f.point(l.x, l.y);
            GLight {
                x,
                y,
                radius_ft: l.radius_ft,
                colour: l.colour,
                owner: l.owner,
            }
        })
        .collect();
    let rooms = c
        .rooms
        .iter()
        .map(|r| {
            let a = f.vertex(r.rect[0], r.rect[1]);
            let b = f.vertex(r.rect[2], r.rect[3]);
            (
                r.kind,
                SquareRect {
                    x0: a.0.min(b.0),
                    y0: a.1.min(b.1),
                    x1: a.0.max(b.0),
                    y1: a.1.max(b.1),
                },
            )
        })
        .collect();
    Interior {
        floors,
        walls,
        props,
        lights,
        rooms,
    }
}
