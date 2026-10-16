//! One colour per realm (logic/16 §political-map): as many hues as the
//! world has realms, evenly spaced round the OKLab hue circle (less the
//! olive and green of lowland relief) in three
//! lightness steps, then assigned so that neighbouring realms are as far
//! apart as the set allows. No colour is used twice, so two realms never
//! read as one country.
//!
//! The arithmetic is `+ − × ÷` and `sqrt` only (cosine, sine and the sRGB
//! gamma by series and Newton steps), so the colours, which `/v1/realms`
//! serves, round alike on every platform.

use crate::canvas::Rgb;
use crate::output::RealmsFile;
use std::collections::BTreeMap;

/// Chroma of a realm colour before it is pulled into the sRGB gamut.
const CHROMA: f64 = 0.19;
/// Lightness steps (OKLab `L`) taken in turn round the hue circle.
const LIGHTNESS: [f64; 3] = [0.64, 0.55, 0.72];
/// Hue of the first colour, degrees (a brick red).
const FIRST_HUE_DEG: f64 = 28.0;
/// OKLab hue arc left out, `[start, width]` degrees: the olive and green
/// of lowland relief, where a band would vanish into the ground.
const GAP_DEG: [f64; 2] = [95.0, 65.0];
/// Weight of lightness in [`delta`].
const L_WEIGHT: f64 = 0.5;
/// Most swap rounds of the neighbour refinement.
const ROUNDS: usize = 12;
/// Above this many realms the swap refinement is skipped (it is quadratic).
const REFINE_MAX: usize = 96;

/// The colour of every realm, by realm id.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Palette {
    by_id: BTreeMap<u64, Rgb>,
}

impl Palette {
    /// The colours of `realms`: one distinct colour each.
    #[must_use]
    pub fn of(realms: &RealmsFile) -> Self {
        let mut ids: Vec<u64> = realms.realms.iter().map(|r| r.id).collect();
        ids.sort_unstable();
        ids.dedup();
        let neighbours: BTreeMap<u64, Vec<u64>> = realms
            .realms
            .iter()
            .map(|r| (r.id, r.neighbours.clone()))
            .collect();
        Self::assign(&ids, &neighbours)
    }

    /// Colours for `ids` given each one's neighbours.
    #[must_use]
    pub fn assign(ids: &[u64], neighbours: &BTreeMap<u64, Vec<u64>>) -> Self {
        let n = ids.len();
        let colours: Vec<[f64; 3]> = (0..n).map(|k| candidate(k, n)).collect();
        let index: BTreeMap<u64, usize> = ids.iter().enumerate().map(|(k, &id)| (id, k)).collect();
        // Neighbour pairs as index pairs, each once.
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        for (&id, ns) in neighbours {
            let Some(&a) = index.get(&id) else { continue };
            for nb in ns {
                if let Some(&b) = index.get(nb) {
                    if a < b {
                        pairs.push((a, b));
                    } else if b < a {
                        pairs.push((b, a));
                    }
                }
            }
        }
        pairs.sort_unstable();
        pairs.dedup();
        let adj = |a: usize| -> Vec<usize> {
            pairs
                .iter()
                .filter_map(|&(p, q)| {
                    if p == a {
                        Some(q)
                    } else if q == a {
                        Some(p)
                    } else {
                        None
                    }
                })
                .collect()
        };
        // Most-constrained realms first take the colour farthest from
        // their coloured neighbours.
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by_key(|&a| (std::cmp::Reverse(adj(a).len()), a));
        let mut slot: Vec<Option<usize>> = vec![None; n];
        let mut used = vec![false; n];
        for a in order {
            let near: Vec<usize> = adj(a).into_iter().filter_map(|b| slot[b]).collect();
            let mut best: Option<(f64, usize)> = None;
            for (k, _) in used.iter().enumerate().filter(|(_, u)| !**u) {
                let score = near
                    .iter()
                    .map(|&c| delta(&colours[k], &colours[c]))
                    .fold(f64::INFINITY, f64::min);
                if best.is_none_or(|(s, _)| score > s) {
                    best = Some((score, k));
                }
            }
            if let Some((_, k)) = best {
                slot[a] = Some(k);
                used[k] = true;
            }
        }
        let mut slot: Vec<usize> = slot.into_iter().map(|s| s.unwrap_or(0)).collect();
        if n <= REFINE_MAX {
            refine(&mut slot, &pairs, &colours);
        }
        Self {
            by_id: ids
                .iter()
                .zip(&slot)
                .map(|(&id, &k)| (id, to_rgb(colours[k])))
                .collect(),
        }
    }

    /// The colour of realm `id`; ink grey for an unknown realm.
    #[must_use]
    pub fn colour(&self, id: u64) -> Rgb {
        self.by_id.get(&id).copied().unwrap_or([96, 88, 80])
    }

    /// The colour of realm `id` as `#rrggbb`, if the realm is known.
    #[must_use]
    pub fn hex(&self, id: u64) -> Option<String> {
        self.by_id
            .get(&id)
            .map(|c| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]))
    }

    /// Every `(realm id, colour)` in id order.
    pub fn iter(&self) -> impl Iterator<Item = (u64, Rgb)> + '_ {
        self.by_id.iter().map(|(&id, &c)| (id, c))
    }
}

/// Pairwise swaps that raise the smallest neighbour difference (then the
/// sum), in a fixed order, until none helps.
fn refine(slot: &mut [usize], pairs: &[(usize, usize)], colours: &[[f64; 3]]) {
    let score = |slot: &[usize]| {
        pairs.iter().fold((f64::INFINITY, 0.0), |(m, s), &(a, b)| {
            let d = delta(&colours[slot[a]], &colours[slot[b]]);
            (m.min(d), s + d)
        })
    };
    let mut best = score(slot);
    for _ in 0..ROUNDS {
        let mut better = false;
        for i in 0..slot.len() {
            for j in i + 1..slot.len() {
                slot.swap(i, j);
                let s = score(slot);
                if s.0 > best.0 + 1e-12 || (s.0 >= best.0 - 1e-12 && s.1 > best.1 + 1e-9) {
                    best = s;
                    better = true;
                } else {
                    slot.swap(i, j);
                }
            }
        }
        if !better {
            break;
        }
    }
}

/// Colour `k` of `n` as OKLab: hues evenly round the circle, lightness
/// stepping so hue neighbours also differ in value.
fn candidate(k: usize, n: usize) -> [f64; 3] {
    let turn = k as f64 / n.max(1) as f64;
    // Spread over the circle less the terrain's olive and green arc.
    let mut deg = FIRST_HUE_DEG + turn * (360.0 - GAP_DEG[1]);
    if deg >= GAP_DEG[0] {
        deg += GAP_DEG[1];
    }
    let h = deg / 360.0 * std::f64::consts::TAU;
    let l = if n < 7 {
        LIGHTNESS[0]
    } else {
        LIGHTNESS[k % 3]
    };
    let (c, s) = cos_sin(h);
    // Pull the chroma in until the colour is inside sRGB.
    let mut ch = CHROMA;
    for _ in 0..40 {
        let lab = [l, ch * c, ch * s];
        if linear_rgb(lab).iter().all(|v| (0.0..=1.0).contains(v)) {
            return lab;
        }
        ch *= 0.94;
    }
    [l, ch * c, ch * s]
}

/// OKLab difference with lightness at half weight: two realms of one hue
/// family in different values still read as kin on a map, so hue and
/// chroma carry the separation.
fn delta(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    let d = [(a[0] - b[0]) * L_WEIGHT, a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// Cosine and sine by range reduction and a Taylor series.
fn cos_sin(x: f64) -> (f64, f64) {
    let tau = std::f64::consts::TAU;
    let mut x = x - (x / tau).floor() * tau;
    if x > std::f64::consts::PI {
        x -= tau;
    }
    let (mut c, mut s) = (0.0, 0.0);
    let (mut tc, mut ts) = (1.0, x);
    for k in 0..24 {
        c += tc;
        s += ts;
        let k = f64::from(k);
        tc *= -x * x / ((2.0 * k + 1.0) * (2.0 * k + 2.0));
        ts *= -x * x / ((2.0 * k + 2.0) * (2.0 * k + 3.0));
    }
    (c, s)
}

/// OKLab to linear sRGB.
fn linear_rgb(lab: [f64; 3]) -> [f64; 3] {
    let [l, a, b] = lab;
    let l_ = l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
    let m_ = l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
    let s_ = l - 0.089_484_177_5 * a - 1.291_485_548 * b;
    let (l3, m3, s3) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
    [
        4.076_741_662_1 * l3 - 3.307_711_591_3 * m3 + 0.230_969_929_2 * s3,
        -1.268_438_004_6 * l3 + 2.609_757_401_1 * m3 - 0.341_319_396_5 * s3,
        -0.004_196_086_3 * l3 - 0.703_418_614_7 * m3 + 1.707_614_701 * s3,
    ]
}

/// `x^(1/2.4)` for `x` in `[0, 1]`: the root of `y^12 = x^5` by Newton steps.
fn root_2_4(x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let target = x * x * x * x * x;
    let mut y = x.sqrt().sqrt().max(0.05);
    for _ in 0..60 {
        let y11 = (0..11).fold(1.0, |p, _| p * y);
        let next = y - (y11 * y - target) / (12.0 * y11);
        if (next - y).abs() < 1e-15 {
            return next;
        }
        y = next.max(1e-9);
    }
    y
}

/// OKLab to 8-bit sRGB.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_rgb(lab: [f64; 3]) -> Rgb {
    linear_rgb(lab).map(|v| {
        let v = v.clamp(0.0, 1.0);
        let e = if v <= 0.003_130_8 {
            12.92 * v
        } else {
            1.055 * root_2_4(v) - 0.055
        };
        (e * 255.0).round().clamp(0.0, 255.0) as u8
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_series_match_libm() {
        for k in 0..64 {
            let x = f64::from(k) * 0.37 - 9.0;
            let (c, s) = cos_sin(x);
            assert!(
                (c - x.cos()).abs() < 1e-12 && (s - x.sin()).abs() < 1e-12,
                "{x}"
            );
        }
        for k in 1..100 {
            let x = f64::from(k) / 100.0;
            assert!((root_2_4(x) - x.powf(1.0 / 2.4)).abs() < 1e-12, "{x}");
        }
    }

    #[test]
    fn black_and_white_round_trip() {
        assert_eq!(to_rgb([1.0, 0.0, 0.0]), [255, 255, 255]);
        assert_eq!(to_rgb([0.0, 0.0, 0.0]), [0, 0, 0]);
    }
}
