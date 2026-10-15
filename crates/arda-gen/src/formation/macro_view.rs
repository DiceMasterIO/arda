//! The macro surface as formation sees it (logic/02 §fine-formation macro
//! warp, masks and basins): the 1 km lattices, their warped view, and the
//! large closed depressions that become endorheic sinks.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use rayon::prelude::*;

use super::drainage::{self, FIXED};
use super::lattice::{alloc, blur_into, Lattice};
use super::{incision, FormationError, BASIN_DEPTH_MM, BASIN_MIN_CELLS, LOWSTAND_MM};
use crate::continent::ContinentGrid;
use crate::noise::value_noise;

/// Macro surface and its local relief as 1 km lattices (mm and m).
pub(super) fn macro_lattices(grid: &ContinentGrid) -> Result<(Lattice, Lattice), FormationError> {
    let (w, h) = (grid.width() as usize, grid.height() as usize);
    let mut m = Lattice::new(w, h, 1_000_000_000)?;
    for y in 0..h {
        for x in 0..w {
            m.z[y * w + x] = grid.get(x as i32, y as i32).raw();
        }
    }
    let rel = local_relief(&m)?;
    Ok((m, rel))
}

/// Radius of the local-relief window, macro cells.
const RELIEF_RADIUS: usize = 12;

/// logic/02 §fine-formation masks: local standard deviation (m) of land
/// height over a ~25 km window measures tectonic relief.
fn local_relief(m: &Lattice) -> Result<Lattice, FormationError> {
    let (w, h) = (m.width, m.height);
    let land_m: Vec<i32> = m.z.iter().map(|&v| v.max(0) / 1000).collect();
    let sq: Vec<i32> = land_m.iter().map(|&v| v * v).collect();
    let mut tmp = alloc(w * h)?;
    let mut mean = alloc(w * h)?;
    let mut mean_sq = alloc(w * h)?;
    blur_into(&land_m, w, h, RELIEF_RADIUS, &mut tmp, &mut mean);
    blur_into(&sq, w, h, RELIEF_RADIUS, &mut tmp, &mut mean_sq);
    let mut rel = Lattice::new(w, h, m.spacing_um)?;
    for i in 0..w * h {
        let var = i64::from(mean_sq[i]) - i64::from(mean[i]) * i64::from(mean[i]);
        rel.z[i] = incision::isqrt(var.max(0) as u64) as i32;
    }
    Ok(rel)
}

/// Recipe 9 (review 3.1-10): after the basin audit fills unexplained
/// basins on `after` (`before` is the macro surface it audited), the local
/// relief of every macro cell within the relief window of a filled cell is
/// recomputed from the filled surface. Elsewhere `relief` keeps its values,
/// including the volcanic cones' full relief. Returns the cells updated.
///
/// # Errors
/// Allocation failure.
pub(super) fn refresh_relief(
    before: &[i32],
    after: &Lattice,
    relief: &mut Lattice,
) -> Result<usize, FormationError> {
    let (w, h) = (after.width, after.height);
    let changed: Vec<i32> = before
        .iter()
        .zip(&after.z)
        .map(|(a, b)| i32::from(a != b) * 1_000)
        .collect();
    if changed.iter().all(|&c| c == 0) {
        return Ok(0);
    }
    let (mut tmp, mut near) = (alloc(w * h)?, alloc(w * h)?);
    blur_into(&changed, w, h, RELIEF_RADIUS, &mut tmp, &mut near);
    let fresh = local_relief(after)?;
    let mut updated = 0;
    for (i, &k) in near.iter().enumerate() {
        if k > 0 && relief.z[i] != fresh.z[i] {
            relief.z[i] = fresh.z[i];
            updated += 1;
        }
    }
    Ok(updated)
}

#[cfg(test)]
mod relief_tests {
    use super::*;

    /// Review 3.1-10: the basin audit fills an unexplained 300 m bowl, but
    /// the level masks kept reading the relief of the unfilled bowl. Recipe
    /// 9 recomputes it around the fill.
    #[test]
    fn filled_basins_lose_their_relief() {
        let (w, h) = (80, 80);
        let mut m = Lattice::new(w, h, 1_000_000_000).unwrap();
        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = (x as i64 - 40, y as i64 - 40);
                m.z[y * w + x] = if dx * dx + dy * dy < 15 * 15 {
                    100_000
                } else {
                    400_000
                };
            }
        }
        let mut relief = local_relief(&m).unwrap();
        let rim = 40 * w + 40 + 15;
        let stale = relief.z[rim];
        assert!(stale >= 100, "the bowl's rim reads {stale} m of relief");
        let before = m.z.clone();
        m.z.fill(400_000);
        assert!(refresh_relief(&before, &m, &mut relief).unwrap() > 0);
        assert_eq!(relief.z[rim], 0, "a filled bowl is flat");
        // Far from the fill nothing changes.
        assert_eq!(refresh_relief(&m.z.clone(), &m, &mut relief).unwrap(), 0);
    }
}

/// The macro surface and its relief as formation sees them: domain-warped
/// by two octaves of value noise (±20 km at 120 km, ±6 km at 45 km), tapered
/// to zero within 40 km of the domain edge so the forced ocean rim holds.
/// The warp bends range crests, basin walls and macro coasts that the
/// plate model draws along near-straight Voronoi edges (logic/02
/// §fine-formation macro warp). Its peak slope stays below one.
///
/// Recipe 5 samples the plain Catmull-Rom surface, reads its level masks
/// from the local relief (`belt` is the relief lattice) and treats every
/// cell below the lowstand as open ocean (`ocean` is empty). Recipe 6
/// clamps the cubic, reads belt relief and a lowstand-ocean mask.
#[derive(Clone, Copy)]
pub(super) struct MacroView<'a> {
    pub height: &'a Lattice,
    pub relief: &'a Lattice,
    pub belt: &'a Lattice,
    /// Lowstand ocean on the macro lattice ([`super::basins::lowstand_ocean`]);
    /// empty for recipe 5.
    pub ocean: &'a [u8],
    pub seed: u64,
    pub extent_um: (i64, i64),
    /// Recipe-6 sampling (clamped cubic, ocean mask).
    pub v6: bool,
    /// Recipe-7 annual runoff and standing-water deficit (mm/yr) on the
    /// macro lattice (logic/02 §fine-formation climate runoff); `None`
    /// before recipe 7, which assumed a nominal runoff.
    pub water: Option<(&'a Lattice, &'a Lattice)>,
    /// The recipe being formed; recipe-9 corrections test it.
    pub recipe: super::Recipe,
}

impl MacroView<'_> {
    fn warp(&self, x_um: i64, y_um: i64) -> (i64, i64) {
        let edge = x_um
            .min(y_um)
            .min(self.extent_um.0 - x_um)
            .min(self.extent_um.1 - y_um)
            .max(0);
        let taper = (edge * 4096 / 40_000_000_000).min(4096);
        if taper == 0 {
            return (x_um, y_um);
        }
        let (xm, ym) = (
            i32::try_from(x_um / 1_000_000).unwrap_or(0),
            i32::try_from(y_um / 1_000_000).unwrap_or(0),
        );
        let n = |salt: u64| {
            i64::from(value_noise(self.seed ^ salt, xm, ym, 120_000)) * 20_000_000_000 / 32_768
                + i64::from(value_noise(self.seed ^ salt ^ 0x55, xm, ym, 45_000)) * 6_000_000_000
                    / 32_768
        };
        (
            x_um + n(0x3A11) * taper / 4096,
            y_um + n(0x3A12) * taper / 4096,
        )
    }

    pub(super) fn height(&self, x_um: i64, y_um: i64) -> i32 {
        let (x, y) = self.warp(x_um, y_um);
        if self.v6 {
            self.height.sample_um_bounded(x, y)
        } else {
            self.height.sample_um(x, y)
        }
    }

    pub(super) fn relief_m(&self, x_um: i64, y_um: i64) -> i64 {
        let (x, y) = self.warp(x_um, y_um);
        i64::from(self.relief.sample_um(x, y)).max(0)
    }

    /// Whether the macro cell nearest a point is lowstand ocean. Recipe 5
    /// has no mask: everything below the lowstand is open ocean.
    pub(super) fn is_ocean(&self, x_um: i64, y_um: i64) -> bool {
        if !self.v6 {
            return true;
        }
        let (x, y) = self.warp(x_um, y_um);
        let s = self.height.spacing_um;
        let (w, h) = (self.height.width as i64, self.height.height as i64);
        let cx = ((x + s / 2) / s).clamp(0, w - 1);
        let cy = ((y + s / 2) / s).clamp(0, h - 1);
        self.ocean
            .get((cy * w + cx) as usize)
            .is_some_and(|&o| o != 0)
    }

    /// Annual runoff (mm/yr) at a point, recipe 7 only.
    pub(super) fn runoff_mm(&self, x_um: i64, y_um: i64) -> Option<i64> {
        let (runoff, _) = self.water?;
        let (x, y) = self.warp(x_um, y_um);
        Some(i64::from(runoff.sample_um_bounded(x, y)).max(0))
    }

    /// Extra annual loss of standing water (mm/yr) at a point, recipe 7 only.
    pub(super) fn deficit_mm(&self, x_um: i64, y_um: i64) -> Option<i64> {
        let (_, deficit) = self.water?;
        let (x, y) = self.warp(x_um, y_um);
        Some(i64::from(deficit.sample_um_bounded(x, y)).max(0))
    }

    /// Belt relief (see [`super::relief::belt_relief`]) for the level
    /// masks; the local relief for recipe 5.
    pub(super) fn belt_m(&self, x_um: i64, y_um: i64) -> i64 {
        let (x, y) = self.warp(x_um, y_um);
        i64::from(self.belt.sample_um(x, y)).max(0)
    }
}

/// Sink points (µm) of large closed macro depressions, found on the base
/// lattice (logic/02 §fine-formation basins). Depths of small or shallow
/// depressions are zero: those are ordinary relief that formation drains.
pub(super) fn macro_basins(
    view: &MacroView<'_>,
    w: usize,
    h: usize,
    d: i64,
) -> Result<Vec<(i64, i64)>, FormationError> {
    let mut z = Lattice::new(w, h, d)?;
    z.z.par_iter_mut().enumerate().for_each(|(i, v)| {
        let (x, y) = ((i % w) as i64 * d, (i / w) as i64 * d);
        *v = view.height(x, y);
    });
    let n = w * h;
    let mut flags: Vec<u8> = alloc(n)?;
    for (i, (f, &zi)) in flags.iter_mut().zip(&z.z).enumerate() {
        let (x, y) = (i % w, i / w);
        let ocean = zi <= -LOWSTAND_MM && view.is_ocean(x as i64 * d, y as i64 * d);
        if ocean || x == 0 || y == 0 || x == w - 1 || y == h - 1 {
            *f = FIXED;
        }
    }
    let mut filled = z.z.clone();
    let mut next: Vec<u32> = alloc(n)?;
    let mut closed: Vec<u8> = alloc(n)?;
    drainage::fill(&mut filled, w, h, &flags, 0, &mut next, &mut closed)?;
    let mut depth = Lattice::new(w, h, d)?;
    for ((dz, &f), &zi) in depth.z.iter_mut().zip(&filled).zip(&z.z) {
        *dz = (f - zi).max(0);
    }
    // Large deep components each get one sink.
    let mut sinks = Vec::new();
    let mut seen: Vec<u8> = alloc(n)?;
    let mut stack = Vec::new();
    let mut comp = Vec::new();
    for s0 in 0..n {
        if seen[s0] != 0 || depth.z[s0] < BASIN_DEPTH_MM {
            continue;
        }
        comp.clear();
        stack.push(s0);
        seen[s0] = 1;
        while let Some(c) = stack.pop() {
            comp.push(c);
            for k in 0..8 {
                if let Some(nb) = drainage::neighbour(c, w, h, k) {
                    if seen[nb] == 0 && depth.z[nb] >= BASIN_DEPTH_MM {
                        seen[nb] = 1;
                        stack.push(nb);
                    }
                }
            }
        }
        if comp.len() < BASIN_MIN_CELLS {
            continue;
        }
        // The sink: the deepest cell, ties broken toward the component
        // centroid (flat floors sink in their middle).
        let n_c = comp.len() as i64;
        let cx = comp.iter().map(|&c| (c % w) as i64).sum::<i64>() / n_c;
        let cy = comp.iter().map(|&c| (c / w) as i64).sum::<i64>() / n_c;
        let key = |c: usize| {
            let (x, y) = ((c % w) as i64, (c / w) as i64);
            (
                std::cmp::Reverse(depth.z[c]),
                (x - cx) * (x - cx) + (y - cy) * (y - cy),
                c,
            )
        };
        if let Some(&c) = comp.iter().min_by_key(|&&c| key(c)) {
            sinks.push(((c % w) as i64 * d, (c / w) as i64 * d));
        }
    }
    Ok(sinks)
}
