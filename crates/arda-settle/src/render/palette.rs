//! One colour per realm (logic/16 §political-map): a hand-curated
//! categorical palette of sixteen strongly distinct hues, none of them
//! the olive or green of lowland relief (every one is at least 0.15
//! OKLab from the Atlas ground), then assigned so that neighbouring
//! realms are as far apart as the set allows.
//!
//! The first twelve (a default-size world) are the best twelve of the
//! hand-picked families, listed farthest-first, and the last four are the
//! farthest additions to them; every pair of the sixteen is at least 0.12
//! OKLab apart. A world of `n <= 16` realms takes the first `n`, so no two
//! realms share a hue family. Beyond sixteen the set is
//! extended deterministically with a deep (dark) and then a pale variant
//! of each, and past forty-eight the whole run repeats.
//!
//! The arithmetic is `+ − × ÷` and `sqrt` only (the sRGB gamma by Newton
//! steps, the colours as stored OKLab triples), so the colours, which
//! `/v1/realms` serves, round alike on every platform.

use crate::canvas::Rgb;
use crate::output::RealmsFile;
use std::collections::BTreeMap;

/// The base palette as OKLab `[L, a, b]`, farthest-first, with the sRGB
/// hex each was designed as.
pub const BASE: [[f64; 3]; 16] = [
    // navy #1c3a94
    [0.387_084, -0.011_961, -0.151_893],
    // lemon yellow #f5dc2e
    [0.888_580, -0.031_524, 0.172_178],
    // hot pink #ff4fa3
    [0.695_024, 0.222_176, -0.018_224],
    // periwinkle #8d9bf0
    [0.713_292, 0.012_307, -0.124_334],
    // umber #6e3f22
    [0.418_425, 0.049_297, 0.060_598],
    // orange #ff8a1f
    [0.748_128, 0.099_908, 0.144_654],
    // turquoise #27d3c5
    [0.783_085, -0.130_362, -0.014_005],
    // crimson #c4122f
    [0.523_600, 0.188_578, 0.075_765],
    // charcoal #25252c
    [0.267_428, 0.003_451, -0.012_450],
    // orchid #b04fd0
    [0.603_957, 0.148_573, -0.139_381],
    // steel blue #4f6d8f
    [0.525_524, -0.019_760, -0.061_282],
    // rose #f7a8be
    [0.813_928, 0.096_562, 0.002_151],
    // plum #5e1f55
    [0.357_463, 0.103_410, -0.050_963],
    // cerulean #1e88e5
    [0.617_879, -0.054_649, -0.157_573],
    // berry #9b1b6e
    [0.469_926, 0.174_160, -0.040_711],
    // mauve #9e6b8a
    [0.590_956, 0.073_851, -0.023_129],
];
/// Realms the base palette serves without variants.
pub const BASE_LEN: usize = BASE.len();
/// Most realms served with colours distinct from each other (base, deep
/// and pale); past this the run repeats.
pub const DISTINCT_MAX: usize = BASE_LEN * 3;
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
        let colours: Vec<[f64; 3]> = (0..n).map(candidate).collect();
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

/// Colour `k` of the run as OKLab: the base set, then its deep variants,
/// then its pale ones, then round again.
fn candidate(k: usize) -> [f64; 3] {
    let k = k % DISTINCT_MAX;
    let [l, a, b] = BASE[k % BASE_LEN];
    match k / BASE_LEN {
        0 => [l, a, b],
        // Deep: darker, a little less chroma.
        1 => [l * 0.72, a * 0.9, b * 0.9],
        // Pale: lifted towards white, chroma eased.
        _ => [l + (1.0 - l) * 0.45, a * 0.75, b * 0.75],
    }
}

/// OKLab distance (plain Euclid: the palette differs in value as well as
/// hue, and the map is read at a glance).
fn delta(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
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
