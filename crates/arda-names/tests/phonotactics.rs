//! Every generated word obeys its language's phonotactics and is spellable.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_names::meaning::ALL;
use arda_names::{
    person_name, place_name, Blocklist, FamilyCtx, Language, PlaceKind, PlaceSpec, Preset, Sex,
    SiteTag,
};

const KINDS: [PlaceKind; 12] = [
    PlaceKind::Hamlet,
    PlaceKind::Village,
    PlaceKind::Town,
    PlaceKind::City,
    PlaceKind::Fort,
    PlaceKind::Port,
    PlaceKind::River,
    PlaceKind::Stream,
    PlaceKind::Mountain,
    PlaceKind::Lake,
    PlaceKind::Forest,
    PlaceKind::Pass,
];
const TAGS: [SiteTag; 8] = [
    SiteTag::Ford,
    SiteTag::Harbour,
    SiteTag::Estuary,
    SiteTag::Forest,
    SiteTag::Marsh,
    SiteTag::Hill,
    SiteTag::Confluence,
    SiteTag::Ore,
];

/// Spellable: capitalised words of lowercase ASCII letters, apostrophes
/// only between letters, no letter three times in a row.
fn spellable(s: &str) -> bool {
    s.split(' ').all(|w| {
        let b = w.as_bytes();
        let ok_chars = w.chars().all(|c| c.is_ascii_alphabetic() || c == '\'');
        let apostrophes_inside = !w.starts_with('\'') && !w.ends_with('\'') && !w.contains("''");
        let no_triples = b
            .windows(3)
            .all(|t| !(t[0].eq_ignore_ascii_case(&t[1]) && t[1].eq_ignore_ascii_case(&t[2])));
        !w.is_empty() && ok_chars && apostrophes_inside && no_triples
    })
}

/// Pronounceable anywhere: a vowel in every word and no more than three
/// consonant letters' worth of consonants in a row.
fn pronounceable(lang: &Language, w: &[arda_names::phoneme::Ph]) -> bool {
    let mut run = 0;
    for p in w {
        run = if p.is_vowel() { 0 } else { run + 1 };
        if run > 3 {
            return false;
        }
    }
    w.iter().any(|p| p.is_vowel()) && !lang.spell(w).is_empty()
}

#[test]
fn lexicon_roots_are_valid() {
    for p in Preset::ALL {
        for seed in 0..8 {
            let lang = Language::new(seed, p);
            for &m in ALL {
                let w = lang.root(m);
                if m.info().class == arda_names::meaning::Class::Affix {
                    assert!(lang.phonology.valid(w), "{p:?} {m:?} {}", lang.spell(w));
                } else {
                    assert!(
                        lang.phonology.valid_root(w),
                        "{p:?} {m:?} {}",
                        lang.spell(w)
                    );
                }
            }
            for w in &lang.name_roots {
                assert!(
                    lang.phonology.valid_root(w),
                    "{p:?} name root {}",
                    lang.spell(w)
                );
            }
        }
    }
}

#[test]
fn place_names_are_valid_spellable_and_clean() {
    let block = Blocklist::builtin();
    for p in Preset::ALL {
        for seed in 0..3 {
            let lang = Language::new(seed, p);
            for key in 0..400_u64 {
                let kind = KINDS[(key % 12) as usize];
                let tag = TAGS[(key / 12 % 8) as usize];
                let n = place_name(&lang, &PlaceSpec::new(kind, key).tags(&[tag]));
                assert!(
                    lang.phonology.valid(&n.word),
                    "{p:?} invalid {} ({:?})",
                    n.native,
                    n.word
                );
                assert!(spellable(&n.native), "{p:?} unspellable {}", n.native);
                assert!(pronounceable(&lang, &n.word), "{p:?} {}", n.native);
                assert!(!block.blocks(&n.native), "{p:?} blocked {}", n.native);
                assert!(!n.gloss.is_empty() && !n.literal.is_empty());
            }
        }
    }
}

#[test]
fn person_names_are_valid_and_spellable() {
    for p in Preset::ALL {
        let lang = Language::new(11, p);
        for id in 0..600_u64 {
            let sex = if id % 3 == 0 { Sex::Male } else { Sex::Female };
            let n = person_name(&lang, sex, &FamilyCtx::new(id));
            assert!(
                lang.phonology.valid(&n.given.word),
                "{p:?} given {}",
                n.given.native
            );
            assert!(spellable(&n.full), "{p:?} {}", n.full);
            if let Some(b) = &n.byname {
                assert!(spellable(&b.native), "{p:?} byname {}", b.native);
            }
        }
    }
}

#[test]
fn stress_lands_on_a_syllable() {
    for p in Preset::ALL {
        let lang = Language::new(3, p);
        let n = place_name(&lang, &PlaceSpec::new(PlaceKind::Town, 1));
        let caps = n
            .pronunciation
            .split('-')
            .filter(|s| s.chars().any(|c| c.is_ascii_uppercase()))
            .count();
        assert_eq!(caps, 1, "{p:?} {}", n.pronunciation);
    }
}
