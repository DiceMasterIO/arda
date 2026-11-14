//! What a site adds to single levels (logic/18 §site-levels, §site-links,
//! §site-image): how many levels it has and how big, the seed of each, the
//! danger tier, the links between exits, and the sealed image of the
//! deepest level. Pure functions; where a site lies is the server's.

use crate::{Dungeon, Exit, ExitKind};
use arda_tactical::layout::{AssetRef, TacticalLayout};
use arda_tactical::noise::{hash2, hash_str};
use serde::{Deserialize, Serialize};

/// The only level sizes a site uses, in squares, smallest first.
pub const LEVEL_SIZES: [(u32, u32); 3] = [(32, 24), (48, 36), (72, 56)];

/// How many seeds placement tries for one level before giving the site up.
pub const MAX_ATTEMPTS: u32 = 8;

fn draw(key: u64, what: &str, n: i64) -> u64 {
    hash2(hash_str(key, what), n, 0)
}

/// The sizes of a site's levels in depth order: 1 level with probability
/// 40 %, 2 with 40 %, 3 with 20 %, at least `min_levels`; no level is
/// smaller than the one above (logic/18 §site-levels).
#[must_use]
pub fn level_sizes(key: u64, min_levels: u32) -> Vec<(u32, u32)> {
    let count: u32 = match draw(key, "arda-site levels", 0) % 100 {
        0..=39 => 1,
        40..=79 => 2,
        _ => 3,
    };
    let count = count.max(min_levels.clamp(1, 3));
    let mut at = 0_usize;
    (0..count)
        .map(|i| {
            let left = LEVEL_SIZES.len() - at;
            let step = draw(key, "arda-site size", i64::from(i)) % (left as u64);
            at += usize::try_from(step).unwrap_or(0);
            LEVEL_SIZES[at]
        })
        .collect()
}

/// The seed of level `level` (1-based) of site `key` at `attempt`.
#[must_use]
pub fn level_seed(world_seed: u64, key: u64, level: u32, attempt: u32) -> u64 {
    hash2(
        hash_str(world_seed ^ key.rotate_left(17), "arda-site level"),
        i64::from(level),
        i64::from(attempt),
    )
}

/// The danger tier of level `level` (1-based) of a site whose base tier is
/// `base` (1 or 2): the SRD 5.1 trap tables' character-level bands, 1 to 4.
#[must_use]
pub fn danger_tier(base: u8, level: u32) -> u8 {
    let down = u8::try_from(level.saturating_sub(1)).unwrap_or(u8::MAX);
    base.saturating_add(down).clamp(1, 4)
}

/// Where an exit leads (logic/18 §site-links).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "to")]
pub enum LinkTo {
    /// The site's entrance square on the surface.
    Surface,
    /// Square `(x, y)` of level `level` (1-based).
    Level {
        /// Destination level.
        level: u32,
        /// Arrival column.
        x: u32,
        /// Arrival row.
        y: u32,
    },
    /// Nowhere: the deepest level's stairs down.
    Sealed,
}

/// One exit of a level and where it leads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    /// The exit's kind.
    pub kind: ExitKind,
    /// The exit's column.
    pub x: u32,
    /// The exit's row.
    pub y: u32,
    /// Where it leads.
    pub to: LinkTo,
}

fn exit(d: &Dungeon, down: bool) -> Option<&Exit> {
    d.exits
        .iter()
        .find(|e| (e.kind == ExitKind::StairsDown) == down)
}

/// The links of every level of a site, one per exit in `exits` order:
/// level 1's way up to the surface, each stairs down to the next level's
/// way up and back, the deepest stairs down sealed. An exit with no
/// counterpart (no level generates without both) is sealed.
#[must_use]
pub fn links(levels: &[Dungeon]) -> Vec<Vec<Link>> {
    let to = |i: usize, e: &Exit| -> LinkTo {
        if e.kind == ExitKind::StairsDown {
            levels
                .get(i + 1)
                .and_then(|next| exit(next, false))
                .map_or(LinkTo::Sealed, |up| LinkTo::Level {
                    level: u32::try_from(i + 2).unwrap_or(u32::MAX),
                    x: up.x,
                    y: up.y,
                })
        } else if i == 0 {
            LinkTo::Surface
        } else {
            exit(&levels[i - 1], true).map_or(LinkTo::Sealed, |d| LinkTo::Level {
                level: u32::try_from(i).unwrap_or(u32::MAX),
                x: d.x,
                y: d.y,
            })
        }
    };
    levels
        .iter()
        .enumerate()
        .map(|(i, d)| {
            d.exits
                .iter()
                .map(|e| Link {
                    kind: e.kind,
                    x: e.x,
                    y: e.y,
                    to: to(i, e),
                })
                .collect()
        })
        .collect()
}

/// The layout the deepest level's image is drawn from: the stairs down
/// under their slab (logic/18 §site-image). The level body keeps the
/// layout as generated; only the image is sealed.
#[must_use]
pub fn sealed(layout: &TacticalLayout) -> TacticalLayout {
    let mut out = layout.clone();
    for p in &mut out.placements {
        if matches!(&p.asset, AssetRef::Id(id) if id == "prop.stairs_down") {
            p.asset = AssetRef::Id("prop.stairs_sealed".to_owned());
        }
    }
    out
}
