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
            let n = scope.place_name(&lang, &spec).unwrap();
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
    let a = scope.place_name(&lang, &spec).unwrap();
    let b = scope.place_name(&lang, &spec).unwrap();
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

#[test]
fn an_exhausted_scope_refuses_instead_of_repeating() {
    // Streams draw from a few dozen names; a world has thousands. Once the
    // space runs out the scope must say so, never return a taken name.
    let lang = Language::new(5, Preset::Heartland);
    let mut scope = NameScope::new();
    let mut seen = BTreeSet::new();
    let mut refused = 0;
    for key in 0..400_u64 {
        match scope.place_name(&lang, &PlaceSpec::new(PlaceKind::Stream, key)) {
            Ok(n) => assert!(seen.insert(skeleton(&n.native)), "repeated {}", n.native),
            Err(e) => {
                assert!(matches!(e, arda_names::NamesError::ScopeExhausted { .. }));
                refused += 1;
            }
        }
    }
    assert!(refused > 0, "400 streams should exhaust the stream names");
}
