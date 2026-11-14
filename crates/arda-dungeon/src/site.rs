//! What a site adds to single levels (logic/18 §site-levels, §site-links,
//! §site-image): how many levels it has and how big, the seed of each, the
//! danger tier, the links between exits, and the sealed image of the
//! deepest level. Pure functions; where a site lies is the server's.

use arda_tactical::noise::{hash2, hash_str};

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
