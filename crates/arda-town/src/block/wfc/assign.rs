//! Zones for the rooms of pass A: the assignment of room kinds to the
//! rectangles the room WFC drew that keeps the programme (required rooms,
//! sizes, the entry share, the front door in an entry zone and a doorway
//! tree from the entrance), with the best total fit.

use super::programme::Programme;
use super::rooms::Room;
use arda_wfc::hash::hash3;

/// Whether rooms `a` and `b` share at least one partition edge.
#[must_use]
pub fn touch(p: [i64; 4], q: [i64; 4]) -> bool {
    let xs = p[0].max(q[0]) < p[2].min(q[2]);
    let ys = p[1].max(q[1]) < p[3].min(q[3]);
    (xs && (p[3] == q[1] || q[3] == p[1])) || (ys && (p[2] == q[0] || q[2] == p[0]))
}

/// A doorway tree from the entrance rooms over the programme's door pairs,
/// or `None` when some room cannot be reached.
fn tree(prog: &Programme, rooms: &[Room], entrances: &[usize]) -> Option<Vec<(usize, usize)>> {
    let ok = |a: usize, b: usize| {
        prog.doors
            .iter()
            .any(|&(x, y)| (x, y) == (a, b) || (y, x) == (a, b))
    };
    let mut seen = vec![false; rooms.len()];
    let mut order = Vec::new();
    for &e in entrances {
        if let Some(s) = seen.get_mut(e) {
            if !*s {
                *s = true;
                order.push(e);
            }
        }
    }
    let mut out = Vec::new();
    let mut k = 0;
    while k < order.len() {
        let p = order[k];
        for c in 0..rooms.len() {
            if !seen[c] && touch(rooms[p].rect, rooms[c].rect) && ok(rooms[p].zone, rooms[c].zone) {
                seen[c] = true;
                order.push(c);
                out.push((c, p));
            }
        }
        k += 1;
    }
    seen.iter().all(|&s| s).then_some(out)
}

/// The finished-programme check for a full zone assignment.
fn accept(prog: &Programme, rooms: &[Room], entrances: &[usize]) -> Option<Vec<(usize, usize)>> {
    let total: i64 = rooms.iter().map(Room::area).sum();
    let mut entry = 0;
    for (z, spec) in prog.zones.iter().enumerate() {
        let n = rooms.iter().filter(|r| r.zone == z).count();
        if spec.required && n == 0 {
            return None;
        }
        if spec.entry {
            entry += rooms
                .iter()
                .filter(|r| r.zone == z)
                .map(Room::area)
                .sum::<i64>();
        }
    }
    if entry * 100 < prog.entry_pct * total {
        return None;
    }
    tree(prog, rooms, entrances)
}

/// A doorway tree: `(child, parent)` room pairs.
type Tree = Vec<(usize, usize)>;

struct Search<'a> {
    prog: &'a Programme,
    rects: &'a [[i64; 4]],
    entrances: &'a [usize],
    /// Preference of zone `z` for room `k`, higher is better.
    score: Vec<Vec<i64>>,
    zones: Vec<usize>,
    counts: Vec<u8>,
    best: Option<(i64, Vec<usize>, Tree)>,
}

impl Search<'_> {
    /// Explores every assignment that keeps the programme and remembers
    /// the one with the highest total preference (the first on ties).
    fn go(&mut self, k: usize, total: i64) {
        if k == self.rects.len() {
            if self.best.as_ref().is_some_and(|b| b.0 >= total) {
                return;
            }
            let rooms: Vec<Room> = self
                .rects
                .iter()
                .zip(&self.zones)
                .map(|(&rect, &zone)| Room { zone, rect })
                .collect();
            if let Some(tree) = accept(self.prog, &rooms, self.entrances) {
                self.best = Some((total, self.zones.clone(), tree));
            }
            return;
        }
        for z in 0..self.prog.zones.len() {
            let spec = &self.prog.zones[z];
            let r = self.rects[k];
            let area = (r[2] - r[0]) * (r[3] - r[1]);
            if self.counts[z] >= spec.max_rooms
                || area < spec.min_area
                || (spec.max_area > 0 && area > spec.max_area)
                || (r[2] - r[0]).min(r[3] - r[1]) < spec.min_side
                || (self.entrances.first() == Some(&k) && !spec.entry)
            {
                continue;
            }
            self.zones[k] = z;
            self.counts[z] += 1;
            self.go(k + 1, total + self.score[k][z]);
            self.counts[z] -= 1;
        }
    }
}

/// Rooms with zones, and the doorway tree.
type Assigned = (Vec<Room>, Vec<(usize, usize)>);

/// Most rooms a programme can name: its cap, and no more than its zones'
/// room slots.
pub(super) fn max_rooms(prog: &Programme) -> usize {
    let slots: usize = prog.zones.iter().map(|z| usize::from(z.max_rooms)).sum();
    prog.max_rooms.min(slots).max(1)
}

/// Assigns zones to room rectangles: the assignment that keeps the
/// programme with the best total fit (front and back zones by depth,
/// larger zones in larger rooms, a little hashed jitter).
pub(super) fn assign(
    prog: &Programme,
    rects: &[[i64; 4]],
    entrances: &[usize],
    d: i64,
    seed: u64,
    key: [i64; 3],
) -> Option<Assigned> {
    if rects.len() > max_rooms(prog) {
        return None;
    }
    let score = rects
        .iter()
        .enumerate()
        .map(|(k, r)| {
            let mid = r[1] + r[3];
            (0..prog.zones.len())
                .map(|z| {
                    let spec = &prog.zones[z];
                    let fit = match spec.depth.signum() {
                        1 => 2 * d - mid,
                        -1 => mid,
                        _ => d,
                    };
                    let k = i64::try_from(k).unwrap_or(0);
                    let jitter = i64::try_from(
                        hash3(seed, 0x2A0E, key[0], k, i64::try_from(z).unwrap_or(0)) % 7,
                    )
                    .unwrap_or(0);
                    let size = (r[2] - r[0]) * (r[3] - r[1]) * spec.min_area / 4;
                    fit * 8 * i64::from(spec.depth.unsigned_abs().max(1)) + jitter + size
                })
                .collect()
        })
        .collect();
    let mut s = Search {
        prog,
        rects,
        entrances,
        score,
        zones: vec![0; rects.len()],
        counts: vec![0; prog.zones.len()],
        best: None,
    };
    s.go(0, 0);
    let (_, zones, tree) = s.best?;
    let rooms = rects
        .iter()
        .zip(&zones)
        .map(|(&rect, &zone)| Room { zone, rect })
        .collect();
    Some((rooms, tree))
}
