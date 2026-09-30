//! Coastal setting: wave exposure, sediment supply, hinterland relief,
//! nearshore depth and rock strength along every shore (logic/02
//! §fine-formation littoral, goals 15-17).
//!
//! Computed on a coarse grid of [`FACTOR`] x [`FACTOR`] fine cells
//! (312.5 m at the finest level). Coastal coarse cells (open sea and land
//! both present) get their own values; every other cell within
//! [`BAND_M`] of the coast inherits the values of its nearest coastal cell,
//! so any fine cell can ask "what kind of coast am I near?".
//! - **Exposure** is the mean open-water fetch over 16 directions, capped
//!   at [`FETCH_MAX_M`] (the domain edge counts as open ocean).
//! - **Sediment** is the catchment of river mouths within
//!   [`SEDIMENT_RADIUS_M`], on a log scale, plus waste from weak exposed
//!   rock.

use rayon::prelude::*;

use super::drainage::{open_sea_flags, FIXED};
use super::flow::Flow;
use super::lattice::{alloc, Lattice};
use super::FormationError;
use crate::noise::value_noise;

/// Fine cells per coarse cell side.
pub const FACTOR: usize = 8;
/// Coastal influence band, metres.
pub const BAND_M: i64 = 6_000;
/// Fetch cap, metres.
pub const FETCH_MAX_M: i64 = 60_000;
/// River-mouth sediment reach, metres.
pub const SEDIMENT_RADIUS_M: i64 = 6_000;
/// Smallest river mouth that supplies sediment, km².
pub const MOUTH_MIN_KM2: u64 = 1;

/// Coast descriptors for one location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Setting {
    /// Wave exposure, 0 (enclosed) ..= 255 (open ocean on every side).
    pub exposure: u8,
    /// Sediment supply, 0 ..= 255 (log catchment of nearby mouths).
    pub sediment: u8,
    /// Highest land within ~600 m of the shore, metres.
    pub hinterland_m: i32,
    /// Mean open-sea depth within ~2 km of the shore, millimetres (>= 0).
    pub nearshore_depth_mm: i32,
    /// Rock strength, 80 (weak) ..= 176 (strong).
    pub rock: u8,
    /// Distance from the coarse cell to its coastal cell, metres.
    pub dist_m: i32,
}

/// Rock strength (logic/02 §fine-formation rock strength): three rotated
/// octaves (1.5 km, 400 m, 150 m) of value noise, Q8 in 80..=176. This is
/// the field every formation level uses for its talus slope.
#[must_use]
pub fn rock_strength_q8(base_seed: u64, xm: i64, ym: i64) -> u8 {
    let rx = ((xm * 3_271 - ym * 2_465) / 4_096) as i32;
    let ry = ((xm * 2_465 + ym * 3_271) / 4_096) as i32;
    let v = i64::from(value_noise(base_seed ^ 0x40C4, rx, ry, 1_500)) * 4
        + i64::from(value_noise(base_seed ^ 0x40C5, rx, ry, 400)) * 2
        + i64::from(value_noise(base_seed ^ 0x40C6, rx, ry, 150)) * 2;
    (128 + v * 42 / (8 * 32_768)).clamp(80, 176) as u8
}

/// River mouths on the open sea: `(fine index, catchment km²)` for every
/// land cell draining straight into open sea with at least
/// [`MOUTH_MIN_KM2`], in index order.
#[must_use]
pub fn river_mouths(g: &Lattice, flow: &Flow) -> Vec<(usize, u64)> {
    let (w, h) = (g.width, g.height);
    (0..w * h)
        .filter_map(|i| {
            let r = flow.receiver(i, w, h);
            (r != i && g.z[i] > 0 && flow.is_open_sea(r, &g.z) && flow.km2(i) >= MOUTH_MIN_KM2)
                .then(|| (i, flow.km2(i)))
        })
        .collect()
}

/// Coast settings on the coarse grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoastSetting {
    cw: usize,
    ch: usize,
    fine_w: usize,
    cell_m: i64,
    /// Nearest coastal coarse cell (`u32::MAX` beyond the band).
    nearest: Vec<u32>,
    dist_m: Vec<i32>,
    values: Vec<Setting>,
    /// Exposure of the nearest coast, blurred over ~1 km, per coarse cell.
    exposure: Vec<i32>,
}

impl CoastSetting {
    /// Setting of the coast nearest to fine cell `i`, if within the band.
    #[must_use]
    pub fn at_fine(&self, i: usize) -> Option<Setting> {
        let (fx, fy) = (i % self.fine_w, i / self.fine_w);
        let (x, y) = (fx / FACTOR, fy / FACTOR);
        let c = y.min(self.ch - 1) * self.cw + x.min(self.cw - 1);
        let k = *self.nearest.get(c)?;
        let mut s = *self.values.get(k as usize)?;
        s.dist_m = self.dist_m[c];
        // Exposure varies smoothly along the shore: bilinear over the
        // blurred field between coarse-cell centres, so shore processes
        // keyed on it do not step at coarse-cell edges.
        let f = FACTOR as i64;
        let (gx, gy) = (fx as i64 * 256 / f - 128, fy as i64 * 256 / f - 128);
        let (x0, y0) = (gx.div_euclid(256), gy.div_euclid(256));
        let (tx, ty) = (gx.rem_euclid(256), gy.rem_euclid(256));
        let at = |x: i64, y: i64| {
            let (x, y) = (
                x.clamp(0, self.cw as i64 - 1),
                y.clamp(0, self.ch as i64 - 1),
            );
            i64::from(self.exposure[y as usize * self.cw + x as usize])
        };
        let top = at(x0, y0) * (256 - tx) + at(x0 + 1, y0) * tx;
        let bot = at(x0, y0 + 1) * (256 - tx) + at(x0 + 1, y0 + 1) * tx;
        s.exposure = u8::try_from((top * (256 - ty) + bot * ty) / 65_536).unwrap_or(255);
        Some(s)
    }

    /// Coarse cell size, metres.
    #[must_use]
    pub const fn cell_m(&self) -> i64 {
        self.cell_m
    }
}

/// Unit direction vectors for 16 compass directions, Q12.
const DIRS: [(i64, i64); 16] = [
    (4_096, 0),
    (3_784, 1_567),
    (2_896, 2_896),
    (1_567, 3_784),
    (0, 4_096),
    (-1_567, 3_784),
    (-2_896, 2_896),
    (-3_784, 1_567),
    (-4_096, 0),
    (-3_784, -1_567),
    (-2_896, -2_896),
    (-1_567, -3_784),
    (0, -4_096),
    (1_567, -3_784),
    (2_896, -2_896),
    (3_784, -1_567),
];

/// Computes the coast setting of `g`. `mouths` are from [`river_mouths`].
///
/// # Errors
/// Allocation failure.
pub fn compute(
    g: &Lattice,
    base_seed: u64,
    mouths: &[(usize, u64)],
) -> Result<CoastSetting, FormationError> {
    let (w, h) = (g.width, g.height);
    let (cw, ch) = (w.div_ceil(FACTOR), h.div_ceil(FACTOR));
    let nc = cw * ch;
    let d_m = (g.spacing_um / 1_000_000).max(1);
    let cell_m = d_m * FACTOR as i64;
    let mut open: Vec<u8> = alloc(w * h)?;
    open_sea_flags(&g.z, w, h, &mut open);
    // Per coarse cell: land count, open-sea count, highest land, sea depth sum.
    let mut land = vec![0_u32; nc];
    let mut sea = vec![0_u32; nc];
    let mut top = vec![0_i32; nc];
    let mut depth_sum = vec![0_i64; nc];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let c = (y / FACTOR) * cw + x / FACTOR;
            if g.z[i] > 0 {
                land[c] += 1;
                top[c] = top[c].max(g.z[i]);
            } else if open[i] & FIXED != 0 {
                sea[c] += 1;
                depth_sum[c] += -i64::from(g.z[i]);
            }
        }
    }
    drop(open);
    // Coastal: land here and open sea here or next door (or the reverse),
    // so shores along a coarse-cell edge still count.
    let near = |c: usize, v: &[u32]| {
        let (x, y) = (c % cw, c / cw);
        v[c] > 0
            || (x > 0 && v[c - 1] > 0)
            || (x + 1 < cw && v[c + 1] > 0)
            || (y > 0 && v[c - cw] > 0)
            || (y + 1 < ch && v[c + cw] > 0)
    };
    let coastal: Vec<bool> = (0..nc)
        .map(|c| (land[c] > 0 && near(c, &sea)) || (sea[c] > 0 && near(c, &land)))
        .collect();
    let solid = |x: i64, y: i64| -> Option<bool> {
        if x < 0 || y < 0 || x >= cw as i64 || y >= ch as i64 {
            return None;
        }
        let c = y as usize * cw + x as usize;
        Some(land[c] * 2 > land[c] + sea[c] + u32::from(land[c] + sea[c] == 0))
    };
    let max_steps = FETCH_MAX_M / (cell_m * 2);
    let exposure: Vec<u8> = (0..nc)
        .into_par_iter()
        .map(|c| {
            if !coastal[c] {
                return 0;
            }
            let (cx, cy) = (
                (c % cw) as i64 * 4_096 + 2_048,
                (c / cw) as i64 * 4_096 + 2_048,
            );
            let mut sum = 0_i64;
            for &(dx, dy) in &DIRS {
                let mut run = max_steps;
                for s in 1..=max_steps {
                    let (px, py) = (cx + dx * s * 2, cy + dy * s * 2);
                    match solid(px.div_euclid(4_096), py.div_euclid(4_096)) {
                        None => break,
                        Some(true) => {
                            run = s - 1;
                            break;
                        }
                        Some(false) => {}
                    }
                }
                sum += run;
            }
            u8::try_from(sum * 255 / (16 * max_steps.max(1))).unwrap_or(255)
        })
        .collect();
    // Sediment: log2 of the mouth catchment within reach, tapering linearly.
    let mut sed_raw = vec![0_i64; nc];
    let reach = SEDIMENT_RADIUS_M / cell_m;
    for &(i, km2) in mouths {
        let (mx, my) = (((i % w) / FACTOR) as i64, ((i / w) / FACTOR) as i64);
        for oy in -reach..=reach {
            for ox in -reach..=reach {
                let (x, y) = (mx + ox, my + oy);
                if x < 0 || y < 0 || x >= cw as i64 || y >= ch as i64 {
                    continue;
                }
                let r = i64::try_from((ox * ox + oy * oy).unsigned_abs().isqrt()).unwrap_or(0);
                if r > reach {
                    continue;
                }
                let c = y as usize * cw + x as usize;
                sed_raw[c] += i64::try_from(km2).unwrap_or(0) * 16 * (reach + 1 - r) / (reach + 1);
            }
        }
    }
    let mut values = Vec::with_capacity(nc);
    for c in 0..nc {
        let (xm, ym) = ((c % cw) as i64 * cell_m, (c / cw) as i64 * cell_m);
        let rock = rock_strength_q8(base_seed, xm, ym);
        // Weak, exposed rock sheds its own beach material.
        let cliff_waste = if rock < 120 {
            i64::from(exposure[c]) / 4
        } else {
            0
        };
        // floor(log2(km²)) + 1: 5 km² -> 3, 50 -> 6, 500 -> 9.
        let bits = 64 - ((sed_raw[c] / 16) as u64).leading_zeros() as i64;
        let sediment =
            u8::try_from(((bits - 1).max(0) * 30 + cliff_waste).clamp(0, 255)).unwrap_or(255);
        // Hinterland: highest land within two coarse cells; nearshore depth:
        // mean open-sea depth within six.
        let (x0, y0) = (c % cw, c / cw);
        let mut hinter = 0;
        let (mut dsum, mut dcnt) = (0_i64, 0_i64);
        for y in y0.saturating_sub(6)..(y0 + 7).min(ch) {
            for x in x0.saturating_sub(6)..(x0 + 7).min(cw) {
                let k = y * cw + x;
                if x.abs_diff(x0) <= 2 && y.abs_diff(y0) <= 2 {
                    hinter = hinter.max(top[k]);
                }
                dsum += depth_sum[k];
                dcnt += i64::from(sea[k]);
            }
        }
        values.push(Setting {
            exposure: exposure[c],
            sediment,
            hinterland_m: hinter / 1_000,
            nearshore_depth_mm: i32::try_from(dsum / dcnt.max(1)).unwrap_or(i32::MAX),
            rock,
            dist_m: 0,
        });
    }
    let (dist_m, nearest) = nearest_coastal(&coastal, cw, ch, cell_m)?;
    let raw: Vec<i32> = nearest
        .iter()
        .map(|&k| values.get(k as usize).map_or(0, |v| i32::from(v.exposure)))
        .collect();
    let (mut tmp, mut exposure) = (alloc(nc)?, alloc(nc)?);
    super::lattice::blur_into(&raw, cw, ch, 2, &mut tmp, &mut exposure);
    Ok(CoastSetting {
        cw,
        ch,
        fine_w: w,
        cell_m,
        nearest,
        dist_m,
        values,
        exposure,
    })
}

/// Labelled chamfer from coastal coarse cells, limited to [`BAND_M`].
fn nearest_coastal(
    coastal: &[bool],
    cw: usize,
    ch: usize,
    cell_m: i64,
) -> Result<(Vec<i32>, Vec<u32>), FormationError> {
    let n = cw * ch;
    let mut dist: Vec<i32> = alloc(n)?;
    let mut near: Vec<u32> = alloc(n)?;
    for c in 0..n {
        if coastal[c] {
            near[c] = c as u32;
        } else {
            dist[c] = i32::MAX / 2;
            near[c] = u32::MAX;
        }
    }
    let card = i32::try_from(cell_m).map_err(|_| FormationError::ArithmeticOverflow)?;
    let diag = card * 181 / 128;
    let relax = |i: usize, j: usize, step: i32, dist: &mut [i32], near: &mut [u32]| {
        let v = dist[j].saturating_add(step);
        if v < dist[i] {
            dist[i] = v;
            near[i] = near[j];
        }
    };
    for y in 0..ch {
        for x in 0..cw {
            let i = y * cw + x;
            if x > 0 {
                relax(i, i - 1, card, &mut dist, &mut near);
            }
            if y > 0 {
                relax(i, i - cw, card, &mut dist, &mut near);
                if x > 0 {
                    relax(i, i - cw - 1, diag, &mut dist, &mut near);
                }
                if x + 1 < cw {
                    relax(i, i - cw + 1, diag, &mut dist, &mut near);
                }
            }
        }
    }
    for y in (0..ch).rev() {
        for x in (0..cw).rev() {
            let i = y * cw + x;
            if x + 1 < cw {
                relax(i, i + 1, card, &mut dist, &mut near);
            }
            if y + 1 < ch {
                relax(i, i + cw, card, &mut dist, &mut near);
                if x + 1 < cw {
                    relax(i, i + cw + 1, diag, &mut dist, &mut near);
                }
                if x > 0 {
                    relax(i, i + cw - 1, diag, &mut dist, &mut near);
                }
            }
        }
    }
    for c in 0..n {
        if i64::from(dist[c]) > BAND_M {
            near[c] = u32::MAX;
        }
    }
    Ok((dist, near))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A square bay: land on three sides of a 3 km inlet, open sea south.
    fn bay() -> Lattice {
        let (w, h) = (512, 512);
        let mut g = Lattice::new(w, h, 39_062_500).unwrap();
        for y in 0..h {
            for x in 0..w {
                let inlet = (180..332).contains(&x) && y > 180;
                g.z[y * w + x] = if y > 400 || inlet { -20_000 } else { 30_000 };
            }
        }
        g
    }

    #[test]
    fn bay_heads_are_sheltered_and_open_coasts_exposed() {
        let g = bay();
        let s = compute(&g, 1, &[]).unwrap();
        let w = g.width;
        let head = s.at_fine(185 * w + 256).unwrap(); // inside the bay head
        let open = s.at_fine(400 * w + 60).unwrap(); // open south coast
        assert!(
            head.exposure < open.exposure,
            "{} vs {}",
            head.exposure,
            open.exposure
        );
        assert_eq!(head.hinterland_m, 30);
    }

    #[test]
    fn river_mouths_supply_sediment() {
        let g = bay();
        let w = g.width;
        let quiet = compute(&g, 1, &[]).unwrap();
        let fed = compute(&g, 1, &[(400 * w + 60, 500)]).unwrap();
        let at = 401 * w + 64;
        assert!(fed.at_fine(at).unwrap().sediment > quiet.at_fine(at).unwrap().sediment + 100);
    }
}
