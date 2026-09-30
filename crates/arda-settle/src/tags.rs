//! Per-cell site tags (spec step 2; artifact "Where people settle").
//!
//! Tags are bits in a `u32` per cell. Point features (a ford, a harbour
//! mouth, a saddle) are found first, then every land cell within a short
//! walk of one inherits the tag, so a settlement "at the ford" sits on the
//! dry terrace beside it rather than in the stream.

use crate::error::SettleError;
use crate::field::{distance_m, Integral};
use crate::grid::{filled, Grid};
use crate::ore;
use arda::{Cover, TerrainKind};

/// Within a short walk of a fordable reach: 2–14 m wide, order ≤ 3, firm banks.
pub const FORD: u32 = 1;
/// Within a short walk of a bridgeable reach: wider than a ford, firm banks.
pub const BRIDGE: u32 = 1 << 1;
/// Near the confluence of two sizeable rivers.
pub const CONFLUENCE: u32 = 1 << 2;
/// Beside a sheltered, deep bay.
pub const HARBOUR: u32 = 1 << 3;
/// Near the head of an estuary (a river mouth of order ≥ 3).
pub const ESTUARY: u32 = 1 << 4;
/// Near a saddle between two basins.
pub const PASS: u32 = 1 << 5;
/// A height standing well above its surrounding kilometre.
pub const DEFENSIBLE: u32 = 1 << 6;
/// Ore proxy: rock outcrop with strong relief, in patchy deposits.
pub const ORE: u32 = 1 << 7;
/// Timber: closed forest across the surrounding kilometre.
pub const TIMBER: u32 = 1 << 8;
/// Fish: sea, lake or a large river within reach.
pub const FISH: u32 = 1 << 9;
/// Salt: warm, flat, low coast or salt marsh.
pub const SALT: u32 = 1 << 10;
/// Spring line: near the head of a channel.
pub const SPRING: u32 = 1 << 11;
/// Within 150 m of a river of order ≥ 2.
pub const RIVER: u32 = 1 << 12;
/// Within 300 m of a lake.
pub const LAKE: u32 = 1 << 13;
/// Within 300 m of the sea.
pub const COAST: u32 = 1 << 14;
/// A hill with a view (prominence ≥ 20 m).
pub const HILL: u32 = 1 << 15;
/// Among forest.
pub const FOREST: u32 = 1 << 16;
/// Beside marsh or fen.
pub const MARSH: u32 = 1 << 17;
/// In the mountains.
pub const MOUNTAIN: u32 = 1 << 18;
/// Within 300 m of a navigable river (order ≥ 4).
pub const NAVIGABLE: u32 = 1 << 19;
/// Arable: gentle, warm and moist enough, above flood level, clearable cover.
pub const ARABLE: u32 = 1 << 20;
/// Refused as a settlement site.
pub const REFUSED: u32 = 1 << 21;

/// Tag names in bit order, for JSON output.
pub const NAMES: [(u32, &str); 20] = [
    (FORD, "ford"),
    (BRIDGE, "bridge_site"),
    (CONFLUENCE, "confluence"),
    (HARBOUR, "harbour"),
    (ESTUARY, "estuary"),
    (PASS, "pass"),
    (DEFENSIBLE, "defensible"),
    (ORE, "ore"),
    (TIMBER, "timber"),
    (FISH, "fish"),
    (SALT, "salt"),
    (SPRING, "spring"),
    (RIVER, "river"),
    (LAKE, "lake"),
    (COAST, "coast"),
    (HILL, "hill"),
    (FOREST, "forest"),
    (MARSH, "marsh"),
    (MOUNTAIN, "mountain"),
    (NAVIGABLE, "navigable"),
];

/// Names of every descriptive tag set in `bits`.
#[must_use]
pub fn names(bits: u32) -> Vec<String> {
    NAMES
        .iter()
        .filter(|(b, _)| bits & b != 0)
        .map(|(_, n)| (*n).to_string())
        .collect()
}

/// Ford reach width range in metres (artifact: two to fourteen metres).
const FORD_WIDTH_M: (u32, u32) = (2, 14);
/// Radius of the "short walk" that carries a point tag to nearby land.
const WALK_M: u32 = 300;
/// Saddle test offset and height margin.
const SADDLE_R: i64 = 10;
const SADDLE_MARGIN_MM: i32 = 30_000;

/// Whether a cell can be ploughed (artifact "arable").
#[must_use]
pub fn arable(g: &Grid, i: usize) -> bool {
    g.is_land(i)
        && g.order[i] == 0
        && matches!(g.cover[i], Cover::Grass | Cover::Scrub | Cover::Forest)
        && g.slope_md[i] <= 8000
        && g.temp_cc[i] >= 400
        && g.moisture[i] >= 60
        && g.har_dm[i] >= 10
}

/// Whether a cell is refused as a settlement site (artifact refusal list).
#[must_use]
pub fn refused(g: &Grid, i: usize) -> bool {
    !g.is_land(i)
        || g.order[i] > 0
        || matches!(g.cover[i], Cover::Marsh | Cover::Rock | Cover::Ice)
        || (g.cover[i] == Cover::Bare && g.is_coast(i))
        || g.slope_md[i] > 14_000
        || g.temp_cc[i] < 300
        || g.har_dm[i] < 15
}

/// Derived per-cell layers the placement stage reads.
#[derive(Debug, Clone)]
pub struct Sites {
    /// Tag bits per cell.
    pub tags: Vec<u32>,
    /// Distance to the nearest open water (watercourse, lake or sea), metres.
    pub water_m: Vec<u32>,
    /// Prominence above the surrounding kilometre, metres (clamped ≥ 0).
    pub prominence_m: Vec<u16>,
}

/// Computes every tag for every cell.
///
/// # Errors
/// [`SettleError::Reserve`] when a raster cannot be allocated.
pub fn compute(g: &Grid, seed: u64) -> Result<Sites, SettleError> {
    let n = g.len();
    let mut tags = filled(n, 0_u32, "site tags")?;
    let heights = Integral::new(g.width, g.height, |i| i64::from(g.height_mm[i]))?;
    let mut prominence_m = filled(n, 0_u16, "prominence")?;
    for (i, p) in prominence_m.iter_mut().enumerate() {
        if g.is_land(i) {
            let (x, y) = g.xy(i);
            let d = (i64::from(g.height_mm[i]) - heights.mean(x, y, 10)) / 1000;
            *p = u16::try_from(d.clamp(0, 9999)).unwrap_or(0);
        }
    }

    let near = |src: &dyn Fn(usize) -> bool, within: u32| -> Result<Vec<bool>, SettleError> {
        let d = distance_m(g, src, within + 1)?;
        Ok(d.into_iter().map(|v| v <= within).collect())
    };
    let bank_firm = |i: usize| {
        g.neighbours8(i)
            .map(|(j, _, _)| j)
            .filter(|&j| g.is_land(j) && g.order[j] == 0)
            .all(|j| g.cover[j] != Cover::Marsh && g.slope_md[j] <= 8000)
    };
    let ford = near(
        &|i| {
            let w = g.width_m(i);
            g.is_watercourse(i)
                && (FORD_WIDTH_M.0..=FORD_WIDTH_M.1).contains(&w)
                && g.order[i] <= 3
                && g.slope_md[i] <= 4000
                && bank_firm(i)
        },
        200,
    )?;
    let bridge = near(
        &|i| g.is_watercourse(i) && g.width_m(i) > FORD_WIDTH_M.1 && bank_firm(i),
        200,
    )?;
    let mut conf_mask = filled(n, false, "confluence mask")?;
    for &c in &g.confluences {
        if let Some(m) = conf_mask.get_mut(c) {
            *m = true;
        }
    }
    let confluence = near(&|i| conf_mask[i], WALK_M)?;
    let sea = |i: usize| g.terrain[i] == TerrainKind::Sea;
    let mouth = near(
        &|i| g.is_watercourse(i) && g.order[i] >= 3 && g.neighbours8(i).any(|(j, _, _)| sea(j)),
        500,
    )?;
    let harbour_cells = harbours(g)?;
    let harbour = near(&|i| harbour_cells[i], WALK_M)?;
    let saddle = saddles(g)?;
    let pass = near(&|i| saddle[i], WALK_M)?;
    let head = near(
        &|i| {
            g.is_watercourse(i)
                && g.order[i] == 1
                && !g
                    .neighbours8(i)
                    .any(|(j, _, _)| g.is_watercourse(j) && g.drainage[j] < g.drainage[i])
        },
        WALK_M,
    )?;
    let river = near(&|i| g.is_watercourse(i) && g.order[i] >= 2, 150)?;
    let big_river = near(&|i| g.is_watercourse(i) && g.order[i] >= 3, 200)?;
    let navigable = near(&|i| g.is_watercourse(i) && g.order[i] >= 4, WALK_M)?;
    let lake = near(&|i| g.terrain[i] == TerrainKind::Lake, WALK_M)?;
    let coast = near(&sea, WALK_M)?;
    let marsh = near(&|i| g.is_land(i) && g.cover[i] == Cover::Marsh, WALK_M)?;
    let rock = Integral::new(g.width, g.height, |i| {
        i64::from(g.is_land(i) && g.cover[i] == Cover::Rock)
    })?;
    let forest = Integral::new(g.width, g.height, |i| {
        i64::from(g.is_land(i) && g.cover[i] == Cover::Forest)
    })?;
    let slope = Integral::new(g.width, g.height, |i| i64::from(g.slope_md[i]))?;

    for i in 0..n {
        if !g.is_land(i) {
            continue;
        }
        let (x, y) = g.xy(i);
        let mut t = 0;
        let mut set = |cond: bool, bit: u32| {
            if cond {
                t |= bit;
            }
        };
        set(ford[i], FORD);
        set(bridge[i], BRIDGE);
        set(confluence[i], CONFLUENCE);
        set(harbour[i], HARBOUR);
        set(mouth[i], ESTUARY);
        set(pass[i], PASS);
        set(prominence_m[i] >= 40, DEFENSIBLE);
        set(prominence_m[i] >= 20, HILL);
        set(head[i], SPRING);
        set(river[i], RIVER);
        set(navigable[i], NAVIGABLE);
        set(lake[i], LAKE);
        set(coast[i], COAST);
        set(marsh[i], MARSH);
        set(forest.permille(x, y, 3) >= 500, FOREST);
        set(forest.permille(x, y, 10) >= 750, TIMBER);
        set(coast[i] || lake[i] || big_river[i], FISH);
        let mean_slope = slope.mean(x, y, 10);
        set(g.height_mm[i] >= 900_000 || mean_slope >= 25_000, MOUNTAIN);
        // Ore proxy (`ore.rs`, documented in the README): rough rocky
        // ground inside a seeded mineral district.
        set(
            ore::deposit(rock.permille(x, y, 10), mean_slope) && ore::in_district(seed, x, y),
            ORE,
        );
        set(
            coast[i]
                && ((g.temp_cc[i] >= 1100 && g.slope_md[i] <= 2000 && g.height_mm[i] <= 5000)
                    || marsh[i]),
            SALT,
        );
        set(arable(g, i), ARABLE);
        set(refused(g, i), REFUSED);
        tags[i] = t;
    }
    let water_m = distance_m(g, |i| g.is_water(i) || g.is_watercourse(i), 5000)?;
    Ok(Sites {
        tags,
        water_m,
        prominence_m,
    })
}

/// Ray directions (x, y) × 1000 at 22.5° steps.
const RAYS: [(i64, i64); 16] = [
    (1000, 0),
    (924, 383),
    (707, 707),
    (383, 924),
    (0, 1000),
    (-383, 924),
    (-707, 707),
    (-924, 383),
    (-1000, 0),
    (-924, -383),
    (-707, -707),
    (-383, -924),
    (0, -1000),
    (383, -924),
    (707, -707),
    (924, -383),
];

/// Rays of this many cells that meet no land count as open to the sea.
const RAY_CELLS: i64 = 40;

/// Number of the 16 rays from `i` that run 4 km over open water.
#[must_use]
pub fn open_rays(g: &Grid, i: usize) -> u32 {
    let (x, y) = g.xy(i);
    let mut open = 0;
    for &(dx, dy) in &RAYS {
        let mut blocked = false;
        for t in 1..=RAY_CELLS {
            let px = x + (dx * t + 500 * dx.signum()) / 1000;
            let py = y + (dy * t + 500 * dy.signum()) / 1000;
            match g.at(px, py) {
                None => break,
                Some(j) if g.is_land(j) => {
                    blocked = true;
                    break;
                }
                Some(_) => {}
            }
        }
        if !blocked {
            open += 1;
        }
    }
    open
}

/// Coastal cells on a sheltered, deep bay: open to the sea through one to
/// four of sixteen rays (a mouth no wider than a right angle) with water at
/// least 3 m deep within 300 m.
fn harbours(g: &Grid) -> Result<Vec<bool>, SettleError> {
    let mut out = filled(g.len(), false, "harbour mask")?;
    for (i, o) in out.iter_mut().enumerate() {
        if !g.is_coast(i) {
            continue;
        }
        let rays = open_rays(g, i);
        if !(1..=4).contains(&rays) {
            continue;
        }
        let (x, y) = g.xy(i);
        let deep = (-3..=3_i64).any(|oy| {
            (-3..=3_i64).any(|ox| {
                g.at(x + ox, y + oy)
                    .is_some_and(|j| g.terrain[j] == TerrainKind::Sea && g.height_mm[j] <= -3000)
            })
        });
        *o = deep;
    }
    Ok(out)
}

/// Saddle cells: higher ground 1 km away on both sides along one axis,
/// lower ground on both sides across it, and the lowest such cell nearby.
fn saddles(g: &Grid) -> Result<Vec<bool>, SettleError> {
    let mut cand = filled(g.len(), false, "saddle mask")?;
    let axes = [((1, 0), (0, 1)), ((1, 1), (1, -1))];
    for (i, c) in cand.iter_mut().enumerate() {
        if !g.is_land(i) || g.height_mm[i] < 150_000 {
            continue;
        }
        let (x, y) = g.xy(i);
        let h = g.height_mm[i];
        let at = |dx: i64, dy: i64| {
            g.at(x + dx * SADDLE_R, y + dy * SADDLE_R)
                .map(|j| g.height_mm[j])
        };
        let side = |(dx, dy): (i64, i64), higher: bool| {
            let (a, b) = (at(dx, dy), at(-dx, -dy));
            match (a, b) {
                (Some(a), Some(b)) if higher => {
                    a >= h + SADDLE_MARGIN_MM && b >= h + SADDLE_MARGIN_MM
                }
                (Some(a), Some(b)) => a <= h - SADDLE_MARGIN_MM && b <= h - SADDLE_MARGIN_MM,
                _ => false,
            }
        };
        *c = axes
            .iter()
            .any(|&(p, q)| (side(p, true) && side(q, false)) || (side(q, true) && side(p, false)));
    }
    let mut out = filled(g.len(), false, "saddle points")?;
    for i in 0..g.len() {
        if !cand[i] {
            continue;
        }
        let (x, y) = g.xy(i);
        let h = g.height_mm[i];
        let lowest = (-5..=5_i64).all(|oy| {
            (-5..=5_i64).all(|ox| {
                g.at(x + ox, y + oy).is_none_or(|j| {
                    !cand[j] || g.height_mm[j] > h || (g.height_mm[j] == h && j >= i)
                })
            })
        });
        out[i] = lowest;
    }
    Ok(out)
}
