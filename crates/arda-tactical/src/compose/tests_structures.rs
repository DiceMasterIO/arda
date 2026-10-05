//! Floor structures: a bridge or boardwalk of floor-layer props draws one
//! take between its touching placements, while separate structures,
//! pinned takes and everything off the floor layer pick as before.
#![allow(clippy::cast_precision_loss)]

use super::tests_variants::{picks, placeholder, plain, put, window, with_takes};
use super::*;
use crate::layout::{AssetRef, Placement};
use crate::layouts;
use std::collections::BTreeSet;

const DECK: &str = "prop.bridge_deck";

/// A horizontal bridge of `n` decks (each 2 × 1 squares turned) along row
/// `y`, starting at column `x0`.
fn bridge(l: &mut TacticalLayout, asset: &AssetRef, x0: u32, y: u32, n: u32) {
    for k in 0..n {
        l.placements.push(Placement {
            asset: asset.clone(),
            x: (x0 + 2 * k) as f32 + 1.0,
            y: y as f32 + 0.5,
            rotation: 90,
            mirror: false,
        });
    }
}

fn ids(
    lib: &Library,
    l: &TacticalLayout,
    seed: u64,
    range: std::ops::Range<usize>,
) -> BTreeSet<String> {
    picks(lib, l, seed)[range]
        .iter()
        .map(|p| p.0.clone())
        .collect()
}

#[test]
fn one_bridge_draws_one_take() {
    let lib = with_takes(&[DECK], 3, &[]);
    let mut l = TacticalLayout::new("bridge", 30, 8, "grass");
    bridge(&mut l, &AssetRef::Id(DECK.into()), 1, 3, 12);
    l.check(&lib).unwrap();
    for origin in [None, Some([4096, -300])] {
        l.origin = origin;
        let mut used = BTreeSet::new();
        for seed in 0..40 {
            let takes = ids(&lib, &l, seed, 0..12);
            assert_eq!(takes.len(), 1, "seed {seed}: {takes:?}");
            used.extend(takes);
        }
        // Across seeds the bridge still draws every take.
        assert_eq!(used.len(), 4, "{used:?}");
    }
}

#[test]
fn separate_bridges_may_differ() {
    let lib = with_takes(&[DECK], 3, &[]);
    let mut l = TacticalLayout::new("bridges", 30, 12, "grass");
    let deck = AssetRef::Id(DECK.into());
    bridge(&mut l, &deck, 1, 2, 6);
    bridge(&mut l, &deck, 1, 8, 6);
    for origin in [None, Some([12, 7])] {
        l.origin = origin;
        let differ = (0..64).any(|seed| {
            let (a, b) = (ids(&lib, &l, seed, 0..6), ids(&lib, &l, seed, 6..12));
            assert_eq!((a.len(), b.len()), (1, 1));
            a != b
        });
        assert!(differ, "two bridges always drew the same take");
    }
}

#[test]
fn pinned_takes_stay_pinned_inside_a_structure() {
    let lib = with_takes(&[DECK], 3, &[]);
    let mut l = TacticalLayout::new("pinned", 30, 8, "grass");
    bridge(&mut l, &AssetRef::Id(DECK.into()), 1, 3, 10);
    l.placements[4].asset = AssetRef::Id(format!("{DECK}.alt2"));
    for seed in 0..20 {
        let p = picks(&lib, &l, seed);
        assert_eq!(p[4].0, format!("{DECK}.alt2"));
        // The free decks on either side of the pin still form one bridge
        // each, and every pin of a whole bridge is kept.
        assert_eq!(ids(&lib, &l, seed, 0..4).len(), 1);
        assert_eq!(ids(&lib, &l, seed, 5..10).len(), 1);
    }
    let mut all = TacticalLayout::new("pinned", 30, 8, "grass");
    bridge(&mut all, &AssetRef::Id(format!("{DECK}.alt1")), 1, 3, 10);
    assert_eq!(
        ids(&lib, &all, 3, 0..10),
        BTreeSet::from([format!("{DECK}.alt1")])
    );
}

#[test]
fn a_structure_draws_only_takes_every_member_allows() {
    let base = with_takes(&[DECK], 3, &[]);
    let mut cat = base.catalog.clone();
    // Take 1 may not be turned, so a turned bridge never draws it.
    cat.assets
        .iter_mut()
        .find(|a| a.id == format!("{DECK}.alt1"))
        .unwrap()
        .rotations = vec![0];
    let images = cat
        .assets
        .iter()
        .map(|a| (a.id.clone(), base.image(&a.id).unwrap().clone()))
        .collect();
    let lib = Library::from_parts(cat, images).unwrap();
    let mut l = TacticalLayout::new("turned", 30, 8, "grass");
    bridge(&mut l, &AssetRef::Id(DECK.into()), 1, 3, 8);
    let mut used = BTreeSet::new();
    for seed in 0..40 {
        let takes = ids(&lib, &l, seed, 0..8);
        assert_eq!(takes.len(), 1);
        used.extend(takes);
    }
    assert!(!used.contains(&format!("{DECK}.alt1")), "{used:?}");
    assert_eq!(used.len(), 3, "{used:?}");
}

#[test]
fn structure_picks_are_deterministic_and_resolve_agrees() {
    let lib = with_takes(&[DECK, "prop.barrel"], 3, &[]);
    let mut l = TacticalLayout::new("mixed", 30, 10, "grass");
    let deck = AssetRef::Id(DECK.into());
    bridge(&mut l, &deck, 1, 2, 6);
    bridge(&mut l, &deck, 3, 6, 5);
    for x in 0..30 {
        put(
            &mut l,
            AssetRef::Id("prop.barrel".into()),
            x as f32 + 0.5,
            9.5,
        );
    }
    assert_eq!(picks(&lib, &l, 7), picks(&lib, &l, 7));
    let all = resolve_all(&lib, &l, 7);
    for (i, r) in all.iter().enumerate() {
        let one = resolve(&lib, &l, i, 7).unwrap();
        assert_eq!(r.unwrap().asset.id, one.asset.id, "placement {i}");
    }
    // Barrels are not floor structures: their row still varies.
    assert!(ids(&lib, &l, 7, 11..41).len() > 1);
    let a = render(&l, &lib, 7, &plain(16)).unwrap();
    let b = render(&l, &lib, 7, &plain(16)).unwrap();
    assert_eq!(blake3::hash(&a.data), blake3::hash(&b.data));
}

#[test]
fn a_whole_structure_in_two_windows_draws_the_same_take() {
    let lib = with_takes(&[DECK, "prop.barrel"], 3, &[]);
    let mut big = TacticalLayout::new("world", 36, 10, "grass");
    for y in [0u32, 9] {
        for x in 0..36 {
            put(
                &mut big,
                AssetRef::Id("prop.barrel".into()),
                x as f32 + 0.5,
                y as f32 + 0.5,
            );
        }
    }
    // Columns 13..23: inside both windows below, which overlap on 12..24.
    bridge(&mut big, &AssetRef::Id(DECK.into()), 13, 4, 5);
    let at = |l: &TacticalLayout, seed: u64, dx: f32| -> BTreeSet<String> {
        l.placements
            .iter()
            .zip(picks(&lib, l, seed))
            .filter(|(p, _)| (p.y - 4.5).abs() < 0.01 && (p.x + dx) > 12.0)
            .map(|(_, r)| r.0)
            .collect()
    };
    let mut seen = BTreeSet::new();
    for seed in 0..24 {
        let a = window(&big, 0, 24);
        let b = window(&big, 12, 24);
        let (ta, tb) = (at(&a, seed, 0.0), at(&b, seed, 12.0));
        assert_eq!(ta.len(), 1, "{ta:?}");
        assert_eq!(ta, tb, "seed {seed}");
        seen.extend(ta);
    }
    assert!(seen.len() > 1, "{seen:?}");
}

#[test]
fn placeholder_structures_resolve_as_before() {
    // The placeholder has no takes, so no placement joins a structure and
    // `resolve_all` is `resolve` placement by placement.
    let lib = placeholder();
    for mut l in layouts::all() {
        for origin in [None, Some([77, -5])] {
            l.origin = origin;
            let all = resolve_all(&lib, &l, 11);
            for (i, r) in all.iter().enumerate() {
                let one = resolve(&lib, &l, i, 11).unwrap();
                let r = r.unwrap();
                assert_eq!(
                    (r.asset.id.as_str(), r.turns, r.mirror),
                    (one.asset.id.as_str(), one.turns, one.mirror)
                );
            }
        }
    }
}
