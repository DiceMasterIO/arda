//! Typed tile sets per building function (goal 44): the zones (room
//! kinds) a building may be partitioned into, which zones may open onto
//! which, and the furniture each zone may hold, must hold and may hold at
//! most. The room tags are logic/10 §town-interiors' `room` tags.

use super::catalogue as c;
use super::piece::Piece;
pub use super::rows::Order;
use crate::function::BuildingFunction as F;
use crate::plan::{Building, WealthLevel};

/// One piece a zone may hold.
#[derive(Debug, Clone, Copy)]
pub struct Item {
    /// The piece.
    pub piece: Piece,
    /// How many each room of the zone must hold.
    pub need: u8,
    /// How many each room may hold (0: no cap).
    pub cap: u8,
    /// Smallest room, squares, in which `need` applies (smaller rooms may
    /// still hold the piece).
    pub min_room: i64,
    /// Laid out by rule in aligned rows before the WFC (`super::rows`).
    pub order: Option<Order>,
}

/// A room kind of a programme.
#[derive(Debug, Clone)]
pub struct Zone {
    /// Room tag.
    pub tag: &'static str,
    /// Whether exterior doors may open into it.
    pub entry: bool,
    /// Whether the building must have one.
    pub required: bool,
    /// Smallest room, squares.
    pub min_area: i64,
    /// Smallest room side, squares.
    pub min_side: i64,
    /// Largest room, squares (0: no limit).
    pub max_area: i64,
    /// Most rooms of this kind.
    pub max_rooms: u8,
    /// Row preference: positive for the front, negative for the back.
    pub depth: i8,
    /// Choice weight of the zone.
    pub weight: u32,
    /// Floor key overriding the building's.
    pub floor: Option<&'static str>,
    /// Choice weight of open floor (lower packs furniture tighter).
    pub floor_weight: u32,
    /// Most furniture, percent of the floor.
    pub density: u8,
    /// Whether the room's spine is an aisle from its entrance to the back
    /// (a nave), whatever its shape.
    pub aisle: bool,
    /// Furniture.
    pub items: Vec<Item>,
}

/// A building function's programme.
#[derive(Debug, Clone)]
pub struct Programme {
    /// Zones; the first is the main entry zone.
    pub zones: Vec<Zone>,
    /// Zone pairs a doorway may join (by index, either order).
    pub doors: Vec<(usize, usize)>,
    /// Smallest share of the floor the entry zones take, percent.
    pub entry_pct: i64,
    /// Most rooms in all.
    pub max_rooms: usize,
}

pub(super) const fn item(piece: Piece) -> Item {
    Item {
        piece,
        need: 0,
        cap: 0,
        min_room: 0,
        order: None,
    }
}

pub(super) const fn need(piece: Piece, n: u8) -> Item {
    Item {
        piece,
        need: n,
        cap: 0,
        min_room: 0,
        order: None,
    }
}

pub(super) const fn capped(piece: Piece, n: u8, cap: u8) -> Item {
    Item {
        piece,
        need: n,
        cap,
        min_room: 0,
        order: None,
    }
}

/// The same item, laid out by rule in aligned rows (`super::rows`).
pub(super) const fn ordered(mut it: Item, order: Order) -> Item {
    it.order = Some(order);
    it
}

/// The same item, needed only in rooms of at least `min_room` squares.
pub(super) const fn from(mut it: Item, min_room: i64) -> Item {
    it.min_room = min_room;
    it
}

pub(super) fn zone(
    tag: &'static str,
    entry: bool,
    min_area: i64,
    depth: i8,
    items: Vec<Item>,
) -> Zone {
    Zone {
        tag,
        entry,
        required: true,
        min_area,
        min_side: if min_area >= 12 { 3 } else { 2 },
        max_area: if entry { 0 } else { (min_area * 5).max(24) },
        max_rooms: 1,
        depth,
        weight: 10,
        floor: None,
        floor_weight: 40,
        density: 35,
        aisle: false,
        items,
    }
}

pub(super) fn packed(mut z: Zone, floor_weight: u32, density: u8) -> Zone {
    z.floor_weight = floor_weight;
    z.density = density;
    z
}

pub(super) fn optional(mut z: Zone) -> Zone {
    z.required = false;
    z
}

pub(super) fn floored(mut z: Zone, key: &'static str) -> Zone {
    z.floor = Some(key);
    z
}

pub(super) fn star(zones: Vec<Zone>, entry_pct: i64, max_rooms: usize) -> Programme {
    let doors = (1..zones.len()).map(|k| (0, k)).collect();
    Programme {
        zones,
        doors,
        entry_pct,
        max_rooms,
    }
}

pub(super) fn rugs(b: &Building, items: &mut Vec<Item>) {
    if b.wealth_level == WealthLevel::Wealthy {
        items.push(capped(c::RUG, 0, 1));
    }
}

/// The programme of a walled building, by function; `None` when the
/// function has no WFC interior (stalls, docks). Homes follow their
/// household (`programmes::household`).
#[must_use]
pub fn of(plan: &crate::plan::TownPlan, b: &Building) -> Option<Programme> {
    Some(match b.function {
        F::House => super::programmes::homes::house(plan, b),
        F::Cottage => super::programmes::homes::cottage(plan, b),
        F::Farmhouse => super::programmes::homes::farmhouse(plan, b),
        F::Manor => super::programmes::homes::manor(b),
        F::Inn | F::Tavern => super::programmes::homes::inn(b),
        F::Smithy => super::programmes::crafts::smithy(b),
        F::Bakery => super::programmes::crafts::bakery(),
        F::Brewery => super::programmes::crafts::brewery(),
        F::Mill => super::programmes::crafts::mill(),
        F::Workshop => super::programmes::crafts::workshop(b),
        F::Tannery => super::programmes::crafts::tannery(),
        F::Apothecary => super::programmes::crafts::apothecary(),
        F::Warehouse => super::programmes::crafts::warehouse(),
        F::MarketHall => super::programmes::crafts::market_hall(),
        F::Boathouse => super::programmes::crafts::boathouse(),
        F::Stable => super::programmes::crafts::stable(),
        F::Barn => super::programmes::crafts::barn(),
        F::Temple | F::Shrine => super::programmes::civic::temple(b),
        F::Library => super::programmes::civic::library(),
        F::School => super::programmes::civic::school(),
        F::Keep => super::programmes::civic::keep(),
        F::Barracks => super::programmes::civic::barracks(),
        F::Guardhouse => super::programmes::civic::guardhouse(),
        _ => return None,
    })
}

pub(super) fn kitchen() -> Zone {
    floored(
        zone(
            "kitchen",
            false,
            6,
            -1,
            vec![
                capped(c::OVEN, 1, 1),
                capped(c::TABLE, 0, 1),
                item(c::BARREL),
                item(c::SACKS),
                item(c::SHELF),
                item(c::CRATE),
                capped(c::WOODPILE, 0, 1),
                capped(c::CUPBOARD, 0, 1),
            ],
        ),
        "stone_floor",
    )
}

pub(super) fn store() -> Zone {
    zone(
        "store",
        false,
        4,
        -1,
        vec![
            item(c::BARREL),
            item(c::CRATE),
            item(c::SACKS),
            item(c::CASK_RACK),
        ],
    )
}

pub(super) fn living(present: bool) -> Zone {
    let z = zone(
        "living",
        false,
        6,
        -1,
        vec![
            capped(c::BED, 1, 2),
            item(c::CHEST),
            capped(c::HEARTH, 0, 1),
            capped(c::SHELF, 0, 1),
            capped(c::CUPBOARD, 0, 1),
        ],
    );
    if present {
        z
    } else {
        optional(z)
    }
}

pub(super) fn temple(vestry: bool) -> Programme {
    let mut nave = packed(
        zone(
            "nave",
            true,
            12,
            1,
            vec![
                ordered(from(need(c::PEW, 2), 20), Order::Pews),
                capped(c::CANDLES, 0, 2),
            ],
        ),
        4,
        60,
    );
    nave.aisle = true;
    let mut zones = vec![
        nave,
        zone(
            "chancel",
            false,
            4,
            -2,
            vec![
                capped(c::ALTAR, 1, 1),
                capped(c::STATUE, 0, 1),
                capped(c::CANDLES, 0, 2),
            ],
        ),
    ];
    if vestry {
        zones.push(optional(zone(
            "vestry",
            false,
            4,
            -1,
            vec![item(c::BOOKSHELF), item(c::CHEST)],
        )));
    }
    Programme {
        doors: vec![(0, 1), (1, 2)],
        zones,
        entry_pct: 55,
        max_rooms: 3,
    }
}

/// Fits a programme to a `w × d` floor: required back rooms that cannot
/// fit are dropped, their required pieces moving into the entry zone (a
/// shrine too small for a chancel keeps its altar in the nave).
#[must_use]
pub fn fit(mut p: Programme, w: i64, d: i64) -> Programme {
    let area = w * d;
    let budget = |p: &Programme| -> i64 {
        p.zones
            .iter()
            .filter(|z| z.required)
            .map(|z| z.min_area)
            .sum()
    };
    while p.zones.len() > 1 && (budget(&p) * 100 > area * 85 || w.min(d) < 2) {
        let Some(k) = (1..p.zones.len()).rev().find(|&k| p.zones[k].required) else {
            break;
        };
        let gone = p.zones.remove(k);
        for it in gone.items.into_iter().filter(|it| it.need > 0) {
            p.zones[0].items.push(Item {
                piece: it.piece,
                need: 1,
                cap: it.cap.max(1),
                min_room: it.min_room,
                order: it.order,
            });
        }
        p.doors = p
            .doors
            .iter()
            .filter(|&&(a, b)| a != k && b != k)
            .map(|&(a, b)| (a - usize::from(a > k), b - usize::from(b > k)))
            .collect();
    }
    // Back rooms may grow with the building (a chancel across a large
    // temple), up to 30 % of the floor.
    for z in &mut p.zones {
        if z.max_area > 0 {
            z.max_area = z.max_area.max(area * 3 / 10);
        }
    }
    // Optional zones that cannot fit beside the required ones go too.
    let spare = area - budget(&p);
    p.zones.retain(|z| z.required || z.min_area * 2 <= spare);
    let n = p.zones.len();
    p.doors.retain(|&(a, b)| a < n && b < n);
    p
}
