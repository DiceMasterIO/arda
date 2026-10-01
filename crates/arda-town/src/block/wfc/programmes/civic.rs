//! Programmes of faith, learning and military buildings.

use crate::block::wfc::catalogue as c;
use crate::block::wfc::programme::*;
use crate::function::BuildingFunction as F;
use crate::plan::Building;

/// The `temple` programme.
pub(crate) fn temple(b: &Building) -> Programme {
    let area = b.rect.w() * b.rect.h();
    let big = |a: i64| area >= a;
    crate::block::wfc::programme::temple(b.function == F::Temple && big(100))
}

/// The `library` programme.
pub(crate) fn library() -> Programme {
    star(
        vec![zone(
            "reading_room",
            true,
            10,
            0,
            vec![
                ordered(need(c::BOOKSHELF, 2), Order::Walls { gap: 0 }),
                capped(c::TABLE, 0, 1),
                item(c::CHAIR),
                capped(c::CANDLES, 0, 2),
            ],
        )],
        100,
        1,
    )
}

/// The `school` programme.
pub(crate) fn school() -> Programme {
    star(
        vec![zone(
            "schoolroom",
            true,
            10,
            0,
            vec![
                capped(c::TABLE, 1, 4),
                item(c::BENCH),
                item(c::BOOKSHELF),
                item(c::CHAIR),
            ],
        )],
        100,
        1,
    )
}

/// The `keep` programme.
pub(crate) fn keep() -> Programme {
    star(
        vec![
            zone(
                "great_hall",
                true,
                30,
                1,
                vec![
                    capped(c::THRONE, 1, 1),
                    capped(c::HEARTH, 1, 1),
                    capped(c::TABLE, 1, 4),
                    item(c::BENCH),
                    item(c::BANNER),
                    capped(c::RUG, 0, 1),
                ],
            ),
            zone(
                "armoury",
                false,
                6,
                -1,
                vec![
                    need(c::WEAPON_RACK, 1),
                    item(c::ARMOUR_STAND),
                    item(c::CHEST),
                ],
            ),
            zone(
                "chamber",
                false,
                6,
                -1,
                vec![need(c::BED, 1), item(c::CHEST), capped(c::STAIRS, 1, 1)],
            ),
        ],
        40,
        4,
    )
}

/// The `barracks` programme.
pub(crate) fn barracks() -> Programme {
    star(
        vec![zone(
            "dormitory",
            true,
            20,
            0,
            vec![
                ordered(need(c::BED, 3), Order::Walls { gap: 1 }),
                item(c::CHEST),
                item(c::WEAPON_RACK),
                item(c::ARMOUR_STAND),
                capped(c::HEARTH, 1, 1),
            ],
        )],
        100,
        1,
    )
}

/// The `guardhouse` programme.
pub(crate) fn guardhouse() -> Programme {
    star(
        vec![
            zone(
                "guardroom",
                true,
                8,
                1,
                vec![
                    capped(c::TABLE, 1, 1),
                    item(c::STOOL),
                    item(c::WEAPON_RACK),
                    item(c::CHEST),
                    capped(c::BRAZIER, 1, 1),
                ],
            ),
            optional(zone("cell", false, 4, -1, vec![item(c::BED)])),
        ],
        50,
        2,
    )
}
