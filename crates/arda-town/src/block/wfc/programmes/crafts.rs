//! Programmes of craft, trade and farm buildings.

use crate::block::wfc::catalogue as c;
use crate::block::wfc::programme::*;
use crate::plan::Building;

/// The `smithy` programme.
pub(crate) fn smithy(b: &Building) -> Programme {
    let area = b.rect.w() * b.rect.h();
    let big = |a: i64| area >= a;
    star(
        vec![
            floored(
                zone(
                    "forge",
                    true,
                    12,
                    1,
                    vec![
                        capped(c::FORGE, 1, 1),
                        capped(c::ANVIL, 1, 1),
                        capped(c::TROUGH, 0, 1),
                        capped(c::GRINDSTONE, 0, 1),
                        item(c::WEAPON_RACK),
                        item(c::WOODPILE),
                        item(c::BUCKET),
                        item(c::BARREL),
                        item(c::CRATE),
                        capped(c::WORKBENCH, 0, 1),
                    ],
                ),
                "packed_earth",
            ),
            living(big(36)),
            optional(store()),
        ],
        50,
        3,
    )
}

/// The `bakery` programme.
pub(crate) fn bakery() -> Programme {
    star(
        vec![
            zone(
                "shop",
                true,
                6,
                1,
                vec![need(c::SHELF, 1), capped(c::TABLE, 0, 1), item(c::SACKS)],
            ),
            floored(
                zone(
                    "bakehouse",
                    false,
                    6,
                    -1,
                    vec![
                        capped(c::OVEN, 1, 2),
                        item(c::SACKS),
                        item(c::WOODPILE),
                        item(c::BARREL),
                        capped(c::TABLE, 0, 1),
                    ],
                ),
                "stone_floor",
            ),
        ],
        25,
        2,
    )
}

/// The `brewery` programme.
pub(crate) fn brewery() -> Programme {
    star(
        vec![
            zone(
                "brewhouse",
                true,
                16,
                0,
                vec![
                    capped(c::HEARTH, 1, 1),
                    need(c::CASK_RACK, 2),
                    item(c::BARREL),
                    item(c::TROUGH),
                    item(c::BUCKET),
                ],
            ),
            optional(store()),
        ],
        50,
        2,
    )
}

/// The `mill` programme.
pub(crate) fn mill() -> Programme {
    star(
        vec![zone(
            "mill_floor",
            true,
            12,
            0,
            vec![
                capped(c::MILLSTONE, 1, 1),
                need(c::SACKS, 2),
                item(c::BARREL),
                item(c::LADDER),
                item(c::CRATE),
            ],
        )],
        100,
        1,
    )
}

/// The `workshop` programme.
pub(crate) fn workshop(b: &Building) -> Programme {
    let area = b.rect.w() * b.rect.h();
    let big = |a: i64| area >= a;
    let craft = b.tags.first().map_or("craft:carpentry", String::as_str);
    let (main, extra) = match craft {
        "craft:weaving" => (c::LOOM, c::SHELF),
        "craft:pottery" => (c::WORKBENCH, c::OVEN),
        "craft:masonry" => (c::WORKBENCH, c::GRINDSTONE),
        "craft:jewellery" | "craft:tinkering" => (c::WORKBENCH, c::CUPBOARD),
        _ => (c::WORKBENCH, c::SHELF),
    };
    star(
        vec![
            zone(
                "workshop",
                true,
                10,
                1,
                vec![
                    capped(main, 1, 2),
                    capped(extra, 0, 1),
                    item(c::STOOL),
                    item(c::CRATE),
                ],
            ),
            living(big(36)),
        ],
        50,
        2,
    )
}

/// The `tannery` programme.
pub(crate) fn tannery() -> Programme {
    star(
        vec![floored(
            zone(
                "tanning_floor",
                true,
                12,
                0,
                vec![
                    need(c::TROUGH, 2),
                    item(c::BARREL),
                    item(c::BUCKET),
                    item(c::WORKBENCH),
                ],
            ),
            "packed_earth",
        )],
        100,
        1,
    )
}

/// The `apothecary` programme.
pub(crate) fn apothecary() -> Programme {
    star(
        vec![
            zone(
                "shop",
                true,
                6,
                1,
                vec![
                    need(c::SHELF, 1),
                    capped(c::TABLE, 0, 1),
                    capped(c::CANDLES, 0, 1),
                ],
            ),
            optional(zone(
                "stillroom",
                false,
                4,
                -1,
                vec![
                    item(c::BOOKSHELF),
                    item(c::CUPBOARD),
                    item(c::BED),
                    capped(c::TABLE, 0, 1),
                ],
            )),
        ],
        40,
        2,
    )
}

/// The `warehouse` programme.
pub(crate) fn warehouse() -> Programme {
    star(
        vec![
            zone(
                "hall",
                true,
                20,
                1,
                vec![
                    ordered(item(c::RACK), Order::Rows { gap: 0 }),
                    item(c::CRATE),
                    item(c::SACKS),
                ],
            ),
            optional(zone(
                "office",
                false,
                4,
                -1,
                vec![capped(c::TABLE, 1, 1), item(c::CHAIR), item(c::CHEST)],
            )),
        ],
        60,
        2,
    )
}

/// The `market_hall` programme.
pub(crate) fn market_hall() -> Programme {
    star(
        vec![zone(
            "arcade",
            true,
            20,
            0,
            vec![
                ordered(capped(c::TABLE, 2, 4), Order::Rows { gap: 1 }),
                item(c::CRATE),
                item(c::SACKS),
                item(c::BARREL),
            ],
        )],
        100,
        1,
    )
}

/// The `boathouse` programme.
pub(crate) fn boathouse() -> Programme {
    star(
        vec![zone(
            "boat_shed",
            true,
            8,
            0,
            vec![
                capped(c::ROWBOAT, 1, 1),
                item(c::CRATE),
                item(c::SACKS),
                item(c::LADDER),
            ],
        )],
        100,
        1,
    )
}

/// The `stable` programme.
pub(crate) fn stable() -> Programme {
    star(
        vec![floored(
            zone(
                "stalls",
                true,
                8,
                0,
                vec![need(c::HAY, 2), need(c::TROUGH, 1), item(c::BUCKET)],
            ),
            "packed_earth",
        )],
        100,
        1,
    )
}

/// The `barn` programme.
pub(crate) fn barn() -> Programme {
    star(
        vec![zone(
            "barn_floor",
            true,
            8,
            0,
            vec![
                need(c::HAY, 3),
                capped(c::BUCKET, 0, 1),
                item(c::SACKS),
                item(c::LADDER),
            ],
        )],
        100,
        1,
    )
}
