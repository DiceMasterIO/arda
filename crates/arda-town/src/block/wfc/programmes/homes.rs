//! Programmes of homes and hostelries: house, cottage, farmhouse, manor,
//! inn and tavern. Homes take their partition, trade, beds, fire and
//! clutter from the building's household (`super::household`), so WFC
//! homes vary as the rule programmes do (goal 64).

use super::household::{self as hh, add, Household};
use crate::block::interior::homes::Partition;
use crate::block::wfc::catalogue as c;
use crate::block::wfc::programme::*;
use crate::function::BuildingFunction as F;
use crate::plan::{Building, TownPlan, WealthLevel};

fn bedchamber(b: &Building, h: &Household) -> Zone {
    let mut items = vec![
        capped(c::BED, 1, h.beds),
        item(c::CHEST),
        capped(c::CUPBOARD, 0, 1),
        capped(c::SHELF, 0, 1),
    ];
    if b.wealth_level == WealthLevel::Wealthy {
        items.push(capped(c::RUG_SMALL, 0, 1));
        items.push(capped(c::CANDLES, 0, 1));
    }
    zone("bedchamber", false, 4, -1, items)
}

fn store_room() -> Zone {
    let mut items = Vec::new();
    hh::stores(&mut items);
    zone("store", false, 4, -1, items)
}

fn workroom(entry: bool, depth: i8, h: &Household) -> Zone {
    let mut items = Vec::new();
    if let Some(t) = h.trade {
        hh::work(&mut items, t);
    }
    add(&mut items, capped(c::STOOL, 0, 2));
    zone("workroom", entry, 6, depth, items)
}

fn level(mut z: Zone, depth: i8) -> Zone {
    z.depth = depth;
    z
}

/// The `house` programme: the household's partition as zones.
pub(crate) fn house(plan: &TownPlan, b: &Building) -> Programme {
    let h = hh::of(plan, b);
    let w = b.wealth_level;
    let mut hall = Vec::new();
    hh::fire(&mut hall, &h);
    add(&mut hall, from(capped(c::TABLE, 1, 1), 12));
    add(&mut hall, item(h.seat));
    if b.storeys > 1 {
        add(&mut hall, capped(c::STAIRS, 0, 1));
    }
    rugs(b, &mut hall);
    // A trade works in its own room (a trade crowding the hall beside the
    // hearth and table leaves the WFC no legal fill), except in an open
    // hall, which has the room.
    // A back room: the trade's workroom, else a store in a large house.
    let big = b.rect.w() * b.rect.h() >= 32;
    let back = || {
        if h.trade.is_some() {
            Some(workroom(false, -1, &h))
        } else {
            big.then(store_room)
        }
    };
    // Small houses have no floor to spare for clutter.
    if b.rect.w() * b.rect.h() >= 24 {
        hh::everyday(&mut hall, w);
    }
    let zones = match h.partition {
        Partition::Open => {
            add(&mut hall, capped(c::BED, 1, h.beds.min(2)));
            if let Some(t) = h.trade {
                hh::work(&mut hall, t);
            }
            vec![zone("hall", true, 6, 0, hall)]
        }
        Partition::Back => vec![zone("hall", true, 6, 1, hall), bedchamber(b, &h)],
        Partition::Side => vec![zone("hall", true, 6, 0, hall), level(bedchamber(b, &h), 0)],
        // The rule programme's shop front becomes a workroom behind the
        // hall: a workroom at the street leaves the WFC no legal fill for
        // the hall behind it.
        Partition::Front => {
            add(&mut hall, capped(c::BED, 1, h.beds.min(2)));
            vec![zone("hall", true, 6, 1, hall), workroom(false, -1, &h)]
        }
        Partition::Cross => vec![zone("hall", true, 6, 0, hall), level(bedchamber(b, &h), 0)],
    };
    let mut zones = zones;
    if !matches!(h.partition, Partition::Open | Partition::Front) {
        if let Some(z) = back() {
            let depth = if h.partition == Partition::Cross {
                0
            } else {
                -1
            };
            zones.push(optional(level(z, depth)));
        }
    }
    star(zones, 30, 3)
}

/// The `cottage` programme.
pub(crate) fn cottage(plan: &TownPlan, b: &Building) -> Programme {
    let h = hh::of(plan, b);
    let mut hall = Vec::new();
    hh::fire(&mut hall, &h);
    add(&mut hall, capped(c::BED, 1, h.beds.min(2)));
    add(&mut hall, capped(c::TABLE, 0, 1));
    add(&mut hall, item(c::STOOL));
    if h.loom {
        add(&mut hall, capped(c::LOOM, 1, 1));
    }
    add(&mut hall, item(c::CHEST));
    add(&mut hall, capped(c::SHELF, 0, 1));
    add(&mut hall, capped(c::WOODPILE, 0, 1));
    hh::everyday(&mut hall, WealthLevel::Poor);
    star(vec![zone("hall", true, 6, 0, hall)], 100, 1)
}

/// The `farmhouse` programme.
pub(crate) fn farmhouse(plan: &TownPlan, b: &Building) -> Programme {
    let h = hh::of(plan, b);
    let mut hall = vec![
        capped(c::HEARTH, 1, 1),
        capped(c::BED, 1, h.beds),
        capped(c::TABLE, 1, 4),
        item(c::BENCH),
        item(c::CHEST),
    ];
    hh::everyday(&mut hall, b.wealth_level);
    let mut byre = vec![need(c::HAY, 2), item(c::TROUGH), item(c::BUCKET)];
    hh::byre(&mut byre);
    star(
        vec![
            zone("hall", true, 12, 0, hall),
            floored(zone("byre", true, 8, 0, byre), "packed_earth"),
        ],
        30,
        2,
    )
}

/// The `manor` programme.
pub(crate) fn manor(b: &Building) -> Programme {
    let mut hall = vec![
        capped(c::HEARTH, 1, 1),
        capped(c::TABLE, 2, 4),
        item(c::CHAIR),
        item(c::BANNER),
        capped(c::STAIRS, 1, 1),
        capped(c::CANDLES, 0, 2),
    ];
    rugs(b, &mut hall);
    let mut solar = vec![
        need(c::BED, 1),
        item(c::CHEST),
        item(c::BOOKSHELF),
        capped(c::RUG_SMALL, 0, 1),
    ];
    hh::everyday(&mut solar, WealthLevel::Wealthy);
    star(
        vec![
            zone("great_hall", true, 24, 1, hall),
            zone("solar", false, 6, -1, solar),
            kitchen(),
        ],
        40,
        4,
    )
}

/// The `inn` programme.
pub(crate) fn inn(b: &Building) -> Programme {
    let common = vec![
        from(capped(c::BAR, 1, 1), 24),
        capped(c::CASK_RACK, 0, 2),
        capped(c::HEARTH, 1, 1),
        capped(c::TABLE, 2, 4),
        item(c::BENCH),
        item(c::STOOL),
    ];
    let back = if b.function == F::Inn {
        zone(
            "stair_hall",
            false,
            6,
            -1,
            vec![
                capped(c::STAIRS, 1, 1),
                item(c::CHEST),
                item(c::BARREL),
                item(c::CRATE),
                capped(c::SHELF, 0, 1),
            ],
        )
    } else {
        store()
    };
    let zones = vec![
        packed(zone("common_room", true, 20, 1, common), 40, 40),
        kitchen(),
        back,
    ];
    star(zones, 45, 4)
}
