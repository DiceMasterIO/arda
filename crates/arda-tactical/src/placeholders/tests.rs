//! Placeholder library tests (goal 61).

use super::*;
use crate::library::Library;

#[test]
fn placeholders_pass_the_validator() {
    let (catalog, images) = generate(DEFAULT_SEED);
    let props = catalog
        .assets
        .iter()
        .filter(|a| a.class == AssetClass::Prop)
        .count();
    assert!(props >= 15, "{props} props");
    assert!(catalog.assets.iter().all(|a| a.licence == LICENCE));
    if let Err(e) = Library::from_parts(catalog, images) {
        panic!("{e}");
    }
}

#[test]
fn placeholders_are_deterministic() {
    let (a, ai) = generate(DEFAULT_SEED);
    let (b, bi) = generate(DEFAULT_SEED);
    assert_eq!(a, b);
    assert_eq!(ai, bi);
}

#[test]
fn committed_library_matches_the_generator() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
    let lib = match Library::load(&dir) {
        Ok(lib) => lib,
        Err(e) => panic!("committed placeholder library: {e}"),
    };
    let (catalog, images) = generate(DEFAULT_SEED);
    assert_eq!(
        lib.catalog, catalog,
        "regenerate with `arda tactical placeholders`"
    );
    for (id, img) in &images {
        assert_eq!(lib.image(id), Some(img), "{id} differs from the generator");
    }
}
