//! Every enum serialises as the snake_case key the rest of Arda uses
//! (review round 2, #26; like I6 and I7): presets as their `key()`,
//! meanings as their lexicon key, and settle's site-tag strings
//! (`bridge_site`) deserialise straight into `SiteTag`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_names::meaning::ALL;
use arda_names::{FamilyStyle, PlaceKind, Preset, Sex, SiteTag};

fn json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap()
}

#[test]
fn enums_serialise_as_snake_case_keys() {
    assert_eq!(json(&PlaceKind::Village), "\"village\"");
    assert_eq!(json(&Sex::Female), "\"female\"");
    assert_eq!(json(&FamilyStyle::Patronymic), "\"patronymic\"");
    for p in Preset::ALL {
        assert_eq!(json(&p), format!("\"{}\"", p.key()), "{p:?}");
        let back: Preset = serde_json::from_str(&json(&p)).unwrap();
        assert_eq!(back, p);
    }
    for m in ALL {
        let key = json(m);
        assert!(
            key.chars()
                .all(|c| c == '"' || c == '_' || c.is_ascii_lowercase() || c.is_ascii_digit()),
            "{key}"
        );
        let from = arda_names::Meaning::from_key(key.trim_matches('"'));
        assert_eq!(from, Some(*m), "{key}");
    }
}

#[test]
fn settle_site_tag_strings_deserialise_directly() {
    let keys = [
        "ford",
        "bridge_site",
        "harbour",
        "estuary",
        "ore",
        "river",
        "arable",
    ];
    for k in keys {
        let tag: SiteTag = serde_json::from_str(&format!("\"{k}\"")).unwrap();
        assert_eq!(SiteTag::from_key(k), Some(tag), "{k}");
        assert_eq!(json(&tag), format!("\"{k}\""));
    }
}
