//! 100 000 names in under a second.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_names::{
    person_name, place_name, FamilyCtx, Language, PlaceKind, PlaceSpec, Preset, Sex, SiteTag,
};
use std::time::Instant;

/// Wall-clock bound, so a release gate (shared CI runners are several
/// times slower): `cargo test --release -p arda-names --test performance -- --ignored`.
#[test]
#[ignore = "wall-clock timing; release gate"]
fn hundred_thousand_names_under_a_second() {
    let langs: Vec<Language> = Preset::ALL.iter().map(|&p| Language::new(77, p)).collect();
    let tags = [
        SiteTag::Ford,
        SiteTag::Harbour,
        SiteTag::Forest,
        SiteTag::Hill,
    ];
    let start = Instant::now();
    let mut chars = 0_usize;
    for i in 0..100_000_u64 {
        let lang = &langs[(i % 13) as usize];
        if i % 2 == 0 {
            let spec = PlaceSpec::new(PlaceKind::Village, i).tags(&[tags[(i % 4) as usize]]);
            chars += place_name(lang, &spec).native.len();
        } else {
            let sex = if i % 4 == 1 { Sex::Female } else { Sex::Male };
            chars += person_name(lang, sex, &FamilyCtx::new(i)).full.len();
        }
    }
    let took = start.elapsed();
    assert!(chars > 0);
    eprintln!("100k names in {took:?}");
    assert!(took.as_secs_f64() < 1.0, "took {took:?}");
}
