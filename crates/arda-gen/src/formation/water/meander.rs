//! Free meanders and oxbow lakes (logic/02 §world-water meanders, goal 9).
//!
//! On alluvial valley floors whose slope lies below the Leopold–Wolman
//! braiding threshold, the channel follows a Kinoshita (skewed
//! sine-generated) curve: its heading oscillates as `ω cos φ` about the
//! valley axis, with `ω` set by the target sinuosity `1/J0(ω)`. Sinuosity
//! rises as the slope falls, `S = 1 + 1.2 (1 - r)²` with `r` the slope over
//! the braiding threshold, and the bends are limited to the valley floor.
//! The valley wavelength is `11 × width`, never below 800 m: the world
//! layer cannot resolve shorter bends. The new course is carved one
//! bankfull depth (at least 1.2 m) into the floor, graded monotonically
//! downstream. Some bends leave a cutoff loop: a crescent carved into the
//! floor beside the belt whose lowest point is a protected sink, so the
//! annual water balance decides whether it holds an oxbow lake.

use arda_core::DischargeMilli;

use super::super::lattice::Lattice;
use super::geom::{
    carve_disc, cos_q14, hash3, isqrt_i, m_to_q8, pick, rotate, sin_q14, CELL_Q8, ONE_Q14, TURN,
};
use super::network::{floor_width, Stream};
use super::{nominal_discharge, WaterFeatures};

/// Streams below this catchment keep their course, km².
pub const MEANDER_MIN_KM2: u64 = 80;
/// Shortest valley wavelength the 100 m world layer resolves, metres.
pub const MIN_WAVELENGTH_M: i64 = 800;
/// Height above the bed that still counts as valley floor, millimetres.
pub const FLOOR_RISE_MM: i64 = 1_500;
/// Deepest lateral cut a migrating meander belt makes into its valley
/// sides, millimetres: the belt is planed to a floodplain, leaving bluffs.
pub const BELT_CUT_MM: i64 = 4_000;
/// Cross-valley rise of a planed floodplain away from its axis, ‰.
pub const FLOODPLAIN_TILT_PERMILLE: i64 = 3;
/// Belt margin beyond the bends and channel, thousandths of a wavelength.
pub const BELT_MARGIN_PERMILLE: i64 = 200;
/// Oxbows need a floor at least this far above sea level, millimetres, so
/// a loop never opens to the sea across a delta plain.
pub const OXBOW_MIN_FLOOR_MM: i64 = 1_500;
/// Least carve depth of a meandering channel, millimetres.
pub const MIN_DEPTH_MM: i64 = 1_200;
/// Sinuosity (‰) to heading amplitude (Q16 turns) of a sine-generated
/// curve, from `S = 1 / J0(ω)`.
const OMEGA: [(i64, i64); 11] = [
    (1_000, 0),
    (1_050, 4_580),
    (1_100, 6_363),
    (1_200, 8_704),
    (1_300, 10_334),
    (1_400, 11_591),
    (1_500, 12_610),
    (1_700, 14_188),
    (2_000, 15_866),
    (2_300, 17_065),
    (2_600, 17_973),
];

fn omega_for(s_permille: i64) -> i64 {
    let s = s_permille.clamp(1_000, 2_600);
    for w in OMEGA.windows(2) {
        let ((s0, o0), (s1, o1)) = (w[0], w[1]);
        if s <= s1 {
            return o0 + (o1 - o0) * (s - s0) / (s1 - s0);
        }
    }
    OMEGA[OMEGA.len() - 1].1
}

/// Target sinuosity (‰) for a valley slope and nominal discharge.
#[must_use]
pub fn target_sinuosity(slope_ppm: i64, q: DischargeMilli) -> i64 {
    let threshold = i64::from(arda_core::water::braiding_slope_ppm(q)).max(1);
    let r = (slope_ppm * 1000 / threshold).min(1000);
    1_000 + 1_200 * (1000 - r) * (1000 - r) / 1_000_000
}

/// Per-position meander plan of one stream.
pub(super) struct Plan {
    /// Heading amplitude, Q16 turns (0 outside meandering reaches).
    pub(super) omega: Vec<i64>,
    /// Valley wavelength, Q8.
    pub(super) lambda: Vec<i64>,
    /// Target sinuosity, ‰.
    pub(super) sinuosity: Vec<i64>,
    /// Lateral offset of the floor middle from the centreline, Q8.
    pub(super) shift: Vec<i64>,
    /// Half-width of the floor available to the belt, Q8.
    pub(super) room: Vec<i64>,
    /// Lateral bend amplitude after the room limit, Q8.
    pub(super) amp: Vec<i64>,
}

fn plan(g: &Lattice, s: &Stream, area: &[u32], seed: u64) -> Plan {
    let m = s.cells.len();
    let cell_m2 = super::cell_m2(g);
    let half = usize::try_from(m_to_q8(g, 1_000) / CELL_Q8)
        .unwrap_or(1)
        .max(1);
    let mut p = Plan {
        omega: vec![0; m],
        lambda: vec![0; m],
        sinuosity: vec![1_000; m],
        shift: vec![0; m],
        room: vec![0; m],
        amp: vec![0; m],
    };
    let jitter = |k: usize| {
        // Slow ±20% variation of the wavelength along the stream (3 km).
        let v = s.along[k] / m_to_q8(g, 3_000).max(1);
        let a = pick(hash3(seed, v, 11), 400) as i64;
        let b = pick(hash3(seed, v + 1, 11), 400) as i64;
        let f = s.along[k] % m_to_q8(g, 3_000).max(1) * 1000 / m_to_q8(g, 3_000).max(1);
        800 + (a * (1000 - f) + b * f) / 1000
    };
    let mut last_floor = (0, 0);
    for k in 0..m {
        let km2 = u64::from(area[s.cells[k]]) * cell_m2 / 1_000_000;
        let q = nominal_discharge(km2);
        let w_m = i64::from(arda_core::hydrology::channel_width_dm(q).unwrap_or(0)) / 10;
        p.lambda[k] = m_to_q8(g, (11 * w_m).max(MIN_WAVELENGTH_M)) * jitter(k) / 1000;
        if km2 < MEANDER_MIN_KM2 {
            continue;
        }
        p.sinuosity[k] = target_sinuosity(s.slope_ppm(g, k, half), q);
        if k % 4 == 0 {
            last_floor = floor_width(g, s, k, FLOOR_RISE_MM + BELT_CUT_MM, m_to_q8(g, 3_000));
        }
        let (l, r) = last_floor;
        p.shift[k] = (l - r) / 2;
        p.room[k] = (l + r) / 2;
    }
    // Smooth sinuosity and room over about one wavelength so bends change
    // gradually (box filter over the positions of ±λ/2).
    let smooth = |v: &[i64], radius: usize| -> Vec<i64> {
        let mut pre = vec![0_i64; m + 1];
        for k in 0..m {
            pre[k + 1] = pre[k] + v[k];
        }
        (0..m)
            .map(|k| {
                let (a, b) = (k.saturating_sub(radius), (k + radius + 1).min(m));
                (pre[b] - pre[a]) / (b - a) as i64
            })
            .collect()
    };
    let radius = usize::try_from(p.lambda[m / 2] / 2 / CELL_Q8)
        .unwrap_or(1)
        .max(1);
    p.sinuosity = smooth(&p.sinuosity, radius);
    p.room = smooth(&p.room, radius);
    p.shift = smooth(&p.shift, radius);
    for k in 0..m {
        let s_t = p.sinuosity[k];
        if s_t < 1_080 {
            continue;
        }
        let omega = omega_for(s_t);
        // Nominal lateral amplitude of a sine-generated curve: ω L / 2π.
        let amp =
            i128::from(omega) * i128::from(p.lambda[k]) * i128::from(s_t) / 1000 / i128::from(TURN);
        let r_c = channel_radius(g, s, area, k);
        // The belt needs the bends plus the channel plus a 0.2 λ margin.
        let room = i128::from((p.room[k] - r_c - p.lambda[k] * BELT_MARGIN_PERMILLE / 1000).max(0));
        let (scaled, amp) = if amp > room {
            (i128::from(omega) * room / amp.max(1), room)
        } else {
            (i128::from(omega), amp)
        };
        p.omega[k] = i64::try_from(scaled).unwrap_or(0);
        p.amp[k] = i64::try_from(amp).unwrap_or(0);
    }
    p
}

/// Planes the meander belt to a floodplain 0.3 m above the bed, rising 3‰
/// away from the belt axis:
/// nodes within the bends, channel and a 0.2 λ margin of the floor middle
/// that stand at most [`BELT_CUT_MM`] + [`FLOOR_RISE_MM`] above the bed.
/// The outer fifth blends back to the valley side, a soft bluff.
fn plane_belt(g: &mut Lattice, s: &Stream, area: &[u32], p: &Plan, (a, b): (usize, usize)) {
    let ramp = p.lambda[a] / 2;
    let (w, h) = (g.width as i64, g.height as i64);
    // Q8 distance to millimetres of rise at FLOODPLAIN_TILT_PERMILLE.
    let lateral = g.spacing_um / 1000 / CELL_Q8 * FLOODPLAIN_TILT_PERMILLE;
    for k in a..=b {
        let edge = (s.along[k] - s.along[a]).min(s.along[b] - s.along[k]);
        let taper = (edge * 1000 / ramp.max(1)).clamp(200, 1000);
        let half =
            (p.amp[k] + channel_radius(g, s, area, k) + p.lambda[k] * BELT_MARGIN_PERMILLE / 1000)
                * taper
                / 1000;
        let (t, c0) = (s.dir[k], s.centre[k]);
        let c = (
            c0.0 - t.1 * p.shift[k] / ONE_Q14,
            c0.1 + t.0 * p.shift[k] / ONE_Q14,
        );
        let floor = s.bed(g, k) + 300;
        let (x0, x1) = ((c.0 - half) / CELL_Q8, (c.0 + half) / CELL_Q8 + 1);
        let (y0, y1) = ((c.1 - half) / CELL_Q8, (c.1 + half) / CELL_Q8 + 1);
        for y in y0.max(1)..=y1.min(h - 2) {
            for x in x0.max(1)..=x1.min(w - 2) {
                let (dx, dy) = (x * CELL_Q8 - c.0, y * CELL_Q8 - c.1);
                let d = isqrt_i(i128::from(dx * dx + dy * dy));
                if d > half {
                    continue;
                }
                let j = (y as usize) * g.width + x as usize;
                let z = i64::from(g.z[j]);
                // The floodplain rises gently away from the belt axis, so it
                // drains to the river rather than into a cutoff crescent.
                let plain = floor + d * lateral / 1000;
                if z <= plain || z - floor > BELT_CUT_MM + FLOOR_RISE_MM {
                    continue;
                }
                let blend = ((d * 1000 / half.max(1) - 800).max(0) * 5).min(1000);
                let target = plain + (z - plain) * blend / 1000;
                g.z[j] = i32::try_from(target).unwrap_or(g.z[j]);
            }
        }
    }
}

/// Carve half-width of the channel at `k`: half the bankfull width, at
/// least 60 m so the 100 m samples always see it (Q8).
pub(super) fn channel_radius(g: &Lattice, s: &Stream, area: &[u32], k: usize) -> i64 {
    let km2 = u64::from(area[s.cells[k]]) * super::cell_m2(g) / 1_000_000;
    let w_dm = arda_core::hydrology::channel_width_dm(nominal_discharge(km2)).unwrap_or(0);
    m_to_q8(g, (i64::from(w_dm) / 20).max(60))
}

pub(super) fn carve_depth_mm(g: &Lattice, s: &Stream, area: &[u32], k: usize) -> i64 {
    let km2 = u64::from(area[s.cells[k]]) * super::cell_m2(g) / 1_000_000;
    let d_cm = arda_core::water::bankfull_depth_cm(nominal_discharge(km2)).unwrap_or(0);
    (i64::from(d_cm) * 10).max(MIN_DEPTH_MM)
}

/// Maximal index ranges where the plan meanders, at least 1.5 wavelengths
/// long and clear of the stream's last wavelength (its confluence).
fn spans(s: &Stream, p: &Plan) -> Vec<(usize, usize)> {
    let m = s.cells.len();
    let mut out = Vec::new();
    let mut k = 0;
    let end_guard = s.along[m - 1] - p.lambda[m - 1];
    while k < m {
        if p.omega[k] == 0 || s.along[k] > end_guard {
            k += 1;
            continue;
        }
        let a = k;
        while k < m && p.omega[k] > 0 && s.along[k] <= end_guard {
            k += 1;
        }
        let b = k - 1;
        if s.along[b] - s.along[a] >= p.lambda[a] * 3 / 2 {
            out.push((a, b));
        }
    }
    out
}

/// Applies meanders to every eligible stream and returns the planned
/// oxbow cutoffs, which [`carve_cutoffs`] carves after the channels have
/// been drained. `claimed` marks stream positions already shaped by another
/// pass.
pub fn apply(
    g: &mut Lattice,
    streams: &[Stream],
    area: &[u32],
    claimed: &mut [Vec<bool>],
    seed: u64,
    features: &mut WaterFeatures,
) -> Vec<super::oxbow::Cutoff> {
    let mut cutoffs = Vec::new();
    for (si, s) in streams.iter().enumerate() {
        let p = plan(g, s, area, hash3(seed, si as i64, 1));
        for (a, b) in spans(s, &p) {
            if claimed[si][a..=b].iter().any(|&c| c) {
                continue;
            }
            let curve = integrate(s, &p, a, b, hash3(seed, si as i64, a as i64));
            if curve.len() < 4 {
                continue;
            }
            plane_belt(g, s, area, &p, (a, b));
            carve_course(g, s, area, &curve, a, b);
            let valley = s.along[b] - s.along[a];
            let length: i64 = curve
                .windows(2)
                .map(|w| {
                    let (dx, dy) = (w[1].0 .0 - w[0].0 .0, w[1].0 .1 - w[0].0 .1);
                    isqrt_i(i128::from(dx * dx + dy * dy))
                })
                .sum();
            features.stats.meander_valley_q8 += valley;
            features.stats.meander_course_q8 += length;
            features.stats.meander_spans += 1;
            super::oxbow::plan(
                g,
                s,
                area,
                &p,
                &curve,
                (a, b),
                hash3(seed, si as i64, -(a as i64)),
                &mut cutoffs,
            );
            claimed[si][a..=b].iter_mut().for_each(|c| *c = true);
        }
    }
    cutoffs
}

/// Smooth 1-D noise in −1000..=1000 over `t` with knots every `period`.
fn wobble(seed: u64, t: i64, period: i64) -> i64 {
    let period = period.max(1);
    let (k, f) = (t.div_euclid(period), t.rem_euclid(period) * 1000 / period);
    let at = |k: i64| pick(hash3(seed, k, 5), 2_001) as i64 - 1_000;
    // Smoothstep between knots.
    let sf = f * f * (3_000 - 2 * f) / 1_000_000;
    at(k) + (at(k + 1) - at(k)) * sf / 1000
}

/// Integrates the Kinoshita curve from `a` to `b`: points (Q8) with the
/// stream position each is nearest to.
fn integrate(s: &Stream, p: &Plan, a: usize, b: usize, seed: u64) -> Vec<((i64, i64), usize)> {
    let pts = |k: usize| s.centre[k];
    let start = s.centre[a];
    let mut pos = start;
    let mut kk = a;
    let mut phase = pick(seed, TURN as u64) as i64;
    let skew = if seed & 1 == 0 { 1 } else { -1 };
    let ds = CELL_Q8 / 2;
    let mut out = vec![(pos, a)];
    let max_steps = usize::try_from((s.along[b] - s.along[a]) * 4 / ds).unwrap_or(0) + 64;
    let ramp_len = p.lambda[a] / 2;
    let mut travelled = 0_i64;
    for _ in 0..max_steps {
        // Nearest centreline position, searching a short window ahead.
        let d2 = |k: usize| {
            let (dx, dy) = (pos.0 - pts(k).0, pos.1 - pts(k).1);
            dx * dx + dy * dy
        };
        let hi = (kk + 24).min(b);
        let mut best = kk;
        for k in kk..=hi {
            if d2(k) < d2(best) {
                best = k;
            }
        }
        kk = best;
        if kk >= b {
            break;
        }
        let t = s.dir[kk];
        let n = (-t.1, t.0);
        // Ends join the original course; the interior centres on the floor.
        let edge = (s.along[kk] - s.along[a]).min(s.along[b] - s.along[kk]);
        let ramp = (edge * 1000 / ramp_len.max(1)).clamp(0, 1000);
        let reference = (
            pts(kk).0 + n.0 * p.shift[kk] / ONE_Q14 * ramp / 1000,
            pts(kk).1 + n.1 * p.shift[kk] / ONE_Q14 * ramp / 1000,
        );
        let e = ((pos.0 - reference.0) * n.0 + (pos.1 - reference.1) * n.1) / ONE_Q14;
        let arc = (p.lambda[kk] * p.sinuosity[kk] / 1000).max(CELL_Q8);
        // Natural bends differ: amplitude varies ±28% over about 1.3 bends
        // and bend length ±25% over about 2 bends.
        let amp_mod = 1_000 + 280 * wobble(seed ^ 0xA1, travelled, arc * 13 / 10) / 1_000;
        let len_mod = 1_000 + 250 * wobble(seed ^ 0xA2, travelled, arc * 21 / 10) / 1_000;
        let omega = p.omega[kk] * ramp / 1000 * amp_mod / 1000;
        // Kinoshita: ω cos φ + ω³ (Js sin 3φ − Jf cos 3φ), Js = 1/12,
        // Jf = 1/64, with ω³ in turns: ω³ (2π)² / TURN². The skew term
        // leans the bends downstream (or upstream) like real loops.
        let w3 = i128::from(omega).pow(3) * 39_478 / 1000 / i128::from(TURN).pow(2);
        let w3 = i64::try_from(w3).unwrap_or(0);
        let kin = omega * cos_q14(phase) / ONE_Q14 + skew * w3 * sin_q14(3 * phase) / ONE_Q14 / 12
            - w3 * cos_q14(3 * phase) / ONE_Q14 / 64;
        let correct = e * TURN / (2 * arc);
        let theta = (kin - correct).clamp(-TURN * 3 / 10, TURN * 3 / 10);
        let h = rotate(t, theta);
        pos = (pos.0 + h.0 * ds / ONE_Q14, pos.1 + h.1 * ds / ONE_Q14);
        phase += ds * TURN / (arc * len_mod / 1000).max(1);
        travelled += ds;
        out.push((pos, kk));
    }
    out.push((s.centre[b], b));
    out
}

/// Carves the new course with a monotone bed one carve depth below the
/// floor, then runs the channel on at that level along the old course until
/// the old bed has fallen below it.
fn carve_course(
    g: &mut Lattice,
    s: &Stream,
    area: &[u32],
    curve: &[((i64, i64), usize)],
    a: usize,
    b: usize,
) {
    let total = curve.len().max(2) - 1;
    let (za, zb) = (s.bed(g, a), s.bed(g, b));
    let mut bed = i64::MAX;
    let beds: Vec<i64> = curve.iter().map(|&(_, k)| s.bed(g, k)).collect();
    for (i, &(p, k)) in curve.iter().enumerate() {
        let depth = carve_depth_mm(g, s, area, k);
        let lin = za + (zb - za) * i as i64 / total as i64;
        bed = bed.min(lin.min(beds[i]) - depth);
        carve_disc(g, p, channel_radius(g, s, area, k), bed, depth);
    }
    for k in b + 1..s.cells.len() {
        if s.bed(g, k) <= bed || g.z[s.cells[k]] <= 0 {
            break;
        }
        let depth = carve_depth_mm(g, s, area, k);
        let c = s.cells[k];
        let p = (
            (c % g.width) as i64 * CELL_Q8,
            (c / g.width) as i64 * CELL_Q8,
        );
        carve_disc(g, p, channel_radius(g, s, area, k), bed, depth);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sinuosity_rises_as_slope_falls() {
        let q = DischargeMilli::new(5_000);
        let steep = target_sinuosity(4_000, q);
        let mid = target_sinuosity(1_000, q);
        let flat = target_sinuosity(100, q);
        assert_eq!(steep, 1_000, "above the braiding threshold: no meanders");
        assert!(flat > mid && mid > 1_050, "{flat} {mid}");
        assert!(flat <= 2_200);
        assert!(omega_for(flat) > omega_for(mid));
    }
}
