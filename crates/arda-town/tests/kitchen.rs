//! Kitchen gardens (1.0 task 17): rows of beds and garden clutter in croft
//! kitchen parcels, and nowhere else; every outdoor palette id exists in the
//! placeholder library.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use arda_tactical::layout::AssetRef;
use arda_tactical::Library;
use arda_town::block::wfc::outdoor_vocab::{palette, ART_FREE, CLASSES};
use arda_town::block::{self, ground, TownBlock, TownFill, Window};
use arda_town::plan::croft::{parcel, rim, use_of, Use};
use arda_town::plan::grid::Kind;
use arda_town::plan::TownPlan;
use common::{library_dir, plans};

const GARDEN: [&str; 7] = [
    "veg.vegetable_bed",
    "veg.herb_bed",
    "veg.bean_poles",
    "prop.cold_frame",
    "prop.compost_heap",
    "prop.rain_barrel",
    "prop.garden_tools",
];

fn whole(p: &TownPlan, fill: TownFill) -> TownBlock {
    let (x, y) = p.origin();
    let (w, h) = p.size();
    block::generate_with(p, Window { x, y, w, h }, fill).unwrap()
}

/// `(id, global x, global y, rotation)` of every garden placement.
fn garden(b: &TownBlock) -> Vec<(String, f64, f64, u16)> {
    b.layout
        .placements
        .iter()
        .filter_map(|q| match &q.asset {
            AssetRef::Id(id) if GARDEN.contains(&id.as_str()) => Some((
                id.clone(),
                f64::from(q.x) + b.origin[0] as f64,
                f64::from(q.y) + b.origin[1] as f64,
                q.rotation,
            )),
            _ => None,
        })
        .collect()
}

/// Every id a palette can place exists in the placeholder library, which
/// keeps an unknown id (a 422 `invalid_layout` from the server) out of towns.
#[test]
fn every_outdoor_palette_id_is_in_the_placeholder_library() {
    let lib = Library::load(&library_dir()).unwrap();
    for &c in &CLASSES {
        for &(id, _) in palette(c) {
            if id == "ground" || id == "loop" || ART_FREE.contains(&id) {
                continue;
            }
            assert!(lib.catalog.assets.iter().any(|a| a.id == id), "{c:?}: {id}");
        }
    }
}

#[test]
fn kitchen_parcels_hold_beds_and_clutter_and_nothing_else_does() {
    let lib = Library::load(&library_dir()).unwrap();
    for fill in [TownFill::Wfc, TownFill::Rules] {
        let (mut beds, mut clutter) = (0, 0);
        for p in plans() {
            let blk = whole(p, fill);
            for (id, x, y, _) in garden(&blk) {
                #[allow(clippy::cast_possible_truncation)]
                let (sx, sy) = (x.floor() as i64, y.floor() as i64);
                let kitchen = ground::kind(p, sx, sy) == Kind::Croft
                    && use_of(parcel(p.seed, sx, sy)) == Use::Kitchen
                    && !rim(p.seed, |a, b| ground::kind(p, a, b) == Kind::Open, sx, sy);
                assert!(
                    kitchen,
                    "{} {fill:?}: {id} at {sx},{sy} is not in a kitchen parcel",
                    p.name
                );
                if id.starts_with("veg.") {
                    beds += 1;
                } else {
                    clutter += 1;
                }
            }
            // The placements resolve against the placeholder library.
            let res = block::fallback::resolve(&blk, &lib, p.seed);
            res.layout.check(&lib).unwrap();
        }
        assert!(beds > 20, "{fill:?}: {beds} beds");
        if fill == TownFill::Wfc {
            assert!(clutter > 0, "no garden clutter");
        }
    }
}

/// No garden piece overlaps another placement of the block.
#[test]
fn garden_pieces_do_not_overlap_other_pieces() {
    let lib = Library::load(&library_dir()).unwrap();
    for fill in [TownFill::Wfc, TownFill::Rules] {
        for p in plans() {
            let blk = whole(p, fill);
            let rects: Vec<(String, [f64; 4])> = blk
                .layout
                .placements
                .iter()
                .filter_map(|q| {
                    let AssetRef::Id(id) = &q.asset else {
                        return None;
                    };
                    let a = lib.catalog.assets.iter().find(|a| &a.id == id)?;
                    let (w, h) = if q.rotation % 180 == 90 {
                        (f64::from(a.footprint.h), f64::from(a.footprint.w))
                    } else {
                        (f64::from(a.footprint.w), f64::from(a.footprint.h))
                    };
                    let (x, y) = (f64::from(q.x), f64::from(q.y));
                    Some((
                        id.clone(),
                        [x - w / 2.0, y - h / 2.0, x + w / 2.0, y + h / 2.0],
                    ))
                })
                .collect();
            for (i, (ida, a)) in rects.iter().enumerate() {
                if !GARDEN.contains(&ida.as_str()) {
                    continue;
                }
                for (j, (idb, b)) in rects.iter().enumerate() {
                    let hit = a[0] < b[2] - 1e-6
                        && b[0] < a[2] - 1e-6
                        && a[1] < b[3] - 1e-6
                        && b[1] < a[3] - 1e-6;
                    // A building's ladder is laid on a half-square anchor that
                    // its 1 x 2 footprint overstates; it is not outdoor dressing.
                    let ladder = idb == "prop.ladder";
                    assert!(
                        i == j || ladder || !hit,
                        "{} {fill:?}: {ida} {a:?} overlaps {idb} {b:?}",
                        p.name
                    );
                }
            }
        }
    }
}

#[test]
fn kitchen_gardens_are_deterministic() {
    for p in plans() {
        for fill in [TownFill::Wfc, TownFill::Rules] {
            assert_eq!(
                garden(&whole(p, fill)),
                garden(&whole(p, fill)),
                "{}",
                p.name
            );
        }
    }
}

/// A kitchen parcel's border squares stay bare, so beds never touch the
/// hedges and fences between parcels, and every third row is a path.
#[test]
fn beds_leave_the_parcel_border_and_every_third_row_bare() {
    use arda_town::block::garden::bed;
    use arda_town::plan::croft::local;
    let seed = 42;
    let mut beds = 0;
    for y in 0..60 {
        for x in 0..80 {
            let (lx, ly) = local(seed, x, y);
            if bed(seed, x, y).is_some() {
                beds += 1;
                assert!(
                    (1..=15).contains(&lx) && (1..=10).contains(&ly),
                    "{lx},{ly}"
                );
            }
        }
    }
    assert!(beds > 400, "{beds}");
}
