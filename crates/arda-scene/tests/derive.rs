//! Review round 2 #39: the public per-square derivation checks its inputs,
//! and a sidecar that clears a canopy's heavy obscurement leaves the
//! foliage's light obscurement.
#![allow(clippy::unwrap_used, missing_docs)]

mod common;

use arda_scene::squares::{derive, Placed};
use arda_scene::{Obscurement, RulesSidecar, SceneError};
use common::{lib, L};

#[test]
fn derive_refuses_a_layout_whose_squares_do_not_fill_it() {
    let mut l = L::new(4, 4);
    l.0.squares.truncate(5);
    let err = derive(&l.0, &[], None).err().unwrap();
    assert!(matches!(err, SceneError::Schema(_)), "{err}");
    let ok = L::new(4, 4);
    assert!(matches!(
        derive(&ok.0, &[], Some(&RulesSidecar::empty(3, 4))),
        Err(SceneError::Schema(_) | SceneError::Sidecar(_))
    ));
    assert!(derive(&ok.0, &[], Some(&RulesSidecar::empty(4, 4))).is_ok());
}

#[test]
fn a_sidecar_clearing_a_dense_canopy_leaves_it_lightly_obscured() {
    let mut dense = lib().asset("veg.tree_oak").unwrap().clone();
    dense.blocks_sight = true;
    let tree = Placed {
        asset: &dense,
        rect: [1.0, 0.0, 3.0, 2.0],
        at: (2.0, 1.0),
    };
    let l = L::new(4, 2);
    let heavy = derive(&l.0, std::slice::from_ref(&tree), None).unwrap();
    assert_eq!(heavy.obscured.0[1], Obscurement::Heavy);
    let mut r = RulesSidecar::empty(4, 2);
    r.cell_mut(1, 0).unwrap().blocks_sight = Some(false);
    let cleared = derive(&l.0, std::slice::from_ref(&tree), Some(&r)).unwrap();
    assert_eq!(
        cleared.obscured.0[1],
        Obscurement::Light,
        "the foliage stays"
    );
    // A square with no canopy over it clears entirely.
    r.cell_mut(0, 0).unwrap().blocks_sight = Some(true);
    let walled = derive(&l.0, &[], Some(&r)).unwrap();
    assert_eq!(walled.obscured.0[0], Obscurement::Heavy);
    r.cell_mut(0, 0).unwrap().blocks_sight = Some(false);
    let open = derive(&l.0, &[], Some(&r)).unwrap();
    assert_eq!(open.obscured.0[0], Obscurement::Clear);
}
