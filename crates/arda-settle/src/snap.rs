//! Border snapping (spec §realm-seats step 3, `logic/06`): realm borders
//! move onto a river or ridge line that runs within a couple of cells.
//!
//! The raw partition's border runs along the equal-cost line between two
//! seats. Every land cell within [`BAND`] cells of it is handed out again
//! by a small flood from the cells just outside the band, which keep their
//! realm. The flood moves freely but pays a heavy toll for stepping off a
//! natural line onto open ground, so each realm fills the band up to the
//! line on its own side and the border settles on the line. Where no line
//! runs through the band, the two floods meet near the old border.
//!
//! The pass then restores a valid partition: a fragment of a realm cut off
//! from its seat goes to the neighbour it shares the most border with,
//! unless it is alone on its own island. Seats never move.

use crate::error::SettleError;
use crate::grid::{filled, Grid, OFFSETS8};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

/// Cells within this many steps of the raw border are reassigned.
pub const BAND: u8 = 3;
/// Snapping floods run in turn, each around the border the last one left.
const PASSES: usize = 2;
/// Cost of one step inside the band.
const STEP: u32 = 10;
/// Toll for stepping off a natural line onto open ground.
const OFF_LINE: u32 = 1000;
/// A border edge counts as on a line when a cell beside it is within
/// this many steps of one.
const ON_LINE: i64 = 1;
/// A border edge can be snapped when a line lies within this many cells.
const REACH: i64 = 2;

const OFFSETS4: [(i64, i64); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

/// Whether land cell `i` touches a land cell of another realm (4-neighbours).
fn on_border(g: &Grid, map: &[u16], i: usize) -> bool {
    let (x, y) = g.xy(i);
    OFFSETS4.iter().any(|&(dx, dy)| {
        g.at(x + dx, y + dy)
            .is_some_and(|j| map[j] != 0 && map[j] != map[i])
    })
}

/// Snaps borders onto `line` cells and repairs the partition in place.
/// `fixed` cells (the seats) keep their realm.
///
/// # Errors
/// [`SettleError::Reserve`] when a raster cannot be allocated.
pub fn snap(g: &Grid, map: &mut [u16], line: &[bool], fixed: &[usize]) -> Result<(), SettleError> {
    // The barrier is the line grown by one cell, which closes the small gaps
    // of a patchy crest so a flood cannot leak round a fragment's end.
    let mut wall = filled(g.len(), false, "snap wall")?;
    for (i, w) in wall.iter_mut().enumerate() {
        let (x, y) = g.xy(i);
        *w = line[i]
            || OFFSETS8
                .iter()
                .any(|&(dx, dy)| g.at(x + dx, y + dy).is_some_and(|j| line[j]));
    }
    for _ in 0..PASSES {
        flood(g, map, &wall, fixed)?;
    }
    repair(g, map, fixed)
}

/// One snapping flood over the band around the current border.
fn flood(g: &Grid, map: &mut [u16], line: &[bool], fixed: &[usize]) -> Result<(), SettleError> {
    // Steps from the raw border, up to BAND + 1.
    let mut depth = filled(g.len(), u8::MAX, "border depth")?;
    let mut queue = std::collections::VecDeque::new();
    for i in 0..g.len() {
        if map[i] != 0 && on_border(g, map, i) {
            depth[i] = 0;
            queue.push_back(i);
        }
    }
    while let Some(a) = queue.pop_front() {
        if depth[a] > BAND {
            continue;
        }
        let (x, y) = g.xy(a);
        for &(dx, dy) in &OFFSETS4 {
            if let Some(b) = g.at(x + dx, y + dy) {
                if map[b] != 0 && depth[b] == u8::MAX {
                    depth[b] = depth[a] + 1;
                    queue.push_back(b);
                }
            }
        }
    }
    let mut cost = filled(g.len(), u32::MAX, "snap cost")?;
    let mut heap = BinaryHeap::new();
    for i in 0..g.len() {
        if depth[i] == BAND + 1 || (depth[i] <= BAND && fixed.contains(&i)) {
            cost[i] = 0;
            heap.push(Reverse((0_u32, map[i], i)));
        }
    }
    let in_band = |i: usize| depth[i] <= BAND && !fixed.contains(&i);
    let mut claimed = filled(g.len(), 0_u16, "snap claims")?;
    while let Some(Reverse((c, id, a))) = heap.pop() {
        if c > cost[a] || (claimed[a] != 0 && claimed[a] != id) {
            continue;
        }
        claimed[a] = id;
        let (x, y) = g.xy(a);
        for &(dx, dy) in &OFFSETS4 {
            let Some(b) = g.at(x + dx, y + dy) else {
                continue;
            };
            if !in_band(b) {
                continue;
            }
            let toll = if line[a] && !line[b] { OFF_LINE } else { 0 };
            let nc = c + STEP + toll;
            if nc < cost[b] || (nc == cost[b] && id < claimed[b]) {
                cost[b] = nc;
                claimed[b] = id;
                heap.push(Reverse((nc, id, b)));
            }
        }
    }
    for i in 0..g.len() {
        if in_band(i) && claimed[i] != 0 {
            map[i] = claimed[i];
        }
    }
    Ok(())
}

/// Gives every realm fragment cut off from its seat to the neighbour it
/// borders most, unless the fragment touches no other realm (an island).
fn repair(g: &Grid, map: &mut [u16], fixed: &[usize]) -> Result<(), SettleError> {
    for _ in 0..8 {
        let mut comp = filled(g.len(), u32::MAX, "realm components")?;
        let mut members: Vec<Vec<usize>> = Vec::new();
        for s in 0..g.len() {
            if map[s] == 0 || comp[s] != u32::MAX {
                continue;
            }
            let k = u32::try_from(members.len()).unwrap_or(u32::MAX);
            let mut cells = vec![s];
            comp[s] = k;
            let mut at = 0;
            while at < cells.len() {
                let a = cells[at];
                at += 1;
                let (x, y) = g.xy(a);
                for &(dx, dy) in &OFFSETS8 {
                    if let Some(b) = g.at(x + dx, y + dy) {
                        if map[b] == map[a] && comp[b] == u32::MAX {
                            comp[b] = k;
                            cells.push(b);
                        }
                    }
                }
            }
            members.push(cells);
        }
        let seat_comp: Vec<u32> = fixed.iter().map(|&c| comp[c]).collect();
        let mut changed = false;
        for (k, cells) in members.iter().enumerate() {
            let k = u32::try_from(k).unwrap_or(u32::MAX);
            if seat_comp.contains(&k) {
                continue;
            }
            let mut touch: BTreeMap<u16, u32> = BTreeMap::new();
            for &a in cells {
                let (x, y) = g.xy(a);
                for &(dx, dy) in &OFFSETS8 {
                    if let Some(b) = g.at(x + dx, y + dy) {
                        if map[b] != 0 && map[b] != map[a] {
                            *touch.entry(map[b]).or_default() += 1;
                        }
                    }
                }
            }
            let Some((&to, _)) = touch.iter().max_by_key(|(&id, &n)| (n, Reverse(id))) else {
                continue;
            };
            for &a in cells {
                map[a] = to;
            }
            changed = true;
        }
        if !changed {
            break;
        }
    }
    Ok(())
}

/// Border length on a natural line, per mille: of the border that has a
/// line within [`REACH`] cells, and of all border.
///
/// # Errors
/// [`SettleError::Reserve`] when a raster cannot be allocated.
pub fn on_line_share(g: &Grid, map: &[u16], line: &[bool]) -> Result<(u32, u32), SettleError> {
    let near = |i: usize, r: i64| {
        let (x, y) = g.xy(i);
        (-r..=r).any(|oy| (-r..=r).any(|ox| g.at(x + ox, y + oy).is_some_and(|j| line[j])))
    };
    // 0 not yet read, 1 no line near, 2 a line within REACH, 3 on a line.
    let mut state = filled(g.len(), 0_u8, "line reach")?;
    let (mut border, mut snappable, mut snapped) = (0_u64, 0_u64, 0_u64);
    for i in 0..g.len() {
        if map[i] == 0 {
            continue;
        }
        let (x, y) = g.xy(i);
        for (dx, dy) in [(1_i64, 0_i64), (0, 1)] {
            let Some(j) = g.at(x + dx, y + dy) else {
                continue;
            };
            if map[j] == 0 || map[j] == map[i] {
                continue;
            }
            for c in [i, j] {
                if state[c] == 0 {
                    state[c] = if near(c, ON_LINE) {
                        3
                    } else if near(c, REACH) {
                        2
                    } else {
                        1
                    };
                }
            }
            border += 1;
            if state[i] >= 2 || state[j] >= 2 {
                snappable += 1;
                snapped += u64::from(state[i] == 3 || state[j] == 3);
            }
        }
    }
    let pm = |a: u64, b: u64| u32::try_from(a * 1000 / b.max(1)).unwrap_or(0);
    Ok((pm(snapped, snappable), pm(snapped, border)))
}
