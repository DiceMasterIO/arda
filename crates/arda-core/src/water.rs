//! River and lake forms (logic/02 §world-water, goals 8-13).
//!
//! Hydraulic geometry and the stored per-segment and per-lake forms that
//! recipe-5 worlds publish beside each area's objects. All laws are integer
//! and shared by generation, loading and rendering.

use crate::coords::CellCoord;
use crate::fixed::DischargeMilli;

/// Bankfull depth in centimetres from mean discharge: `0.3 m × Q^0.4`
/// with `Q` in m³/s (Leopold & Maddock at-a-station exponent 0.4; the
/// matching width law is [`crate::hydrology::channel_width_dm`], `4 m × Q^0.5`).
/// Returns `None` only on arithmetic overflow.
#[must_use]
pub fn bankfull_depth_cm(discharge: DischargeMilli) -> Option<u32> {
    // depth_cm = 30 × (raw / 1000)^0.4 = (30^5 × raw² / 1000²)^(1/5)
    //          = (243 × raw² / 10)^(1/5).
    let q = u128::from(discharge.raw());
    let n = q.checked_mul(q)?.checked_mul(243)? / 10;
    u32::try_from(iroot5(n)).ok()
}

/// Floor of the fifth root of `n`.
fn iroot5(n: u128) -> u128 {
    if n < 2 {
        return n;
    }
    // Binary search on the root: at most 26 bits for any u128 input.
    let (mut lo, mut hi) = (1_u128, 1_u128 << 26);
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let p = mid
            .checked_mul(mid)
            .and_then(|v| v.checked_mul(mid))
            .and_then(|v| v.checked_mul(mid))
            .and_then(|v| v.checked_mul(mid));
        if p.is_some_and(|p| p <= n) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo
}

/// Leopold–Wolman braiding threshold slope in parts per million for a mean
/// discharge: `0.0125 × (4 Q)^-0.44` (SI, `Q` in m³/s), with bankfull
/// discharge taken as four times the mean. Above it a channel with ample bed
/// load braids; below it a free alluvial channel meanders.
#[must_use]
pub fn braiding_slope_ppm(discharge: DischargeMilli) -> u32 {
    // log2(4 Q_m3s) = log2(raw) + 2 - log2(1000), in Q16.
    let q = discharge.raw().max(1);
    let l = log2_q16(q) + 2 * 65_536 - log2_q16(1_000);
    // 12_500 ppm × 2^(-0.44 l).
    let e = -(l * 44 / 100);
    let v = u128::from(exp2_q16(e)) * 12_500 / 65_536;
    u32::try_from(v).unwrap_or(u32::MAX)
}

/// Base-2 logarithm of `x >= 1` in Q16 (16 fractional bits, exact to the
/// last bit by repeated squaring).
#[must_use]
pub fn log2_q16(x: u64) -> i64 {
    let x = x.max(1);
    let int = 63 - i64::from(x.leading_zeros());
    // Mantissa in Q62: m in [1, 2).
    let mut m = u128::from(x) << (62 - int);
    let mut frac = 0_i64;
    for bit in (0..16).rev() {
        m = (m * m) >> 62;
        if m >= 2 << 62 {
            m >>= 1;
            frac |= 1 << bit;
        }
    }
    int * 65_536 + frac
}

/// `2^(e / 65536)` in Q16 for `e` in about ±40 × 65536; saturates.
#[must_use]
pub fn exp2_q16(e: i64) -> u64 {
    let int = e.div_euclid(65_536);
    let frac = e.rem_euclid(65_536);
    // 2^frac by binary expansion of the fraction: product of 2^(2^-k).
    // ROOTS[k] = 2^(2^-(k+1)) in Q30.
    const ROOTS: [u64; 16] = [
        1_518_500_250,
        1_276_901_417,
        1_170_923_762,
        1_121_280_436,
        1_097_253_708,
        1_085_434_106,
        1_079_572_136,
        1_076_653_033,
        1_075_196_443,
        1_074_468_888,
        1_074_105_294,
        1_073_923_544,
        1_073_832_680,
        1_073_787_251,
        1_073_764_537,
        1_073_753_181,
    ];
    let mut v: u128 = 1 << 30;
    for (k, &r) in ROOTS.iter().enumerate() {
        if frac & (1 << (15 - k)) != 0 {
            v = (v * u128::from(r)) >> 30;
        }
    }
    // Scale Q30 to Q16 and apply the integer part.
    let shift = int + 16 - 30;
    let out = if shift >= 0 {
        v.checked_shl(u32::try_from(shift).unwrap_or(127))
            .unwrap_or(u128::MAX)
    } else {
        v >> u32::try_from(-shift).unwrap_or(127).min(127)
    };
    u64::try_from(out).unwrap_or(u64::MAX)
}

/// Window over which sinuosity is measured, cells (2 km).
const WINDOW: usize = 20;

/// Length of one D8 step between neighbouring cells, thousandths of a cell.
#[must_use]
pub fn d8_step_permille(a: CellCoord, b: CellCoord) -> i64 {
    let (dx, dy) = (
        i64::from(a.x()) - i64::from(b.x()),
        i64::from(a.y()) - i64::from(b.y()),
    );
    if dx != 0 && dy != 0 {
        1_414
    } else {
        1_000 * (dx.abs() + dy.abs()).min(1)
    }
}

/// Sinuosity (‰) of a saved 100 m course: course length over straight
/// distance, averaged over 2 km (20-cell) windows starting within `own`;
/// the whole path's ratio when it is shorter than a window. A straight D8
/// staircase measures at most about 1.08.
#[must_use]
pub fn course_sinuosity_permille(path: &[CellCoord], own: std::ops::Range<usize>) -> u16 {
    let len = |a: usize, b: usize| {
        (a..b)
            .map(|i| d8_step_permille(path[i], path[i + 1]))
            .sum::<i64>()
    };
    let exact_chord = |a: CellCoord, b: CellCoord| {
        let (dx, dy) = (
            i64::from(a.x()) - i64::from(b.x()),
            i64::from(a.y()) - i64::from(b.y()),
        );
        i64::try_from(((dx * dx + dy * dy) * 1_000_000).unsigned_abs().isqrt()).unwrap_or(0)
    };
    let m = path.len();
    if m < 2 {
        return 1_000;
    }
    let ratio = |a: usize, b: usize| len(a, b) * 1000 / exact_chord(path[a], path[b]).max(1);
    if m <= WINDOW {
        return u16::try_from(ratio(0, m - 1).clamp(1_000, 9_999)).unwrap_or(1_000);
    }
    let (mut sum, mut n) = (0, 0);
    for i in own.start.min(m - 1 - WINDOW)..own.end.min(m - WINDOW) {
        sum += ratio(i, i + WINDOW);
        n += 1;
    }
    if n == 0 {
        return u16::try_from(ratio(0, m - 1).clamp(1_000, 9_999)).unwrap_or(1_000);
    }
    u16::try_from((sum / n).clamp(1_000, 9_999)).unwrap_or(1_000)
}

/// A 2 km measuring window centred on one river segment, following the
/// largest upstream feeder and the downstream chain across segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseWindow {
    /// Course length over straight distance, ‰.
    pub sinuosity_permille: u16,
    /// Course length of the window, thousandths of a cell.
    pub run_permille: i64,
    /// Upstream end of the window.
    pub first: CellCoord,
    /// Downstream end of the window.
    pub last: CellCoord,
}

/// Measuring windows for every segment of one area, in order. Segments
/// whose connected course is shorter than a window use all of it.
#[must_use]
pub fn course_windows(rivers: &[crate::objects::RiverSegment]) -> Vec<CourseWindow> {
    use std::collections::BTreeMap;
    let by_id: BTreeMap<u32, usize> = rivers.iter().enumerate().map(|(i, r)| (r.id, i)).collect();
    // Largest feeder of each segment.
    let mut feeder: Vec<Option<usize>> = vec![None; rivers.len()];
    for (i, r) in rivers.iter().enumerate() {
        if let Some(&d) = r.feeds.and_then(|id| by_id.get(&id)) {
            let better = feeder[d].is_none_or(|f| {
                (rivers[i].discharge, rivers[i].id) > (rivers[f].discharge, rivers[f].id)
            });
            if better {
                feeder[d] = Some(i);
            }
        }
    }
    let half = WINDOW / 2 + 1;
    let push = |path: &mut Vec<CellCoord>, cells: &[CellCoord]| {
        for &c in cells {
            if path.last() != Some(&c) {
                path.push(c);
            }
        }
    };
    rivers
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut up: Vec<usize> = Vec::new();
            let (mut n, mut cur) = (0, i);
            while n < half {
                let Some(f) = feeder[cur] else { break };
                if up.contains(&f) {
                    break;
                }
                up.push(f);
                n += rivers[f].course.len();
                cur = f;
            }
            let mut path = Vec::new();
            for &u in up.iter().rev() {
                push(&mut path, &rivers[u].course);
            }
            let tail = path.len().saturating_sub(half);
            path.drain(..tail);
            let own = path.len()..path.len() + r.course.len();
            push(&mut path, &r.course);
            let (mut next, mut steps) = (r.feeds, 0);
            while let Some(&d) = next.and_then(|id| by_id.get(&id)) {
                push(&mut path, &rivers[d].course);
                steps += 1;
                if path.len() >= own.end + half || steps > 4 * WINDOW {
                    break;
                }
                next = rivers[d].feeds;
            }
            path.truncate(own.end + half);
            let m = path.len();
            let (a, b) = if m > WINDOW {
                let mid = (own.start + own.end) / 2;
                let a = mid.saturating_sub(WINDOW / 2).min(m - 1 - WINDOW);
                (a, a + WINDOW)
            } else {
                (0, m.saturating_sub(1))
            };
            let window = &path[a..=b];
            CourseWindow {
                sinuosity_permille: course_sinuosity_permille(window, 0..1),
                run_permille: window
                    .windows(2)
                    .map(|w| d8_step_permille(w[0], w[1]))
                    .sum(),
                first: window[0],
                last: window[window.len() - 1],
            }
        })
        .collect()
}

/// Planform of a river segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ChannelPattern {
    /// Single thread, sinuosity below 1.25.
    #[default]
    Straight = 0,
    /// Single thread, sinuosity at least 1.25 on a low gradient.
    Meandering = 1,
    /// Several threads across a gravel braidplain.
    Braided = 2,
}

impl ChannelPattern {
    /// Decodes a stored discriminant.
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Straight),
            1 => Some(Self::Meandering),
            2 => Some(Self::Braided),
            _ => None,
        }
    }
}

/// Geological origin of a lake basin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
#[repr(u8)]
pub enum LakeOrigin {
    /// No recorded formation cause.
    #[default]
    Unclassified = 0,
    /// Endorheic tectonic basin sink.
    Tectonic = 1,
    /// Overdeepened glacial trough behind a moraine.
    Glacial = 2,
    /// Abandoned meander loop.
    Oxbow = 3,
    /// Karst polje.
    Karst = 4,
}

impl LakeOrigin {
    /// Decodes a stored discriminant.
    #[must_use]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Unclassified),
            1 => Some(Self::Tectonic),
            2 => Some(Self::Glacial),
            3 => Some(Self::Oxbow),
            4 => Some(Self::Karst),
            _ => None,
        }
    }
}

/// Stored form of one area river segment, in the order of the area's rivers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SegmentForm {
    /// Planform.
    pub pattern: ChannelPattern,
    /// Single-thread bankfull width, decimetres (`4 m × Q^0.5`).
    pub bankfull_width_dm: u32,
    /// Bankfull depth, centimetres (`0.3 m × Q^0.4`).
    pub bankfull_depth_cm: u32,
    /// Width of the active belt: the braidplain for braided reaches, the
    /// bankfull channel otherwise; decimetres.
    pub belt_width_dm: u32,
    /// Course length over straight-line length, thousandths.
    pub sinuosity_permille: u16,
    /// Channel bed slope along the course, parts per million.
    pub slope_ppm: u32,
}

/// Stored form of one area lake fragment, in the order of the area's lakes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LakeForm {
    /// Geological origin.
    pub origin: LakeOrigin,
    /// No surface outflow: a terminal, evaporation-balanced (saline) lake.
    pub terminal: bool,
}

/// A river-mouth delta whose apex lies in this area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeltaForm {
    /// Apex cell.
    pub apex: CellCoord,
    /// Fan radius, metres.
    pub radius_m: u32,
    /// Catchment feeding it, km².
    pub catchment_km2: u32,
}

/// A karst doline (closed solution hollow) below the 100 m cell scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Doline {
    /// Cell containing its centre.
    pub at: CellCoord,
    /// Rim radius, decimetres.
    pub radius_dm: u16,
    /// Depth below the rim, decimetres.
    pub depth_dm: u16,
}

/// Every stored water form of one area (`areas/<ax>_<ay>/water.bin`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AreaWater {
    /// One form per river segment, same order as the area's rivers.
    pub segments: Vec<SegmentForm>,
    /// One form per lake fragment, same order as the area's lakes.
    pub lakes: Vec<LakeForm>,
    /// Deltas with their apex in this area.
    pub deltas: Vec<DeltaForm>,
    /// Karst dolines in this area.
    pub dolines: Vec<Doline>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(x: u16, y: u16) -> CellCoord {
        CellCoord::new(x, y).unwrap()
    }

    #[test]
    fn straight_courses_measure_near_one_and_bends_more() {
        let straight: Vec<_> = (0..40).map(|i| c(10 + i, 20)).collect();
        assert_eq!(course_sinuosity_permille(&straight, 0..40), 1_000);
        // A D8 staircase at about 27° stays well below meandering.
        let stair: Vec<_> = (0..40).map(|i| c(10 + i, 20 + i / 2)).collect();
        assert!(course_sinuosity_permille(&stair, 0..40) < 1_100);
        // A square wave of amplitude 3 cells every 8 cells.
        let mut wave = Vec::new();
        let (mut x, mut y) = (10_u16, 50_u16);
        for k in 0..6 {
            for _ in 0..4 {
                x += 1;
                wave.push(c(x, y));
            }
            for _ in 0..3 {
                y = if k % 2 == 0 { y + 1 } else { y - 1 };
                wave.push(c(x, y));
            }
        }
        let s = course_sinuosity_permille(&wave, 0..wave.len());
        assert!(s >= 1_600, "{s}");
    }

    #[test]
    fn depth_follows_the_q_to_the_0_4_law() {
        // 1 m³/s → 30 cm; 100 m³/s → 30 × 100^0.4 = 189 cm.
        assert_eq!(bankfull_depth_cm(DischargeMilli::new(1_000)), Some(30));
        assert_eq!(bankfull_depth_cm(DischargeMilli::new(100_000)), Some(189));
        assert_eq!(
            bankfull_depth_cm(DischargeMilli::new(10_000_000)),
            Some(1_194)
        );
        assert_eq!(bankfull_depth_cm(DischargeMilli::new(0)), Some(0));
        // Tenfold discharge deepens by 10^0.4 = 2.51 and widens by 3.16.
        let d1 = bankfull_depth_cm(DischargeMilli::new(20_000)).unwrap();
        let d10 = bankfull_depth_cm(DischargeMilli::new(200_000)).unwrap();
        assert!((250..=253).contains(&(d10 * 100 / d1)), "{d1} {d10}");
        let w1 = crate::hydrology::channel_width_dm(DischargeMilli::new(20_000)).unwrap();
        let w10 = crate::hydrology::channel_width_dm(DischargeMilli::new(200_000)).unwrap();
        assert!((315..=317).contains(&(w10 * 100 / w1)), "{w1} {w10}");
    }

    #[test]
    fn braiding_threshold_falls_with_discharge() {
        let s = |q| braiding_slope_ppm(DischargeMilli::new(q));
        // 1 m³/s: 12500 × 4^-0.44 = 6,792 ppm; 10 m³/s: 12500 × 40^-0.44 = 2,466 ppm.
        assert!((6_780..=6_800).contains(&s(1_000)), "{}", s(1_000));
        assert!((2_460..=2_470).contains(&s(10_000)), "{}", s(10_000));
        assert!(s(1_000) > s(5_000) && s(5_000) > s(10_000));
    }

    #[test]
    fn fixed_point_log_and_exp_are_inverse() {
        assert_eq!(log2_q16(1), 0);
        assert_eq!(log2_q16(1024), 10 * 65_536);
        assert_eq!(exp2_q16(0), 65_536);
        assert_eq!(exp2_q16(3 * 65_536), 8 * 65_536);
        for x in [3_u64, 1_000, 123_456, 9_876_543_210] {
            let back = u128::from(exp2_q16(log2_q16(x)));
            let x = u128::from(x) * 65_536;
            assert!(
                back * 1000 / x >= 998 && back * 1000 / x <= 1002,
                "{x} {back}"
            );
        }
    }
}
