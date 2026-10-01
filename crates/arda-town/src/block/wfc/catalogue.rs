//! The indoor piece catalogue: canonical vocabulary props (vocabulary.md)
//! as WFC pieces, with the wall, front and seat needs that make furniture
//! respect walls, access and function. Sizes follow
//! `block::interior::size` so pieces draw the same footprints as the rule
//! programmes.

use super::piece::{Need, Piece, What, ALL, ONE, TWO};
use crate::block::interior::{CANDLE, FIRE};
use Need::{Any, Table, Walk, Wall};

const FORGE_LIGHT: [u8; 3] = [255, 140, 60];

/// Open floor.
pub const FLOOR: Piece = Piece::plain("floor", 100);

/// A doorway through a partition (placed by door anchors only).
pub const DOOR: Piece = Piece {
    name: "doorway",
    what: What::Door,
    w: 1,
    h: 1,
    needs: [Need::Door, Any, Any, Any],
    orients: ALL,
    blocking: false,
    table: false,
    light: None,
    weight: 0,
};

const fn against(id: &'static str, w: i64, weight: u32) -> Piece {
    Piece::prop(id, w, 1, [Wall, Any, Any, Any], weight)
}

const fn facing(id: &'static str, w: i64, weight: u32) -> Piece {
    Piece::prop(id, w, 1, [Wall, Any, Walk, Any], weight)
}

const fn loose(id: &'static str, weight: u32) -> Piece {
    Piece::prop(id, 1, 1, [Any; 4], weight).orients(ONE)
}

/// A hearth against a wall with room in front, lit.
pub const HEARTH: Piece = facing("prop.hearth", 2, 6).lit(20, FIRE);
/// An oven against a wall, lit.
pub const OVEN: Piece = facing("prop.oven", 2, 6).lit(15, FIRE);
/// A forge against a wall, lit.
pub const FORGE: Piece = facing("prop.forge", 2, 6).lit(20, FORGE_LIGHT);
/// A workbench against a wall.
pub const WORKBENCH: Piece = facing("prop.workbench", 2, 5);
/// A loom against a wall.
pub const LOOM: Piece = facing("prop.loom", 2, 5);
/// An altar against a wall, facing the room.
pub const ALTAR: Piece = facing("prop.altar", 2, 6).lit(10, CANDLE);
/// A throne against a wall.
pub const THRONE: Piece = facing("prop.throne", 1, 6);
/// Stairs to the upper floor, foot clear.
pub const STAIRS: Piece = Piece::prop("prop.stairs", 1, 2, [Wall, Any, Walk, Any], 5);
/// A bar counter with room in front.
pub const BAR: Piece = Piece::prop("prop.bar_counter", 2, 1, [Any, Any, Walk, Any], 4);
/// A cask rack against a wall.
pub const CASK_RACK: Piece = against("prop.cask_rack", 2, 5);
/// A shelf against a wall.
pub const SHELF: Piece = against("prop.shelf", 2, 5);
/// A free-standing storage rack worked from its front (warehouse racks
/// laid out in rows by `super::rows`; drawn with the 2 x 1 cask rack).
pub const RACK: Piece = Piece::prop("prop.cask_rack", 2, 1, [Walk, Any, Any, Any], 5).named("rack");
/// A bookshelf against a wall.
pub const BOOKSHELF: Piece = against("prop.bookshelf", 2, 6);
/// A cupboard against a wall.
pub const CUPBOARD: Piece = against("prop.cupboard", 1, 4);
/// A weapon rack against a wall.
pub const WEAPON_RACK: Piece = against("prop.weapon_rack", 2, 5);
/// An armour stand against a wall.
pub const ARMOUR_STAND: Piece = against("prop.armour_stand", 1, 4);
/// A trough against a wall.
pub const TROUGH: Piece = against("prop.trough", 2, 4);
/// A woodpile against a wall.
pub const WOODPILE: Piece = against("prop.woodpile", 1, 4);
/// A chest against a wall.
pub const CHEST: Piece = against("prop.chest", 1, 5);
/// A ladder against a wall.
pub const LADDER: Piece = against("prop.ladder", 1, 2);
/// A statue against a wall.
pub const STATUE: Piece = against("prop.statue", 1, 4);
/// A banner on a wall (walkable).
pub const BANNER: Piece = against("prop.banner", 1, 3).walkable();
/// A bed, head to a wall.
pub const BED: Piece = Piece::prop("prop.bed", 1, 2, [Wall, Any, Any, Any], 6);
/// A table.
pub const TABLE: Piece = Piece::prop("prop.table", 2, 1, [Any; 4], 4)
    .orients(TWO)
    .table();
/// A chair at a table (seats can be stepped past: walkable).
pub const CHAIR: Piece = Piece::prop("prop.chair", 1, 1, [Any, Any, Table, Any], 30).walkable();
/// A stool at a table (walkable).
pub const STOOL: Piece = Piece::prop("prop.stool", 1, 1, [Any, Any, Table, Any], 30).walkable();
/// A bench at a table (walkable).
pub const BENCH: Piece = Piece::prop("prop.bench", 2, 1, [Any, Any, Table, Any], 30).walkable();
/// A pew, facing the chancel (canonical south), with legroom behind and
/// in front so every row opens onto the aisle.
pub const PEW: Piece = Piece::prop("prop.pew", 2, 1, [Walk, Any, Walk, Any], 14).orients(ONE);
/// A candle stand, lit.
pub const CANDLES: Piece = against("prop.candle_stand", 1, 2).lit(10, CANDLE);
/// A brazier, lit.
pub const BRAZIER: Piece = loose("prop.brazier", 2).lit(15, FIRE);
/// An anvil with room to work on one side.
pub const ANVIL: Piece = Piece::prop("prop.anvil", 1, 1, [Walk, Any, Any, Any], 4);
/// A grindstone.
pub const GRINDSTONE: Piece = loose("prop.grindstone", 2);
/// A barrel against a wall.
pub const BARREL: Piece = against("prop.barrel", 1, 5);
/// A crate against a wall.
pub const CRATE: Piece = against("prop.crate", 1, 5);
/// Sacks against a wall.
pub const SACKS: Piece = against("prop.sacks", 1, 5);
/// A bucket.
pub const BUCKET: Piece = loose("prop.bucket", 1);
/// A hay bale.
pub const HAY: Piece = against("prop.hay_bale", 1, 6);
/// A hay cart.
pub const HAYCART: Piece = Piece::prop("prop.haycart", 1, 2, [Walk, Any, Any, Any], 3);
/// A millstone.
pub const MILLSTONE: Piece = Piece::prop("prop.millstone", 2, 2, [Any, Any, Walk, Any], 6);
/// A rowboat on its trestles.
pub const ROWBOAT: Piece = Piece::prop("prop.rowboat", 1, 2, [Any, Walk, Any, Any], 6);
/// A small rug (walkable).
pub const RUG_SMALL: Piece = Piece::prop("prop.rug_small", 2, 1, [Any; 4], 2)
    .orients(TWO)
    .walkable();
/// A 3 × 2 rug painted as the `rug` ground accent (walkable).
pub const RUG: Piece = Piece {
    name: "rug",
    what: What::Ground("rug"),
    w: 3,
    h: 2,
    needs: [Any; 4],
    orients: TWO,
    blocking: false,
    table: false,
    light: None,
    weight: 3,
};
