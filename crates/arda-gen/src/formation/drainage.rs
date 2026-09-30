//! Drainage on one level lattice: depression filling, receivers, upstream
//! order and contributing area (logic/02 §fine-formation drainage).

use rayon::prelude::*;

use crate::noise::hash_2d;

use super::FormationError;

/// Receiver code for "drains to itself" (base level or local sink).
pub const SELF: u8 = 8;
/// Neighbour offsets; codes 0..4 are cardinal, 4..8 diagonal.
pub const NB: [(i64, i64); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (-1, 1),
    (1, -1),
    (-1, -1),
];
/// Integer slope weights: a height drop over a diagonal counts
/// 128/181 (1/sqrt 2 to 0.01%) of the same drop over a cardinal step.
pub const SLOPE_WEIGHT: [i64; 8] = [181, 181, 181, 181, 128, 128, 128, 128];

/// Per-cell flags.
pub const FIXED: u8 = 1;

#[inline]
pub(crate) fn neighbour(i: usize, width: usize, height: usize, k: usize) -> Option<usize> {
    let (x, y) = ((i % width) as i64, (i / width) as i64);
    let (dx, dy) = NB[k];
    let (nx, ny) = (x + dx, y + dy);
    if nx < 0 || ny < 0 || nx >= width as i64 || ny >= height as i64 {
        return None;
    }
    usize::try_from(ny * width as i64 + nx).ok()
}

/// Marks `FIXED` exactly the open sea: cells at or below sea level that are
/// connected to the domain rim through such cells, plus the rim itself.
/// Enclosed below-sea pockets stay unfixed so a fill lifts them to their
/// spill level instead of publishing them as lakes.
pub fn open_sea_flags(z: &[i32], width: usize, height: usize, flags: &mut [u8]) {
    flags.fill(0);
    let mut stack = Vec::new();
    for i in 0..z.len() {
        let (x, y) = (i % width, i / width);
        if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
            flags[i] = FIXED;
            if z[i] <= 0 {
                stack.push(i);
            }
        }
    }
    while let Some(c) = stack.pop() {
        for k in 0..4 {
            if let Some(n) = neighbour(c, width, height, k) {
                if flags[n] == 0 && z[n] <= 0 {
                    flags[n] = FIXED;
                    stack.push(n);
                }
            }
        }
    }
}

/// [`open_sea_flags`] with eight-neighbour connectivity: the sea exactly as
/// the shared hydrology finds it on a 100 m grid (the D8 component of cells
/// at or below sea level connected to the rim). A lagoon joined to the sea
/// only across a diagonal is sea there; the point-sampled drainage and the
/// shore survey must agree, or they fill or skip what hydrology calls sea.
pub fn open_sea_flags_d8(z: &[i32], width: usize, height: usize, flags: &mut [u8]) {
    flags.fill(0);
    let mut stack = Vec::new();
    for i in 0..z.len() {
        let (x, y) = (i % width, i / width);
        if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
            flags[i] = FIXED;
            if z[i] <= 0 {
                stack.push(i);
            }
        }
    }
    while let Some(c) = stack.pop() {
        for k in 0..8 {
            if let Some(n) = neighbour(c, width, height, k) {
                if flags[n] == 0 && z[n] <= 0 {
                    flags[n] = FIXED;
                    stack.push(n);
                }
            }
        }
    }
}

/// Raises every closed depression to its spill level plus a small gradient
/// so that all water reaches a fixed cell (Barnes 2014 priority-flood with
/// epsilon, exact millimetre bucket queue). `next` and `closed` are scratch.
///
/// # Errors
/// Allocation failure for the bucket heads.
pub fn fill(
    z: &mut [i32],
    width: usize,
    height: usize,
    flags: &[u8],
    eps_mm: i32,
    next: &mut [u32],
    closed: &mut [u8],
) -> Result<(), FormationError> {
    let n = z.len();
    let (lo, hi) = z
        .par_iter()
        .fold(|| (i32::MAX, i32::MIN), |(a, b), &v| (a.min(v), b.max(v)))
        .reduce(
            || (i32::MAX, i32::MIN),
            |(a, b), (c, d)| (a.min(c), b.max(d)),
        );
    if n == 0 {
        return Ok(());
    }
    // Raised cells never exceed hi + n * eps; they are served from the FIFO
    // pit queue, so buckets only need the original range.
    let range = usize::try_from(i64::from(hi) - i64::from(lo) + 1)
        .map_err(|_| FormationError::ArithmeticOverflow)?;
    let mut head: Vec<u32> = super::lattice::alloc(range)?;
    let mut tail: Vec<u32> = super::lattice::alloc(range)?;
    head.fill(u32::MAX);
    const NONE: u32 = u32::MAX;
    closed.fill(0);
    let push_bucket = |head: &mut [u32], tail: &mut [u32], next: &mut [u32], b: usize, i: u32| {
        next[i as usize] = NONE;
        if head[b] == NONE {
            head[b] = i;
        } else {
            next[tail[b] as usize] = i;
        }
        tail[b] = i;
    };
    let bucket = |v: i32| usize::try_from(i64::from(v) - i64::from(lo)).unwrap_or(0);
    for i in 0..n {
        if flags[i] & FIXED != 0 {
            closed[i] = 1;
            push_bucket(&mut head, &mut tail, next, bucket(z[i]), i as u32);
        }
    }
    // Pit FIFO stored as a separate intrusive list through `next` would
    // collide with bucket links, so it uses its own ring vector.
    let mut pit: std::collections::VecDeque<u32> = std::collections::VecDeque::new();
    let mut b = 0_usize;
    loop {
        let c = if let Some(c) = pit.pop_front() {
            c as usize
        } else {
            while b < range && head[b] == NONE {
                b += 1;
            }
            if b >= range {
                break;
            }
            let c = head[b] as usize;
            head[b] = next[c];
            c
        };
        let zc = z[c];
        for k in 0..8 {
            let Some(ni) = neighbour(c, width, height, k) else {
                continue;
            };
            if closed[ni] != 0 {
                continue;
            }
            closed[ni] = 1;
            if z[ni] <= zc {
                z[ni] = zc.saturating_add(eps_mm);
                pit.push_back(ni as u32);
            } else {
                let nb = bucket(z[ni]);
                // A neighbour above the current bucket always sorts later.
                push_bucket(&mut head, &mut tail, next, nb, ni as u32);
                if nb < b {
                    b = nb;
                }
            }
        }
    }
    Ok(())
}

/// Closed pits: non-fixed cells with no strictly lower neighbour, ascending.
fn pits(z: &[i32], width: usize, height: usize, flags: &[u8]) -> Vec<u32> {
    (0..z.len())
        .into_par_iter()
        .filter(|&i| {
            flags[i] & FIXED == 0
                && !(0..8).any(|k| neighbour(i, width, height, k).is_some_and(|n| z[n] < z[i]))
        })
        .map(|i| i as u32)
        .collect()
}

/// Local depression fill: from each closed pit, a small priority flood
/// grows until it pops a cell lower than the running water level; the cell
/// that pushed it is the spill. Every flooded cell is then raised to the
/// spill level plus `eps_mm` per step of breadth-first distance to the
/// spill, so it drains through the spill. Pits are handled in index order,
/// so the result is deterministic. Two local rounds clear almost all pits
/// (MICRO: ~13,000 to ~130); with `exact` any remainder is cleared by the
/// global [`fill`] (logic/02 §fine-formation drainage).
///
/// # Errors
/// Allocation failure in the fallback.
pub fn fill_local(
    z: &mut [i32],
    width: usize,
    height: usize,
    flags: &[u8],
    eps_mm: i32,
    scratch: (&mut [u32], &mut [u8]),
    exact: bool,
) -> Result<(), FormationError> {
    use std::cmp::Reverse;
    let (stamp, closed) = scratch;
    use std::collections::{BinaryHeap, VecDeque};
    for _round in 0..2 {
        let pit_list = pits(z, width, height, flags);
        if pit_list.is_empty() {
            return Ok(());
        }
        // Region stamps: 0 = unvisited; region ids start at 1 each round.
        stamp.par_iter_mut().for_each(|s| *s = 0);
        let mut heap: BinaryHeap<Reverse<(i32, u32, u32)>> = BinaryHeap::new();
        let mut region: Vec<u32> = Vec::new();
        let mut queue: VecDeque<(u32, u32)> = VecDeque::new();
        for (id, &p) in pit_list.iter().enumerate() {
            let p = p as usize;
            // An earlier region may already have drained this pit.
            if (0..8).any(|k| neighbour(p, width, height, k).is_some_and(|n| z[n] < z[p])) {
                continue;
            }
            let rid = u32::try_from(id + 1).map_err(|_| FormationError::ArithmeticOverflow)?;
            heap.clear();
            region.clear();
            heap.push(Reverse((z[p], p as u32, p as u32)));
            stamp[p] = rid;
            let mut level = z[p];
            let mut outlet: Option<usize> = None;
            let popped = rid | 0x4000_0000;
            while let Some(Reverse((zc, c, _from))) = heap.pop() {
                let c = c as usize;
                if zc < level || flags[c] & FIXED != 0 {
                    // A fixed outlet above the water level still drains the
                    // region once the region is raised above it.
                    level = level.max(zc);
                    outlet = Some(c);
                    break;
                }
                level = level.max(zc);
                stamp[c] = popped;
                region.push(c as u32);
                for k in 0..8 {
                    let Some(n) = neighbour(c, width, height, k) else {
                        continue;
                    };
                    if stamp[n] == rid || stamp[n] == popped {
                        continue;
                    }
                    stamp[n] = rid;
                    heap.push(Reverse((z[n], n as u32, c as u32)));
                }
                if region.len() > 1 << 20 {
                    break; // pathological basin: leave it to the global fill
                }
            }
            let Some(outlet) = outlet else { continue };
            // Breadth-first from the outlet (below the water level by
            // construction) through the flooded cells only.
            queue.clear();
            queue.push_back((outlet as u32, 0));
            let done = rid | 0x8000_0000;
            while let Some((c, dist)) = queue.pop_front() {
                let c = c as usize;
                if c != outlet {
                    let target = level.saturating_add(
                        eps_mm.saturating_mul(i32::try_from(dist).unwrap_or(i32::MAX)),
                    );
                    if z[c] < target {
                        z[c] = target;
                    }
                }
                for k in 0..8 {
                    let Some(n) = neighbour(c, width, height, k) else {
                        continue;
                    };
                    if stamp[n] == popped {
                        stamp[n] = done;
                        queue.push_back((n as u32, dist + 1));
                    }
                }
            }
        }
    }
    // Mutually draining depression pairs can survive local rounds; they act
    // as one-iteration sinks unless an exact result is required.
    if !exact || pits(z, width, height, flags).is_empty() {
        return Ok(());
    }
    fill(z, width, height, flags, eps_mm, stamp, closed)
}

/// Receivers: steepest descent, or with probability `stoch_q16 / 65536`
/// a slope-weighted random downhill neighbour chosen by a deterministic
/// hash. The randomisation averages the eight-direction quantisation away
/// over iterations (logic/02 §fine-formation receivers).
pub fn receivers(
    z: &[i32],
    width: usize,
    height: usize,
    flags: &[u8],
    seed: u64,
    stoch_q16: u32,
    rcv: &mut [u8],
) {
    rcv.par_chunks_mut(width).enumerate().for_each(|(y, row)| {
        for (x, out) in row.iter_mut().enumerate() {
            let i = y * width + x;
            *out = SELF;
            if flags[i] & FIXED != 0 {
                continue;
            }
            let mut best = 0_i64;
            let mut weights = [0_i64; 8];
            let mut total = 0_i64;
            for k in 0..8 {
                let Some(ni) = neighbour(i, width, height, k) else {
                    continue;
                };
                let drop = i64::from(z[i]) - i64::from(z[ni]);
                if drop <= 0 {
                    continue;
                }
                let s = drop * SLOPE_WEIGHT[k];
                weights[k] = s;
                total += s;
                if s > best {
                    best = s;
                    *out = k as u8;
                }
            }
            if stoch_q16 == 0 || total == 0 {
                continue;
            }
            let xi = i32::try_from(x).unwrap_or(0);
            let yi = i32::try_from(y).unwrap_or(0);
            if (hash_2d(seed ^ 0x5710_C4A5, xi, yi) >> 16) >= stoch_q16 {
                continue;
            }
            let pick = (i64::from(hash_2d(seed, xi, yi)) * total) >> 32;
            let mut acc = 0_i64;
            for (k, &wk) in weights.iter().enumerate() {
                acc += wk;
                if wk > 0 && pick < acc {
                    *out = k as u8;
                    break;
                }
            }
        }
    });
}

/// Receiver index of cell `i` (itself for [`SELF`]).
#[inline]
pub fn receiver_index(i: usize, width: usize, height: usize, code: u8) -> usize {
    if code == SELF {
        return i;
    }
    neighbour(i, width, height, usize::from(code)).unwrap_or(i)
}

/// Upstream-first topological order (Kahn), written into `order`.
/// `indeg` is scratch.
pub fn upstream_order(
    rcv: &[u8],
    width: usize,
    height: usize,
    indeg: &mut [u8],
    order: &mut [u32],
) {
    indeg.fill(0);
    for (i, &code) in rcv.iter().enumerate() {
        let r = receiver_index(i, width, height, code);
        if r != i {
            indeg[r] = indeg[r].saturating_add(1);
        }
    }
    let mut len = 0;
    for (i, &d) in indeg.iter().enumerate() {
        if d == 0 {
            order[len] = i as u32;
            len += 1;
        }
    }
    let mut head = 0;
    while head < len {
        let i = order[head] as usize;
        head += 1;
        let r = receiver_index(i, width, height, rcv[i]);
        if r != i {
            indeg[r] -= 1;
            if indeg[r] == 0 {
                order[len] = r as u32;
                len += 1;
            }
        }
    }
    debug_assert_eq!(len, rcv.len(), "receiver graph must be acyclic");
}

/// Contributing area in weighted finest-cell units: each cell contributes
/// `unit * weight[i]` (weights Q8), plus any injected inflow.
pub fn accumulate(
    rcv: &[u8],
    width: usize,
    height: usize,
    order: &[u32],
    unit: u64,
    weight: Option<&[u8]>,
    area: &mut [u64],
) {
    match weight {
        Some(weight) => area
            .par_iter_mut()
            .zip(weight.par_iter())
            .for_each(|(a, &w)| *a = unit * u64::from(w)),
        None => area.par_iter_mut().for_each(|a| *a = unit * 256),
    }
    for &i in order {
        let i = i as usize;
        let r = receiver_index(i, width, height, rcv[i]);
        if r != i {
            area[r] += area[i];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rim(width: usize, height: usize) -> Vec<u8> {
        (0..width * height)
            .map(|i| {
                let (x, y) = (i % width, i / width);
                u8::from(x == 0 || y == 0 || x == width - 1 || y == height - 1)
            })
            .collect()
    }

    #[test]
    fn fill_removes_every_closed_pit() {
        let (w, h) = (40, 30);
        let mut z: Vec<i32> = (0..w * h)
            .map(|i| i32::try_from(hash_2d(3, (i % w) as i32, (i / w) as i32) % 5000).unwrap())
            .collect();
        let flags = rim(w, h);
        let mut next = vec![0; w * h];
        let mut closed = vec![0; w * h];
        fill(&mut z, w, h, &flags, 1, &mut next, &mut closed).unwrap();
        for i in 0..w * h {
            if flags[i] & FIXED != 0 {
                continue;
            }
            let lower = (0..8).any(|k| neighbour(i, w, h, k).is_some_and(|n| z[n] < z[i]));
            assert!(lower, "cell {i} remains a pit");
        }
    }

    #[test]
    fn local_fill_removes_every_closed_pit_like_the_global_fill() {
        let (w, h) = (60, 45);
        let mut z: Vec<i32> = (0..w * h)
            .map(|i| {
                let (x, y) = ((i % w) as i32, (i / w) as i32);
                x * 400 + y * 150 + i32::try_from(hash_2d(4, x, y) % 1500).unwrap()
            })
            .collect();
        let flags = rim(w, h);
        let mut stamp = vec![0; w * h];
        let mut closed = vec![0; w * h];
        let before = z.clone();
        fill_local(&mut z, w, h, &flags, 1, (&mut stamp, &mut closed), true).unwrap();
        assert!(pits(&z, w, h, &flags).is_empty());
        assert!(
            z.iter().zip(&before).all(|(a, b)| a >= b),
            "fill only raises"
        );
    }

    #[test]
    fn enclosed_below_sea_pockets_are_filled_not_kept_as_sea() {
        // Sea on the west third; an enclosed -2 m pocket inside the land.
        let (w, h) = (30, 20);
        let mut z: Vec<i32> = (0..w * h)
            .map(|i| {
                if i % w < 10 {
                    -5_000
                } else {
                    3_000 + (i % w) as i32 * 100
                }
            })
            .collect();
        z[10 * w + 20] = -2_000;
        let mut flags = vec![0; w * h];
        open_sea_flags(&z, w, h, &mut flags);
        assert_eq!(flags[10 * w + 5] & FIXED, FIXED, "open sea is fixed");
        assert_eq!(flags[10 * w + 20] & FIXED, 0, "the pocket is not sea");
        let mut next = vec![0; w * h];
        let mut closed = vec![0; w * h];
        fill(&mut z, w, h, &flags, 1, &mut next, &mut closed).unwrap();
        assert!(z[10 * w + 20] > 0, "the pocket is lifted to its spill");
    }

    #[test]
    fn order_is_upstream_first_and_area_conserves_cells() {
        let (w, h) = (25, 19);
        let mut z: Vec<i32> = (0..w * h)
            .map(|i| i32::try_from(hash_2d(9, (i % w) as i32, (i / w) as i32) % 3000).unwrap())
            .collect();
        let flags = rim(w, h);
        let mut next = vec![0; w * h];
        let mut scratch = vec![0; w * h];
        fill(&mut z, w, h, &flags, 1, &mut next, &mut scratch).unwrap();
        let mut rcv = vec![0; w * h];
        receivers(&z, w, h, &flags, 5, 40_000, &mut rcv);
        let mut order = vec![0; w * h];
        upstream_order(&rcv, w, h, &mut scratch, &mut order);
        let mut pos = vec![0; w * h];
        for (p, &i) in order.iter().enumerate() {
            pos[i as usize] = p;
        }
        for i in 0..w * h {
            let r = receiver_index(i, w, h, rcv[i]);
            if r != i {
                assert!(pos[i] < pos[r]);
                assert!(z[r] < z[i], "receiver must be lower");
            }
        }
        let weight = vec![1_u8; w * h];
        let mut area = vec![0; w * h];
        accumulate(&rcv, w, h, &order, 1, Some(&weight), &mut area);
        let outlets: u64 = (0..w * h)
            .filter(|&i| receiver_index(i, w, h, rcv[i]) == i)
            .map(|i| area[i])
            .sum();
        assert_eq!(outlets, (w * h) as u64);
    }
}
