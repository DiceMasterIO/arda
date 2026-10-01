//! What homes hold (goal 64): clutter pools by wealth, stores and byres,
//! and the household trades that flavour a house's clutter.

use super::clutter::Pool;
use crate::plan::WealthLevel as W;
use crate::rng::Rng;

/// Everyday clutter by wealth.
pub fn household(w: W) -> Pool {
    match w {
        W::Poor => &[
            ("prop.sacks", 4),
            ("prop.barrel", 2),
            ("prop.bucket", 3),
            ("prop.woodpile", 3),
            ("prop.crate", 2),
            ("prop.stool", 1),
        ],
        W::Modest => &[
            ("prop.barrel", 3),
            ("prop.crate", 3),
            ("prop.sacks", 3),
            ("prop.chest", 2),
            ("prop.shelf", 2),
            ("prop.bucket", 1),
            ("prop.cupboard", 1),
        ],
        W::Wealthy => &[
            ("prop.chest", 3),
            ("prop.cupboard", 2),
            ("prop.shelf", 2),
            ("prop.candle_stand", 2),
            ("prop.barrel", 1),
            ("prop.bookshelf", 1),
        ],
    }
}

/// Stores and pantries.
pub const STORE: Pool = &[
    ("prop.barrel", 4),
    ("prop.sacks", 4),
    ("prop.crate", 3),
    ("prop.shelf", 2),
    ("prop.woodpile", 1),
    ("prop.cask_rack", 1),
];

/// A household trade: its main piece and the clutter it brings.
pub struct Trade {
    /// The trade's main piece.
    pub main: &'static str,
    /// Clutter the trade brings.
    pub pool: Pool,
}

/// Weaver, cooper, carpenter, fisher, scribe and chandler households.
pub const TRADES: [Trade; 6] = [
    Trade {
        main: "prop.loom",
        pool: &[("prop.sacks", 3), ("prop.shelf", 1), ("prop.stool", 1)],
    },
    Trade {
        main: "prop.cask_rack",
        pool: &[("prop.barrel", 4), ("prop.bucket", 1)],
    },
    Trade {
        main: "prop.workbench",
        pool: &[("prop.crate", 2), ("prop.woodpile", 2), ("prop.ladder", 1)],
    },
    Trade {
        main: "prop.table",
        pool: &[("prop.crate", 3), ("prop.barrel", 2), ("prop.bucket", 2)],
    },
    Trade {
        main: "prop.bookshelf",
        pool: &[
            ("prop.candle_stand", 1),
            ("prop.chest", 2),
            ("prop.shelf", 1),
        ],
    },
    Trade {
        main: "prop.shelf",
        pool: &[("prop.sacks", 2), ("prop.crate", 2), ("prop.barrel", 1)],
    },
];

pub fn trade(rng: &mut Rng, w: W) -> Option<&'static Trade> {
    let p = match w {
        W::Poor => 0.3,
        W::Modest => 0.5,
        W::Wealthy => 0.35,
    };
    rng.chance(p).then(|| &TRADES[rng.index(TRADES.len())])
}

/// Byre stock.
pub const BYRE: Pool = &[
    ("prop.hay_bale", 5),
    ("prop.sacks", 1),
    ("prop.bucket", 2),
    ("prop.wheelbarrow", 1),
];
