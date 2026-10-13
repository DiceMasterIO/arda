//! Natural-feature kinds (sea to marsh) name uniquely, deterministically
//! and past the blocklist in every preset.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_names::scope::skeleton;
use arda_names::{place_name, Language, NameScope, PlaceKind, PlaceSpec, Preset};
use std::collections::BTreeSet;

const KINDS: [PlaceKind; 10] = [
    PlaceKind::Sea,
    PlaceKind::Bay,
    PlaceKind::Lake,
    PlaceKind::Island,
    PlaceKind::Cape,
    PlaceKind::Forest,
    PlaceKind::Marsh,
    PlaceKind::Mountain,
    PlaceKind::Vale,
    PlaceKind::Hill,
];

#[test]
fn feature_kinds_are_deterministic_and_unique() {
    for p in Preset::ALL {
        let lang = Language::new(11, p);
        let mut scope = NameScope::new();
        let mut seen = BTreeSet::new();
        for key in 0..16_u64 {
            for kind in KINDS {
                let spec = PlaceSpec::new(kind, key);
                assert_eq!(place_name(&lang, &spec), place_name(&lang, &spec));
                let n = scope.place_name(&lang, &spec).unwrap();
                assert!(!n.native.is_empty(), "{p:?} {kind:?}");
                assert!(
                    seen.insert(skeleton(&n.native)),
                    "{p:?} {kind:?} repeated {}",
                    n.native
                );
            }
        }
    }
}

#[test]
fn sea_keys_parse() {
    assert_eq!(PlaceKind::from_key("sea"), Some(PlaceKind::Sea));
    assert_eq!(PlaceKind::from_key("gulf"), Some(PlaceKind::Sea));
}

#[test]
fn existing_kinds_keep_the_numbers_they_seed_with() {
    // Names draw from `kind as u64`; adding a kind must not move these.
    assert_eq!(PlaceKind::Pass as u64, 15);
    assert_eq!(PlaceKind::Bay as u64, 16);
    assert_eq!(PlaceKind::Region as u64, 20);
    assert_eq!(PlaceKind::Realm as u64, 21);
    assert_eq!(PlaceKind::Sea as u64, 22);
}
