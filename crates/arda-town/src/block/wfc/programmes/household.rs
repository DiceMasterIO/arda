//! Household variety for WFC homes (goal 64 feeding goal 44).
//!
//! The rule programmes vary a home by its wealth, size, storeys and own
//! stream: a partition (open hall, back chambers, side chamber, front
//! workroom, cross passage), a household trade, the number of beds, an
//! open fire in poor homes and clutter pools by wealth. The WFC homes draw
//! the same choices from the same kind of stream (keyed by the building's
//! interior salt, so adjacent homes that the rules would draw alike are
//! pushed apart) and turn them into room programmes: zones, required
//! pieces and caps. The rule programmes stay the relaxed fill.

use crate::block::frame::Frame;
use crate::block::interior::homes::{partition, Partition};
use crate::block::interior::stock::{household, trade, Trade, BYRE, STORE, TRADES};
use crate::block::wfc::catalogue as c;
use crate::block::wfc::piece::{Piece, What};
use crate::block::wfc::programme::{capped, Item};
use crate::plan::{Building, TownPlan, WealthLevel as W};
use crate::rng::Rng;

/// What a household brings to its rooms.
pub(crate) struct Household {
    /// How the house is partitioned.
    pub partition: Partition,
    /// The household trade, if any.
    pub trade: Option<&'static Trade>,
    /// Most beds.
    pub beds: u8,
    /// An open fire pit instead of a hearth.
    pub pit: bool,
    /// A loom in a cottage.
    pub loom: bool,
    /// The seat at the table.
    pub seat: Piece,
}

/// The household of home `b`.
pub(crate) fn of(plan: &TownPlan, b: &Building) -> Household {
    let salt = u64::from(plan.interior_salt(b));
    let mut rng = Rng::keyed(plan.seed, 0x40E5 ^ (salt << 20), b.id.0);
    let f = Frame::new(b.rect, b.front.turns());
    let w = b.wealth_level;
    // Houses under 20 squares keep one room, as the WFC programme always
    // did; the rule programme partitions by shape alone.
    let partition = match partition(&mut rng, f.w, f.d, w) {
        _ if f.w * f.d < 20 => Partition::Open,
        p => p,
    };
    let trade = trade(&mut rng, w)
        .or_else(|| {
            // A front workroom always has a trade.
            (partition == Partition::Front).then(|| &TRADES[rng.index(TRADES.len())])
        })
        // A trade's bench or loom does not fit beside the hearth and bed
        // of a one-room house under 24 squares.
        .filter(|_| f.w * f.d >= 24);
    let most = u8::try_from((f.w * f.d / 16).clamp(1, 3)).unwrap_or(1);
    let beds = 1 + u8::try_from(rng.index(usize::from(most))).unwrap_or(0);
    let pit = w == W::Poor && rng.chance(0.35);
    let loom = rng.chance(0.25);
    let u = rng.f64();
    let seat = match w {
        W::Poor if u < 0.7 => c::STOOL,
        W::Poor | W::Modest if u < 0.4 => c::BENCH,
        W::Modest if u < 0.75 => c::CHAIR,
        W::Modest => c::STOOL,
        _ => c::CHAIR,
    };
    Household {
        partition,
        trade,
        beds,
        pit,
        loom,
        seat,
    }
}

/// The WFC piece of a vocabulary id the rule programmes use.
fn piece(id: &str) -> Option<Piece> {
    Some(match id {
        "prop.barrel" => c::BARREL,
        "prop.bookshelf" => c::BOOKSHELF,
        "prop.bucket" => c::BUCKET,
        "prop.candle_stand" => c::CANDLES,
        "prop.cask_rack" => c::CASK_RACK,
        "prop.chest" => c::CHEST,
        "prop.crate" => c::CRATE,
        "prop.cupboard" => c::CUPBOARD,
        "prop.hay_bale" => c::HAY,
        "prop.ladder" => c::LADDER,
        "prop.loom" => c::LOOM,
        "prop.sacks" => c::SACKS,
        "prop.shelf" => c::SHELF,
        "prop.stool" => c::STOOL,
        "prop.table" => c::TABLE,
        "prop.woodpile" => c::WOODPILE,
        "prop.workbench" => c::WORKBENCH,
        _ => return None,
    })
}

/// Adds `it` unless the zone already holds its piece.
pub(crate) fn add(items: &mut Vec<Item>, it: Item) {
    if !items.iter().any(|x| x.piece.name == it.piece.name) {
        items.push(it);
    }
}

/// A clutter pool as optional pieces, each capped at two. Clutter draws
/// with a low weight under its own piece name (vocabularies are cached by
/// name), so it adds variety without crowding the floor.
pub(crate) fn pool(items: &mut Vec<Item>, pool: &[(&'static str, u32)]) {
    for &(id, _) in pool {
        let taken = items.iter().any(|x| match x.piece.what {
            What::Prop(p) => p == id,
            _ => false,
        });
        if let (false, Some(p)) = (taken, clutter(id)) {
            add(items, capped(p, 0, 2));
        }
    }
}

/// The low-weight clutter piece of a vocabulary id.
fn clutter(id: &str) -> Option<Piece> {
    let name = match id {
        "prop.barrel" => "clutter.barrel",
        "prop.bookshelf" => "clutter.bookshelf",
        "prop.bucket" => "clutter.bucket",
        "prop.candle_stand" => "clutter.candle_stand",
        "prop.cask_rack" => "clutter.cask_rack",
        "prop.chest" => "clutter.chest",
        "prop.crate" => "clutter.crate",
        "prop.cupboard" => "clutter.cupboard",
        "prop.hay_bale" => "clutter.hay_bale",
        "prop.ladder" => "clutter.ladder",
        "prop.sacks" => "clutter.sacks",
        "prop.shelf" => "clutter.shelf",
        "prop.stool" => "clutter.stool",
        "prop.woodpile" => "clutter.woodpile",
        _ => return None,
    };
    piece(id).map(|p| p.named(name).weighted(1))
}

/// Everyday clutter of a household's wealth.
pub(crate) fn everyday(items: &mut Vec<Item>, w: W) {
    pool(items, household(w));
}

/// Stores and pantries.
pub(crate) fn stores(items: &mut Vec<Item>) {
    pool(items, STORE);
}

/// Byre stock.
pub(crate) fn byre(items: &mut Vec<Item>) {
    pool(items, BYRE);
}

/// A trade's main piece (required) and its clutter.
pub(crate) fn work(items: &mut Vec<Item>, t: &Trade) {
    if let Some(p) = piece(t.main) {
        add(items, capped(p, 1, 1));
    }
    pool(items, t.pool);
}

/// The fire of a hall: a hearth, or an open fire pit.
pub(crate) fn fire(items: &mut Vec<Item>, h: &Household) {
    let p = if h.pit { c::BRAZIER } else { c::HEARTH };
    add(items, capped(p, 1, 1));
}
