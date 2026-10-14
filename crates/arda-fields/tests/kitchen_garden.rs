//! The farmstead's kitchen-garden plot (1.0 task 17): beds, poles, a cold
//! frame and clutter on its farmland, all known to the placeholder library.

#![allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use arda_fields::generate;
use arda_fields::supplement::supplemented_library;
use arda_fields::synthetic;
use arda_tactical::layout::AssetRef;

#[test]
fn farmsteads_carry_a_kitchen_garden_the_library_knows() {
    let s = synthetic::orchard_farmstead();
    let win = generate(&s.inputs(), s.origin_m, 256, 256, 11).unwrap();
    let count = |want: &str| {
        win.layout
            .placements
            .iter()
            .filter(|q| matches!(&q.asset, AssetRef::Id(id) if id == want))
            .count()
    };
    assert!(count("veg.vegetable_bed") >= 7, "vegetable beds");
    assert!(count("veg.herb_bed") >= 2, "herb beds");
    assert!(count("veg.bean_poles") >= 2, "bean poles");
    for id in [
        "prop.cold_frame",
        "prop.compost_heap",
        "prop.rain_barrel",
        "prop.garden_tools",
    ] {
        assert!(count(id) >= 1, "{id}");
    }
    let (lib, _) = supplemented_library().unwrap();
    win.layout.check(&lib).unwrap();
}

/// Every plot piece stands on farmland.
#[test]
fn garden_pieces_stand_on_farmland() {
    let s = synthetic::orchard_farmstead();
    let win = generate(&s.inputs(), s.origin_m, 256, 256, 11).unwrap();
    let l = &win.layout;
    for q in &l.placements {
        let AssetRef::Id(id) = &q.asset else { continue };
        if !id.starts_with("veg.vegetable_bed") && !id.starts_with("veg.herb_bed") {
            continue;
        }
        let i = (q.y.floor() as usize) * l.width as usize + q.x.floor() as usize;
        assert_eq!(l.squares[i].ground, "farmland", "{id} at {},{}", q.x, q.y);
    }
}
