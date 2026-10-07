//! The outdoor tile vocabulary: square classes read off the plan (street
//! carriageway and kerb, plaza and its paved edge, market pitches, plot
//! fronts, yards, gardens, churchyards, greens, croft uses, the curtain
//! wall and its gates) and the pieces each class may hold, with weights.

use super::piece::{Need, Piece, What, ALL, ONE, TWO};
use crate::block::interior::FIRE;
use Need::{Any, Plain, Wall};

/// What a square is, for the outdoor WFC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Class {
    /// Not part of the problem (buildings, water, open country).
    Void,
    /// The carriageway of a street, kept clear.
    Carriage,
    /// The edge of a paved street: kerb stones and drains.
    KerbPaved,
    /// The edge of an unpaved street.
    KerbDirt,
    /// The market square.
    Plaza,
    /// The paved edge of the market square.
    PlazaEdge,
    /// Plaza squares beside a stall.
    Pitch,
    /// A plot's front strip.
    Front,
    /// A working yard (and castle baileys).
    Yard,
    /// A garden.
    Garden,
    /// A churchyard.
    Churchyard,
    /// A village green.
    Green,
    /// Croft paddock.
    Paddock,
    /// Croft orchard.
    Orchard,
    /// Croft kitchen garden.
    Kitchen,
    /// Croft meadow.
    Meadow,
    /// Croft work yard.
    Workyard,
    /// Rough rim of the built-up footprint.
    Rim,
    /// The curtain wall's walk.
    Wall,
    /// A gate passage.
    Gate,
}

const WARM: [u8; 3] = [255, 200, 120];

const fn loose(id: &'static str) -> Piece {
    Piece::prop(id, 1, 1, [Any; 4], 1).orients(ONE)
}

const fn walkable(id: &'static str) -> Piece {
    loose(id).walkable()
}

/// Vocabulary pieces no asset library draws yet (`prop.drain`, a street
/// drain grate). The WFC still places them, so kerbs keep their rhythm and
/// the solve does not change when art arrives, but their props stay out of
/// town layouts: the validator, the fallback log and the renders stay
/// clean. Remove an id here once the libraries carry it.
pub const ART_FREE: &[&str] = &["prop.drain"];

/// Outdoor pieces, in vocabulary order (index 0 is plain ground).
pub const PIECES: &[Piece] = &[
    Piece::plain("ground", 1),
    walkable("prop.drain"),
    loose("prop.barrel"),
    loose("prop.crate"),
    loose("prop.sacks"),
    loose("prop.bucket"),
    loose("prop.wheelbarrow"),
    Piece::prop("prop.woodpile", 1, 1, [Wall, Any, Any, Any], 1),
    loose("prop.lantern").lit(15, WARM),
    loose("prop.signpost"),
    Piece::prop("prop.bench", 2, 1, [Wall, Any, Any, Any], 1),
    loose("prop.hay_bale"),
    Piece::prop("prop.trough", 2, 1, [Any; 4], 1).orients(TWO),
    Piece::prop("prop.cart", 1, 2, [Any; 4], 1).orients(TWO),
    loose("prop.well"),
    Piece::prop("prop.brazier", 1, 1, [Wall, Any, Any, Any], 1).lit(15, FIRE),
    Piece::prop("veg.tree_fruit", 2, 2, [Any; 4], 1).orients(ONE),
    Piece::prop("veg.tree_oak", 3, 3, [Any; 4], 1).orients(ONE),
    Piece::prop("veg.tree_elm", 3, 3, [Any; 4], 1).orients(ONE),
    Piece::prop("veg.tree_birch", 2, 2, [Any; 4], 1).orients(ONE),
    loose("veg.bush"),
    walkable("veg.bush_flowering"),
    walkable("veg.flower_patch"),
    walkable("veg.tall_grass"),
    Piece::prop("prop.grave", 1, 1, [Plain, Any, Plain, Any], 1).orients(ONE),
    Piece {
        name: "loop",
        what: What::Loop,
        w: 1,
        h: 1,
        needs: [Wall, Plain, Any, Plain],
        orients: ALL,
        blocking: false,
        table: false,
        light: None,
        weight: 1,
    },
];

/// The pieces a class may hold, with weights (plain ground first).
#[must_use]
pub fn palette(c: Class) -> &'static [(&'static str, u32)] {
    match c {
        Class::Void => &[("ground", 1)],
        Class::Carriage => &[("ground", 1)],
        Class::KerbPaved => &[
            ("ground", 400),
            ("prop.drain", 14),
            ("prop.barrel", 3),
            ("prop.crate", 3),
            ("prop.lantern", 3),
            ("prop.cart", 2),
        ],
        Class::KerbDirt => &[
            ("ground", 400),
            ("prop.barrel", 3),
            ("prop.crate", 2),
            ("prop.sacks", 2),
            ("prop.wheelbarrow", 1),
            ("prop.cart", 2),
            ("veg.tall_grass", 4),
        ],
        Class::Plaza => &[
            ("ground", 600),
            ("prop.barrel", 3),
            ("prop.crate", 3),
            ("prop.sacks", 3),
            ("prop.cart", 3),
            ("prop.trough", 2),
            ("prop.signpost", 1),
        ],
        Class::PlazaEdge => &[
            ("ground", 300),
            ("prop.bench", 6),
            ("prop.lantern", 3),
            ("prop.barrel", 2),
        ],
        Class::Pitch => &[
            ("ground", 60),
            ("prop.crate", 10),
            ("prop.sacks", 10),
            ("prop.barrel", 8),
        ],
        Class::Front => &[
            ("ground", 200),
            ("prop.bench", 8),
            ("prop.barrel", 6),
            ("prop.crate", 4),
            ("prop.woodpile", 3),
            ("veg.bush_flowering", 4),
        ],
        // A building's yard is dressed for its trade afterwards
        // (`block::yard::working`), sparsely and clear of its doors.
        Class::Yard => &[("ground", 1)],
        Class::Garden => &[
            ("ground", 150),
            ("veg.tree_fruit", 3),
            ("veg.bush_flowering", 5),
            ("prop.bucket", 1),
        ],
        Class::Churchyard => &[
            ("ground", 30),
            ("prop.grave", 40),
            ("veg.tree_elm", 1),
            ("prop.bench", 1),
        ],
        Class::Green => &[
            ("ground", 400),
            ("veg.tree_oak", 1),
            ("prop.bench", 1),
            ("veg.flower_patch", 3),
        ],
        Class::Paddock => &[("ground", 300), ("prop.trough", 2), ("veg.tall_grass", 8)],
        Class::Orchard => &[
            ("ground", 60),
            ("veg.tree_fruit", 10),
            ("veg.tall_grass", 3),
        ],
        Class::Kitchen => &[("ground", 300), ("prop.bucket", 2), ("prop.wheelbarrow", 1)],
        Class::Meadow => &[
            ("ground", 200),
            ("veg.flower_patch", 10),
            ("veg.tall_grass", 8),
        ],
        Class::Workyard => &[
            ("ground", 600),
            ("prop.hay_bale", 4),
            ("prop.woodpile", 3),
            ("prop.cart", 2),
            ("prop.barrel", 2),
        ],
        Class::Rim => &[
            ("ground", 100),
            ("veg.bush", 12),
            ("veg.tree_birch", 3),
            ("veg.tall_grass", 8),
        ],
        Class::Wall => &[("ground", 100), ("loop", 12)],
        Class::Gate => &[("ground", 100), ("prop.brazier", 30)],
    }
}

/// Every class, for building domains.
pub const CLASSES: [Class; 20] = [
    Class::Void,
    Class::Carriage,
    Class::KerbPaved,
    Class::KerbDirt,
    Class::Plaza,
    Class::PlazaEdge,
    Class::Pitch,
    Class::Front,
    Class::Yard,
    Class::Garden,
    Class::Churchyard,
    Class::Green,
    Class::Paddock,
    Class::Orchard,
    Class::Kitchen,
    Class::Meadow,
    Class::Workyard,
    Class::Rim,
    Class::Wall,
    Class::Gate,
];

/// Caps per chunk: `(piece, most)`.
pub const CAPS: &[(&str, u32)] = &[
    ("prop.cart", 2),
    ("prop.well", 1),
    ("prop.lantern", 6),
    ("prop.brazier", 4),
    ("veg.tree_oak", 2),
    ("veg.tree_elm", 2),
    ("prop.signpost", 1),
];
