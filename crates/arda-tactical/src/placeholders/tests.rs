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

#[test]
fn effect_and_damage_art_is_in_the_library() {
    let (catalog, images) = generate(DEFAULT_SEED);
    for name in effects::TEXTURES.iter().chain(&effects::DECALS) {
        for v in 0..effects::VARIANTS {
            let id = format!("effect.{name}.{v}");
            let a = catalog.assets.iter().find(|a| a.id == id);
            assert!(a.is_some_and(|a| a.class == AssetClass::Effect), "{id}");
            assert!(images.contains_key(&id), "{id} image");
        }
    }
    for id in ["prop.rubble_heap", "prop.broken_deck", "prop.charred_floor"] {
        let a = catalog.assets.iter().find(|a| a.id == id);
        assert!(
            a.is_some_and(|a| a.tags.free.iter().any(|t| t == "damage")),
            "{id}"
        );
    }
}

/// Effects are never drawn by the compositor: with the effect and damage
/// props removed (version held), every built-in layout renders byte for byte.
#[test]
fn compositor_ignores_effect_art() {
    use crate::{layouts, render, RenderOptions};
    let (catalog, images) = generate(DEFAULT_SEED);
    let full = Library::from_parts(catalog.clone(), images.clone()).unwrap();
    let mut cat = catalog;
    cat.assets
        .retain(|a| a.class != AssetClass::Effect && !a.tags.free.iter().any(|t| t == "damage"));
    let bare = Library::from_parts(cat, images).unwrap();
    let opts = RenderOptions {
        ppsq: 32,
        grid: true,
        lighting: true,
    };
    for name in ["riverside", "stone_warehouse", "forest_glade", "marsh"] {
        let Some(layout) = layouts::by_name(name) else {
            panic!("no layout {name}");
        };
        let a = render(&layout, &full, 5, &opts);
        let b = render(&layout, &bare, 5, &opts);
        match (a, b) {
            (Ok(a), Ok(b)) => assert_eq!(a.data, b.data, "{name}"),
            (a, b) => panic!("{name}: {:?} {:?}", a.err(), b.err()),
        }
    }
}
