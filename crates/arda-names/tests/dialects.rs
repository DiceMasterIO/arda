//! Dialect similarity decays with distance across a realm.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_names::meaning::ALL;
use arda_names::{place_name, DialectMap, Language, PlaceKind, PlaceSpec, Preset};

fn levenshtein(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            cur.push(
                (prev[j] + usize::from(ca != cb))
                    .min(prev[j + 1] + 1)
                    .min(cur[j] + 1),
            );
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Mean normalised edit distance between two dialects' lexicons.
fn distance(a: &Language, b: &Language) -> f64 {
    let total: f64 = ALL
        .iter()
        .map(|&m| {
            let (x, y) = (a.word(m), b.word(m));
            levenshtein(&x, &y) as f64 / x.len().max(y.len()).max(1) as f64
        })
        .sum();
    total / ALL.len() as f64
}

#[test]
fn neighbours_sound_alike_and_far_towns_differ() {
    const SIZE: i64 = 1000;
    for p in Preset::ALL {
        let mut bins = [0.0_f64; 4];
        for seed in 0..6_u64 {
            let lang = Language::new(seed, p);
            let map = DialectMap::new(&lang, seed, SIZE, SIZE);
            assert!(
                map.isoglosses.len() >= 5,
                "{p:?}: {} isoglosses",
                map.isoglosses.len()
            );
            for origin in 0..8_i64 {
                let (ox, oy) = (40 + origin * 30, 60 + origin * 25);
                let here = map.dialect_at(ox, oy);
                for (bin, step) in [10_i64, 100, 300, 700].into_iter().enumerate() {
                    let (dx, dy) = if origin % 2 == 0 {
                        (step, 0)
                    } else {
                        (0, step)
                    };
                    let there = map.dialect_at(ox + dx, oy + dy);
                    bins[bin] += distance(&here, &there);
                }
            }
        }
        assert!(bins[0] < bins[1] && bins[1] < bins[2], "{p:?}: {bins:?}");
        assert!(bins[2] <= bins[3] * 1.15, "{p:?}: {bins:?}");
        assert!(bins[0] < bins[3] * 0.25, "{p:?}: {bins:?}");
    }
}

#[test]
fn dialects_change_names_but_keep_them_readable() {
    let lang = Language::new(4, Preset::Coastal);
    let map = DialectMap::new(&lang, 4, 500, 500);
    let spec = PlaceSpec::new(PlaceKind::Town, 9);
    let mut forms = std::collections::BTreeSet::new();
    for x in (0..500).step_by(50) {
        let d = map.dialect_at(x, 250);
        let n = place_name(&d, &spec);
        assert!(
            n.native
                .chars()
                .all(|c| c.is_ascii_alphabetic() || c == '\''),
            "{}",
            n.native
        );
        forms.insert(n.native);
    }
    assert!(forms.len() >= 2, "{forms:?}");
    // Extreme coordinates must not overflow the isogloss side test.
    let _ = map.dialect_at(i64::MIN, i64::MAX);
    let _ = map.dialect_at(i64::MAX, i64::MIN);
}
