//! Recipe-8 plains (logic/02 §fine-formation plains, goal 5): alluvial
//! valley floors scaled to discharge.
//!
//! **Alluvial floors.** A lowland river migrates across its valley and
//! buries the valley bottom in its own sediment, so a wide, low-gradient
//! valley ends up with a flat floor at about bankfull height, bounded by
//! bluffs where the migrating channel last cut into the valley side. Every
//! cell is compared with the first channel it drains to (at least
//! [`FLOOR_CHANNEL_KM2`] equivalent km², runoff-weighted) and with its flow
//! distance to that channel:
//! - **Width.** The floor's half-width is `60 m × A^0.4` (A equivalent km²:
//!   about 0.4 km wide at 20 km², 0.8 km at 100 km², 1.9 km at 1,000 km²),
//!   wandering ±35% along the valley with smooth rotated noise, so bluff
//!   lines are scalloped rather than parallel to the river.
//! - **Setting.** Bluff height scales with a setting weight and width with
//!   its square root. The weight is the least of three ramps: bankfull specific stream power `9810 √Q S`
//!   W/m² (full to 150, none from 400: rivers with more power per unit
//!   width are confined and cut down rather than migrate, Nanson & Croke
//!   1992), channel slope over ~2.5 km (full to 1%, none from 3%) and belt
//!   relief (full to 400 m, none from 800 m). Mountain streams keep their
//!   V-shape.
//! - **Height.** The floor sits at the channel's height, rising 0.4‰ away
//!   from it so it still drains to it; the §world-water channels carve the
//!   bankfull channel into it later. Lower ground is
//!   raised (aggradation); higher ground is planed down (lateral erosion),
//!   but only up to a bluff of `4 D` (`D = 2.5 m × A^¼`, §floodplain; 21 m
//!   at 20 km², 32 m at 100 km², 56 m at 1,000 km²):
//!   valley sides that rise higher within the floor width stay as spurs and
//!   bluffs. The floor fades into the untouched side over `80 m + B/4`.
//! - **Openness.** A confined valley would only get a trench one or two
//!   cells wide below a bluff. So the floor is built in full only where at
//!   least 65% of the ground within its width lies below 0.6 of the bluff,
//!   not at all below 35%, and never narrower than 100 m a side.
//!
//! Interfluves are left alone: measured on the full-size seed-42 world,
//! plains (2 km relief under 50 m) already have a median slope of 0.5°
//! and interfluves of 0.4-0.8°, so smoothing them would only add sheen.
//!
//! The terraces (§terraces) run next on the result, so terrace treads and
//! the valley bluff stand above the new floor.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use arda_core::water::{exp2_q16, log2_q16};

use super::lattice::{alloc, Lattice};
use super::FormationError;
use crate::noise::value_noise;

/// Smallest channel with an alluvial floor, equivalent km².
pub const FLOOR_CHANNEL_KM2: u64 = 2;
/// Floor half-width coefficient, metres per (equivalent km²)^0.4.
const HALF_WIDTH_M: i64 = 60;
/// Specific stream power (W/m²) below which a reach builds a floodplain
/// by lateral migration, and from which it is confined (Nanson & Croke
/// 1992: medium-energy floodplains up to ~300 W/m²).
const POWER_W_M2: (i64, i64) = (150, 400);
/// Channel slope (ppm) up to which a reach can be alluvial at all, and
/// from which it is a mountain stream whatever its discharge.
const SLOPE_PPM: (i64, i64) = (10_000, 30_000);
/// Belt relief (m) where the setting is fully lowland / no longer.
const BELT_M: (i64, i64) = (400, 800);
/// Channel slope is measured over this many receiver steps (~2.5 km), so
/// fill flats and the steps below them average out.
const SLOPE_STEPS: usize = 64;
/// Floor rise away from the channel, ppm (0.4‰).
const CROSS_PPM: i64 = 400;
/// Highest bluff lateral erosion cuts, in units of the floodplain depth D.
const BLUFF_D: i64 = 4;
/// Share of the ground within the floor width that must lie below 0.6 of
/// the bluff for a full floor / no floor, Q8 (65% and 35%).
const OPEN_SHARE_Q8: (i64, i64) = (166, 90);
/// Narrower floors are not built (they read as trenches), metres.
const MIN_HALF_M: i64 = 100;
/// Smoothing radius of the height change, metres (as §terraces).
const SMOOTH_M: i64 = 40;
/// Rivers below this height (drowned or at the shore) are left alone, mm.
const MIN_RIVER_MM: i32 = 250;

/// What formation knows about a point that the plains read: belt relief
/// (m), runoff weight (Q8, 256 = 500 mm/yr) and whether it lies in a
/// protected sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Setting {
    /// Belt relief, metres.
    pub belt_m: i64,
    /// Runoff weight, Q8.
    pub runoff_q8: i64,
    /// Inside a fixed endorheic sink.
    pub sink: bool,
}

fn ramp_q8(v: i64, (full, none): (i64, i64)) -> i64 {
    (256 * (none - v) / (none - full)).clamp(0, 256)
}

fn smooth_q8(t: i64) -> i64 {
    let t = t.clamp(0, 256);
    t * t * (3 * 256 - 2 * t) / (256 * 256)
}

/// `a^p` for `a` in Q8 (km²) and `p` in Q16, result Q16.
fn pow_q16(a_q8: u64, p_q16: i64) -> i64 {
    let l = log2_q16(a_q8.max(1)) - 8 * 65_536;
    i64::try_from(exp2_q16(l * p_q16 / 65_536)).unwrap_or(i64::MAX)
}

/// Floor wander at `(xm, ym)` metres, Q8 around 256 (0.65-1.35).
fn wander_q8(seed: u64, xm: i64, ym: i64) -> i64 {
    let (ax, ay) = (
        ((xm * 3_513 - ym * 2_106) / 4_096) as i32,
        ((xm * 2_106 + ym * 3_513) / 4_096) as i32,
    );
    let v = i64::from(value_noise(seed ^ 0x9A11, ax, ay, 2_500)) * 2
        + i64::from(value_noise(seed ^ 0x9A12, ay, -ax, 900));
    256 + v * 90 / (3 * 32_768)
}

/// Builds alluvial valley floors on `g` (logic/02 §fine-formation
/// plains). `setting` describes a point given in micrometres. Returns the
/// number of cells changed.
///
/// # Errors
/// Allocation failure.
pub fn apply(
    g: &mut Lattice,
    setting: &(dyn Fn(i64, i64) -> Setting + Sync),
    seed: u64,
) -> Result<usize, FormationError> {
    alluvial_floors(g, setting, seed)
}

/// Packed per-channel floor: half-width (m, bits 0-15) and bluff height
/// (m, bits 16-31).
fn pack(half_m: i64, cut_m: i64) -> u32 {
    (half_m.clamp(0, 65_535) as u32) | ((cut_m.clamp(0, 65_535) as u32) << 16)
}

/// The floor of one channel as a cell draining to it sees it.
struct Floor {
    half: i64,
    cut_mm: i64,
}

impl Floor {
    /// The floor of channel `c` for cell `i`, if `i` is land off the
    /// channel and `c` has one.
    fn at(floor: &[u32], c: usize, i: usize, g: &Lattice) -> Option<Self> {
        let p = floor[c];
        (c != i && p != 0 && g.z[i] > 0).then(|| Self {
            half: i64::from(p & 0xFFFF),
            cut_mm: i64::from(p >> 16) * 1_000,
        })
    }

    /// Height of `i` above the floor at flow distance `l` (m) from `c`:
    /// the channel height plus the 0.4‰ rise away from it.
    fn above(&self, g: &Lattice, c: usize, i: usize, l: i64) -> i64 {
        i64::from(g.z[i]) - i64::from(g.z[c]) - l * CROSS_PPM / 1_000
    }
}

/// Builds the alluvial floors. Returns the number of cells changed.
fn alluvial_floors(
    g: &mut Lattice,
    setting: &(dyn Fn(i64, i64) -> Setting + Sync),
    seed: u64,
) -> Result<usize, FormationError> {
    let (w, h, d) = (g.width, g.height, g.spacing_um);
    let n = w * h;
    let flow = super::flow::route(g)?;
    let cell_m2 = flow.cell_m2;
    let at = |i: usize| ((i % w) as i64 * d, (i / w) as i64 * d);
    // Smallest raw area that can reach the threshold at the largest
    // runoff weight (4x).
    let raw_min = FLOOR_CHANNEL_KM2 * 1_000_000 / 4 / cell_m2;
    // Equivalent area (km², Q8) of a channel cell, 0 below the threshold.
    let eq_q8 = |i: usize| -> u64 {
        let a = u64::from(flow.area[i]);
        if a < raw_min {
            return 0;
        }
        let (x, y) = at(i);
        let wq8 = setting(x, y).runoff_q8.max(1) as u64;
        let q8 = a * cell_m2 * wq8 / 1_000_000;
        if q8 >= FLOOR_CHANNEL_KM2 * 256 {
            q8
        } else {
            0
        }
    };
    // First channel downstream and the flow distance to it (m).
    let mut chan: Vec<u32> = alloc(n)?;
    let mut dist: Vec<u16> = alloc(n)?;
    let mut floor: Vec<u32> = alloc(n)?;
    let step_m = |i: usize, r: usize| {
        let diag = i.abs_diff(r) != 1 && i.abs_diff(r) != w;
        (d * if diag { 1_414 } else { 1_000 } / 1_000_000_000) as u16
    };
    for &i in flow.order.iter().rev() {
        let i = i as usize;
        let r = flow.receiver(i, w, h);
        let a = eq_q8(i);
        if a > 0 || r == i {
            chan[i] = i as u32;
            dist[i] = 0;
            if a > 0 && g.z[i] >= MIN_RIVER_MM && !flow.is_open_sea(i, &g.z) {
                floor[i] = channel_floor(g, &flow, i, a, setting);
            }
        } else {
            chan[i] = chan[r];
            dist[i] = dist[r].saturating_add(step_m(i, r));
        }
    }
    drop(flow);
    // Valley openness: a floor is built only where most of the ground
    // within the floor width lies below the bluff. In a confined valley the
    // bluff would stand one or two cells from the channel, a trench rather
    // than a floodplain (seed-3 MICRO at mid zoom), so the floor fades out.
    let mut open: Vec<u32> = alloc(n)?;
    for i in 0..n {
        let c = chan[i] as usize;
        let Some(f) = Floor::at(&floor, c, i, g) else {
            continue;
        };
        let l = i64::from(dist[i]);
        if l > f.half {
            continue;
        }
        let ok = f.above(g, c, i, l) <= f.cut_mm * 3 / 5;
        let (n_in, n_ok) = (open[c] & 0xFFFF, open[c] >> 16);
        open[c] =
            (n_in + u32::from(n_in < 0xFFFF)) | ((n_ok + u32::from(ok && n_ok < 0xFFFF)) << 16);
    }
    for (p, &o) in floor.iter_mut().zip(&open) {
        if *p == 0 {
            continue;
        }
        let (n_in, n_ok) = (i64::from(o & 0xFFFF), i64::from(o >> 16));
        let share = if n_in < 4 { 0 } else { n_ok * 256 / n_in };
        let k = ramp_q8(share, OPEN_SHARE_Q8);
        let half = i64::from(*p & 0xFFFF) * k / 256;
        *p = if half < MIN_HALF_M {
            0
        } else {
            pack(half, i64::from(*p >> 16) * k / 256)
        };
    }
    drop(open);
    let d_m = d / 1_000_000;
    let mut delta: Vec<i32> = alloc(n)?;
    // The floor height under every cell a floor touches: smoothing may not
    // lower a cell below it, or the channel's banks would sink under it.
    let mut lowest: Vec<i32> = alloc(n)?;
    lowest.fill(i32::MIN);
    for i in 0..n {
        let c = chan[i] as usize;
        let Some(f) = Floor::at(&floor, c, i, g) else {
            continue;
        };
        let edge = 80 + f.half / 4;
        let l = i64::from(dist[i]);
        if l >= f.half * 3 / 2 + edge || l == i64::from(u16::MAX) {
            continue;
        }
        let (x, y) = at(i);
        if setting(x, y).sink {
            continue;
        }
        let b = f.half * wander_q8(seed, (i % w) as i64 * d_m, (i / w) as i64 * d_m) / 256;
        let lateral = 256 - smooth_q8((l - b) * 256 / edge);
        if lateral <= 0 {
            continue;
        }
        let above = f.above(g, c, i, l);
        // Planation stops at the bluff: higher valley sides stay as spurs.
        let wt = if above <= 0 {
            lateral
        } else if f.cut_mm == 0 || above >= f.cut_mm {
            0
        } else {
            lateral * (256 - smooth_q8((above * 256 / f.cut_mm - 154) * 256 / 102)) / 256
        };
        if wt <= 0 {
            continue;
        }
        delta[i] = i32::try_from(-above * wt / 256).unwrap_or(0);
        lowest[i] = g.z[i].saturating_sub(i32::try_from(above).unwrap_or(0).max(0));
    }
    // Channel cells keep their height, but the change field runs through
    // them at their banks' value, so the smoothing below neither drags the
    // channel down nor leaves a step at the bank.
    let mut is_chan: Vec<u8> = alloc(n)?;
    for i in 0..n {
        if chan[i] as usize != i || floor[i] == 0 {
            continue;
        }
        is_chan[i] = 1;
        let (mut s, mut k) = (0_i64, 0_i64);
        for nb in 0..8 {
            if let Some(j) = super::drainage::neighbour(i, w, h, nb) {
                if chan[j] as usize == i && delta[j] != 0 {
                    s += i64::from(delta[j]);
                    k += 1;
                }
            }
        }
        if k > 0 {
            delta[i] = i32::try_from(s / k).unwrap_or(0);
        }
    }
    drop(chan);
    drop(dist);
    drop(floor);
    // Neighbours that drain to different channels (one with a floor, one
    // without) would step at their divide; smoothing the change over ~80 m
    // removes those one-cell seams, as for the terraces.
    let r = usize::try_from(SMOOTH_M * 1_000_000 / d)
        .unwrap_or(1)
        .max(1);
    let (mut tmp, mut smooth) = (alloc(n)?, alloc(n)?);
    super::lattice::blur_into(&delta, w, h, r, &mut tmp, &mut smooth);
    drop(tmp);
    drop(delta);
    let mut changed = 0;
    for (((z, &dz), &c), &low) in g.z.iter_mut().zip(&smooth).zip(&is_chan).zip(&lowest) {
        if dz == 0 || *z <= 0 || c != 0 {
            continue;
        }
        // Land stays land: a floor never drops below 0.3 m, nor below the
        // floor itself.
        let nz = z.saturating_add(dz).max((*z).min(300)).max(low.min(*z));
        if nz != *z {
            *z = nz;
            changed += 1;
        }
    }
    Ok(changed)
}

/// The packed floor of channel cell `c` with equivalent area `a_q8`.
fn channel_floor(
    g: &Lattice,
    flow: &super::flow::Flow,
    c: usize,
    a_q8: u64,
    setting: &(dyn Fn(i64, i64) -> Setting + Sync),
) -> u32 {
    let (w, h, d) = (g.width, g.height, g.spacing_um);
    // Channel slope over ~2.5 km downstream (to the sea at most).
    let (mut j, mut len_um) = (c, 0_i64);
    for _ in 0..SLOPE_STEPS {
        let r = flow.receiver(j, w, h);
        if r == j || flow.is_open_sea(r, &g.z) {
            break;
        }
        let diag = j.abs_diff(r) != 1 && j.abs_diff(r) != w;
        len_um += d * if diag { 1_414 } else { 1_000 } / 1_000;
        j = r;
    }
    // A mouth cell has no reach to measure: it keeps its form.
    if len_um == 0 {
        return 0;
    }
    let slope_ppm = (i64::from(g.z[c]) - i64::from(g.z[j])).max(0) * 1_000_000_000 / len_um;
    let s = setting((c % w) as i64 * d, (c / w) as i64 * d);
    let km2 = a_q8 / 256;
    let q = super::water::nominal_discharge(km2);
    // Bankfull specific stream power ρg·4Q·S / (4 m × √Q) = 9810·√Q·S.
    let q_root = i64::try_from(super::incision::isqrt(q.raw() * 1_000)).unwrap_or(0);
    let power = 981 * q_root * slope_ppm / 100_000_000;
    let weight = ramp_q8(power, POWER_W_M2)
        .min(ramp_q8(slope_ppm, SLOPE_PPM))
        .min(ramp_q8(s.belt_m, BELT_M));
    if weight == 0 {
        return 0;
    }
    // Width follows √weight: a partly confined reach still migrates
    // across most of its belt; the bluff it can cut falls off faster.
    let root_w = i64::try_from(super::incision::isqrt((weight * 256) as u64)).unwrap_or(0);
    let half_m = HALF_WIDTH_M * pow_q16(a_q8, 26_214) / 65_536 * root_w / 256;
    let depth_mm = 2_500 * pow_q16(a_q8, 16_384) / 65_536;
    let cut_m = depth_mm * BLUFF_D * weight / 256 / 1_000;
    pack(half_m, cut_m)
}

#[cfg(test)]
#[path = "plains_tests.rs"]
mod tests;
