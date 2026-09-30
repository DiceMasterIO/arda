//! Braided reaches (logic/02 §world-water braids, goal 10).
//!
//! A reach braids where three things coincide:
//! - its slope exceeds the Leopold–Wolman threshold `0.0125 (4Q)^-0.44`
//!   (and stays below 4%, beyond which beds are step-pool bedrock);
//! - bed load is plentiful: a piedmont on low relief (mask < 150), where
//!   the stream has left high macro relief (mask ≥ 200, or 60 above the
//!   reach's own) within the last 15 km or has fallen 500 m over its last
//!   10 km; or a proglacial reach at any relief, whose course rose above
//!   2,300 m within the last 25 km;
//! - the valley floor opens to at least 40% of the belt.
//!
//! The belt, `6 × width` wide on each side (120 m to 1 km), is planed to a
//! gravel plain at the bed level with 0.3 m bars, and three interweaving
//! threads are carved into it (the deepest, 0.8 m, carries the routed flow).

use super::super::lattice::Lattice;
use super::geom::{carve_disc, hash3, m_to_q8, pick, q8_to_um, sin_q14, CELL_Q8, ONE_Q14, TURN};
use super::network::{floor_width, Stream};
use super::{nominal_discharge, WaterFeatures};

/// Streams below this catchment do not braid, km².
pub const BRAID_MIN_KM2: u64 = 20;
/// Steepest braided slope, parts per million.
pub const MAX_SLOPE_PPM: i64 = 40_000;
/// Deepest cut allowed when planing the belt, millimetres.
pub const MAX_CUT_MM: i64 = 6_000;
/// Fall over the last 10 km that marks sediment-shedding headwaters, mm.
pub const STEEP_DROP_MM: i64 = 500_000;
/// Course elevation that marks a glacier-fed stream, millimetres.
pub const PROGLACIAL_MM: i64 = 2_300_000;

fn belt_half_q8(g: &Lattice, km2: u64) -> i64 {
    let w_dm = arda_core::hydrology::channel_width_dm(nominal_discharge(km2)).unwrap_or(0);
    m_to_q8(g, (i64::from(w_dm) * 6 / 10).clamp(120, 1_000))
}

/// Braided positions of one stream (before span filtering).
fn candidates(g: &Lattice, s: &Stream, area: &[u32], relief_q8: &[u8]) -> Vec<bool> {
    let m = s.cells.len();
    let cell_m2 = super::cell_m2(g);
    let half = usize::try_from(m_to_q8(g, 500) / CELL_Q8)
        .unwrap_or(1)
        .max(1);
    let back_pied = m_to_q8(g, 15_000);
    let back_ice = m_to_q8(g, 25_000);
    let back_steep = m_to_q8(g, 10_000);
    let mut out = vec![false; m];
    let mut j_pied = 0;
    let mut floor = (0, 0);
    for k in 0..m {
        let km2 = u64::from(area[s.cells[k]]) * cell_m2 / 1_000_000;
        if km2 < BRAID_MIN_KM2 {
            continue;
        }
        let low = relief_q8[s.cells[k]] < 150;
        let q = nominal_discharge(km2);
        let slope = s.slope_ppm(g, k, half);
        let threshold = i64::from(arda_core::water::braiding_slope_ppm(q));
        if slope <= threshold || slope > MAX_SLOPE_PPM {
            continue;
        }
        while s.along[k] - s.along[j_pied] > back_ice {
            j_pied += 1;
        }
        // Bed load: the stream left high relief, or fell at least 500 m
        // over its last 10 km (steep, sediment-shedding headwaters).
        let here = relief_q8[s.cells[k]];
        let left_relief = (j_pied..k).any(|j| {
            s.along[k] - s.along[j] <= back_pied
                && (relief_q8[s.cells[j]] >= 200
                    || relief_q8[s.cells[j]] >= here.saturating_add(60))
        });
        let steep_above = (j_pied..k).any(|j| {
            s.along[k] - s.along[j] <= back_steep && s.bed(g, j) - s.bed(g, k) >= STEEP_DROP_MM
        });
        let mountain = low && (left_relief || steep_above);
        let glacier = (j_pied..k).any(|j| s.bed(g, j) > PROGLACIAL_MM);
        if !(mountain || glacier) {
            continue;
        }
        if k % 4 == 0 || floor == (0, 0) {
            floor = floor_width(g, s, k, 4_000, m_to_q8(g, 2_000));
        }
        let b = belt_half_q8(g, km2);
        if (floor.0 + floor.1) / 2 * 10 < b * 4 {
            continue;
        }
        out[k] = true;
    }
    out
}

/// Builds braided belts on eligible reaches at least 1.5 km long.
pub fn apply(
    g: &mut Lattice,
    streams: &[Stream],
    area: &[u32],
    relief_q8: &[u8],
    claimed: &mut [Vec<bool>],
    seed: u64,
    features: &mut WaterFeatures,
) {
    let min_len = m_to_q8(g, 1_500);
    for (si, s) in streams.iter().enumerate() {
        let c = candidates(g, s, area, relief_q8);
        // Close gaps shorter than 300 m, then take runs.
        let gap = usize::try_from(m_to_q8(g, 300) / CELL_Q8).unwrap_or(1);
        let mut k = 0;
        let m = s.cells.len();
        while k < m {
            if !c[k] {
                k += 1;
                continue;
            }
            let a = k;
            let mut b = k;
            while k < m && (c[k] || (k + gap < m && c[k..k + gap].iter().any(|&v| v))) {
                if c[k] {
                    b = k;
                }
                k += 1;
            }
            if s.along[b] - s.along[a] < min_len || claimed[si][a..=b].iter().any(|&v| v) {
                continue;
            }
            build(
                g,
                s,
                area,
                (a, b),
                hash3(seed, si as i64, a as i64),
                features,
            );
            claimed[si][a..=b].iter_mut().for_each(|v| *v = true);
        }
    }
}

fn build(
    g: &mut Lattice,
    s: &Stream,
    area: &[u32],
    (a, b): (usize, usize),
    seed: u64,
    features: &mut WaterFeatures,
) {
    let cell_m2 = super::cell_m2(g);
    let ramp = m_to_q8(g, 500);
    let km2_at = |k: usize| u64::from(area[s.cells[k]]) * cell_m2 / 1_000_000;
    let (w, h) = (g.width as i64, g.height as i64);
    let bar = m_to_q8(g, 60).max(1);
    // Plane the belt.
    for k in a..=b {
        let edge = (s.along[k] - s.along[a]).min(s.along[b] - s.along[k]);
        let half = belt_half_q8(g, km2_at(k)) * (edge * 1000 / ramp.max(1)).clamp(250, 1000) / 1000;
        let plain = s.bed(g, k);
        let c = s.centre[k];
        let (x0, x1) = ((c.0 - half) / CELL_Q8, (c.0 + half) / CELL_Q8 + 1);
        let (y0, y1) = ((c.1 - half) / CELL_Q8, (c.1 + half) / CELL_Q8 + 1);
        for y in y0.max(1)..=y1.min(h - 2) {
            for x in x0.max(1)..=x1.min(w - 2) {
                let (dx, dy) = (x * CELL_Q8 - c.0, y * CELL_Q8 - c.1);
                if dx * dx + dy * dy > half * half {
                    continue;
                }
                let j = (y as usize) * g.width + x as usize;
                let bars = pick(hash3(seed, x * CELL_Q8 / bar, y * CELL_Q8 / bar), 300) as i64;
                let target = plain + bars;
                let z = i64::from(g.z[j]);
                if z > target && z - target <= MAX_CUT_MM {
                    g.z[j] = i32::try_from(target.max(1)).unwrap_or(g.z[j]);
                }
            }
        }
    }
    // Three interweaving threads of different wavelength and phase.
    for thread in 0..3_i64 {
        let phase0 = pick(hash3(seed, thread, 7), TURN as u64) as i64;
        let depth = if thread == 0 { 800 } else { 400 };
        let amp_permille = if thread == 0 { 350 } else { 650 };
        let mut k = a;
        while k <= b {
            let half = belt_half_q8(g, km2_at(k));
            let lam = half * [31, 43, 59][thread as usize] / 10;
            let phase = phase0 + s.along[k] * TURN / lam.max(1);
            let off = half * amp_permille / 1000 * sin_q14(phase) / ONE_Q14;
            let (t, c) = (s.dir[k], s.centre[k]);
            let p = (c.0 - t.1 * off / ONE_Q14, c.1 + t.0 * off / ONE_Q14);
            carve_disc(g, p, m_to_q8(g, 40), s.bed(g, k) - depth, depth);
            k += 1;
        }
    }
    // Record the belt's 100 m cells (every 50 m across the belt).
    let cell = |p: (i64, i64)| {
        let (x, y) = q8_to_um(g, p);
        (
            u32::try_from(x / 100_000_000).unwrap_or(0),
            u32::try_from(y / 100_000_000).unwrap_or(0),
        )
    };
    let step = m_to_q8(g, 50).max(1);
    for k in a..=b {
        let half = belt_half_q8(g, km2_at(k));
        let belt_m = u32::try_from(half * 2 * g.spacing_um / CELL_Q8 / 1_000_000).unwrap_or(0);
        let (t, c) = (s.dir[k], s.centre[k]);
        let mut off = -half;
        while off <= half {
            let (x, y) = cell((c.0 - t.1 * off / ONE_Q14, c.1 + t.0 * off / ONE_Q14));
            features.braided_cells.push((x, y, belt_m));
            off += step;
        }
    }
    features.stats.braided_reaches += 1;
    features.stats.braided_q8 += s.along[b] - s.along[a];
}
