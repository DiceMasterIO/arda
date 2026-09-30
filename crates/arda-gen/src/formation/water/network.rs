//! Main-stem stream network on a drained fine lattice (logic/02
//! §world-water network).
//!
//! Steepest-descent receivers and contributing area, then a main-stem
//! decomposition: at every confluence the larger donor continues, so each
//! channel cell belongs to exactly one stream, listed upstream to
//! downstream. Each stream carries its smoothed valley centreline, valley
//! direction, bed profile, contributing area and valley-floor width.

use rayon::prelude::*;

use super::super::drainage::{
    neighbour, open_sea_flags, receiver_index, receivers, upstream_order, SELF,
};
use super::super::lattice::{alloc, Lattice};
use super::super::FormationError;
use super::geom::{isqrt_i, unit, CELL_Q8};

/// Steepest-descent drainage of the lattice.
pub struct Network {
    /// Receiver direction codes ([`SELF`] for outlets and sinks).
    pub rcv: Vec<u8>,
    /// Contributing area in lattice cells.
    pub area: Vec<u32>,
    /// Direction code of the largest donor, or [`SELF`] for none.
    pub main: Vec<u8>,
}

/// Builds receivers, area and main donors for `g` against the open sea.
///
/// # Errors
/// Allocation failure.
pub fn build(g: &Lattice) -> Result<Network, FormationError> {
    let (w, h) = (g.width, g.height);
    let n = w * h;
    let mut flags: Vec<u8> = alloc(n)?;
    open_sea_flags(&g.z, w, h, &mut flags);
    let mut rcv: Vec<u8> = alloc(n)?;
    receivers(&g.z, w, h, &flags, 0, 0, &mut rcv);
    drop(flags);
    let mut area: Vec<u32> = alloc(n)?;
    {
        let mut order: Vec<u32> = alloc(n)?;
        let mut indeg: Vec<u8> = alloc(n)?;
        upstream_order(&rcv, w, h, &mut indeg, &mut order);
        area.fill(1);
        for &i in &order {
            let i = i as usize;
            let r = receiver_index(i, w, h, rcv[i]);
            if r != i {
                area[r] = area[r].saturating_add(area[i]);
            }
        }
    }
    let mut main: Vec<u8> = alloc(n)?;
    {
        let (rcv, area) = (&rcv, &area);
        main.par_iter_mut().enumerate().for_each(|(i, m)| {
            *m = SELF;
            let mut best = 0_u32;
            for k in 0..8 {
                let Some(nb) = neighbour(i, w, h, k) else {
                    continue;
                };
                if rcv[nb] != SELF && receiver_index(nb, w, h, rcv[nb]) == i && area[nb] > best {
                    best = area[nb];
                    *m = k as u8;
                }
            }
        });
    }
    Ok(Network { rcv, area, main })
}

/// One main stem, upstream to downstream. The last cell is where it ends:
/// a confluence cell of a larger stream, the sea, or a sink.
pub struct Stream {
    /// Lattice indices.
    pub cells: Vec<usize>,
    /// Smoothed valley centreline, Q8.
    pub centre: Vec<(i64, i64)>,
    /// Down-valley unit vector, Q14.
    pub dir: Vec<(i64, i64)>,
    /// Arc length along the centreline from the head, Q8.
    pub along: Vec<i64>,
}

impl Stream {
    /// Bed height at stream position `k`.
    #[must_use]
    pub fn bed(&self, g: &Lattice, k: usize) -> i64 {
        i64::from(g.z[self.cells[k]])
    }

    /// Valley bed slope around `k` over `±half` positions, parts per million.
    #[must_use]
    pub fn slope_ppm(&self, g: &Lattice, k: usize, half: usize) -> i64 {
        let a = k.saturating_sub(half);
        let b = (k + half).min(self.cells.len() - 1);
        let run_q8 = (self.along[b] - self.along[a]).max(1);
        // mm over (Q8 / 256 cells × spacing mm): ppm = mm × 1e6 / mm.
        let run_mm = i128::from(run_q8) * i128::from(g.spacing_um / 1000) / i128::from(CELL_Q8);
        let drop = i128::from(self.bed(g, a) - self.bed(g, b)).max(0);
        i64::try_from(drop * 1_000_000 / run_mm.max(1)).unwrap_or(i64::MAX)
    }
}

/// Extracts every main stem whose cells reach `min_cells` of contributing
/// area, heads in index order (deterministic), with centrelines smoothed
/// over `±smooth` positions.
#[must_use]
pub fn streams(g: &Lattice, net: &Network, min_cells: u32, smooth: usize) -> Vec<Stream> {
    let (w, h) = (g.width, g.height);
    let mut out = Vec::new();
    for head in 0..w * h {
        if net.area[head] < min_cells || g.z[head] <= 0 {
            continue;
        }
        // A head: no donor of its own is a channel cell.
        let m = net.main[head];
        if m != super::super::drainage::SELF
            && neighbour(head, w, h, usize::from(m)).is_some_and(|d| net.area[d] >= min_cells)
        {
            continue;
        }
        let mut cells = vec![head];
        let mut cur = head;
        loop {
            let r = receiver_index(cur, w, h, net.rcv[cur]);
            if r == cur {
                break;
            }
            cells.push(r);
            let main_stem = neighbour(r, w, h, usize::from(net.main[r])) == Some(cur);
            if !main_stem || g.z[r] <= 0 {
                break;
            }
            cur = r;
        }
        if cells.len() < 4 {
            continue;
        }
        out.push(geometry(cells, w, smooth));
    }
    out
}

fn geometry(cells: Vec<usize>, w: usize, smooth: usize) -> Stream {
    let m = cells.len();
    let pts: Vec<(i64, i64)> = cells
        .iter()
        .map(|&c| ((c % w) as i64 * CELL_Q8, (c / w) as i64 * CELL_Q8))
        .collect();
    // Prefix sums give O(1) window means.
    let mut sx = vec![0_i64; m + 1];
    let mut sy = vec![0_i64; m + 1];
    for (k, p) in pts.iter().enumerate() {
        sx[k + 1] = sx[k] + p.0;
        sy[k + 1] = sy[k] + p.1;
    }
    let centre: Vec<(i64, i64)> = (0..m)
        .map(|k| {
            let (a, b) = (k.saturating_sub(smooth), (k + smooth + 1).min(m));
            let n = (b - a) as i64;
            ((sx[b] - sx[a]) / n, (sy[b] - sy[a]) / n)
        })
        .collect();
    let dir = (0..m)
        .map(|k| {
            let (a, b) = (k.saturating_sub(smooth), (k + smooth).min(m - 1));
            unit(centre[b].0 - centre[a].0, centre[b].1 - centre[a].1)
        })
        .collect();
    let mut along = vec![0_i64; m];
    for k in 1..m {
        let (dx, dy) = (centre[k].0 - centre[k - 1].0, centre[k].1 - centre[k - 1].1);
        along[k] = along[k - 1] + isqrt_i(i128::from(dx * dx + dy * dy)).max(1);
    }
    Stream {
        cells,
        centre,
        dir,
        along,
    }
}

/// Valley-floor extent across a stream at `k`: distances (Q8) to the left
/// and right of the centreline at which the surface first rises more than
/// `rise_mm` above the bed, marching up to `max_q8`.
#[must_use]
pub fn floor_width(g: &Lattice, s: &Stream, k: usize, rise_mm: i64, max_q8: i64) -> (i64, i64) {
    let bed = s.bed(g, k);
    let (dx, dy) = s.dir[k];
    let normal = (-dy, dx);
    let c = s.centre[k];
    let side = |sign: i64| {
        let mut d = 0;
        while d < max_q8 {
            let next = d + CELL_Q8 / 2;
            let p = (
                c.0 + sign * normal.0 * next / super::geom::ONE_Q14,
                c.1 + sign * normal.1 * next / super::geom::ONE_Q14,
            );
            let z = i64::from(super::geom::height_at(g, p));
            if z <= 0 || z > bed + rise_mm {
                break;
            }
            d = next;
        }
        d
    };
    (side(1), side(-1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tilted_valley_yields_one_main_stem_to_the_sea() {
        // V-valley along y draining south into sea rows.
        let (w, h) = (41, 120);
        let mut g = Lattice::new(w, h, 39_062_500).unwrap();
        for y in 0..h {
            for x in 0..w {
                let across = (x as i32 - 20).abs() * 2_000;
                g.z[y * w + x] = if y >= h - 4 {
                    -20_000
                } else {
                    200_000 - y as i32 * 1_000 + across
                };
            }
        }
        let net = build(&g).unwrap();
        let s = streams(&g, &net, 400, 3);
        assert_eq!(s.len(), 1, "one trunk");
        let t = &s[0];
        assert_eq!(t.cells[0] % w, 20);
        assert!(g.z[*t.cells.last().unwrap()] <= 0, "ends at the sea");
        assert!(t.dir[t.cells.len() / 2].1 > 16_000, "flows south");
        let slope = t.slope_ppm(&g, t.cells.len() / 2, 10);
        // 1 m per 39.0625 m cell.
        assert!((25_000..=26_000).contains(&slope), "{slope}");
        let (l, r) = floor_width(&g, t, 40, 2_500, 20 * CELL_Q8);
        assert!((CELL_Q8 / 2..=CELL_Q8 * 2).contains(&l), "{l}");
        assert!((CELL_Q8 / 2..=CELL_Q8 * 2).contains(&r), "{r}");
    }
}
