//! Prints 20 place names and 20 person names per preset, with glosses.
//!
//! `cargo run -p arda-names --example showcase [seed]`

use arda_names::{
    person_name, FamilyCtx, Language, Meaning, NameScope, PlaceKind as K, PlaceSpec, Preset, Sex,
    SiteTag as T,
};

fn specs() -> Vec<PlaceSpec> {
    let s = |kind, key, tags: &[T], feats: &[Meaning]| {
        PlaceSpec::new(kind, key).tags(tags).features(feats)
    };
    vec![
        s(K::Village, 1, &[T::Ford, T::River], &[Meaning::Oak]),
        s(K::Town, 2, &[T::Bridge, T::River, T::Navigable], &[]),
        s(K::Port, 3, &[T::Harbour, T::Coast], &[]),
        s(K::Hamlet, 4, &[T::Forest, T::Timber], &[]),
        s(K::Village, 5, &[T::Marsh], &[]),
        s(K::City, 6, &[T::Estuary, T::Coast, T::Navigable], &[]),
        s(K::Fort, 7, &[T::Defensible, T::Hill], &[]),
        s(K::Abbey, 8, &[T::Spring], &[]),
        s(K::Mine, 9, &[T::Ore, T::Mountain], &[]),
        s(K::Village, 10, &[T::Confluence, T::River], &[]),
        s(K::Hamlet, 11, &[T::Arable], &[]),
        s(K::Town, 12, &[T::Pass, T::Mountain], &[]),
        s(K::Village, 13, &[T::Lake, T::Fish], &[]),
        s(K::River, 14, &[], &[]),
        s(K::River, 15, &[], &[]),
        s(K::Stream, 16, &[], &[]),
        s(K::Mountain, 17, &[], &[]),
        s(K::Forest, 18, &[], &[]),
        s(K::Lake, 19, &[], &[]),
    ]
}

fn main() {
    let seed = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2026_u64);
    for preset in Preset::ALL {
        let lang = Language::new(seed, preset);
        println!("== {} ==", preset.key());
        let mut scope = NameScope::new();
        let mut capital = None;
        for spec in specs() {
            let Ok(n) = scope.place_name(&lang, &spec) else {
                continue;
            };
            println!(
                "  {:<9} {:<22} {:<18} {:<26} [{}]",
                format!("{:?}", spec.kind),
                n.native,
                n.gloss,
                n.literal,
                n.pronunciation
            );
            if spec.kind == K::City {
                capital = Some(n);
            }
        }
        if let Some(cap) = &capital {
            let (Ok(region), Ok(realm)) = (
                scope.place_name(&lang, &PlaceSpec::new(K::Region, 100).from_name(cap)),
                scope.place_name(&lang, &PlaceSpec::new(K::Realm, 101).from_name(cap)),
            ) else {
                continue;
            };
            println!(
                "  {:<9} {:<22} {:<18} {}",
                "Region", region.native, region.gloss, region.literal
            );
            println!(
                "  {:<9} {:<22} {:<18} {}",
                "Realm", realm.native, realm.gloss, realm.literal
            );
        }
        println!("  -- people --");
        for id in 0..20_u64 {
            let sex = if id % 2 == 0 { Sex::Female } else { Sex::Male };
            let p = person_name(&lang, sex, &FamilyCtx::new(id));
            let by = p
                .byname
                .as_ref()
                .map_or(String::new(), |b| format!(" \"{}\"", b.native));
            println!("  {:<34} {}", format!("{}{by}", p.full), p.gloss);
        }
        println!();
    }
}
