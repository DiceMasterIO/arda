//! Same inputs, same names: across calls, clones and serde round trips.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_names::{
    person_name, place_name, FamilyCtx, Language, Meaning, NameScope, PlaceKind, PlaceSpec, Preset,
    Sex, SiteTag,
};

fn sample(lang: &Language) -> Vec<String> {
    let mut out = Vec::new();
    for key in 0..200_u64 {
        let spec = PlaceSpec::new(PlaceKind::Village, key)
            .tags(&[SiteTag::Ford])
            .features(&[Meaning::Oak]);
        let n = place_name(lang, &spec);
        out.push(format!(
            "{}|{}|{}|{}",
            n.native, n.gloss, n.literal, n.pronunciation
        ));
        let sex = if key % 2 == 0 { Sex::Female } else { Sex::Male };
        let p = person_name(lang, sex, &FamilyCtx::new(key));
        out.push(format!("{}|{}", p.full, p.gloss));
    }
    out
}

#[test]
fn same_seed_same_language_and_names() {
    for p in Preset::ALL {
        let a = Language::new(99, p);
        let b = Language::new(99, p);
        assert_eq!(a, b, "{p:?}");
        assert_eq!(sample(&a), sample(&b), "{p:?}");
    }
}

#[test]
fn different_seeds_differ() {
    for p in Preset::ALL {
        let a = Language::new(1, p);
        let b = Language::new(2, p);
        assert_ne!(a.lexicon, b.lexicon, "{p:?}");
    }
}

#[test]
fn serde_round_trip_preserves_everything() {
    for p in Preset::ALL {
        let lang = Language::new(7, p);
        let json = serde_json::to_string(&lang).unwrap();
        let back: Language = serde_json::from_str(&json).unwrap();
        assert_eq!(lang, back);
        assert_eq!(sample(&lang), sample(&back));
        let name = place_name(&lang, &PlaceSpec::new(PlaceKind::Town, 3));
        let n2: arda_names::Name =
            serde_json::from_str(&serde_json::to_string(&name).unwrap()).unwrap();
        assert_eq!(name, n2);
        let mut scope = NameScope::new();
        scope
            .place_name(&lang, &PlaceSpec::new(PlaceKind::City, 1))
            .unwrap();
        let s2: NameScope = serde_json::from_str(&serde_json::to_string(&scope).unwrap()).unwrap();
        assert_eq!(scope, s2);
    }
}

#[test]
fn golden_names_are_stable() {
    // Pinned output: a change here means every saved world renames itself.
    let lang = Language::new(2026, Preset::Heartland);
    let n = place_name(
        &lang,
        &PlaceSpec::new(PlaceKind::Village, 1)
            .tags(&[SiteTag::Ford])
            .features(&[Meaning::Oak]),
    );
    let p = person_name(&lang, Sex::Female, &FamilyCtx::new(1));
    let got = format!("{} / {} / {}", n.native, n.gloss, p.full);
    assert_eq!(got, GOLDEN);
}

const GOLDEN: &str = "Erena / Oakford / Teemin Tarnero";

#[test]
fn readme_example_holds() {
    let lang = Language::new(2026, Preset::Heartland);
    let spec = PlaceSpec::new(PlaceKind::Village, 1)
        .tags(&[SiteTag::Ford])
        .features(&[Meaning::Oak]);
    let n = place_name(&lang, &spec);
    let p = person_name(&lang, Sex::Female, &FamilyCtx::new(1));
    assert_eq!(
        (
            n.native.as_str(),
            n.gloss.as_str(),
            n.literal.as_str(),
            n.pronunciation.as_str(),
            p.full.as_str(),
            p.gloss.as_str()
        ),
        (
            "Erena",
            "Oakford",
            "ford of the oaks",
            "E-re-na",
            "Teemin Tarnero",
            "Teemin, daughter of Tarner"
        )
    );
}
