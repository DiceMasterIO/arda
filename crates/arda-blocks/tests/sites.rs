//! Site entrances on the surface map (logic/18 §site-entrance): drawn on
//! clear passable ground, the same in every window, and absent from
//! windows that hold none.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

use arda_blocks::sites::{Entrance, SiteOverlays, CLEAR};
use arda_blocks::{Owner, Pipeline};
use arda_refine::synthetic::{self, GRASS_CELL};
use arda_tactical::layout::AssetRef;
use arda_tactical::{Library, TacticalLayout};
use std::path::Path;
use std::sync::OnceLock;

fn lib() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
        Library::load(&dir).unwrap()
    })
}

fn pipeline() -> Pipeline {
    Pipeline::new(Box::new(synthetic::world(42)), 64)
}

fn has(l: &TacticalLayout, id: &str) -> bool {
    l.placements
        .iter()
        .any(|p| matches!(&p.asset, AssetRef::Id(i) if i == id))
}

/// The grass cell's window and its centre square in world squares.
fn grass() -> ((i64, i64, u32, u32), (i64, i64)) {
    let (x0, y0) = (GRASS_CELL.x * 64, GRASS_CELL.y * 64);
    ((x0, y0, 64, 64), (x0 + 32, y0 + 32))
}

#[test]
fn an_entrance_is_drawn_on_clear_passable_ground() {
    let (window, (gsx, gsy)) = grass();
    for cave in [false, true] {
        let o = SiteOverlays::new(None, vec![Entrance { gsx, gsy, cave }]);
        let c = pipeline().window(window, lib(), Some(&o)).unwrap();
        assert!(c.overlays.contains(&"site"), "{:?}", c.overlays);
        let prop = if cave {
            "prop.cave_mouth"
        } else {
            "prop.site_entrance"
        };
        assert!(has(&c.layout, prop), "{prop}");
        for dy in -CLEAR..=CLEAR {
            for dx in -CLEAR..=CLEAR {
                let i = ((32 + dy) * 64 + 32 + dx) as usize;
                assert_eq!(c.owners[i], Owner::Site);
                assert_eq!(c.layout.squares[i].water_depth_ft, 0);
                let r = &c.rules.squares[i];
                assert_ne!(r.blocks_movement, Some(true), "{dx},{dy}");
                assert_ne!(r.difficult, Some(true), "{dx},{dy}");
            }
        }
    }
}

#[test]
fn a_window_without_an_entrance_is_unchanged() {
    let (window, _) = grass();
    let far = Entrance {
        gsx: 9000,
        gsy: 9000,
        cave: false,
    };
    let o = SiteOverlays::new(None, vec![far]);
    let with = pipeline().window(window, lib(), Some(&o)).unwrap();
    let without = pipeline().window(window, lib(), None).unwrap();
    assert_eq!(with.layout, without.layout);
    assert_eq!(with.rules, without.rules);
    assert_eq!(with.owners, without.owners);
}

#[test]
fn overlapping_windows_agree_on_the_entrance() {
    let ((x0, y0, ..), (gsx, gsy)) = grass();
    let o = SiteOverlays::new(
        None,
        vec![Entrance {
            gsx,
            gsy,
            cave: true,
        }],
    );
    let a = pipeline()
        .window((x0, y0, 64, 64), lib(), Some(&o))
        .unwrap();
    let b = pipeline()
        .window((x0 + 16, y0 + 16, 40, 40), lib(), Some(&o))
        .unwrap();
    for y in 0..40_i64 {
        for x in 0..40_i64 {
            let (ia, ib) = (((y + 16) * 64 + x + 16) as usize, (y * 40 + x) as usize);
            assert_eq!(a.layout.squares[ia], b.layout.squares[ib], "{x},{y}");
            assert_eq!(a.owners[ia], b.owners[ib], "{x},{y}");
        }
    }
    assert!(has(&b.layout, "prop.cave_mouth"));
}
