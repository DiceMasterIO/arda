//! Karst lakes and dolines (logic/02 §world-water karst, goal 12).
//!
//! Rock-type proxy: carbonate platforms are two rotated octaves of value
//! noise (48 km and 17 km) fixed in world coordinates and thresholded to
//! about a fifth of the domain. The world has no lithology model yet, so
//! this field stands in for limestone and dolomite outcrop; it is
//! independent of relief, as carbonates build both plateaus (Causses) and
//! ranges (Dinarides).
//!
//! - **Poljes**: along karst valley floors (3-400 km² catchment, slope
//!   below 0.6%, ground within 5 m of the bed at least 250 m to either side
//!   on average), closed basins 1.2-5 km long with a lobed outline, the
//!   valley pulled down smoothly towards a flat floor 5-14 m
//!   deep, at least 8 km apart and at most one per 150 km²
//!   of karst. The stream enters at the upstream end and the basin spills
//!   over the old valley floor at the downstream end, so a full polje lake
//!   has a natural inlet and outlet. The centre is a protected sink.
//! - **Dolines**: on a 300 m jittered grid, 45% of karst sites away from
//!   channels and on slopes below 25% hold a doline 25-90 m across. They
//!   are recorded, not carved: at the 100 m hydrology scale each would be a
//!   pit that the annual balance fills, while real dolines drain to
//!   groundwater, which the shared model does not carry.

use crate::noise::value_noise;

use super::super::lattice::Lattice;
use super::super::FormationError;
use super::geom::{hash3, m_to_q8, pick, q8_to_um, CELL_Q8, ONE_Q14};
use super::network::Stream;
use super::{DolineSite, Sink, SinkKind, WaterFeatures};

/// Carbonate proxy threshold on the two-octave sum (±3 × 32768).
pub const KARST_THRESHOLD: i64 = 24_000;
/// Least distance between poljes, metres.
pub const POLJE_SPACING_M: i64 = 8_000;
/// Steepest valley floor that holds a polje, parts per million.
pub const POLJE_MAX_SLOPE_PPM: i64 = 6_000;
/// Least mean half-width of the ground within 5 m of the bed, metres.
pub const POLJE_MIN_FLOOR_M: i64 = 150;
/// Least fine nodes a polje floods below its spill (about 25 cells of
/// 100 m); smaller basins are not carved, so no speckle lake appears.
pub const POLJE_MIN_NODES: usize = 160;
/// Karst area per polje, km².
pub const KARST_KM2_PER_POLJE: u64 = 150;

/// Whether the carbonate proxy holds at an absolute position (metres).
#[must_use]
pub fn is_karst(seed: u64, x_m: i64, y_m: i64) -> bool {
    let rx = i32::try_from((x_m * 3_600 - y_m * 1_950) / 4_096).unwrap_or(0);
    let ry = i32::try_from((x_m * 1_950 + y_m * 3_600) / 4_096).unwrap_or(0);
    let v = 2 * i64::from(value_noise(seed ^ 0xCA2B, rx, ry, 48_000))
        + i64::from(value_noise(seed ^ 0xCA2C, rx, ry, 17_000));
    v > KARST_THRESHOLD
}

fn node_m(g: &Lattice, i: usize) -> (i64, i64) {
    let d = g.spacing_um;
    (
        (i % g.width) as i64 * d / 1_000_000,
        (i / g.width) as i64 * d / 1_000_000,
    )
}

/// Carves poljes and records dolines.
///
/// # Errors
/// None at present; kept fallible for allocation-bounded extensions.
pub fn apply(
    g: &mut Lattice,
    streams: &[Stream],
    area: &[u32],
    claimed: &[Vec<bool>],
    seed: u64,
    features: &mut WaterFeatures,
) -> Result<(), FormationError> {
    let cell_m2 = super::cell_m2(g);
    // Karst land share, sampled every 8th node in each axis.
    let (mut land, mut karst) = (0_u64, 0_u64);
    for y in (0..g.height).step_by(8) {
        for x in (0..g.width).step_by(8) {
            let i = y * g.width + x;
            if g.z[i] > 0 {
                land += 1;
                let (xm, ym) = node_m(g, i);
                karst += u64::from(is_karst(seed, xm, ym));
            }
        }
    }
    features.stats.karst_permille = karst * 1000 / land.max(1);
    let karst_km2 = karst * 64 * cell_m2 / 1_000_000;
    let budget = karst_km2 / KARST_KM2_PER_POLJE;
    // Candidates about every kilometre along karst valley floors.
    let step = usize::try_from(m_to_q8(g, 1_000) / CELL_Q8)
        .unwrap_or(1)
        .max(1);
    let half = usize::try_from(m_to_q8(g, 500) / CELL_Q8)
        .unwrap_or(1)
        .max(1);
    let mut cands: Vec<(u64, usize, usize)> = Vec::new();
    for (si, s) in streams.iter().enumerate() {
        let m = s.cells.len();
        let mut k = step;
        while k + step < m {
            let c = s.cells[k];
            let km2 = u64::from(area[c]) * cell_m2 / 1_000_000;
            let (xm, ym) = node_m(g, c);
            if (3..=400).contains(&km2)
                && !claimed[si][k.saturating_sub(step)..(k + step).min(m)]
                    .iter()
                    .any(|&v| v)
                && g.z[c] > 30_000
                && s.slope_ppm(g, k, half) < POLJE_MAX_SLOPE_PPM
                && is_karst(seed, xm, ym)
                && open_floor(g, s, k)
            {
                cands.push((hash3(seed, xm, ym), si, k));
            }
            k += step;
        }
    }
    cands.sort_unstable();
    let mut placed: Vec<(i64, i64)> = Vec::new();
    for &(h, si, k) in &cands {
        if placed.len() as u64 >= budget {
            break;
        }
        let s = &streams[si];
        let (xm, ym) = node_m(g, s.cells[k]);
        if placed.iter().any(|&(px, py)| {
            (px - xm) * (px - xm) + (py - ym) * (py - ym) < POLJE_SPACING_M * POLJE_SPACING_M
        }) {
            continue;
        }
        let km2 = u64::from(area[s.cells[k]]) * cell_m2 / 1_000_000;
        if polje(g, s, k, km2, h, features) {
            placed.push((xm, ym));
        }
    }
    features.stats.poljes = placed.len() as u64;
    dolines(g, area, seed, features);
    Ok(())
}

/// Poljes are broad flat-floored depressions: the ground within 5 m of
/// the valley bed must reach at least [`POLJE_MIN_FLOOR_M`] to either side
/// on average, or a basin would only flood the thalweg of a V-valley.
fn open_floor(g: &Lattice, s: &Stream, k: usize) -> bool {
    let (l, r) = super::network::floor_width(g, s, k, 5_000, m_to_q8(g, 2_000));
    (l + r) / 2 >= m_to_q8(g, POLJE_MIN_FLOOR_M)
}

fn polje(
    g: &mut Lattice,
    s: &Stream,
    k: usize,
    km2: u64,
    h: u64,
    features: &mut WaterFeatures,
) -> bool {
    let root = i64::try_from(super::super::incision::isqrt(km2)).unwrap_or(0);
    let jitter = 800 + pick(h, 400) as i64;
    let a_m = ((600 + 100 * root).clamp(600, 2_500)) * jitter / 1000;
    let b_m = a_m * (400 + pick(h >> 8, 150) as i64) / 1000;
    let depth = 5_000 + pick(h >> 16, 9_000) as i64;
    let (a, b) = (m_to_q8(g, a_m), m_to_q8(g, b_m));
    // The rim spills over the old floor at the downstream end.
    let down = (k..s.cells.len())
        .find(|&j| s.along[j] - s.along[k] >= a)
        .unwrap_or(s.cells.len() - 1);
    let floor = s.bed(g, down) - depth;
    let (t, c) = (s.dir[k], s.centre[k]);
    let centre_z = s.bed(g, k);
    let r = a.max(b) * 5 / 4;
    let (w, hh) = (g.width as i64, g.height as i64);
    let (x0, x1) = ((c.0 - r) / CELL_Q8, (c.0 + r) / CELL_Q8 + 1);
    let (y0, y1) = ((c.1 - r) / CELL_Q8, (c.1 + r) / CELL_Q8 + 1);
    let d_m = g.spacing_um / 1_000_000;
    let spill = s.bed(g, down);
    let mut undo: Vec<(usize, i32)> = Vec::new();
    for y in y0.max(1)..=y1.min(hh - 2) {
        for x in x0.max(1)..=x1.min(w - 2) {
            let (dx, dy) = (x * CELL_Q8 - c.0, y * CELL_Q8 - c.1);
            let u = (dx * t.0 + dy * t.1) / ONE_Q14;
            let v = (-dx * t.1 + dy * t.0) / ONE_Q14;
            // Irregular outline: the ellipse radius varies ±25% with 600 m
            // value noise, so the basin has lobes and bays.
            let wob = i64::from(value_noise(
                h ^ 0x9011,
                i32::try_from(x * d_m).unwrap_or(0),
                i32::try_from(y * d_m).unwrap_or(0),
                600,
            ));
            let scale = 1000 + wob * 250 / 32_768;
            let q = (u * u * 1000 / (a * a).max(1) + v * v * 1000 / (b * b).max(1)) * 1_000_000
                / (scale * scale);
            if q >= 1000 {
                continue;
            }
            // The valley is pulled down smoothly, most at the centre and not
            // at all at the rim, and levelled at the floor: the lake that
            // fills it to the downstream spill keeps the valley's contours.
            let pull = (centre_z - floor + 2_000) * (1000 - q) / 1000;
            let j = (y as usize) * g.width + x as usize;
            let z = i64::from(g.z[j]);
            let target = (z - pull).max(floor);
            if g.z[j] > 0 && z > target {
                undo.push((j, g.z[j]));
                g.z[j] = i32::try_from(target.max(1)).unwrap_or(g.z[j]);
            }
        }
    }
    // Too small a flooded floor would publish a speckle lake: undo.
    if undo
        .iter()
        .filter(|&&(j, _)| i64::from(g.z[j]) < spill)
        .count()
        < POLJE_MIN_NODES
    {
        for &(j, z) in undo.iter().rev() {
            g.z[j] = z;
        }
        return false;
    }
    let (x_um, y_um) = q8_to_um(g, c);
    features.sinks.push(Sink {
        x_um,
        y_um,
        radius_um: 150_000_000,
        kind: SinkKind::Karst,
    });
    true
}

fn dolines(g: &Lattice, area: &[u32], seed: u64, features: &mut WaterFeatures) {
    const GRID_M: i64 = 300;
    let d_m = g.spacing_um / 1_000_000;
    let (wm, hm) = (g.width as i64 * d_m, g.height as i64 * d_m);
    // 0.05 km² in lattice cells: no dolines in channels.
    let channel = 50_000 / super::cell_m2(g).max(1);
    for gy in 0..hm / GRID_M {
        for gx in 0..wm / GRID_M {
            let h = hash3(seed ^ 0xD011, gx, gy);
            if pick(h, 100) >= 45 {
                continue;
            }
            let x_m = gx * GRID_M + pick(h >> 8, GRID_M as u64) as i64;
            let y_m = gy * GRID_M + pick(h >> 24, GRID_M as u64) as i64;
            let (x, y) = (x_m / d_m.max(1), y_m / d_m.max(1));
            if x < 1 || y < 1 || x >= g.width as i64 - 1 || y >= g.height as i64 - 1 {
                continue;
            }
            let i = (y as usize) * g.width + x as usize;
            if g.z[i] <= 20_000 || u64::from(area[i]) >= channel || !is_karst(seed, x_m, y_m) {
                continue;
            }
            let gxz = i64::from(g.z[i + 1]) - i64::from(g.z[i - 1]);
            let gyz = i64::from(g.z[i + g.width]) - i64::from(g.z[i - g.width]);
            let rise = super::geom::isqrt_i(i128::from(gxz * gxz + gyz * gyz));
            // Slope below 25%: rise over two cells < 0.5 cell.
            if rise * 2 >= d_m * 1000 {
                continue;
            }
            let radius_mm = (25_000 + pick(h >> 40, 65_000) as i64) / 2 * 2;
            let depth_mm = radius_mm * (100 + pick(h >> 48, 200) as i64) / 1000;
            features.dolines.push(DolineSite {
                x_um: x_m * 1_000_000,
                y_um: y_m * 1_000_000,
                radius_mm: radius_mm / 2,
                depth_mm,
            });
        }
    }
}
