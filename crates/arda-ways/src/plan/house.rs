//! Toll houses and waystations at major bridges: a small walled building
//! beside the road on one bank, door facing the road (goal 37).

use super::crossing::CrossingPlan;
use super::{square_centre, ChannelPlan, WayPlan};
use crate::input::{CrossingKind, RoadClass, SQUARE_M};
use arda_tactical::noise::hash2;

/// Building length along the road, squares.
pub const LEN_SQ: i64 = 6;
/// Building depth away from the road, squares.
pub const DEPTH_SQ: i64 = 5;
/// Rivers at least this wide make any bridge major, metres.
pub const MAJOR_WIDTH_M: f64 = 18.0;

/// A planned building on the global lattice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct House {
    /// The crossing it serves.
    pub crossing: u64,
    /// Inclusive global rectangle `(gx0, gy0, gx1, gy1)`.
    pub rect: (i64, i64, i64, i64),
    /// Wall kit.
    pub kit: &'static str,
    /// Floor ground key.
    pub floor: &'static str,
    /// Which side of the house faces the road: north, east, south or west.
    pub door_side: Side,
    /// Tag: `toll_house` on highways, `waystation` otherwise.
    pub function: &'static str,
}

/// A rectangle side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// y minimum.
    North,
    /// x maximum.
    East,
    /// y maximum.
    South,
    /// x minimum.
    West,
}

/// A bridge of a crossing record on a highway or a wide river (crossings
/// found only on the refined river are too minor for a toll).
fn major(c: &CrossingPlan) -> bool {
    c.kind == CrossingKind::Bridge
        && c.id & super::wet::SYNTHETIC_ID == 0
        && (c.class == RoadClass::Highway || c.width_m >= MAJOR_WIDTH_M)
}

/// Whether every square of `rect`, grown by one, is dry and clear of roads.
fn clear(rect: (i64, i64, i64, i64), ways: &[WayPlan], channels: &[ChannelPlan]) -> bool {
    for gy in rect.1 - 1..=rect.3 + 1 {
        for gx in rect.0 - 1..=rect.2 + 1 {
            let p = square_centre(gx, gy);
            if channels
                .iter()
                .any(|c| c.dist(p, c.width_m / 2.0 + SQUARE_M).is_some())
            {
                return false;
            }
            for w in ways {
                let band = (f64::from(w.spec.width_sq) / 2.0 + w.spec.verge_sq + 1.0) * SQUARE_M
                    + if w.spec.ditches { SQUARE_M } else { 0.0 };
                if w.dense.nearest(p, band).is_some() {
                    return false;
                }
            }
        }
    }
    true
}

/// Plans a building for every major bridge that has room on a bank.
#[must_use]
pub fn plan_all(
    crossings: &[CrossingPlan],
    ways: &[WayPlan],
    channels: &[ChannelPlan],
    seed: u64,
) -> Vec<House> {
    let mut out = Vec::new();
    for c in crossings.iter().filter(|c| major(c)) {
        let w = &ways[c.way];
        // Clear of the surface, verge and ditch, plus a yard of three squares.
        let deck = c.rows.1 - c.rows.0 + 1;
        let band = f64::from(w.spec.width_sq) / 2.0
            + w.spec.verge_sq
            + if w.spec.ditches { 1.0 } else { 0.0 };
        #[allow(clippy::cast_possible_truncation)] // a few squares
        let gap = (band - (deck as f64) / 2.0).ceil() as i64 + 3;
        // (along start, across start, door faces the road on the near side)
        let mut cands = Vec::new();
        for end in [1, 0] {
            let a0 = if end == 1 {
                c.span.1 + 1
            } else {
                c.span.0 - LEN_SQ
            };
            for side in [1, 0] {
                let r0 = if side == 1 {
                    c.rows.1 + gap
                } else {
                    c.rows.0 - gap - DEPTH_SQ + 1
                };
                cands.push((a0, r0, side));
            }
        }
        let start = usize::try_from(hash2(seed ^ 0x7011, c.id.cast_signed(), 0) % 4).unwrap_or(0);
        for k in 0..4 {
            let (a0, r0, side) = cands[(start + k) % 4];
            let (x0, y0) = c.square(a0, r0);
            let (x1, y1) = c.square(a0 + LEN_SQ - 1, r0 + DEPTH_SQ - 1);
            let rect = (x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1));
            if !clear(rect, ways, channels) {
                continue;
            }
            let door_side = match (c.east_west, side) {
                (true, 1) => Side::North,
                (true, _) => Side::South,
                (false, 1) => Side::West,
                (false, _) => Side::East,
            };
            let stone = c.stone;
            out.push(House {
                crossing: c.id,
                rect,
                kit: if stone { "stone" } else { "timber" },
                floor: if stone { "flagstone" } else { "planks" },
                door_side,
                function: if c.class == RoadClass::Highway {
                    "toll_house"
                } else {
                    "waystation"
                },
            });
            break;
        }
    }
    out
}
