//! Wall kit pieces on a building's shell by WFC (goal 44, walls): each
//! shell edge takes a `run` or a `window` piece. Edge constraints decide:
//! doors and gates are fixed from the plan, corners and party walls stay
//! runs, a wall that furniture stands against stays a run, and windows
//! never sit side by side or beside a door. One row per side.

use arda_wfc::{solve, Dir, Problem, Rules, TileSet};

/// A plain run.
pub const RUN: u16 = 0;
/// A window.
pub const WINDOW: u16 = 1;
/// A fixed piece (door, gate) or no edge.
pub const FIXED: u16 = 2;

/// What an edge may take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// A run or a window.
    Free,
    /// A run only.
    Run,
    /// A door, gate or no edge at all.
    Fixed,
}

fn rules() -> Option<Rules> {
    let mut r = Rules::new(3, 1).ok()?;
    for (a, b) in [(0, 0), (0, 1), (1, 0), (0, 2), (2, 0), (2, 2)] {
        r.allow(a, Dir::E, b, 0);
    }
    for a in 0..3 {
        for b in 0..3 {
            r.allow(a, Dir::S, b, 0);
        }
    }
    Some(r)
}

/// Chooses pieces for `sides` (each a row of edge slots, clockwise from
/// the front). Returns the piece per slot and whether the relaxed fill
/// (all runs) was used.
#[must_use]
pub fn choose(
    sides: &[Vec<Slot>],
    window_weight: u32,
    seed: u64,
    key: [i64; 3],
) -> (Vec<Vec<u16>>, bool) {
    let relaxed = || {
        let rows = sides
            .iter()
            .map(|s| {
                s.iter()
                    .map(|&x| if x == Slot::Fixed { FIXED } else { RUN })
                    .collect()
            })
            .collect();
        (rows, true)
    };
    let Some(r) = rules() else { return relaxed() };
    let w = sides.iter().map(Vec::len).max().unwrap_or(0);
    let h = sides.len();
    if w == 0 {
        return (sides.iter().map(|_| Vec::new()).collect(), false);
    }
    let mut domains = vec![TileSet::single(usize::from(FIXED)); w * h];
    for (y, s) in sides.iter().enumerate() {
        for (x, &slot) in s.iter().enumerate() {
            domains[y * w + x] = match slot {
                Slot::Free => TileSet::of(&[0, 1]),
                Slot::Run => TileSet::single(0),
                Slot::Fixed => TileSet::single(2),
            };
        }
    }
    let weight = |_: usize, t: usize| if t == 1 { window_weight } else { 5 };
    let p = Problem {
        rules: &r,
        w,
        h,
        domains,
        edges: vec![[0; 4]; w * h],
        weight: &weight,
        anchors: Vec::new(),
        limits: Vec::new(),
        seed,
        key,
    };
    match solve(&p, arda_wfc::MAX_ATTEMPTS, &|_| true) {
        Ok(sol) => (
            sides
                .iter()
                .enumerate()
                .map(|(y, s)| sol.tiles[y * w..y * w + s.len()].to_vec())
                .collect(),
            false,
        ),
        Err(_) => relaxed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_never_touch_each_other_or_doors() {
        let side = vec![
            Slot::Run,
            Slot::Free,
            Slot::Free,
            Slot::Fixed,
            Slot::Free,
            Slot::Free,
            Slot::Free,
            Slot::Run,
        ];
        let (rows, relaxed) = choose(&[side.clone(), side], 5, 42, [1, 2, 3]);
        assert!(!relaxed);
        for r in rows {
            assert_eq!(r[3], FIXED);
            for k in 1..r.len() {
                assert!(!(r[k] == WINDOW && (r[k - 1] == WINDOW || r[k - 1] == FIXED)));
                assert!(!(r[k - 1] == WINDOW && r[k] == FIXED));
            }
        }
    }
}
