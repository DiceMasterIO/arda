//! Headlands (logic/09 §reservations): worked ground, the ploughland,
//! stubble and fallow of fields, crofts and town gardens, stops short of
//! the shore, open water and a town's built ground, leaving a strip of
//! grass rather than running its furrows to the sand or the house wall
//! (review 2026-10-15, gallery 08 and 10).
//!
//! The rule reads only the composed window within [`SHORE_HEADLAND`] +
//! [`OPENING`] squares of a square, under the [`crate::COMPOSE_MARGIN`] a
//! window is composed with, so overlapping windows agree on every square
//! they share.

use crate::overlays::Owner;
use arda_scene::RulesSidecar;
use arda_tactical::TacticalLayout;
use std::collections::VecDeque;

/// Squares of grass between worked ground and a beach, cliff or open
/// water (about 4.7 m).
pub const SHORE_HEADLAND: u8 = 3;
/// Squares of grass between worked ground and built ground: a building
/// (a town's or a farmstead's), a town's yards and its walls (about
/// 3.1 m).
pub const TOWN_HEADLAND: u8 = 2;
/// Worked ground left narrower than this many squares across by the
/// headlands is grassed too (a morphological opening of radius 1).
pub const OPENING: u8 = 1;
/// The ground a headland replaces worked ground with.
pub const HEADLAND_GROUND: &str = "grass";

const _: () = assert!((SHORE_HEADLAND + 2 * OPENING) as u32 <= crate::pipeline::COMPOSE_MARGIN);

/// Whether a ground key is worked soil (the crisp-edged parcel keys).
#[must_use]
pub fn worked(ground: &str) -> bool {
    matches!(ground, "farmland" | "stubble" | "fallow")
}

/// Whether a town's ground key is built or trodden hard: floors, yards,
/// walls and squares (streets and crofts are not).
fn laid(ground: &str) -> bool {
    matches!(
        ground,
        "planks" | "stone_floor" | "rug" | "flagstone" | "packed_earth"
    )
}

/// Grasses the worked squares of a composed window that lie within the
/// headlands. Returns how many squares it grassed.
pub fn apply(layout: &mut TacticalLayout, rules: &mut RulesSidecar, owners: &[Owner]) -> usize {
    let (w, h) = (layout.width as usize, layout.height as usize);
    let n = w * h;
    if owners.len() != n || layout.squares.len() != n || rules.squares.len() != n {
        return 0;
    }
    let tilled: Vec<bool> = (0..n)
        .map(|i| {
            matches!(owners[i], Owner::Fields | Owner::Croft | Owner::Town)
                && worked(&layout.squares[i].ground)
        })
        .collect();
    let shore: Vec<bool> = owners
        .iter()
        .map(|o| matches!(o, Owner::Shore | Owner::Water))
        .collect();
    let built: Vec<bool> = (0..n)
        .map(|i| {
            let building = rules.squares[i]
                .ext
                .as_ref()
                .is_some_and(|e| e.contains_key("building"));
            building || (owners[i] == Owner::Town && laid(&layout.squares[i].ground))
        })
        .collect();
    let near_shore = within(&shore, (w, h), SHORE_HEADLAND);
    let near_town = within(&built, (w, h), TOWN_HEADLAND);
    let kept: Vec<bool> = (0..n)
        .map(|i| tilled[i] && !near_shore[i] && !near_town[i])
        .collect();
    let opened = open(&kept, (w, h));
    let mut count = 0;
    for i in 0..n {
        if tilled[i] && !opened[i] {
            HEADLAND_GROUND.clone_into(&mut layout.squares[i].ground);
            let cell = &mut rules.squares[i];
            // Grass is not difficult going; the crop and furrow go with
            // the soil.
            cell.difficult = None;
            if let Some(ext) = cell.ext.as_mut() {
                ext.remove("crop");
                ext.remove("furrow");
            }
            count += 1;
        }
    }
    count
}

/// Squares within `reach` (Chebyshev) of a `src` square.
fn within(src: &[bool], (w, h): (usize, usize), reach: u8) -> Vec<bool> {
    let mut dist = vec![u8::MAX; src.len()];
    let mut queue = VecDeque::new();
    for (i, &s) in src.iter().enumerate() {
        if s {
            dist[i] = 0;
            queue.push_back(i);
        }
    }
    while let Some(i) = queue.pop_front() {
        let d = dist[i];
        if d >= reach {
            continue;
        }
        let (x, y) = (i % w, i / w);
        for (dx, dy) in NEIGHBOURS {
            let (Some(nx), Some(ny)) = (x.checked_add_signed(dx), y.checked_add_signed(dy)) else {
                continue;
            };
            if nx < w && ny < h {
                let j = ny * w + nx;
                if dist[j] > d + 1 {
                    dist[j] = d + 1;
                    queue.push_back(j);
                }
            }
        }
    }
    dist.iter().map(|&d| d <= reach).collect()
}

const NEIGHBOURS: [(isize, isize); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// The opening of `mask` by a 3 × 3 square: erode, then dilate within the
/// mask. Squares beyond the window count as the nearest edge square.
fn open(mask: &[bool], (w, h): (usize, usize)) -> Vec<bool> {
    let at = |m: &[bool], x: isize, y: isize| {
        let cx = usize::try_from(x.max(0))
            .unwrap_or(0)
            .min(w.saturating_sub(1));
        let cy = usize::try_from(y.max(0))
            .unwrap_or(0)
            .min(h.saturating_sub(1));
        m.get(cy * w + cx).copied().unwrap_or(false)
    };
    let around = |m: &[bool], i: usize, all: bool| {
        let (x, y) = (
            isize::try_from(i % w).unwrap_or(0),
            isize::try_from(i / w).unwrap_or(0),
        );
        let mut v = NEIGHBOURS.iter().map(|&(dx, dy)| at(m, x + dx, y + dy));
        if all {
            v.all(|b| b)
        } else {
            v.any(|b| b)
        }
    };
    let eroded: Vec<bool> = (0..mask.len())
        .map(|i| mask[i] && around(mask, i, true))
        .collect();
    (0..mask.len())
        .map(|i| mask[i] && (eroded[i] || around(&eroded, i, false)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `w × h` window of ploughland owned by fields.
    fn window(w: u32, h: u32) -> (TacticalLayout, RulesSidecar, Vec<Owner>) {
        let layout = TacticalLayout::new("t", w, h, "farmland");
        let rules = RulesSidecar::empty(w, h);
        let owners = vec![Owner::Fields; (w * h) as usize];
        (layout, rules, owners)
    }

    #[test]
    fn ploughland_stops_short_of_the_beach_and_the_houses() {
        let (w, h) = (24_u32, 16_u32);
        let (mut layout, mut rules, mut owners) = window(w, h);
        // A beach down the west edge, a building in the east.
        for y in 0..h as usize {
            let i = y * w as usize;
            owners[i] = Owner::Shore;
            "sand".clone_into(&mut layout.squares[i].ground);
        }
        for y in 6..9 {
            for x in 18..21 {
                let i = y * w as usize + x;
                owners[i] = Owner::Town;
                "planks".clone_into(&mut layout.squares[i].ground);
            }
        }
        let n = apply(&mut layout, &mut rules, &owners);
        assert!(n > 0);
        let ground = |x: usize, y: usize| layout.squares[y * w as usize + x].ground.as_str();
        for y in 0..h as usize {
            for x in 1..=usize::from(SHORE_HEADLAND) {
                assert_eq!(ground(x, y), "grass", "({x}, {y}) by the beach");
            }
        }
        assert_eq!(ground(16, 7), "grass", "two squares from the wall");
        assert_eq!(ground(10, 7), "farmland", "the field's heart is ploughed");
        assert_eq!(ground(0, 3), "sand", "the beach is untouched");
    }

    #[test]
    fn a_thin_rib_of_ploughland_left_between_headlands_is_grassed() {
        let (w, h) = (20_u32, 11_u32);
        let (mut layout, mut rules, mut owners) = window(w, h);
        // Water above and below leaves a one-square rib in the middle row.
        for x in 0..w as usize {
            for y in [1_usize, 9] {
                owners[y * w as usize + x] = Owner::Water;
                "water_shallow".clone_into(&mut layout.squares[y * w as usize + x].ground);
            }
        }
        apply(&mut layout, &mut rules, &owners);
        assert!(layout.squares.iter().all(|s| s.ground != "farmland"));
    }

    #[test]
    fn natural_ground_and_pasture_are_left_alone() {
        let (w, h) = (8_u32, 8_u32);
        let (mut layout, mut rules, mut owners) = window(w, h);
        for s in &mut layout.squares {
            "pasture".clone_into(&mut s.ground);
        }
        owners[0] = Owner::Water;
        assert_eq!(apply(&mut layout, &mut rules, &owners), 0);
    }
}
