//! Drainage guarantee for downstream point-sampled lattices
//! (logic/02 §fine-formation sampled drainage).
//!
//! The 100 m prepared bed is a bilinear point sample of the fine field. A
//! drained 39 m valley floor narrower than the sample spacing can fall
//! between samples and leave a closed one-cell depression, which the
//! water balance would publish as a speckle lake. This pass samples the
//! lattice exactly as `arda_core` does, fills it, and raises the four fine
//! support nodes of every filled sample until the sampled lattice drains.

use rayon::prelude::*;

use super::drainage::fill;
use super::lattice::{alloc, Lattice};
use super::FormationError;

/// Deficit from which a filled sample is treated as a pocket and its whole
/// footprint levelled, millimetres.
const POCKET_MM: i32 = 500;

/// Bilinear sample identical to `arda_core::TerrainField::sample` for a
/// lattice with origin 0 (round half away from zero, clamped to corners).
pub(crate) fn sample(g: &Lattice, x_um: i128, y_um: i128) -> i32 {
    let s = i128::from(g.spacing_um);
    let x = (x_um / s).min(g.width as i128 - 2);
    let y = (y_um / s).min(g.height as i128 - 2);
    let (fx, fy) = (x_um - x * s, y_um - y * s);
    let (xi, yi) = (x as usize, y as usize);
    let at = |dx: usize, dy: usize| i128::from(g.z[(yi + dy) * g.width + xi + dx]);
    let raw = [at(0, 0), at(1, 0), at(0, 1), at(1, 1)];
    let n = raw[0] * (s - fx) * (s - fy)
        + raw[1] * fx * (s - fy)
        + raw[2] * (s - fx) * fy
        + raw[3] * fx * fy;
    let d = s * s;
    let r = if n < 0 {
        -((-n + d / 2) / d)
    } else {
        (n + d / 2) / d
    };
    let lo = raw.iter().copied().min().unwrap_or(0);
    let hi = raw.iter().copied().max().unwrap_or(0);
    i32::try_from(r.clamp(lo, hi)).unwrap_or(0)
}

/// Makes the `spacing_um` point-sampled lattice of `g` free of closed
/// depressions over land. Returns the number of fine nodes raised.
///
/// # Errors
/// Allocation failure.
pub fn drain_sampled(
    g: &mut Lattice,
    spacing_um: i64,
    basin: &(dyn Fn(i64, i64) -> bool + Sync),
) -> Result<usize, FormationError> {
    let s = i128::from(spacing_um);
    let span_x = (g.width as i128 - 1) * i128::from(g.spacing_um);
    let span_y = (g.height as i128 - 1) * i128::from(g.spacing_um);
    let (w, h) = ((span_x / s) as usize + 1, (span_y / s) as usize + 1);
    let n = w * h;
    let mut z: Vec<i32> = alloc(n)?;
    let mut filled: Vec<i32> = alloc(n)?;
    let mut flags: Vec<u8> = alloc(n)?;
    let mut next: Vec<u32> = alloc(n)?;
    let mut closed: Vec<u8> = alloc(n)?;
    let mut raised = 0;
    // Each round only raises nodes; at most a handful of rounds are needed
    // because a raise can only newly close a sample whose outlet it lifted.
    for _ in 0..64 {
        {
            let gref = &*g;
            z.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
                for (x, v) in row.iter_mut().enumerate() {
                    *v = sample(gref, x as i128 * s, y as i128 * s);
                }
            });
        }
        // The sea as the hydrology finds it: eight-connected.
        super::drainage::open_sea_flags_d8(&z, w, h, &mut flags);
        // Tectonic basins are deliberate closed depressions.
        flags.par_iter_mut().enumerate().for_each(|(i, f)| {
            let (x, y) = ((i % w) as i64 * spacing_um, (i / w) as i64 * spacing_um);
            if basin(x, y) {
                *f = super::drainage::FIXED;
            }
        });
        filled.copy_from_slice(&z);
        fill(&mut filled, w, h, &flags, 1, &mut next, &mut closed)?;
        let mut changed = 0;
        let gs = i128::from(g.spacing_um);
        // Splat each filled sample's required raise onto its four support
        // nodes, keeping the largest raise per node, so every filled sample
        // rises by at least its deficit (plus rounding margin).
        let mut lift: Vec<(usize, i32)> = Vec::new();
        let mut level: Vec<(usize, i32)> = Vec::new();
        for i in 0..n {
            if filled[i] <= z[i] {
                continue;
            }
            let (px, py) = ((i % w) as i128 * s, (i / w) as i128 * s);
            let x = (px / gs).min(g.width as i128 - 2) as usize;
            let y = (py / gs).min(g.height as i128 - 2) as usize;
            let deficit = filled[i] - z[i] + 1;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                lift.push(((y + dy) * g.width + x + dx, deficit));
            }
            if deficit >= POCKET_MM {
                // A filled pocket (an enclosed lagoon, a closed hollow):
                // level every fine node nearer this sample than any other,
                // not only the four support nodes, or the render shows the
                // raised supports as a 100 m grid.
                let reach = s / 2;
                let lo = |p: i128| ((p - reach + gs - 1) / gs).max(0) as usize;
                let hi = |p: i128, len: usize| (((p + reach) / gs) as usize).min(len - 1);
                for fy in lo(py)..=hi(py, g.height) {
                    for fx in lo(px)..=hi(px, g.width) {
                        level.push((fy * g.width + fx, filled[i]));
                    }
                }
            }
        }
        lift.sort_unstable();
        let mut k = 0;
        while k < lift.len() {
            let (j, mut d) = lift[k];
            while k < lift.len() && lift[k].0 == j {
                d = d.max(lift[k].1);
                k += 1;
            }
            g.z[j] = g.z[j].saturating_add(d);
            changed += 1;
        }
        for &(j, l) in &level {
            g.z[j] = g.z[j].max(l);
        }
        raised += changed;
        if changed == 0 {
            return Ok(raised);
        }
    }
    Ok(raised)
}

#[cfg(test)]
mod tests {
    use super::super::drainage::neighbour;
    use super::*;

    #[test]
    fn sampled_lattice_drains_after_the_pass() {
        // A 39.0625 m field with a narrow diagonal gully the 100 m samples
        // straddle, draining to the sea at x = 0.
        let (w, h) = (257, 257);
        let mut g = Lattice::new(w, h, 39_062_500).unwrap();
        for y in 0..h {
            for x in 0..w {
                let base = 5_000 + 40 * x as i32;
                let gully = if (x + y) % 7 == 0 { -900 } else { 0 };
                g.z[y * w + x] = if x == 0 {
                    -10_000
                } else {
                    base + gully + 300 * ((x * y) % 5) as i32
                };
            }
        }
        drain_sampled(&mut g, 100_000_000, &|_, _| false).unwrap();
        let (sw, sh) = (101, 101);
        let z: Vec<i32> = (0..sw * sh)
            .map(|i| {
                sample(
                    &g,
                    (i % sw) as i128 * 100_000_000,
                    (i / sw) as i128 * 100_000_000,
                )
            })
            .collect();
        for i in 0..sw * sh {
            let (x, y) = (i % sw, i / sw);
            if z[i] <= 0 || x == 0 || y == 0 || x == sw - 1 || y == sh - 1 {
                continue;
            }
            let lower = (0..8).any(|k| neighbour(i, sw, sh, k).is_some_and(|n| z[n] < z[i]));
            assert!(lower, "sample {x},{y} is a closed depression");
        }
    }

    /// A fine field constant over each 100 m sample's footprint, from a
    /// sample-level map (fine nodes take their nearest sample's value).
    fn from_samples(m: impl Fn(i64, i64) -> i32) -> Lattice {
        let (w, h) = (129, 129);
        let mut g = Lattice::new(w, h, 39_062_500).unwrap();
        for y in 0..h {
            for x in 0..w {
                let sx = (x as i64 * 39_062_500 + 50_000_000) / 100_000_000;
                let sy = (y as i64 * 39_062_500 + 50_000_000) / 100_000_000;
                g.z[y * w + x] = m(sx, sy);
            }
        }
        g
    }

    fn lagoon(link: bool) -> impl Fn(i64, i64) -> i32 {
        move |sx, sy| {
            let sea = sx <= 3 && sy <= 3;
            let pool = (5..=12).contains(&sx) && (5..=12).contains(&sy);
            if link && (sx, sy) == (4, 4) {
                // A shallow inlet.
                -500
            } else if sea || pool {
                -5_000
            } else {
                5_000
            }
        }
    }

    #[test]
    fn a_lagoon_joined_to_the_sea_across_a_diagonal_stays_sea() {
        // Regression (seed-5 MICRO lagoon): the hydrology's sea is
        // eight-connected; a four-connected pass filled the lagoon by
        // raising only its samples' support nodes, a 100 m grid texture.
        let mut g = from_samples(lagoon(true));
        drain_sampled(&mut g, 100_000_000, &|_, _| false).unwrap();
        let s = sample(&g, 800_000_000, 800_000_000);
        assert!(s <= -4_000, "lagoon sample raised to {s}");
    }

    #[test]
    fn an_enclosed_pocket_is_levelled_whole_not_on_a_grid() {
        let mut g = from_samples(lagoon(false));
        drain_sampled(&mut g, 100_000_000, &|_, _| false).unwrap();
        // Every fine node inside the pocket rose with its samples.
        let inside: Vec<i32> = (0..g.width * g.height)
            .filter(|&i| {
                let (x, y) = ((i % g.width) as i64, (i / g.width) as i64);
                (15..=30).contains(&x) && (15..=30).contains(&y)
            })
            .map(|i| g.z[i])
            .collect();
        let (lo, hi) = (inside.iter().min().unwrap(), inside.iter().max().unwrap());
        assert!(*lo >= 0, "a fine node left at {lo} mm");
        assert!(hi - lo <= 200, "pocket relief {lo}..{hi} mm");
    }
}
