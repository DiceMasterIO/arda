//! Names are unique within a scope, and the blocklist keeps words out.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_names::scope::skeleton;
use arda_names::{
    place_name, Blocklist, Language, NameScope, Options, PlaceKind, PlaceSpec, Preset, SiteTag,
};
use std::collections::BTreeSet;

#[test]
fn a_scope_never_repeats_a_name() {
    let tags = [
        SiteTag::Ford,
        SiteTag::Hill,
        SiteTag::Forest,
        SiteTag::Coast,
        SiteTag::Arable,
    ];
    for p in Preset::ALL {
        let lang = Language::new(5, p);
        let mut scope = NameScope::new();
        let mut seen = BTreeSet::new();
        for key in 0..1500_u64 {
            let kind = if key % 10 == 0 {
                PlaceKind::Town
            } else {
                PlaceKind::Hamlet
            };
            let spec = PlaceSpec::new(kind, key).tags(&[tags[(key % 5) as usize]]);
            let n = scope.place_name(&lang, &spec);
            assert!(
                seen.insert(skeleton(&n.native)),
                "{p:?} repeated {}",
                n.native
            );
        }
        assert_eq!(scope.len(), 1500);
    }
}

#[test]
fn identical_specs_are_told_apart() {
    let lang = Language::new(8, Preset::Highland);
    let mut scope = NameScope::new();
    let spec = PlaceSpec::new(PlaceKind::Village, 1).tags(&[SiteTag::Ford]);
    let a = scope.place_name(&lang, &spec);
    let b = scope.place_name(&lang, &spec);
    assert_eq!(a, place_name(&lang, &spec));
    assert_ne!(skeleton(&a.native), skeleton(&b.native));
}

#[test]
fn extending_the_blocklist_removes_a_name() {
    let lang = Language::new(21, Preset::Southern);
    let spec = PlaceSpec::new(PlaceKind::Town, 4).tags(&[SiteTag::Bridge]);
    let first = place_name(&lang, &spec);
    let mut list = Blocklist::builtin().clone();
    list.add_word(&first.native);
    let opts = Options {
        blocklist: Some(list),
        ..Options::default()
    };
    let strict = Language::with_options(21, Preset::Southern, &opts);
    let second = place_name(&strict, &spec);
    assert_ne!(first.native, second.native);
    assert!(strict.blocklist.blocks(&first.native));
}

/// Names `count` places of `kind` in one scope; every name must be new.
fn crowd(lang: &Language, kind: PlaceKind, count: u64) -> Vec<String> {
    let mut scope = NameScope::new();
    let mut seen = BTreeSet::new();
    (0..count)
        .map(|key| {
            let n = scope.place_name(lang, &PlaceSpec::new(kind, key));
            // Numbered names share a skeleton by design ("II", "III").
            let plain = !n.native.rsplit(' ').next().is_some_and(|t| {
                t.len() < n.native.len() && t.chars().all(|c| "IVXLCDM".contains(c))
            });
            let key = if plain {
                skeleton(&n.native)
            } else {
                n.native.clone()
            };
            assert!(seen.insert(key), "repeated {}", n.native);
            assert!(!n.native.is_empty() && !n.gloss.is_empty());
            n.native
        })
        .collect()
}

#[test]
fn a_crowded_scope_falls_back_instead_of_refusing() {
    // Streams draw from a few dozen plain names; a default-size world has
    // thousands in one scope (OI-49). Past the plain names the scope names
    // them after people ("Haldor's Brook"), and past those it numbers them,
    // never refusing and never repeating.
    let lang = Language::new(5, Preset::Heartland);
    let streams = crowd(&lang, PlaceKind::Stream, 4000);
    let numbered = streams.iter().filter(|n| n.ends_with(" II")).count();
    assert!(
        numbered * 100 <= streams.len(),
        "{numbered} numbered streams"
    );
    assert_eq!(
        streams,
        crowd(&lang, PlaceKind::Stream, 4000),
        "deterministic"
    );
    for kind in [PlaceKind::Pass, PlaceKind::Mountain, PlaceKind::Lake] {
        crowd(&lang, kind, 1000);
    }
}

#[test]
fn derived_names_that_collide_are_numbered() {
    // A realm's name comes from its capital's alone, so every redraw gives
    // the same name: the scope must still hand out distinct ones.
    let lang = Language::new(9, Preset::Coastal);
    let capital = place_name(&lang, &PlaceSpec::new(PlaceKind::City, 1));
    let mut scope = NameScope::new();
    let mut seen = BTreeSet::new();
    for key in 0..40_u64 {
        let spec = PlaceSpec::new(PlaceKind::Realm, key).from_name(&capital);
        let n = scope.place_name(&lang, &spec);
        assert!(seen.insert(n.native.clone()), "repeated {}", n.native);
    }
}
