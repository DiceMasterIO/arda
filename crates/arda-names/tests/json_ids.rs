//! u64 keys, ids and seeds travel as JSON strings (I5, I17), and numbers
//! are still read.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_names::{FamilyCtx, Language, PlaceKind, PlaceSpec, Preset};

#[test]
fn u64_keys_ids_and_seeds_are_json_strings() {
    let spec = PlaceSpec::new(PlaceKind::Village, u64::MAX);
    let j = serde_json::to_value(&spec).unwrap();
    assert_eq!(j["key"], "18446744073709551615");
    let back: PlaceSpec = serde_json::from_value(j).unwrap();
    assert_eq!(back, spec);

    let fam = FamilyCtx::new(u64::MAX);
    let j = serde_json::to_value(&fam).unwrap();
    assert_eq!(j["id"], "18446744073709551615");

    let lang = Language::new(u64::MAX, Preset::Heartland);
    let j = serde_json::to_value(&lang).unwrap();
    assert_eq!(j["seed"], "18446744073709551615");
    let back: Language = serde_json::from_value(j).unwrap();
    assert_eq!(back.seed, u64::MAX);

    let mut j = serde_json::to_value(PlaceSpec::new(PlaceKind::Village, 7)).unwrap();
    j["key"] = 7.into();
    let n: PlaceSpec = serde_json::from_value(j).unwrap();
    assert_eq!(n.key, 7);
}
