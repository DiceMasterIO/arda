//! Presets sound clearly different: their phoneme and bigram distributions
//! are far apart, and further apart than two seeds of one preset.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_names::meaning::ALL;
use arda_names::{given_name, place_name, Language, PlaceKind, PlaceSpec, Preset, Sex};
use std::collections::BTreeMap;

type Dist = BTreeMap<String, f64>;

/// Vocabulary of one language: every root plus sample names.
fn words(lang: &Language) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = ALL
        .iter()
        .map(|&m| lang.root(m).iter().map(|p| p.0).collect())
        .collect();
    out.extend(
        lang.name_roots
            .iter()
            .map(|w| w.iter().map(|p| p.0).collect()),
    );
    for key in 0..150 {
        let n = place_name(lang, &PlaceSpec::new(PlaceKind::Village, key));
        out.push(n.word.iter().map(|p| p.0).collect());
        let g = given_name(lang, Sex::Female, key);
        out.push(g.word.iter().map(|p| p.0).collect());
    }
    out
}

/// Pooled vocabulary of a preset over several seeds, so the statistic
/// measures the preset rather than one seed's random lexicon.
fn pooled(p: Preset, seeds: std::ops::Range<u64>) -> (Dist, Dist) {
    let ws: Vec<Vec<u8>> = seeds.flat_map(|s| words(&Language::new(s, p))).collect();
    distributions(&ws)
}

fn distributions(ws: &[Vec<u8>]) -> (Dist, Dist) {
    let (mut uni, mut bi) = (Dist::new(), Dist::new());
    for w in ws {
        let padded: Vec<String> = std::iter::once("#".to_string())
            .chain(w.iter().map(|p| p.to_string()))
            .chain(std::iter::once("#".to_string()))
            .collect();
        for p in &padded[1..padded.len() - 1] {
            *uni.entry(p.clone()).or_default() += 1.0;
        }
        for pair in padded.windows(2) {
            *bi.entry(format!("{}-{}", pair[0], pair[1])).or_default() += 1.0;
        }
    }
    (normalise(uni), normalise(bi))
}

fn normalise(mut d: Dist) -> Dist {
    let total: f64 = d.values().sum();
    d.values_mut().for_each(|v| *v /= total);
    d
}

/// Jensen-Shannon divergence in bits (0 identical, 1 disjoint).
fn jsd(a: &Dist, b: &Dist) -> f64 {
    let keys: std::collections::BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    let mut sum = 0.0;
    for k in keys {
        let (p, q) = (
            a.get(k).copied().unwrap_or(0.0),
            b.get(k).copied().unwrap_or(0.0),
        );
        let m = (p + q) / 2.0;
        if p > 0.0 {
            sum += 0.5 * p * (p / m).log2();
        }
        if q > 0.0 {
            sum += 0.5 * q * (q / m).log2();
        }
    }
    sum
}

#[test]
fn every_pair_of_presets_is_far_apart() {
    let dists: Vec<(Preset, (Dist, Dist))> =
        Preset::ALL.iter().map(|&p| (p, pooled(p, 0..6))).collect();
    let mut closest = (1.0_f64, 1.0_f64, String::new());
    for (i, (pa, (ua, ba))) in dists.iter().enumerate() {
        for (pb, (ub, bb)) in &dists[i + 1..] {
            let (u, b) = (jsd(ua, ub), jsd(ba, bb));
            if b < closest.1 {
                closest = (u, b, format!("{pa:?}/{pb:?}"));
            }
            assert!(u > 0.05, "{pa:?} vs {pb:?}: phoneme JSD {u:.3}");
            assert!(b > 0.15, "{pa:?} vs {pb:?}: bigram JSD {b:.3}");
        }
    }
    eprintln!(
        "closest presets {}: phoneme {:.3}, bigram {:.3}",
        closest.2, closest.0, closest.1
    );
}

#[test]
fn seeds_of_a_preset_are_closer_than_other_presets() {
    for &p in &Preset::ALL {
        let base = pooled(p, 0..6);
        let same = pooled(p, 6..12);
        let within = jsd(&base.1, &same.1);
        let nearest_other = Preset::ALL
            .iter()
            .filter(|&&q| q != p)
            .map(|&q| (jsd(&base.1, &pooled(q, 6..12).1), q))
            .fold((f64::MAX, p), |a, b| if b.0 < a.0 { b } else { a });
        let (nearest_other, other) = nearest_other;
        eprintln!("{p:?}: within {within:.3} vs nearest other {other:?} {nearest_other:.3}");
        assert!(
            within * 2.0 < nearest_other,
            "{p:?}: within {within:.3} vs nearest other {nearest_other:.3}"
        );
    }
}
